//! S1d-4a gate — **one statement, two `Kernel`s, both stores.**
//!
//! The S1a–S1c gates compared the arena against a hand-written mirror of the
//! engine's write chain: two spellings, and the test's job was to find where
//! they drifted. This one is different in kind. `compute_effect`/`apply_effect`
//! are generic over `K: Kernel`, so BOTH sides here run the SAME executor code
//! over the same `Stmt`; the only thing that differs is which implementor
//! answers the kernel calls. A divergence therefore cannot come from a restated
//! rule — it can only come from `native/kernel.rs`, which is exactly the surface
//! this slice added.
//!
//! Three things are compared per statement, because the store alone does not see
//! all three:
//!
//! 1. **The store** — what the write landed.
//! 2. **The NBA queue** (`seq`, destination shape, sampled value, sampled
//!    offsets) — a `<=` writes NOTHING now, so a store-only comparison would
//!    pass on a kernel that dropped every nonblocking assign on the floor.
//! 3. **The EFFECT KIND** — which `StmtEffect` variant `compute_effect` built.
//!    This is the one the other two cannot see: if the two kernels answered a
//!    `k_*_rhs` predicate differently they would build different statements, and
//!    for the many statements whose two readings happen to write the same bits
//!    (a `$feof` rhs, say) both the store and the queue would agree while the
//!    backends had quietly disagreed about what the statement IS.

use std::collections::BTreeMap;

use sim_ir::{SimIr, Stmt};

use super::test_common as common;
use super::tests::{
    assert_stores_equal, build_with_opts, fresh_state, mirror_state, set_bit, NullSink,
};
use crate::exec::{apply_effect, compute_effect, Kernel, StmtEffect};
use crate::native::arena::NetArena;
use crate::native::kernel::NativeKernel;
use crate::sched::Scheduler;
use common::{corpus, Rng};

/// The destination an NBA entry names, flattened to comparable parts.
///
/// `NbaLhs`/`NbaUpdate` (`sched/mod.rs`) derive nothing — no `PartialEq`, no
/// `Debug` — so the first version of this gate compared `(seq, sampled, offsets)`
/// and silently omitted the destination, while its own module doc claimed
/// otherwise. That omission covered exactly the one hand-restated function in
/// `native/kernel.rs`: `k_schedule_nba_scalar` builds `NbaLhs::One(chunk)` by
/// hand, and a kernel queueing the right VALUE at the wrong DESTINATION passed.
/// `(net, word, offset, width)` per chunk — the parts that name a destination.
type DestParts = Vec<(u32, Option<u32>, Option<u32>, Option<u32>)>;

fn nba_dest(u: &crate::sched::NbaUpdate) -> DestParts {
    let chunks: Vec<&sim_ir::LvalChunk> = match &u.lhs {
        crate::sched::NbaLhs::One(c) => vec![c],
        crate::sched::NbaLhs::Many(l) => l.chunks.iter().collect(),
    };
    chunks
        .into_iter()
        .map(|c| (c.net, c.word, c.offset, c.width))
        .collect()
}

/// The variant tag of an effect, for the comparison the store cannot make.
fn effect_tag(e: &StmtEffect<'_>) -> &'static str {
    match e {
        StmtEffect::Blocking { .. } => "Blocking",
        StmtEffect::QPop { .. } => "QPop",
        StmtEffect::AssocIter { .. } => "AssocIter",
        StmtEffect::SeededRandom { .. } => "SeededRandom",
        StmtEffect::SeededDist { .. } => "SeededDist",
        StmtEffect::Cast { .. } => "Cast",
        StmtEffect::ValuePlusargs { .. } => "ValuePlusargs",
        StmtEffect::Fopen { .. } => "Fopen",
        StmtEffect::Fgetc { .. } => "Fgetc",
        StmtEffect::Feof { .. } => "Feof",
        StmtEffect::Ungetc { .. } => "Ungetc",
        StmtEffect::Fgets { .. } => "Fgets",
        StmtEffect::Fread { .. } => "Fread",
        StmtEffect::Scanf { .. } => "Scanf",
        StmtEffect::Sformatf { .. } => "Sformatf",
        StmtEffect::ClassNew { .. } => "ClassNew",
        StmtEffect::Nonblocking { .. } => "Nonblocking",
        StmtEffect::SysTask { .. } => "SysTask",
        StmtEffect::Force { .. } => "Force",
        StmtEffect::Release { .. } => "Release",
        StmtEffect::DisableFork => "DisableFork",
        StmtEffect::Nop => "Nop",
    }
}

/// Statement ids this gate executes: those whose every reached `Kernel` method
/// is implemented.
///
/// **This filter is deliberately STRICTER than the S0 design gate, and the first
/// draft's claim that it mirrored the gate was measured false twice.** It used
/// `sim_ir::rhs_is_stmt_effect`, which answers `false` for `$sformatf` on
/// purpose, so a `$sformatf`-rhs design — eligible, arena-buildable — would have
/// been admitted into a walk whose kernel panics on it. It now asks
/// `kpred::rhs_routes_to_worker`, the disjunction of the arms `compute_effect`
/// actually branches on, which cannot under-approximate that way.
///
/// The `delay.is_none()` line is the same shape: the S0 gate does NOT inspect
/// `NonblockingAssign.delay`, so a transport-delay NBA is eligible and reaches
/// the unbuilt `k_schedule_nba_at`. This filter, not the gate, is what keeps it
/// out — said plainly here because a reader who assumes otherwise will conclude
/// the gate covers something it does not.
fn executable_sites(ir: &SimIr, class_new_sites: &BTreeMap<u32, u32>) -> Vec<u32> {
    (0..ir.stmts.len() as u32)
        .filter(|&sid| {
            if class_new_sites.contains_key(&sid) {
                return false; // ClassNew → k_class_alloc, gate-refused
            }
            match &ir.stmts[sid as usize] {
                Stmt::BlockingAssign { rhs, .. } => {
                    !crate::exec::kpred::rhs_routes_to_worker(ir.exprs.as_slice(), *rhs)
                }
                Stmt::NonblockingAssign { delay, .. } => delay.is_none(),
                _ => false,
            }
        })
        .collect()
}

/// One design: mirror the stores, then run every executable statement through
/// the shared executor on both kernels, comparing all three observables.
/// Returns the number of (statement × state) comparisons made.
fn s1d4a_walk(src: &str, name: &str, seed: u64) -> usize {
    let (ir, opts) = build_with_opts(src);
    let arena = match NetArena::build(&ir, &opts) {
        Ok(a) => a,
        // Not a skip to paper over: the arena refuses frame-local storage and
        // the heap kinds, and `native::runtime_gate` refuses the same designs.
        Err(_) => return 0,
    };
    let sink = NullSink;
    let mut st = fresh_state(&ir, &sink);
    // `NetArena::build` reads `opts.two_state_nets` into `Slot.two_state` and the
    // write funnel coerces X/Z→0 on it, so leaving the ENGINE 4-state would make
    // the two stores disagree about a CORE S0 sidecar — the coercion arm is one
    // of only two the arena funnel kept. Every earlier walk installs it; this one
    // did not, which left that arm at zero coverage (soundness-review find).
    for &n in &opts.two_state_nets {
        st.two_state[n as usize] = true;
    }
    let n_nets = ir.nets.len() as u32;
    let empty_sites: BTreeMap<u32, u32> = BTreeMap::new();
    let sites = executable_sites(&ir, &empty_sites);
    if sites.is_empty() {
        return 0;
    }
    let mut st_n = fresh_state(&ir, &sink);
    for &n in &opts.two_state_nets {
        st_n.two_state[n as usize] = true;
    }
    let mut sched_n = Scheduler::new(&mut st_n, 33_000, 10_000, None, Default::default());
    let mut nk = NativeKernel::new(&ir, arena, &mut sched_n, &empty_sites, 10_000);
    let mut rng = Rng::new(seed);
    let mut compared = 0usize;

    for pass in 0..4 {
        // pass 0/2: full-range · 1/3: small values; odd passes carry X/Z.
        mirror_state(
            &mut st,
            &mut nk.arena,
            &mut rng,
            n_nets,
            pass % 2 == 1,
            pass >= 2,
        );
        nk.nba.clear();
        nk.nba_seq = 0;
        // DISTINCT values: `max_deltas` and `max_body_steps` were both 10_000, so the
        // `k_max_deltas` comparison was 10_000 == 10_000 and blind to a swap of the
        // two — the confusion the kernel's own constructor doc warns about.
        let mut sched = Scheduler::new(&mut st, 33_000, 10_000, None, Default::default());
        for &sid in &sites {
            let stmt = &ir.stmts[sid as usize];
            let eff_e = compute_effect(&sched, stmt, sid);
            let eff_n = compute_effect(&nk, stmt, sid);
            assert_eq!(
                effect_tag(&eff_e),
                effect_tag(&eff_n),
                "{name}/pass{pass}/sid{sid}: the two kernels built DIFFERENT statements"
            );
            apply_effect(&mut sched, eff_e);
            apply_effect(&mut nk, eff_n);
            assert_stores_equal(
                sched.st,
                &nk.arena,
                n_nets,
                &format!("{name}/pass{pass}/sid{sid}"),
            );
            assert_eq!(
                sched.nba.len(),
                nk.nba.len(),
                "{name}/pass{pass}/sid{sid}: NBA queue depth diverged"
            );
            for (a, b) in sched.nba.iter().zip(nk.nba.iter()) {
                assert_eq!(
                    (a.seq, &a.sampled, a.offsets.as_slice(), nba_dest(a)),
                    (b.seq, &b.sampled, b.offsets.as_slice(), nba_dest(b)),
                    "{name}/pass{pass}/sid{sid}: NBA entry diverged"
                );
            }
            compared += 1;
        }
        // The scheduler's queue is per-pass state on the engine side; the
        // kernel's is cleared above so the two stay index-aligned.
        sched.nba.clear();
    }
    compared
}

#[test]
fn s1d4a_shared_executor_agrees_on_both_stores_over_corpus() {
    let mut compared = 0usize;
    for (i, d) in corpus(0x5EED_F00D, 72).into_iter().enumerate() {
        let n = s1d4a_walk(&d.src, &d.name, 0x4A00_0000 + i as u64);
        // PER DESIGN, not just in the sum: `s1d4a_walk` returns 0 both when the
        // arena refuses a design and when it has no executable statement, and a
        // summed floor cannot tell "72 designs walked" from "36 walked, 36
        // silently skipped". Measured today: 72/72 arenas build.
        assert!(n > 0, "{}: produced no comparisons", d.name);
        compared += n;
    }
    assert_eq!(
        compared, 2044,
        "S1d-4a corpus coverage moved — re-pin deliberately"
    );
}

/// The shapes the corpus does not carry: a nonblocking assign to a dynamic
/// array element (the NBA sample-at-schedule rule), a concat LHS, a part-select
/// destination, and an X index. Each is a place where a kernel could get the
/// STORE right and the queued SAMPLE wrong.
#[test]
fn s1d4a_shared_executor_agrees_on_adversarial_assigns() {
    let designs: [(&str, &str); 5] = [
        (
            "nba_dynamic_index",
            "module t;\n\
               reg [15:0] m [0:7]; integer i; reg [15:0] x;\n\
               initial begin\n\
                 i = 3; x = 16'hbeef;\n\
                 m[i] <= x; i = i + 1; m[i] <= x + 1;\n\
               end\n\
             endmodule\n",
        ),
        (
            "nba_concat_lhs",
            "module t;\n\
               reg [7:0] a; reg [7:0] b; reg [15:0] w;\n\
               initial begin w = 16'h1234; {a, b} <= w; {b, a} <= ~w; end\n\
             endmodule\n",
        ),
        (
            "nba_part_select",
            "module t;\n\
               reg [31:0] y; reg [7:0] s; integer k;\n\
               initial begin\n\
                 y = 32'h0; s = 8'ha5; k = 8;\n\
                 y[15:8] <= s; y[k +: 8] <= ~s; y[31:24] <= s;\n\
               end\n\
             endmodule\n",
        ),
        (
            // 2-STATE destinations: `Slot.two_state` drives an X/Z→0 coercion in
            // the arena funnel, one of only two arms it kept. The P6 corpus has
            // ZERO two-state nets (measured), so without this design that arm has
            // no coverage in this gate at all.
            "two_state_coercion",
            "module t;\n\
               bit [7:0] b8; int i32; byte b; logic [7:0] l8;\n\
               initial begin\n\
                 l8 = 8'bxx01_z1z0; b8 = l8; i32 = l8; b = l8;\n\
                 b8 = 8'hff; i32 = -1; b = 8'sh80;\n\
               end\n\
             endmodule\n",
        ),
        (
            "nba_x_index_drops",
            "module t;\n\
               reg [7:0] m [0:3]; integer i; reg [7:0] v;\n\
               initial begin\n\
                 v = 8'h77; i = 'bx; m[i] <= v; i = 2; m[i] <= v;\n\
               end\n\
             endmodule\n",
        ),
    ];
    let mut compared = 0usize;
    for (i, (name, src)) in designs.iter().enumerate() {
        let n = s1d4a_walk(src, name, 0x4A5E_0000 + i as u64);
        assert!(n > 0, "{name}: produced no comparisons");
        compared += n;
    }
    assert_eq!(
        compared, 104,
        "adversarial coverage moved — re-pin deliberately"
    );
}

/// TEETH for the refused workers: each one must actually be reached and must
/// actually be loud. A `panic!` body is worthless if the method is dead — and
/// "unreachable by the gate" is precisely the condition that makes it look dead.
///
/// Rather than assert on 20 methods by name (a list that rots), this drives the
/// two that a legal, gate-REFUSED design reaches through the shared executor:
/// a `force` statement and a seeded-`$random` rhs. Both are refused by
/// `design_eligibility`, so reaching either in production is the bug the panic
/// is there to report.
#[test]
fn s1d4a_refused_workers_are_loud_not_silent() {
    let cases: [(&str, &str, &str); 2] = [
        (
            "force",
            "k_force",
            "module t; reg a; reg b;\n\
             initial begin a = 0; b = 1; force a = b; end\n\
             endmodule\n",
        ),
        (
            "seeded_random",
            "k_random_seeded",
            "module t; integer s; integer r;\n\
             initial begin s = 7; r = $random(s); end\n\
             endmodule\n",
        ),
    ];
    for (name, expect, src) in cases {
        let (ir, opts) = build_with_opts(src);
        // The design gate must refuse it — otherwise the panic below would be a
        // production path, not a backstop.
        let el = crate::native::design_eligibility(&ir, &opts);
        assert!(
            !el.eligible,
            "{name}: expected the S0 gate to refuse this design"
        );
        let arena = NetArena::build(&ir, &opts).expect("arena builds");
        let empty: BTreeMap<u32, u32> = BTreeMap::new();
        let sink = NullSink;
        let mut st_n = fresh_state(&ir, &sink);
        let mut sched_n = Scheduler::new(&mut st_n, 33_000, 10_000, None, Default::default());
        let mut nk = NativeKernel::new(&ir, arena, &mut sched_n, &empty, 10_000);
        nk.nba.clear();
        nk.nba_seq = 0;
        let sid = (0..ir.stmts.len() as u32)
            .find(|&s| match &ir.stmts[s as usize] {
                Stmt::Force { .. } => name == "force",
                Stmt::BlockingAssign { rhs, .. } => {
                    name == "seeded_random" && nk.k_random_seeded_rhs(*rhs)
                }
                _ => false,
            })
            .unwrap_or_else(|| panic!("{name}: no statement of the expected shape"));
        let hit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let eff = compute_effect(&nk, &ir.stmts[sid as usize], sid);
            apply_effect(&mut nk, eff);
        }));
        // The MESSAGE, not merely "something panicked": an index-out-of-bounds or
        // a `debug_assert` would otherwise read as success, which is the failure
        // mode a test whose whole subject is a panic can least afford.
        let payload = hit.expect_err(&format!("{name}: the refused worker did NOT panic"));
        let msg = match payload.downcast::<String>() {
            Ok(b) => *b,
            Err(p) => match p.downcast::<&'static str>() {
                Ok(b) => (*b).to_string(),
                Err(_) => String::from("<non-string panic payload>"),
            },
        };
        assert!(
            msg.contains("tier-3 native kernel")
                && msg.contains("design_eligibility")
                && msg.contains(expect),
            "{name}: panicked, but not with the gate-refused message naming \
             `{expect}` — got: {msg}"
        );
    }
}

/// The predicates must be answered for real, not stubbed `false`. A stub would
/// not show up in the corpus walk (those designs contain none of these rhs
/// forms) and would not show up as a wrong value (the workers panic) — it would
/// show up the day the gate widens, as a statement quietly taking the pure-eval
/// path. So assert directly that both kernels classify the same rhs the same way.
#[test]
fn s1d4a_both_kernels_classify_the_same_rhs_identically() {
    // No `string` net: the arena refuses heap storage, and this test needs a
    // NativeKernel instance. Every family still appears as an rhs EXPRESSION,
    // which is all a classification predicate reads — `$fgets` into a
    // fixed-width reg and `$sformatf` on a numeric lhs are exactly the forms
    // that keep the expression while staying inside R1 storage.
    let src = "module t;\n\
                 integer s; integer r; integer c; reg [63:0] w; integer fd;\n\
                 byte cd;\n\
                 initial begin\n\
                   s = 1; r = $random(s); r = $dist_uniform(s, 0, 9);\n\
                   c = $cast(cd, s);\n\
                   c = $value$plusargs(\"N=%d\", w);\n\
                   fd = $fopen(\"x.txt\", \"r\"); c = $fgetc(fd); c = $feof(fd);\n\
                   c = $ungetc(65, fd); c = $fgets(w, fd); c = $fread(w, fd);\n\
                   c = $fscanf(fd, \"%d\", w); c = $sscanf(\"1\", \"%d\", w);\n\
                   r = $clog2(w); r = w + 1;\n\
                 end\n\
               endmodule\n";
    let (ir, opts) = build_with_opts(src);
    let arena = NetArena::build(&ir, &opts).expect("arena builds");
    let empty: BTreeMap<u32, u32> = BTreeMap::new();
    let sink = NullSink;
    let mut st = fresh_state(&ir, &sink);
    let mut st_n = fresh_state(&ir, &sink);
    let mut sched_n = Scheduler::new(&mut st_n, 33_000, 10_000, None, Default::default());
    let nk = NativeKernel::new(&ir, arena, &mut sched_n, &empty, 10_000);
    let sched = Scheduler::new(&mut st, 10_000, 10_000, None, Default::default());

    // Every predicate, over every expression in the design — so a stub cannot
    // hide behind "the corpus never asks". Counted PER FAMILY: the first version
    // summed them and asserted `>= 20` against an actual of 22, so stubbing any
    // ONE family in the shared `kpred` dropped it to 21 and stayed green. An
    // aggregate floor cannot guard fifteen separate properties.
    let mut fired = std::collections::BTreeMap::<&str, usize>::new();
    for eid in 0..ir.exprs.len() as u32 {
        let e: [(&str, bool, bool); 16] = [
            (
                "queue_pop",
                sched.k_queue_pop_rhs(eid),
                nk.k_queue_pop_rhs(eid),
            ),
            (
                "random_seeded",
                sched.k_random_seeded_rhs(eid),
                nk.k_random_seeded_rhs(eid),
            ),
            (
                "dist_seeded",
                sched.k_dist_seeded_rhs(eid),
                nk.k_dist_seeded_rhs(eid),
            ),
            ("cast", sched.k_cast_rhs(eid), nk.k_cast_rhs(eid)),
            (
                "value_plusargs",
                sched.k_value_plusargs_rhs(eid),
                nk.k_value_plusargs_rhs(eid),
            ),
            ("fopen", sched.k_fopen_rhs(eid), nk.k_fopen_rhs(eid)),
            ("fgetc", sched.k_fgetc_rhs(eid), nk.k_fgetc_rhs(eid)),
            ("feof", sched.k_feof_rhs(eid), nk.k_feof_rhs(eid)),
            ("ungetc", sched.k_ungetc_rhs(eid), nk.k_ungetc_rhs(eid)),
            ("fgets", sched.k_fgets_rhs(eid), nk.k_fgets_rhs(eid)),
            ("fread", sched.k_fread_rhs(eid), nk.k_fread_rhs(eid)),
            ("fscanf", sched.k_fscanf_rhs(eid), nk.k_fscanf_rhs(eid)),
            ("sscanf", sched.k_sscanf_rhs(eid), nk.k_sscanf_rhs(eid)),
            (
                "sformatf",
                sched.k_sformatf_rhs(eid),
                nk.k_sformatf_rhs(eid),
            ),
            (
                "assoc_iter",
                sched.k_assoc_iter_rhs(eid),
                nk.k_assoc_iter_rhs(eid),
            ),
            (
                "stmt_effect_family",
                sched.k_rhs_is_stmt_effect_family(eid),
                nk.k_rhs_is_stmt_effect_family(eid),
            ),
        ];
        for (what, a, b) in e {
            assert_eq!(a, b, "predicate `{what}` diverged at expr {eid}");
            if a {
                *fired.entry(what).or_default() += 1;
            }
        }
    }
    // Without this the assertion above is vacuously satisfiable by two stubs
    // that both answer `false`: the design carries one rhs of every family, so
    // a real classification must fire many times.
    // EXACTLY the families this design carries must fire, each at least once.
    // The three it cannot carry are named, with the reason, rather than left to
    // be inferred from a total: `queue_pop` and `assoc_iter` need queue/assoc
    // storage and `sformatf` needs a `string` lhs — all three are storage kinds
    // `NetArena::build` refuses, so no NativeKernel can exist for a design that
    // has them. (`$cast` WAS omissible and was omitted silently in the first
    // version; it is carried above now — the arena builds `byte`/`integer`.)
    let must_fire = [
        "random_seeded",
        "dist_seeded",
        "cast",
        "value_plusargs",
        "fopen",
        "fgetc",
        "feof",
        "ungetc",
        "fgets",
        "fread",
        "fscanf",
        "sscanf",
        "stmt_effect_family",
    ];
    for f in must_fire {
        assert!(
            fired.get(f).copied().unwrap_or(0) > 0,
            "predicate `{f}` never fired — a stub `false` in `exec::kpred` would \
             pass this test. Fired: {fired:?}"
        );
    }
    let unreachable_here = ["queue_pop", "assoc_iter", "sformatf"];
    for f in unreachable_here {
        assert_eq!(
            fired.get(f).copied().unwrap_or(0),
            0,
            "`{f}` fired — this design was believed unable to carry it, so the \
             list of families this test cannot cover is stale"
        );
    }
}

/// The CONTROL + VM surface: `k_truthy`, `k_truthy_value`, `k_delay_ticks`,
/// `k_eval_native`, and the compiler's specialised write/NBA twins.
///
/// These are NOT reachable from `compute_effect` on an assignment — they are
/// what a body walk and a compiled body call — so the corpus walk above enters
/// none of them. Measured, not assumed: five mutations (`k_truthy` inverted,
/// `k_delay_ticks` → 0, `k_eval_native` → X, `k_write_scalar` → no-op, and the
/// assignment-context width dropped) all SURVIVED the first version of this
/// gate. A method with an honest body and no entry is the same shape as the
/// bit-serial store point §4.5.289 found: it looks covered because the file is.
fn s1d4a_control_walk(src: &str, name: &str, seed: u64) -> (usize, usize) {
    let (ir, opts) = build_with_opts(src);
    let arena = match NetArena::build(&ir, &opts) {
        Ok(a) => a,
        Err(_) => return (0, 0),
    };
    let sink = NullSink;
    let mut st = fresh_state(&ir, &sink);
    for &n in &opts.two_state_nets {
        st.two_state[n as usize] = true;
    }
    let n_nets = ir.nets.len() as u32;
    let wt = crate::width::WidthTable::build(&ir, &crate::FuncTable::new());
    let empty: BTreeMap<u32, u32> = BTreeMap::new();
    let mut st_n = fresh_state(&ir, &sink);
    for &n in &opts.two_state_nets {
        st_n.two_state[n as usize] = true;
    }
    let mut arena = arena;
    let mut rng = Rng::new(seed);
    // Whole-net scalar destinations — the shape the compiler proves before it
    // emits the specialised twins.
    let scalars: Vec<sim_ir::Lvalue> = ir
        .stmts
        .iter()
        .filter_map(|s| match s {
            Stmt::BlockingAssign { lhs, .. } | Stmt::NonblockingAssign { lhs, .. } => {
                match lhs.chunks.as_slice() {
                    [c] if c.word.is_none() && c.offset.is_none() && c.width.is_none() => {
                        Some(lhs.clone())
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    let nonint: Vec<bool> = vec![false; ir.exprs.len()];
    // TWO counters, not one: they were summed, and the expression half alone
    // already exceeded the floor — so the specialised-write half could have gone
    // to zero (removing the coverage a surviving mutation had just forced us to
    // add) with the gate still green. A summed floor cannot see its own halves.
    let (mut n_expr, mut n_spec) = (0usize, 0usize);
    if scalars.is_empty() {
        return (0, 0);
    }

    for pass in 0..3 {
        mirror_state(&mut st, &mut arena, &mut rng, n_nets, pass == 1, pass == 2);
        // A NON-UNIT timescale on both sides. With `cur_time_mult == 1`
        // everywhere (every corpus design is 1ns/1ns), dropping the multiplier
        // from `k_delay_ticks` entirely was invisible — measured, it survived.
        // The two-stage real rounding needs `prec_mult != mult` to be visible
        // at all, so pass 2 splits them.
        // …and a non-zero `now`, for the same reason: `$time`/`$realtime` read
        // it, and a kernel whose context reported the wrong simulation time was
        // invisible while every pass ran at t=0 (measured — it survived).
        // Set on BOTH states: the kernel reads its context from `st_n` now, so a
        // per-pass context has to be installed on both or the comparison is
        // between two different clocks rather than between two stores.
        let (tm, pm, now) = [(1u64, 1u64, 0u64), (1000, 1, 41), (1000, 10, 7_500)][pass];
        for t in [&mut st, &mut st_n] {
            t.cur_time_mult = tm;
            t.cur_prec_mult = pm;
            t.now = now;
        }
        let mut sched_n = Scheduler::new(&mut st_n, 33_000, 10_000, None, Default::default());
        let mut nk = NativeKernel::new(&ir, arena, &mut sched_n, &empty, 10_000);
        nk.nba.clear();
        nk.nba_seq = 0;
        // DISTINCT values: `max_deltas` and `max_body_steps` were both 10_000, so the
        // `k_max_deltas` comparison was 10_000 == 10_000 and blind to a swap of the
        // two — the confusion the kernel's own constructor doc warns about.
        let mut sched = Scheduler::new(&mut st, 33_000, 10_000, None, Default::default());
        for eid in 0..ir.exprs.len() as u32 {
            assert_eq!(
                sched.k_truthy(eid),
                nk.k_truthy(eid),
                "{name}/pass{pass}/e{eid}: k_truthy diverged"
            );
            assert_eq!(
                sched.k_delay_ticks(eid),
                nk.k_delay_ticks(eid),
                "{name}/pass{pass}/e{eid}: k_delay_ticks diverged"
            );
            // `k_truthy_value` takes an ALREADY-COMPUTED value, so build it
            // WITHOUT an extra evaluation on either side. The first version called
            // `sched.k_eval_for_lvalue` here — engine-only — which drove the two
            // `RngCells` a different number of times; an unseeded `$random()` rhs
            // (not gate-refused) would then desynchronise the streams and fail the
            // gate for a harness reason. Synthesising the value keeps both sides at
            // exactly the evaluations the comparison needs.
            let sw = wt.get(eid);
            for v in [
                crate::value::Value::from_i128(0, sw.width.max(1), sw.signed),
                crate::value::Value::from_i128(1, sw.width.max(1), sw.signed),
                crate::value::Value::x1(),
            ] {
                assert_eq!(
                    sched.k_truthy_value(&v),
                    nk.k_truthy_value(&v),
                    "{name}/pass{pass}/e{eid}: k_truthy_value diverged"
                );
            }
            if let Some(prog) =
                crate::native_eval::try_compile(&ir, &wt, &nonint, eid, sw.width, sw.signed)
            {
                assert_eq!(
                    sched.k_eval_native(&prog),
                    nk.k_eval_native(&prog),
                    "{name}/pass{pass}/e{eid}: k_eval_native diverged"
                );
            }
            n_expr += 1;
        }
        // The specialised twins, on a state the two stores still share.
        for (i, lhs) in scalars.iter().enumerate() {
            let net = lhs.chunks[0].net;
            let val = crate::value::Value::from_i128(
                (0x5A5A_0000u64 as i128) + i as i128,
                ir.nets[net as usize].width.max(1),
                false,
            );
            sched.k_write_scalar(lhs, net, val.clone());
            nk.k_write_scalar(lhs, net, val.clone());
            sched.k_schedule_nba_scalar(lhs, val.clone());
            nk.k_schedule_nba_scalar(lhs, val);
            n_spec += 1;
        }
        assert_stores_equal(
            sched.st,
            &nk.arena,
            n_nets,
            &format!("{name}/pass{pass}/specialised-writes"),
        );
        assert_eq!(
            sched.nba.len(),
            nk.nba.len(),
            "{name}/pass{pass}: specialised NBA depth diverged"
        );
        for (a, b) in sched.nba.iter().zip(nk.nba.iter()) {
            assert_eq!(
                (a.seq, &a.sampled, a.offsets.as_slice(), nba_dest(a)),
                (b.seq, &b.sampled, b.offsets.as_slice(), nba_dest(b)),
                "{name}/pass{pass}: specialised NBA entry diverged"
            );
        }
        assert_eq!(
            sched.k_max_deltas(),
            nk.k_max_deltas(),
            "{name}: max_deltas"
        );
        sched.nba.clear();
        // Hand the store back: the kernel is rebuilt each pass so its state's
        // render context can vary with the engine's.
        drop(sched);
        arena = nk.arena;
    }
    (n_expr, n_spec)
}

#[test]
fn s1d4a_control_and_vm_surface_agrees_over_corpus() {
    let (mut expr, mut spec) = (0usize, 0usize);
    for (i, d) in corpus(0x5EED_F00D, 72).into_iter().enumerate() {
        let (e, s) = s1d4a_control_walk(&d.src, &d.name, 0x4AC0_0000 + i as u64);
        assert!(e > 0 && s > 0, "{}: produced no comparisons", d.name);
        expr += e;
        spec += s;
    }
    // EXACT pins, matching this package's convention (`native/tests.rs` pins
    // 17940 / 3150 / 270 / 2240 / 920 with "re-pin deliberately"). A `>=` floor
    // let half the walk disappear silently.
    assert_eq!(
        (expr, spec),
        (5382, 1386),
        "control-surface coverage moved — re-pin deliberately"
    );
}

/// The control-surface shapes the corpus does not contain, each pinned by a
/// mutation that survived without it: REAL-valued delay amounts (the two-stage
/// rounding's `prec_mult` stage is a no-op on an integral delay, so swapping it
/// for 1 was invisible) and `$time`/`$realtime` reads (which are the only way
/// the context's `now` reaches a value at all).
#[test]
fn s1d4a_control_surface_on_real_delays_and_time_reads() {
    let designs: [(&str, &str); 3] = [
        (
            // No `real` NET — the arena refuses `NetKind::Real`. Real VALUES
            // still appear as expressions (literals, `$itor`, real division),
            // which is all `k_delay_ticks`'s real branch reads. This is the
            // same value/destination split §4.5.287 found in the write funnel.
            "real_delays",
            "module t;\n\
               reg [31:0] a; reg [7:0] b;\n\
               initial begin\n\
                 a = 1.5; b = 1; a = 0.0004; a = 2.75; a = -1.0;\n\
                 a = 1.0e9; a = $itor(3) / 2.0; a = 2.5e-4 * 4.0;\n\
               end\n\
             endmodule\n",
        ),
        (
            "time_reads",
            "module t;\n\
               reg [63:0] t1; reg [31:0] t2; reg [7:0] a;\n\
               initial begin\n\
                 t1 = $time; t2 = $realtime; a = 8'd3;\n\
                 t1 = $time + 3; t1 = $stime;\n\
               end\n\
             endmodule\n",
        ),
        (
            "wide_and_xz_delays",
            "module t;\n\
               reg [63:0] big; reg [7:0] a;\n\
               initial begin\n\
                 big = 64'hffff_ffff_ffff_ffff; a = 8'bxxxx_xxxx;\n\
                 big = 64'h8000_0000_0000_0000; big = 0;\n\
               end\n\
             endmodule\n",
        ),
    ];
    let (mut expr, mut spec) = (0usize, 0usize);
    for (i, (name, src)) in designs.iter().enumerate() {
        let (e, sp) = s1d4a_control_walk(src, name, 0x4AD0_0000 + i as u64);
        assert!(e > 0 && sp > 0, "{name}: produced no comparisons");
        expr += e;
        spec += sp;
    }
    assert_eq!(
        (expr, spec),
        (75, 51),
        "coverage moved — re-pin deliberately"
    );
}

/// `k_class_new_site` — the one classification question that is NOT a function
/// of `ir.exprs` but of the `class_new_sites` side table, and therefore the one
/// a shared `kpred` call cannot make structural. Stubbing it to `None` survived
/// the corpus walk (no eligible design has a `new` site, so `Some` never came
/// up), which is precisely the shape that ships a stub.
///
/// A real class design cannot be used — the arena refuses class storage — so the
/// table is installed directly on both sides for the same statement ids. Only
/// the EFFECT KIND is compared: applying would call `k_class_alloc`, which is
/// honestly loud here, and that is the correct answer rather than a limitation.
#[test]
fn s1d4a_class_new_site_is_read_not_stubbed() {
    let src = "module t;\n\
                 reg [7:0] a; reg [7:0] b;\n\
                 initial begin a = 8'h11; b = 8'h22; a = b; end\n\
               endmodule\n";
    let (ir, opts) = build_with_opts(src);
    let arena = NetArena::build(&ir, &opts).expect("arena builds");
    let sink = NullSink;
    let mut st = fresh_state(&ir, &sink);
    // Mark every blocking assign as an allocation site, on BOTH sides.
    let mut sites: BTreeMap<u32, u32> = BTreeMap::new();
    for sid in 0..ir.stmts.len() as u32 {
        if matches!(ir.stmts[sid as usize], Stmt::BlockingAssign { .. }) {
            sites.insert(sid, 7);
        }
    }
    assert!(
        !sites.is_empty(),
        "the design must contain blocking assigns"
    );
    st.class_new_sites = sites.clone();
    let mut st_n = fresh_state(&ir, &sink);
    let mut sched_n = Scheduler::new(&mut st_n, 33_000, 10_000, None, Default::default());
    let nk = NativeKernel::new(&ir, arena, &mut sched_n, &sites, 10_000);
    let sched = Scheduler::new(&mut st, 10_000, 10_000, None, Default::default());
    let mut class_new = 0usize;
    for (&sid, _) in sites.iter() {
        assert_eq!(
            sched.k_class_new_site(sid),
            nk.k_class_new_site(sid),
            "k_class_new_site diverged at sid {sid}"
        );
        let stmt = &ir.stmts[sid as usize];
        assert_eq!(
            effect_tag(&compute_effect(&sched, stmt, sid)),
            effect_tag(&compute_effect(&nk, stmt, sid)),
            "sid {sid}: the two kernels built different statements"
        );
        assert_eq!(
            effect_tag(&compute_effect(&nk, stmt, sid)),
            "ClassNew",
            "sid {sid}: a marked site must build ClassNew, or this test is vacuous"
        );
        class_new += 1;
    }
    assert!(class_new >= 3, "too few marked sites ({class_new})");
}

/// The ASSIGNMENT-CONTEXT rule (IEEE §11.6.1): the rhs is evaluated at
/// `max(lhs_width, self_width(rhs))`, not at its own width. Dropping the lhs
/// half survived the first gate — every corpus assignment happened to have an
/// rhs at least as wide as its destination, so self-width and context-width
/// produced the same bits. These designs are built so the two answers DIFFER:
/// a narrow-operand sum that must not truncate, a shift that must not lose the
/// high bits, and a signed narrow rhs that must sign-extend into a wide lhs.
#[test]
fn s1d4a_assignment_context_width_is_carried() {
    let designs: [(&str, &str); 4] = [
        (
            "widening_sum",
            "module t;\n\
               reg [7:0] a; reg [7:0] b; reg [31:0] wide; reg [7:0] narrow;\n\
               initial begin\n\
                 a = 8'hff; b = 8'h02;\n\
                 wide = a + b; narrow = a + b; wide = a * b;\n\
               end\n\
             endmodule\n",
        ),
        (
            "widening_shift",
            "module t;\n\
               reg [7:0] a; reg [31:0] wide; reg [15:0] mid;\n\
               initial begin\n\
                 a = 8'h81; wide = a << 8; mid = a << 4; wide = a << 20;\n\
               end\n\
             endmodule\n",
        ),
        (
            "signed_widening",
            "module t;\n\
               reg signed [7:0] s8; reg signed [31:0] s32; reg [31:0] u32;\n\
               initial begin\n\
                 s8 = -3; s32 = s8; u32 = s8; s32 = s8 * s8; s32 = s8 + 8'sd1;\n\
               end\n\
             endmodule\n",
        ),
        (
            "nba_widening",
            "module t;\n\
               reg [7:0] a; reg [7:0] b; reg [31:0] wide;\n\
               initial begin a = 8'hf0; b = 8'h20; wide <= a + b; wide <= a * b; end\n\
             endmodule\n",
        ),
    ];
    let mut compared = 0usize;
    for (i, (name, src)) in designs.iter().enumerate() {
        let n = s1d4a_walk(src, name, 0x4AC7_0000 + i as u64);
        assert!(n > 0, "{name}: produced no comparisons");
        compared += n;
    }
    assert_eq!(
        compared, 72,
        "context-width coverage moved — re-pin deliberately"
    );
}

/// S1d-4b-1 — **the format engine reads the store it is GIVEN.**
///
/// `k_dispatch_systask` was S1d-4a's one CORE method left unbuilt, because
/// `builtins` renders through `&SimState` while tier-3's nets live in a
/// `NetArena`. The seam is `format_args_str_with`: `st` keeps supplying every
/// cold field (IR, `now`, widths, time multiplier, format state) and the reader
/// becomes a parameter. The old entry point is a literal forward passing `st`,
/// so no engine call site changed.
///
/// **The first version of this gate was vacuous and four mutations proved it.**
/// It mirrored the two stores and asserted the two renders matched — but with
/// identical stores, a reader that ignores its parameter and reads `SimState`
/// produces the identical string. Every "ignore the `nets` argument" mutation
/// survived. Sameness cannot test provenance.
///
/// So the stores are made to DISAGREE. State A goes into `SimState`, state B
/// into the arena, and rendering with the arena as reader must equal what a
/// `SimState` holding B renders — not what the `SimState` holding A renders.
/// The `differing` counter is the anti-vacuity guard: if A and B happened to
/// render alike everywhere, the assertion would be satisfiable by either store.
fn s1d4b_render_walk(src: &str, name: &str, seed: u64) -> (usize, usize, Vec<bool>) {
    let (ir, opts) = build_with_opts(src);
    let arena = match NetArena::build(&ir, &opts) {
        Ok(a) => a,
        Err(_) => return (0, 0, Vec::new()),
    };
    let sink = NullSink;
    // TWO states over the same design: cold fields identical (same `fresh_state`,
    // same ir), net values independent. `st_a` is the one handed to the renderer;
    // `st_b` exists only to produce the expected string for the arena's contents.
    let mut st_a = fresh_state(&ir, &sink);
    let mut st_b = fresh_state(&ir, &sink);
    for &n in &opts.two_state_nets {
        st_a.two_state[n as usize] = true;
        st_b.two_state[n as usize] = true;
    }
    let n_nets = ir.nets.len() as u32;
    let mut arena = arena;
    let sites: Vec<(Option<u32>, Vec<u32>)> = ir
        .stmts
        .iter()
        .filter_map(|s| match s {
            // `Display | Write` ONLY. `$monitor`/`$strobe` were in this list and
            // collected zero sites in every design — and had they collected any,
            // the assertion would have been aimed at the wrong entry point:
            // neither renders through `format_args_str` at dispatch time. They
            // register a `FmtCapture` and render later in `flush_postponed`
            // (`sched/run_loop.rs`), which this slice did not thread. A filter
            // arm that cannot fire reads as coverage that does not exist.
            Stmt::SysTask {
                which: sim_ir::SysTaskId::Display | sim_ir::SysTaskId::Write,
                fmt,
                args,
            } => Some((*fmt, args.clone())),
            _ => None,
        })
        .collect();
    if sites.is_empty() {
        return (0, 0, Vec::new());
    }
    let mut rng = Rng::new(seed);
    let (mut compared, mut differing) = (0usize, 0usize);
    // PER SITE, not just in the aggregate: a global "half the comparisons
    // differ" guard is satisfied by the net-formatting sites while a site that
    // silently rendered from `st_a` sits beside them unnoticed. A site whose
    // output cannot depend on the store at all (`%m`, a string literal, a real
    // literal) is expected to be uniform — the caller says which.
    let mut site_differs = vec![false; sites.len()];
    for pass in 0..4 {
        // State A into st_a (and transiently the arena), then state B into BOTH
        // st_b and the arena — leaving st_a at A and the arena at B.
        let mut scratch = NetArena::build(&ir, &opts).expect("arena rebuilds");
        mirror_state(
            &mut st_a,
            &mut scratch,
            &mut rng,
            n_nets,
            pass % 2 == 1,
            pass >= 2,
        );
        mirror_state(
            &mut st_b,
            &mut arena,
            &mut rng,
            n_nets,
            pass % 2 == 1,
            pass >= 2,
        );
        for (i, (fmt, args)) in sites.iter().enumerate() {
            for radix in [None, Some(b'h'), Some(b'b'), Some(b'o')] {
                let from_a = crate::builtins::format_args_str(&st_a, *fmt, args, radix);
                let from_b = crate::builtins::format_args_str(&st_b, *fmt, args, radix);
                let via_arena =
                    crate::builtins::format_args_str_with(&st_a, &arena, *fmt, args, radix);
                // The reader decides. Rendering through `st_a` with the ARENA as
                // the net source must produce B's string, not A's.
                assert_eq!(
                    via_arena, from_b,
                    "{name}/pass{pass}/site{i}/radix{radix:?}: the arena reader did not \
                     supply the values (got A's render, or a third thing)"
                );
                assert!(
                    !via_arena.is_empty(),
                    "{name}/pass{pass}/site{i}: rendered nothing"
                );
                if from_a != from_b {
                    differing += 1;
                    site_differs[i] = true;
                }
                compared += 1;
            }
        }
    }
    (compared, differing, site_differs)
}

#[test]
fn s1d4b_format_engine_renders_identically_from_either_store() {
    let (mut compared, mut differing) = (0usize, 0usize);
    for (i, d) in corpus(0x5EED_F00D, 72).into_iter().enumerate() {
        let (c, d2, per_site) = s1d4b_render_walk(&d.src, &d.name, 0x4B00_0000 + i as u64);
        // EVERY corpus render site formats a net (verified: the generator's
        // templates all interpolate one), so every one must discriminate.
        for (k, ok) in per_site.iter().enumerate() {
            assert!(*ok, "{}/site{k}: never differed between stores", d.name);
        }
        compared += c;
        differing += d2;
    }
    assert_eq!(
        compared, 1264,
        "render coverage moved — re-pin deliberately"
    );
    // ANTI-VACUITY: on this many sites the two stores must actually render
    // differently, or the equality above is satisfiable by reading either one.
    assert!(
        differing * 2 >= compared,
        "the two stores render alike too often ({differing}/{compared}) — the \
         provenance assertion is not discriminating"
    );
}

/// The format shapes the corpus does not exercise: every conversion the engine
/// supports, X/Z operands, string values, a real, `%t`, `%m`, width/pad specs,
/// and the trailing-arg rule (an arg not consumed by the format string prints
/// sequentially, and a string arg is itself a format segment).
#[test]
fn s1d4b_render_agrees_on_the_full_conversion_matrix() {
    let designs: [(&str, &str); 3] = [
        (
            "conversions",
            "module t;\n\
               reg [31:0] w; reg [7:0] b; reg signed [15:0] sv; reg [63:0] q;\n\
               initial begin\n\
                 w = 32'hdead_beef; b = 8'b1010_xz01; sv = -1234; q = 64'h0123_4567_89ab_cdef;\n\
                 $display(\"%d %h %o %b %c %s %0d %8d %-8d|\", w, w, w, b, b, b, w, w, w);\n\
                 $display(\"%x %X %H %B %O %v %u %z\", w, w, w, b, b, b, w, w);\n\
                 $display(\"sv=%d %0d %h\", sv, sv, sv);\n\
                 $display(\"q=%h %d\", q, q);\n\
                 $display(\"xz=%b %h %d %o\", b, b, b, b);\n\
                 $write(\"nofmt\", w, b, sv);\n\
                 $display(\"%%literal%% %m\");\n\
                 $display(\"%t\", w);\n\
                 $display(\"lit=%s\", \"a string literal\");\n\
               end\n\
             endmodule\n",
        ),
        (
            "real_and_time",
            "module t;\n\
               reg [31:0] a; reg [63:0] tv;\n\
               initial begin\n\
                 a = 42; tv = 64'd1234567;\n\
                 $display(\"%f %e %g\", 1.5, 2.5e10, 0.0001);\n\
                 $display(\"%0f|%12.3f|%-12.3e|\", 3.14159, 3.14159, 3.14159);\n\
                 $display(\"t=%t a=%0d\", tv, a);\n\
               end\n\
             endmodule\n",
        ),
        (
            "trailing_args",
            "module t;\n\
               reg [7:0] a; reg [7:0] b;\n\
               initial begin\n\
                 a = 8'h41; b = 8'h42;\n\
                 $display(a, b);\n\
                 $display(\"lead %d\", a, \" mid %d \", b, a);\n\
                 $display(\"%s and %s\", a, b);\n\
               end\n\
             endmodule\n",
        ),
    ];
    let (mut compared, mut differing) = (0usize, 0usize);
    for (i, (name, src)) in designs.iter().enumerate() {
        let (c, d, per_site) = s1d4b_render_walk(src, name, 0x4BC0_0000 + i as u64);
        assert!(c > 0, "{name}: produced no comparisons");
        // These designs DO carry sites whose output cannot depend on the store —
        // `%m`, a string literal, real literals — so a per-site requirement is
        // stated as a count of sites that must discriminate, named per design.
        // EXACT, measured. `conversions` has 9 sites of which 7 read a net —
        // the two that cannot are `%%literal%% %m` and `lit=%s` of a string
        // literal. `real_and_time` has 3, of which only the `%t`/`%0d` line reads
        // one (the other two format real LITERALS). `trailing_args` reads a net
        // at all 3. An inequality here would let a site quietly stop
        // discriminating; this catches it.
        let must: usize = match *name {
            "conversions" => 7,
            "real_and_time" => 1,
            _ => 3,
        };
        let got = per_site.iter().filter(|b| **b).count();
        assert_eq!(
            got,
            must,
            "{name}: {got} of {} sites discriminate between the stores (expected \
             {must}) — re-pin deliberately",
            per_site.len()
        );
        compared += c;
        differing += d;
    }
    assert_eq!(
        compared, 240,
        "conversion-matrix coverage moved — re-pin deliberately"
    );
    assert!(
        differing * 2 >= compared,
        "the two stores render alike too often ({differing}/{compared})"
    );
    // THREE arms of the threaded path are deliberately NOT pinned, and saying
    // which beats letting a surviving mutation look like an oversight. The list
    // said TWO until an adversarial mutation sweep found the third — the failure
    // mode this list exists to prevent, committed in the list itself. All three
    // measured, not assumed:
    //
    // 1. Three helpers on the render path are deliberately NOT reader-threaded,
    //    because none of them can read a net: `expr_const_string` and
    //    `str_const_of_expr` match `Expr::Const` and return early otherwise, and
    //    `arg_string`'s one render call site is guarded on a `StrUtf8` const.
    //    Threading them would ship parameters no mutation could ever kill, which
    //    is indistinguishable from a real gap. `$display("lit=%s", "...")` above
    //    drives the third arm, so it is REACHED; its provenance simply is not a
    //    question. (`$dumpfile(name)` DOES pass an arbitrary expression to
    //    `arg_string`; threading that is S1d-4b-2, where a test can pin it.)
    // 2. The RUNTIME-STRING trailing arm (`format_args_str_with`'s `v.is_str`
    //    branch, where a string-VALUED argument becomes its own format segment).
    //    Every shape that reaches it needs a `string` net or a string method on
    //    one, so `NetArena::build` refuses the design — measured: `$display("x",
    //    s)` and `$display("x", s.toupper())` both report `refused: "string"`.
    //    A `{"p","q"}` concat IS eligible but folds to a numeric const and never
    //    enters the arm (instrumented). Correct code, untestable today.
    // 3. The nested-`$sformatf` arm (`$display("<%s>", $sformatf(…))`, which is
    //    also every string CONCAT after elaborate's desugar). Unreachable for any
    //    design tier-3 can build: the `$sformatf` hoist allocates a `string`
    //    module net for its temp, and `NetArena::build` refuses string storage.
    //    Measured — `run.json` reports `refused: "string"` for exactly that
    //    design. It becomes testable when S3 gives the arena string storage —
    //    which is also when arm 2 becomes testable, since both are gated on the
    //    same missing storage.
    //
    //    ⚠️ The stated REASON for this one is narrower than the arm: the hoist
    //    (`elaborate/stmt_main.rs`) closes the literal `$sformatf` spelling, but
    //    a string CONCAT reaches the same IR node through `lower_string_concat_
    //    parts` at expression lowering, and what refuses THAT is the `string`
    //    net its parts need. Same conclusion, two different mechanisms — the
    //    §4.5.287 shape, where the argument answers the destination side while
    //    the condition lives on the value side.
}

/// S1d-4b-2 — **`$display` actually runs on the arena.**
///
/// 4b-1 proved the format engine renders from a supplied reader; this proves the
/// whole dispatch path does, end to end, through `k_dispatch_systask`. Same
/// provenance construction as 4b-1 and for the same reason: with two identical
/// stores, a dispatch that ignored its reader would produce identical bytes.
/// State A goes into the kernel's own `SimState`, state B into the arena, and
/// the bytes `k_dispatch_systask` writes must be the bytes a `SimState` holding
/// B produces — not the ones its own state holds.
///
/// The captured sink is what makes this an OUTPUT test rather than a string
/// test: `$display` reaches `write_out(st.out)`, so anything that renders
/// correctly and then writes the wrong thing (or nothing) still fails here.
fn s1d4b2_dispatch_walk(src: &str, name: &str, seed: u64) -> (usize, usize) {
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A sink that keeps what was written, so the gate compares BYTES.
    #[derive(Clone, Default)]
    struct Cap(Rc<RefCell<Vec<u8>>>);

    /// …and the DIAGNOSTIC stream, because `$error`/`$info` do not reach
    /// `st.out` at all — they emit a `LogEvent::Diagnostic`. Capturing only
    /// stdout made a severity-only design look like "the kernel wrote nothing",
    /// which is a true statement about the wrong stream.
    #[derive(Clone, Default)]
    struct CapLog(Rc<RefCell<Vec<String>>>);
    impl diag::LogSink for CapLog {
        fn emit(&self, e: diag::LogEvent) {
            self.0.borrow_mut().push(match e {
                diag::LogEvent::Diagnostic(d) => format!("D:{}", d.message),
                diag::LogEvent::RtlOutput(t) => format!("R:{}", t.text),
                diag::LogEvent::Progress(p) => format!("P:{}", p.message),
            });
        }
    }
    impl std::io::Write for Cap {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let (ir, opts) = build_with_opts(src);
    // Build once only to learn whether the design is arena-buildable; each pass
    // makes its own. (An earlier version kept this one and never used it.)
    if NetArena::build(&ir, &opts).is_err() {
        return (0, 0);
    }
    let sites: Vec<(sim_ir::SysTaskId, Option<u32>, Vec<u32>, u32)> = ir
        .stmts
        .iter()
        .enumerate()
        .filter_map(|(sid, s)| match s {
            // `Fdisplay`/`Fwrite` too: they are a DIFFERENT task id rendering
            // through a THIRD site in `dispatch`, and leaving them out left that
            // site's reader unpinned — measured, dropping it survived.
            Stmt::SysTask {
                which:
                    which @ (sim_ir::SysTaskId::Display
                    | sim_ir::SysTaskId::Write
                    | sim_ir::SysTaskId::Fdisplay
                    | sim_ir::SysTaskId::Fwrite),
                fmt,
                args,
            } => Some((*which, *fmt, args.clone(), sid as u32)),
            _ => None,
        })
        .collect();
    if sites.is_empty() {
        return (0, 0);
    }
    let mut rng = Rng::new(seed);
    let (mut compared, mut differing) = (0usize, 0usize);
    let n_nets = ir.nets.len() as u32;

    for pass in 0..4 {
        // Build the two states, then split them: A into the kernel's state, B
        // into the arena AND into the reference state.
        let cap_k = Cap::default();
        let cap_b = Cap::default();
        let log_k = CapLog::default();
        let log_b = CapLog::default();
        let mut st_a = crate::SimState::new(
            &ir,
            Box::new(cap_k.clone()),
            &log_k,
            "1ns".to_string(),
            "test".to_string(),
            None,
        );
        let mut st_b = crate::SimState::new(
            &ir,
            Box::new(cap_b.clone()),
            &log_b,
            "1ns".to_string(),
            "test".to_string(),
            None,
        );
        for &n in &opts.two_state_nets {
            st_a.two_state[n as usize] = true;
            st_b.two_state[n as usize] = true;
        }
        // Without this, `$error`/`$info` take the plain-display path and
        // `run_severity_with` — a render site outside `dispatch.rs` — is never entered.
        st_a.severities = opts.severities.clone();
        st_b.severities = opts.severities.clone();
        let mut arena_k = NetArena::build(&ir, &opts).expect("arena rebuilds");
        let mut scratch = NetArena::build(&ir, &opts).expect("arena rebuilds");
        mirror_state(
            &mut st_a,
            &mut scratch,
            &mut rng,
            n_nets,
            pass % 2 == 1,
            pass >= 2,
        );
        mirror_state(
            &mut st_b,
            &mut arena_k,
            &mut rng,
            n_nets,
            pass % 2 == 1,
            pass >= 2,
        );
        // A's stored nets, kept so the anti-vacuity check below can render from
        // the ACTUAL A rather than a fresh draw.
        let st_a_nets: Vec<sim_ir::BitPacked> = st_a.nets.iter().map(|n| n.cur.clone()).collect();
        let (now, tm) = [(0u64, 1u64), (41, 1000), (7_500, 1000), (123_456, 10)][pass];
        for t in [&mut st_a, &mut st_b] {
            t.now = now;
            t.cur_time_mult = tm;
        }

        // Reference: dispatch on state B through the ordinary engine path.
        {
            let mut sched_b = Scheduler::new(&mut st_b, 33_000, 10_000, None, Default::default());
            for (which, fmt, args, sid) in &sites {
                crate::builtins::dispatch(&mut sched_b, *which, *fmt, args, *sid);
            }
        }
        // Under test: dispatch on state A through the KERNEL, arena = B.
        {
            let empty: BTreeMap<u32, u32> = BTreeMap::new();
            let mut sched_a = Scheduler::new(&mut st_a, 33_000, 10_000, None, Default::default());
            let mut nk = NativeKernel::new(&ir, arena_k, &mut sched_a, &empty, 10_000);
            for (which, fmt, args, sid) in &sites {
                nk.k_dispatch_systask(*which, *fmt, args, *sid);
            }
        }
        // BOTH streams: `$display`/`$write` land in `out`, `$error`/`$info` in
        // the diagnostic sink, and a gate that watched only one would call a
        // severity-only design empty.
        let via_kernel = (cap_k.0.borrow().clone(), log_k.0.borrow().clone());
        let from_b = (cap_b.0.borrow().clone(), log_b.0.borrow().clone());
        assert_eq!(
            (String::from_utf8_lossy(&via_kernel.0), &via_kernel.1),
            (String::from_utf8_lossy(&from_b.0), &from_b.1),
            "{name}/pass{pass}: kernel dispatch did not read the arena"
        );
        assert!(
            !via_kernel.0.is_empty() || !via_kernel.1.is_empty(),
            "{name}/pass{pass}: the kernel wrote nothing on either stream"
        );
        // ANTI-VACUITY. The first version of this built a THIRD state here — it
        // re-drew from an `rng` the two `mirror_state` calls above had already
        // advanced, with `small`/`xz` hard-coded false where A had used the
        // pass's settings, and it overwrote `scratch`, the only copy of A. So it
        // compared B against A′, certified nothing about A, and its pin comment
        // told a causal story ("a small-value pass can collide") that could not
        // apply. 4b-1's walk did this correctly and 4b-2 had dropped it.
        //
        // A is `st_a`'s own store, which is still intact: render it through the
        // ordinary engine path and require it to differ from B.
        let cap_a = Cap::default();
        let log_a = CapLog::default();
        {
            let mut st_a2 = crate::SimState::new(
                &ir,
                Box::new(cap_a.clone()),
                &log_a,
                "1ns".to_string(),
                "test".to_string(),
                None,
            );
            for &n in &opts.two_state_nets {
                st_a2.two_state[n as usize] = true;
            }
            st_a2.severities = opts.severities.clone();
            // Copy A's nets across verbatim — NOT a fresh draw.
            for (slot, cur) in st_a2.nets.iter_mut().zip(st_a_nets.iter()) {
                slot.cur = cur.clone();
            }
            st_a2.now = now;
            st_a2.cur_time_mult = tm;
            let mut s2 = Scheduler::new(&mut st_a2, 33_000, 10_000, None, Default::default());
            for (which, fmt, args, sid) in &sites {
                crate::builtins::dispatch(&mut s2, *which, *fmt, args, *sid);
            }
        }
        // COUNTED, not asserted: some (design, pass) pairs genuinely cannot
        // discriminate — an X-heavy pass on a narrow design renders `x` from both
        // states, and that is a true fact about the design, not a test defect.
        // The count is pinned exactly so the number of CERTIFIED pairs cannot
        // quietly fall.
        if (cap_a.0.borrow().clone(), log_a.0.borrow().clone()) != from_b {
            differing += 1;
        }
        compared += 1;
    }
    (compared, differing)
}

#[test]
fn s1d4b2_kernel_dispatch_reads_the_arena() {
    let (mut compared, mut differing) = (0usize, 0usize);
    for (i, d) in corpus(0x5EED_F00D, 72).into_iter().enumerate() {
        let (c, df) = s1d4b2_dispatch_walk(&d.src, &d.name, 0x4B20_0000 + i as u64);
        compared += c;
        differing += df;
    }
    assert!(compared > 0, "no corpus design dispatches a $display");
    // EXACT: 256 of 288 (design, pass) pairs are CERTIFIED — state A demonstrably
    // renders different bytes from state B, so the provenance assertion above
    // cannot be satisfied by reading either store. The other 32 render alike from
    // both, overwhelmingly X-heavy passes on narrow designs where every value
    // formats as `x`.
    //
    // The previous pin said 285, and it was measured against a THIRD random state
    // rather than A — which is why it was higher, and why it certified nothing.
    // 256 is what the property is actually worth.
    assert_eq!(
        (compared, differing),
        (288, 256),
        "dispatch coverage moved — re-pin deliberately"
    );
}

/// The dispatch shapes the corpus does not carry. Measured, not guessed: with
/// the corpus alone, dropping the reader from the `$write` arm SURVIVED — no
/// generated design uses `$write`, and an arm the gate never enters is an arm
/// the gate does not cover, whatever the file looks like.
#[test]
fn s1d4b2_dispatch_agrees_on_the_task_variants() {
    let designs: [(&str, &str); 3] = [
        (
            "write_and_radix_variants",
            "module t;\n\
               reg [31:0] w; reg [7:0] b;\n\
               initial begin\n\
                 w = 32'hfeed_face; b = 8'b1010_0101;\n\
                 $write(\"w=%0d b=%0h\\n\", w, b);\n\
                 $write(w, b);\n\
                 $displayb(w, b); $displayo(w, b); $displayh(w, b);\n\
                 $writeb(w); $writeo(w); $writeh(w);\n\
                 $fdisplay(32'h8000_0001, \"fd w=%0d b=%0h\", w, b);\n\
                 $fwrite(32'h8000_0001, \"fw %0b|%0o\", w, b);\n\
               end\n\
             endmodule\n",
        ),
        (
            "severity_family",
            "module t;\n\
               reg [15:0] v;\n\
               initial begin\n\
                 v = 16'hbeef;\n\
                 $info(\"info v=%0h\", v);\n\
                 $warning(\"warn v=%0d\", v);\n\
                 $error(\"err v=%0b\", v);\n\
               end\n\
             endmodule\n",
        ),
        (
            "wide_and_xz",
            "module t;\n\
               reg [95:0] q; reg [7:0] x;\n\
               initial begin\n\
                 q = 96'h1234_5678_9abc_def0_1122_3344; x = 8'bxx01_zz10;\n\
                 $display(\"q=%h|%d|%o|%b\", q, q, q, q);\n\
                 $write(\"x=%h|%b|%c|%s\", x, x, x, x);\n\
               end\n\
             endmodule\n",
        ),
    ];
    let (mut compared, mut differing) = (0usize, 0usize);
    for (i, (name, src)) in designs.iter().enumerate() {
        let (c, d) = s1d4b2_dispatch_walk(src, name, 0x4B2C_0000 + i as u64);
        assert!(c > 0, "{name}: produced no comparisons");
        compared += c;
        differing += d;
    }
    assert_eq!(
        (compared, differing),
        (12, 12),
        "variant coverage moved — re-pin deliberately"
    );
}

/// `k_sformatf` on an input `compute_effect` never produces: the engine answers
/// an empty string, and this must answer the same. It was a `not_built!` panic —
/// two `Kernel` implementors disagreeing on one input, in a slice whose thesis is
/// that they structurally cannot. Unreachable through `compute_effect`, which is
/// why only a direct call can pin it.
#[test]
fn s1d4b2_sformatf_matches_the_engine_on_an_impossible_rhs() {
    let src = "module t; reg [7:0] a; initial begin a = 8'd1; end endmodule\n";
    let (ir, opts) = build_with_opts(src);
    let arena = NetArena::build(&ir, &opts).expect("arena builds");
    let sink = NullSink;
    let mut st = fresh_state(&ir, &sink);
    let empty: BTreeMap<u32, u32> = BTreeMap::new();
    // An expression that is NOT a `SysFunc` — the design's own `8'd1` const.
    let non_sysfunc = (0..ir.exprs.len() as u32)
        .find(|&e| matches!(ir.exprs[e as usize], sim_ir::Expr::Const { .. }))
        .expect("a const expr");
    let mut sched = Scheduler::new(&mut st, 33_000, 10_000, None, Default::default());
    let engine = sched.k_sformatf(non_sysfunc);
    let mut nk = NativeKernel::new(&ir, arena, &mut sched, &empty, 10_000);
    let native = nk.k_sformatf(non_sysfunc);
    assert_eq!(
        engine, native,
        "the two Kernel implementors disagree on a non-SysFunc `$sformatf` rhs"
    );
}

/// TEETH for `k_dispatch_systask`'s REFUSED arms. They are not gate-refused —
/// an eligible design reaches them — so the only thing standing between a
/// `$dumpvars` design and a VCD written from the wrong store is this match. A
/// refusal nothing tests is a refusal that can be deleted by accident: measured,
/// turning the `$dumpvars` arm into a no-op guard survived every other test.
#[test]
fn s1d4b2_store_reading_tasks_are_refused_not_dispatched() {
    let src = "module t;\n\
                 reg [7:0] m [0:3]; reg [7:0] v; integer fd;\n\
                 initial begin\n\
                   v = 8'h5a; m[0] = v; fd = 1;\n\
                   $dumpfile(\"x.vcd\"); $dumpvars(0, t); $dumpon; $dumpall;\n\
                   $dumplimit(1000); $writememb(\"m.txt\", m); $writememh(\"m.hex\", m);\n\
                   $fclose(fd);\n\
                   $monitor(\"m v=%0d\", v); $strobe(\"s v=%0d\", v);\n\
                 end\n\
               endmodule\n";
    let (ir, opts) = build_with_opts(src);
    let arena = NetArena::build(&ir, &opts).expect("arena builds");
    let sink = NullSink;
    let mut st = fresh_state(&ir, &sink);
    let empty: BTreeMap<u32, u32> = BTreeMap::new();
    let mut sched = Scheduler::new(&mut st, 33_000, 10_000, None, Default::default());
    let mut nk = NativeKernel::new(&ir, arena, &mut sched, &empty, 10_000);
    let mut refused = 0usize;
    for (sid, stmt) in ir.stmts.iter().enumerate() {
        let Stmt::SysTask { which, fmt, args } = stmt else {
            continue;
        };
        let expect_refusal = matches!(
            which,
            sim_ir::SysTaskId::DumpVars
                | sim_ir::SysTaskId::DumpAll
                | sim_ir::SysTaskId::DumpOn
                | sim_ir::SysTaskId::DumpFile
                | sim_ir::SysTaskId::DumpLimit
                | sim_ir::SysTaskId::Fclose
                | sim_ir::SysTaskId::WritememB
                | sim_ir::SysTaskId::WritememH
                // …and the two whose RENDER happens outside dispatch entirely.
                | sim_ir::SysTaskId::Monitor
                | sim_ir::SysTaskId::Strobe
        );
        if !expect_refusal {
            continue;
        }
        let (which, fmt, args) = (*which, *fmt, args.clone());
        let hit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            nk.k_dispatch_systask(which, fmt, &args, sid as u32);
        }));
        // `Err` = it panicked = it refused. (The first version of this line had
        // the polarity backwards and reported the opposite of what happened.)
        let payload = match hit {
            Err(p) => p,
            Ok(()) => panic!("{which:?}: DISPATCHED instead of refusing"),
        };
        let msg = match payload.downcast::<String>() {
            Ok(b) => *b,
            Err(p) => match p.downcast::<&'static str>() {
                Ok(b) => (*b).to_string(),
                Err(_) => String::new(),
            },
        };
        assert!(
            msg.contains("tier-3 native kernel") && msg.contains("NOT built yet"),
            "{which:?}: panicked, but not with the not-built refusal — got: {msg}"
        );
        refused += 1;
    }
    assert_eq!(
        refused, 10,
        "expected all ten refused tasks to refuse — got {refused}"
    );
}

/// The task arguments that are NET READS but never pass through the format
/// engine: `$fdisplay`'s file descriptor and `$timeformat`'s units/precision.
/// Threading the formatter alone left both reading `SimState`.
///
/// Both reviews found this independently, and neither the corpus walk nor the
/// variant walk could see it — they use a CONSTANT fd (`32'h8000_0001`), which
/// reads the same from either store. A literal is exactly the shape that hides a
/// provenance bug.
///
/// Measured before the fix: `$fdisplay(fd, …)` with a NET fd read the untouched
/// engine store, got X, and DROPPED the line with a bad-descriptor warning — on
/// a design `run.json` reports `eligible: true, buildable: true`.
///
/// This one does NOT use the random walk. A randomly-drawn fd is invalid in both
/// stores on most passes, so the two agree by both failing — measured, only 2 of
/// 4 passes discriminated. The values are therefore chosen: the arena gets a
/// USABLE descriptor and `SimState` an unusable one, so "which store did it
/// read" is the difference between a line and a dropped line.
///
/// ⚠️ `$timeformat`'s arguments are threaded the same way and are NOT pinned
/// here. This harness builds its `SimOpts` by hand and its `timeformat_stmts`
/// sids do not reach `dispatch`'s intercept, so the task prints its own
/// arguments instead of applying them — a harness limit, not a product one:
/// the end-to-end differential measured `$timeformat` with runtime-variable
/// arguments PRE == POST == iverilog. Saying so beats a test that passes for the
/// wrong reason.
#[test]
fn s1d4b2_non_format_task_args_read_the_arena() {
    use std::cell::RefCell;
    use std::rc::Rc;
    #[derive(Clone, Default)]
    struct Cap(Rc<RefCell<Vec<u8>>>);
    impl std::io::Write for Cap {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // (name, source, the net to diverge, arena value, SimState value, what the
    // arena's value must make appear in the output)
    let cases: [(&str, &str, u64, u64, &str); 1] = [(
        "fdisplay_net_fd",
        "module t;\n\
               integer fd; reg [15:0] v;\n\
               initial begin fd = 0; v = 16'habcd;\n\
                 $fdisplay(fd, \"v=%0h\", v);\n\
               end\n\
             endmodule\n",
        0x8000_0001, // stdout descriptor — the line prints
        0x0000_0000, // no channel — the line goes nowhere
        "v=",
    )];

    for (name, src, arena_val, state_val, expect) in cases {
        let (ir, opts) = build_with_opts(src);
        // ELIGIBLE is what makes the hole reachable rather than hypothetical.
        let el = crate::native::design_eligibility(&ir, &opts);
        assert!(el.eligible, "{name}: expected an eligible design");
        let mut arena = NetArena::build(&ir, &opts).expect("arena builds");
        // The net is `args[0]` of the design's one system task — resolved from the
        // IR rather than by name, because net names live in `SimOpts`, not the IR.
        let net = ir
            .stmts
            .iter()
            .find_map(|st| match st {
                Stmt::SysTask { args, .. } => match ir.exprs.get(*args.first()? as usize) {
                    Some(sim_ir::Expr::Signal { net, .. }) => Some(*net),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name}: no net-valued first argument"));

        let cap = Cap::default();
        let sink = NullSink;
        let mut st = crate::SimState::new(
            &ir,
            Box::new(cap.clone()),
            &sink,
            "1ns".to_string(),
            "test".to_string(),
            None,
        );
        // Give both stores the SAME defined values first, so the ONE diverging net
        // is the only thing that can explain a difference. (Without this the other
        // nets sit at their t0 X in the arena and every render is `xxxx`, which
        // hides the very thing under test.)
        let mut rng = Rng::new(0x4B4F_0001);
        mirror_state(
            &mut st,
            &mut arena,
            &mut rng,
            ir.nets.len() as u32,
            true,
            false,
        );
        st.now = 7_000;

        let w = ir.nets[net as usize].width.max(1);
        let words = vec![arena_val; 1];
        arena.set_elem(net, 0, &words, &[0u64]);
        for i in 0..w {
            let bit = if i < 64 { (state_val >> i) & 1 } else { 0 };
            set_bit(&mut st.nets[net as usize].cur, i, bit, 0);
        }

        let empty: BTreeMap<u32, u32> = BTreeMap::new();
        {
            let mut sched = Scheduler::new(&mut st, 33_000, 10_000, None, Default::default());
            let mut nk = NativeKernel::new(&ir, arena, &mut sched, &empty, 10_000);
            for (sid, stmt) in ir.stmts.iter().enumerate() {
                if let Stmt::SysTask { which, fmt, args } = stmt {
                    nk.k_dispatch_systask(*which, *fmt, args, sid as u32);
                }
            }
        }
        let out = String::from_utf8_lossy(&cap.0.borrow()).into_owned();
        assert!(
            out.contains(expect),
            "{name}: output {out:?} does not contain {expect:?} — the argument was \
             read from `SimState` (which holds {state_val:#x}) rather than the \
             arena (which holds {arena_val:#x})"
        );
    }
}
