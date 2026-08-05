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
    assert_stores_equal, build_with_opts, fresh_state, fresh_state_with, mirror_state, set_bit,
    NullSink,
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
/// Transport-delay NBAs are admitted since S1d-4c-1 built `k_schedule_nba_at`.
/// Note the S0 gate still does NOT inspect `NonblockingAssign.delay` — it never
/// needed to, and saying so keeps a reader from concluding the gate covers
/// something it does not.
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
                // Transport-delay NBAs are ADMITTED now: S1d-4c-1 implemented
                // `k_schedule_nba_at`. The filter previously excluded them and
                // said so, because the method was `not_built!`.
                Stmt::NonblockingAssign { .. } => true,
                _ => false,
            }
        })
        .collect()
}

/// Multiset difference `after - before`, sorted. One spelling for both sides, so
/// a bug in the differencing cannot make them agree.
fn resume_delta(
    before: &[(u64, bool, u32, u32)],
    after: &[(u64, bool, u32, u32)],
) -> Vec<(u64, bool, u32, u32)> {
    let mut rest: Vec<(u64, bool, u32, u32)> = before.to_vec();
    let mut out = Vec::new();
    for e in after {
        match rest.iter().position(|x| x == e) {
            Some(i) => {
                rest.swap_remove(i);
            }
            None => out.push(*e),
        }
    }
    out.sort_unstable();
    out
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
        // A NON-ZERO `now`, on BOTH sides. `k_schedule_nba_at` files a transport
        // under `now + ticks`, and with `now == 0` everywhere that term is
        // indistinguishable from `ticks` alone — measured, replacing it with
        // `ticks` survived the entire workspace suite. In production the same
        // mutation would file every transport under a key BELOW `now`, which is
        // never drained: the update vanishes silently.
        const WALK_NOW: u64 = 7;
        st.now = WALK_NOW;
        nk.sched.st.now = WALK_NOW;
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
        // ── S1d-4c-1: DRAIN both queues and compare the stores again ──
        //
        // Queueing was already compared entry by entry above, but a queue that
        // holds the right entries and applies them in the wrong ORDER lands
        // different bits — NBA order is `seq` order (statement order), not queue
        // order, and the two differ the moment a transport update joins the tick.
        // Draining is also the only thing that makes a transport NBA observable
        // at all: it writes nothing when scheduled.
        // TICK BY TICK, not once: a transport update is filed under `now + d`,
        // so a single drain at `now` reaches only the same-tick queue and leaves
        // every delayed bucket untouched — which is what made "file it at `now`"
        // and "never move the due bucket" both survive mutation. The range covers
        // the largest delay these designs use.
        let now = sched.st.now;
        for tick in now..=now + 8 {
            sched.take_due_delayed(tick);
            nk.take_due_delayed(tick);
            sched.apply_nba();
            nk.apply_nba();
            assert_stores_equal(
                sched.st,
                &nk.arena,
                n_nets,
                &format!("{name}/pass{pass}/tick{tick}/after-nba-drain"),
            );
        }
        // The window is now ASSERTED, not assumed. A bucket filed past `now + 8`
        // was silently dropped on BOTH sides, so raising a design's delays past
        // the window (or mis-filing every bucket at `ticks * 2`) left the gate
        // green — measured. This turns the cliff into a failure.
        assert!(
            nk.delayed_nba.is_empty(),
            "{name}/pass{pass}: transport buckets remain past the drain window \
             ({:?}) — widen the window or the walk is not draining them",
            nk.delayed_nba.keys().collect::<Vec<_>>()
        );

        // Both queues are empty after `apply_nba` hands its drained Vec back, so
        // there is nothing to clear here — the line that used to do it, and the
        // comment explaining why, described a state that no longer exists. What
        // DOES need resetting is the kernel's delayed map: the engine's dies with
        // its per-pass `Scheduler` and the kernel's would otherwise carry buckets
        // into the next pass.
        assert!(sched.nba.is_empty() && nk.nba.is_empty());
    }
    // (`drained` was asserted == 36 here; it is 4 x 9 from two loop literals and
    // no design property can move it, so it said nothing. The window assertion
    // above is what that check was reaching for.)
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

/// TRANSPORT-delay NBAs (`a <= #3 b`), which S1d-4c-1 implemented and which the
/// corpus does not contain a single one of (measured). They are the only shape
/// where the queue's ORDER can differ from its contents: a delayed update filed
/// under an earlier tick and a same-tick update interleave by `seq`, not by
/// which queue they came from — so the drain, not the push, is what can get them
/// wrong. And a transport NBA writes nothing when scheduled, so before the drain
/// existed it was unobservable in this walk regardless.
#[test]
fn s1d4c1_transport_delay_nbas_drain_in_seq_order() {
    let designs: [(&str, &str); 4] = [
        (
            "transport_mixed_with_same_tick",
            "module t;\n\
               reg [15:0] a; reg [15:0] b; reg [15:0] c;\n\
               initial begin\n\
                 b = 16'h1111; c = 16'h2222;\n\
                 a <= #3 b;  a <= c;  a <= #1 (b ^ c);\n\
                 a <= b + c; a <= #2 16'hbeef;\n\
               end\n\
             endmodule\n",
        ),
        (
            "transport_same_destination_last_wins",
            "module t;\n\
               reg [7:0] d; reg [7:0] s1; reg [7:0] s2;\n\
               initial begin\n\
                 s1 = 8'haa; s2 = 8'h55;\n\
                 d <= #1 s1; d <= #1 s2; d <= #2 (s1 & s2); d <= #1 (s1 | s2);\n\
               end\n\
             endmodule\n",
        ),
        (
            // CONCAT destination — `NbaLhs::of`'s two-arm split. Measured: forcing
            // the `One` arm (so `{a,b} <= #1 w` writes only `a`, from the wrong
            // half of `w`) survived every test in the package. The same-tick twin
            // `nba_concat_lhs` exists for exactly this hazard; its transport
            // counterpart did not.
            "transport_concat_lhs",
            "module t;\n\
               reg [7:0] hi; reg [7:0] lo; reg [15:0] w;\n\
               initial begin\n\
                 w = 16'h1234; hi = 8'h00; lo = 8'h00;\n\
                 {hi, lo} <= #1 w;  {lo, hi} <= #2 ~w;\n\
                 {hi, lo} <= w + 16'h1111;\n\
               end\n\
             endmodule\n",
        ),
        (
            "transport_into_array_and_partselect",
            "module t;\n\
               reg [31:0] w; reg [7:0] m [0:3]; integer i; reg [7:0] v;\n\
               initial begin\n\
                 i = 2; v = 8'h3c;\n\
                 m[i] <= #2 v; w[15:8] <= #1 v; i = 0; m[i] <= #3 ~v;\n\
                 w[7:0] <= v; w[31:24] <= #1 8'hff;\n\
               end\n\
             endmodule\n",
        ),
    ];
    let mut compared = 0usize;
    for (i, (name, src)) in designs.iter().enumerate() {
        let n = s1d4a_walk(src, name, 0x4C10_0000 + i as u64);
        assert!(n > 0, "{name}: produced no comparisons");
        // The design must actually carry a transport NBA, or the walk is testing
        // the same-tick path under a name that says otherwise.
        let (ir, _) = build_with_opts(src);
        let transports = ir
            .stmts
            .iter()
            .filter(|s| matches!(s, Stmt::NonblockingAssign { delay: Some(_), .. }))
            .count();
        assert!(transports > 0, "{name}: no transport-delay NBA in the IR");
        compared += n;
    }
    assert_eq!(
        compared, 108,
        "transport coverage moved — re-pin deliberately"
    );
}

/// The shapes the corpus does not carry: a nonblocking assign to a dynamic
/// array element (the NBA sample-at-schedule rule), a concat LHS, a part-select
/// destination, an X index, and 2-state destinations. Each is a place where a
/// kernel could get the STORE right and the queued SAMPLE wrong.
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
        // A NON-ZERO `now`, on BOTH sides. `k_schedule_nba_at` files a transport
        // under `now + ticks`, and with `now == 0` everywhere that term is
        // indistinguishable from `ticks` alone — measured, replacing it with
        // `ticks` survived the entire workspace suite. In production the same
        // mutation would file every transport under a key BELOW `now`, which is
        // never drained: the update vanishes silently.
        const WALK_NOW: u64 = 7;
        st.now = WALK_NOW;
        nk.sched.st.now = WALK_NOW;
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
        (5550, 1386),
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
        // ⚠️ `$dumpvars` and `$dumpfile` LEFT this list in S1d-4d-2 — the first
        // because `full_snapshot_with` takes the arena as its reader, the second
        // because its argument is a string constant and never reaches the store
        // at all. What remains genuinely reads somewhere this seam does not go.
        let expect_refusal = matches!(
            which,
            sim_ir::SysTaskId::DumpAll
                | sim_ir::SysTaskId::DumpOn
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
        refused, 8,
        "expected all eight refused tasks to refuse — got {refused}"
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

/// The `seq` SORT in `apply_nba`, which nothing else here can reach.
///
/// NBA order is statement order, not queue order, and the two diverge only when
/// a delayed bucket merges into a queue that still holds same-tick entries —
/// entries with HIGHER `seq` than the delayed ones scheduled before them. The
/// walk cannot produce that: it drains at every tick, so by the time a bucket
/// arrives the same-tick queue is already empty, and removing the sort survived
/// every other test.
///
/// So the configuration is built directly: schedule a mix, merge every bucket
/// WITHOUT draining in between, then apply once.
///
/// ⚠️ The store comparison is NOT what has teeth here, and saying so matters:
/// both kernels get the identical call sequence, so removing the sort from BOTH
/// leaves them agreeing — measured. What catches that is the explicit
/// highest-seq expectation at the end. The store compare catches only a
/// kernel-ONLY divergence. Two assertions, two different failures.
#[test]
fn s1d4c1_apply_nba_sorts_by_seq_not_by_queue() {
    let src = "module t;\n\
                 reg [15:0] a; reg [15:0] p; reg [15:0] q;\n\
                 initial begin p = 16'h1111; q = 16'h2222; end\n\
               endmodule\n";
    let (ir, opts) = build_with_opts(src);
    let arena = NetArena::build(&ir, &opts).expect("arena builds");
    let sink = NullSink;
    let mut st_e = fresh_state(&ir, &sink);
    let mut st_n = fresh_state(&ir, &sink);
    let empty: BTreeMap<u32, u32> = BTreeMap::new();
    // One destination, so ORDER decides the surviving value.
    let dest = ir
        .stmts
        .iter()
        .find_map(|s| match s {
            Stmt::BlockingAssign { lhs, .. } => Some(lhs.clone()),
            _ => None,
        })
        .expect("an lvalue to reuse");

    let mut sched_n = Scheduler::new(&mut st_n, 33_000, 10_000, None, Default::default());
    let mut nk = NativeKernel::new(&ir, arena, &mut sched_n, &empty, 10_000);
    let mut sched_e = Scheduler::new(&mut st_e, 33_000, 10_000, None, Default::default());

    // Interleave: delayed, same-tick, delayed, same-tick … so the merged queue
    // is NOT in `seq` order and only the sort can restore it.
    for (i, ticks) in [Some(2u64), None, Some(1), None, Some(2), Some(1), None]
        .iter()
        .enumerate()
    {
        let v = crate::value::Value::from_i128(0x1000 + i as i128, 16, false);
        match ticks {
            Some(t) => {
                sched_e.k_schedule_nba_at(&dest, v.clone(), *t);
                nk.k_schedule_nba_at(&dest, v, *t);
            }
            None => {
                sched_e.k_schedule_nba(&dest, v.clone());
                nk.k_schedule_nba(&dest, v);
            }
        }
    }
    // Merge every bucket BEFORE applying — the whole point.
    for tick in 0..=3u64 {
        sched_e.take_due_delayed(tick);
        nk.take_due_delayed(tick);
    }
    assert_eq!(
        nk.nba.len(),
        7,
        "expected exactly every update queued, once"
    );
    assert!(
        nk.nba.windows(2).any(|w| w[0].seq > w[1].seq),
        "the merged queue must be OUT of seq order, or the sort is a no-op here"
    );
    sched_e.apply_nba();
    nk.apply_nba();
    assert_stores_equal(sched_e.st, &nk.arena, ir.nets.len() as u32, "seq-sort");
    // …and the surviving value must be the LAST by seq, not the last in queue
    // order — pinned explicitly so "both agree" cannot mean "both wrong".
    let net = dest.chunks[0].net;
    assert_eq!(
        crate::eval::NetReader::read_net(&nk.arena, net, None).to_u64(),
        Some(0x1006),
        "the highest-seq update must win"
    );
}

/// S1d-4c-2a — **`k_rearm` against the ENGINE, not against my expectations.**
///
/// Re-arming is one `match` and its whole content is an asymmetry: `Edge` and
/// `Initial` must NOT re-register (an edge entry is read, not consumed, so
/// re-registering makes a process fire 2^k times on edge k), while `Comb`,
/// `Latch` and `Level` MUST (their waiter is consumed when it fires, so without
/// it they wake once and never again). Getting either half backwards is silent —
/// no value is wrong, a process just fires too often or stops firing.
///
/// **The first version compared against hard-coded booleans while its name and
/// doc claimed a differential.** It never called `Scheduler::rearm` nor read a
/// single engine waiter. Building the real one immediately found that
/// `rearm_level` was a PARTIAL restatement: `arm_sensitivity` registers nothing
/// when the read set is empty, and the kernel armed unconditionally — divergent
/// on `always_comb o = 1'b0;`, a design the gate reports eligible and buildable.
///
/// The engine-side observable is "is there a live static-level waiter", which is
/// exactly what `level_armed` models; both are read after the same
/// consume-then-re-arm sequence.
#[test]
fn s1d4c2a_rearm_matches_the_engine_on_both_halves_of_the_asymmetry() {
    // One design per sensitivity kind the S0 gate admits — and the first version
    // said exactly that while carrying no `Latch` at all, so moving `Latch` into
    // the do-nothing arm survived the whole package. The corpus cannot cover it
    // either: measured, it carries `Comb` and `Edge` only.
    let designs: [(&str, &str); 6] = [
        (
            "edge",
            "module t;\n\
               reg clk; reg [7:0] q; reg [7:0] d;\n\
               always @(posedge clk) q <= d;\n\
               initial begin clk = 0; d = 8'h11; end\n\
             endmodule\n",
        ),
        (
            "level",
            "module t;\n\
               reg a; reg b; reg [7:0] y;\n\
               always @(a or b) y = {7'b0, a ^ b};\n\
               initial begin a = 0; b = 0; end\n\
             endmodule\n",
        ),
        (
            "comb",
            "module t;\n\
               reg [7:0] x; reg [7:0] z;\n\
               always @(*) z = x + 8'd1;\n\
               initial x = 8'h20;\n\
             endmodule\n",
        ),
        (
            "latch",
            "module t;\n\
               reg en; reg [7:0] din; reg [7:0] dout;\n\
               always_latch if (en) dout = din;\n\
               initial begin en = 0; din = 8'h5a; end\n\
             endmodule\n",
        ),
        (
            // EMPTY read set: `arm_sensitivity` registers nothing, so re-arming
            // must register nothing. This is the shape the two diverged on.
            "comb_empty_readset",
            "module t;\n\
               reg [7:0] o;\n\
               always @(*) o = 8'd0;\n\
               initial begin o = 8'd1; end\n\
             endmodule\n",
        ),
        (
            "mixed",
            "module t;\n\
               reg clk; reg en; reg [7:0] r; reg [7:0] c;\n\
               always @(posedge clk) r <= r + 8'd1;\n\
               always @(*) c = r ^ {7'b0, en};\n\
               initial begin clk = 0; en = 0; r = 0; end\n\
             endmodule\n",
        ),
    ];
    let mut kinds_seen: std::collections::BTreeSet<String> = Default::default();
    let mut compared = 0usize;
    for (name, src) in designs {
        let (ir, opts) = build_with_opts(src);
        let arena = NetArena::build(&ir, &opts).expect("arena builds");
        let sink = NullSink;
        // TWO states: the engine arms on its own, the kernel on its own.
        let mut st_e = fresh_state(&ir, &sink);
        let mut st_n = fresh_state(&ir, &sink);
        let empty: BTreeMap<u32, u32> = BTreeMap::new();
        let mut sched_e = Scheduler::new(&mut st_e, 33_000, 10_000, None, opts.fork_modes.clone());
        sched_e.arm_processes();
        let mut sched_n = Scheduler::new(&mut st_n, 33_000, 10_000, None, Default::default());
        let mut nk = NativeKernel::new(&ir, arena, &mut sched_n, &empty, 10_000);

        // Snapshot the edge registrations BEFORE any re-arm, on both sides.
        let edges_before_e: Vec<usize> = (0..ir.processes.len() as u32)
            .map(|p| sched_e.edge_registration_count(p))
            .collect();
        let edges_before_n: Vec<usize> = (0..ir.processes.len() as u32)
            .map(|p| nk.wake.edge_registration_count(p))
            .collect();
        for (pi, p) in ir.processes.iter().enumerate() {
            let kind = p.sensitivity.kind;
            kinds_seen.insert(format!("{kind:?}"));
            let pi = pi as u32;
            // t0 state must already agree, or the re-arm comparison starts from
            // two different places.
            assert_eq!(
                sched_e.has_static_level_waiter(pi),
                nk.wake.level_armed_for_test(pi),
                "{name}/proc{pi}/{kind:?}: t0 arm state diverged"
            );
            // CONSUME as a fire does, then re-arm on BOTH sides.
            sched_e.consume_static_level_waiter_for_test(pi);
            nk.wake.set_level_armed_for_test(pi, false);
            sched_e.rearm(pi);
            nk.k_rearm(pi);
            assert_eq!(
                sched_e.has_static_level_waiter(pi),
                nk.wake.level_armed_for_test(pi),
                "{name}/proc{pi}/{kind:?}: arm state after re-arm diverged"
            );
            // …and the EDGE half. Without this the differential was one-sided:
            // the level probe cannot see `net_to_edge`, so an engine `rearm` that
            // re-registered an Edge — the 2^k bug this whole asymmetry exists to
            // prevent — passed. Measured, on the engine side.
            assert_eq!(
                sched_e.edge_registration_count(pi),
                edges_before_e[pi as usize],
                "{name}/proc{pi}/{kind:?}: the engine's EDGE registrations changed \
                 across re-arm — an edge entry is permanent, so re-registering \
                 makes the process fire 2^k times on edge k"
            );
            assert_eq!(
                nk.wake.edge_registration_count(pi),
                edges_before_n[pi as usize],
                "{name}/proc{pi}/{kind:?}: the kernel's EDGE registrations changed"
            );
            assert_eq!(
                sched_e.edge_registration_count(pi),
                nk.wake.edge_registration_count(pi),
                "{name}/proc{pi}/{kind:?}: edge registration COUNT diverged"
            );
            compared += 1;
        }
    }
    // Every kind the gate admits must appear, NAMED — a floor on a count let
    // `Latch` go missing while two `Comb` satisfied the number.
    for k in ["Edge", "Level", "Comb", "Latch", "Initial"] {
        assert!(
            kinds_seen.contains(k),
            "sensitivity kind {k} never appeared — the designs do not cover the \
             match. Seen: {kinds_seen:?}"
        );
    }
    assert_eq!(compared, 13, "re-arm coverage moved — re-pin deliberately");
}

/// Is `Initial` ever a process with a NON-EMPTY sensitivity read set? If it were,
/// folding it into `k_rearm`'s re-arm arm would be a real defect; if it never is,
/// that fold is an EQUIVALENT mutation and saying so is more useful than a test
/// that cannot distinguish it.
///
/// Measured here rather than asserted from the grammar, over the P6 corpus plus
/// every hand-written design in this file's sensitivity set.
#[test]
fn s1d4c2a_initial_processes_never_carry_a_read_set() {
    let mut initials = 0usize;
    for d in corpus(0x5EED_F00D, 72) {
        let (ir, _) = build_with_opts(&d.src);
        for p in &ir.processes {
            if p.sensitivity.kind == sim_ir::SensKind::Initial {
                assert!(
                    p.sensitivity.edges.is_empty(),
                    "{}: an Initial process carries a read set — `k_rearm`'s \
                     Edge|Initial arm is then load-bearing on its own",
                    d.name
                );
                initials += 1;
            }
        }
    }
    assert!(
        initials >= 50,
        "too few Initial processes seen ({initials})"
    );
}

/// S1d-4c-2b — **whole PROCESS BODIES, not statements.**
///
/// Every earlier gate drove one statement at a time through the shared executor.
/// This drives the block loop: `Goto`, `Branch` and `Return`, the statement
/// sequence inside each block, and the in-body step guard.
///
/// It does NOT drive the `call_fatal` boundary check, and an earlier version of
/// this sentence claimed it did. That check cannot fire for the class the gate
/// admits — every site that latches `call_fatal` is frame machinery, which needs
/// a `func_table`, which the arena refuses — so deleting it survives the whole
/// workspace. It is kept because the same loop is generic and the rule is
/// load-bearing for `K = Scheduler`; see the note at its call site. Those are `run_process`'s decisions, and `run_process` is
/// `Scheduler`-fixed, so they are the one part that had to be restated — which
/// makes them the one part a differential has to cover.
///
/// The engine side runs its own `run_process` on its own state; the arena side
/// runs `native::body::run_body`. Same design, same start state, and afterwards
/// the two stores must agree — a control-flow decision that differs shows up as
/// different bits, because a taken branch writes different things.
fn s1d4c2b_body_walk(src: &str, name: &str, seed: u64) -> usize {
    let (ir, opts) = build_with_opts(src);
    let Ok(arena) = NetArena::build(&ir, &opts) else {
        return 0;
    };
    // Only processes the walk can run. TWO conditions, and the second was
    // discovered by this gate the moment S1d-4c-2c made `Delay` walkable: the
    // corpus bodies that suspend also carry `$dumpfile`/`$dumpvars`, which
    // `k_dispatch_systask` refuses, so admitting them on the terminator scan
    // alone panicked. Both halves are the SAME predicate the production run gate
    // asks (`native::run::body_admissible`) rather than a local restatement.
    let runnable: Vec<u32> = (0..ir.processes.len() as u32)
        .filter(|&i| crate::native::run::body_admissible(&ir, i))
        .collect();
    if runnable.is_empty() {
        return 0;
    }
    let sink = NullSink;
    let mut st_e = fresh_state_with(&ir, &sink, &opts);
    let mut st_n = fresh_state_with(&ir, &sink, &opts);
    let n_nets = ir.nets.len() as u32;
    let empty: BTreeMap<u32, u32> = BTreeMap::new();
    let mut arena = arena;
    let mut rng = Rng::new(seed);
    let mut compared = 0usize;
    let mut nba_applied = 0usize;

    for pass in 0..4 {
        // Same start state on both sides, then each runs its own executor.
        mirror_state(
            &mut st_e,
            &mut arena,
            &mut rng,
            n_nets,
            pass % 2 == 1,
            pass >= 2,
        );
        {
            let mut scratch = NetArena::build(&ir, &opts).expect("arena rebuilds");
            let mut rng2 = Rng::new(seed);
            for _ in 0..=pass {
                mirror_state(
                    &mut st_n,
                    &mut scratch,
                    &mut rng2,
                    n_nets,
                    pass % 2 == 1,
                    pass >= 2,
                );
            }
        }
        assert_stores_equal(&st_e, &arena, n_nets, &format!("{name}/pass{pass}/start"));

        let mut sched_e = Scheduler::new(&mut st_e, 33_000, 10_000, None, opts.fork_modes.clone());
        sched_e.arm_processes();
        let mut sched_n = Scheduler::new(&mut st_n, 33_000, 10_000, None, Default::default());
        let mut nk = NativeKernel::new(&ir, arena, &mut sched_n, &empty, 10_000);

        for &pi in &runnable {
            let entry = ir.processes[pi as usize].entry;
            let before_e = sched_e.pending_resumes_for_test();
            let before_n = nk.pending_resumes_for_test();
            let step_e = crate::exec::run_process(&mut sched_e, pi, entry);
            let step_n = crate::native::body::run_body(&mut nk, &ir, pi, entry);
            assert_eq!(
                format!("{step_e:?}"),
                format!("{step_n:?}"),
                "{name}/pass{pass}/proc{pi}: the two walks ended differently"
            );
            // NBA QUEUES FIRST — 46 of the 83 statements these bodies execute are
            // nonblocking assigns, and an NBA's entire effect is a queue push.
            // A store-only comparison saw none of them: measured, dropping EVERY
            // NBA from the walk left all three tests green. The gate's own claim
            // that "a control-flow decision that differs shows up as different
            // bits" was false for the majority of what it ran.
            assert_eq!(
                sched_e.nba.len(),
                nk.nba.len(),
                "{name}/pass{pass}/proc{pi}: NBA queue depth diverged"
            );
            for (a, b) in sched_e.nba.iter().zip(nk.nba.iter()) {
                assert_eq!(
                    (a.seq, &a.sampled, a.offsets.as_slice(), nba_dest(a)),
                    (b.seq, &b.sampled, b.offsets.as_slice(), nba_dest(b)),
                    "{name}/pass{pass}/proc{pi}: NBA entry diverged"
                );
            }
            // THE RESUME, which neither the store nor the NBA queue can see: a
            // `Terminator::Delay` writes nothing at all — its whole effect is
            // WHERE it filed the activation. Compared as a multiset difference
            // against the pre-body snapshot because the engine enters with a t0
            // Active queue this kernel does not have. (Intra-tick ORDER is not
            // compared here and does not need to be: a body suspends at most
            // once, so each delta is a single entry. Order is what the
            // end-to-end run differential compares, through stdout.)
            let after_e = sched_e.pending_resumes_for_test();
            let after_n = nk.pending_resumes_for_test();
            assert_eq!(
                resume_delta(&before_e, &after_e),
                resume_delta(&before_n, &after_n),
                "{name}/pass{pass}/proc{pi}: the two walks filed the resume differently"
            );
            // …then APPLY them, so the queued values land in the stores and the
            // comparison below covers what the NBAs actually wrote.
            sched_e.apply_nba();
            nk.apply_nba();
            nba_applied += 1;
            assert_stores_equal(
                sched_e.st,
                &nk.arena,
                n_nets,
                &format!("{name}/pass{pass}/proc{pi}/after-body"),
            );
            // The store is not the only thing a body walk changes. `Return` calls
            // `k_rearm`, and `enter_body` installs the per-process context —
            // neither writes a net, so both were invisible to a store-only
            // comparison (measured: skipping either survived).
            assert_eq!(
                sched_e.has_static_level_waiter(pi),
                nk.wake.level_armed_for_test(pi),
                "{name}/pass{pass}/proc{pi}: arm state after the body diverged"
            );
            assert_eq!(
                (
                    sched_e.st.cur_time_mult,
                    sched_e.st.cur_prec_mult,
                    sched_e.st.cur_scope.clone()
                ),
                (
                    nk.sched.st.cur_time_mult,
                    nk.sched.st.cur_prec_mult,
                    nk.sched.st.cur_scope.clone()
                ),
                "{name}/pass{pass}/proc{pi}: per-process context diverged — \
                 `enter_body` sets `$time`'s multiplier and `%m`'s scope"
            );
            compared += 1;
        }
        drop(sched_e);
        arena = nk.arena;
    }
    assert!(nba_applied > 0, "{name}: no body ran");
    compared
}

#[test]
fn s1d4c2b_body_walk_matches_the_engine_over_corpus() {
    let mut compared = 0usize;
    let mut designs = 0usize;
    for (i, d) in corpus(0x5EED_F00D, 72).into_iter().enumerate() {
        let n = s1d4c2b_body_walk(&d.src, &d.name, 0x4C20_0000 + i as u64);
        if n > 0 {
            designs += 1;
        }
        compared += n;
    }
    // Not a floor, and the number has moved twice. **30 of 72** when `Delay` was
    // unwalkable, **58** when S1d-4c-2c gave it an arm, and **all 72** since
    // S1d-4d-2 wired `$dumpfile`/`$dumpvars` — the 14 that were absent carried a
    // dump task, which `body_dispatch_ok` refused.
    assert_eq!(
        (designs, compared),
        (72, 556),
        "body-walk coverage moved — re-pin deliberately"
    );
}

/// Multi-block suspend-free bodies — `Goto` and `Branch`, which the corpus walk
/// cannot reach.
///
/// Measured: `Goto → Done` and swapping the `Branch` arms BOTH survive the
/// corpus walk. (An earlier version of this comment explained that as "every
/// runnable corpus body is a single block" — 8 of the 67 have two or more
/// reachable blocks, so the explanation was wrong even though the consequence it
/// predicted is real and reproduced.) These designs are what catches them.
/// Two things a single-module design cannot exercise: the per-process CONTEXT
/// (`%m`'s scope differs per instance, so `enter_body` has something to install)
/// and the in-body STEP GUARD (a body long enough to exceed a small budget).
///
/// Both survived mutation without this: with one module every process shares one
/// scope, so skipping `enter_body` changed nothing observable, and no corpus body
/// iterates enough to reach any plausible budget.
#[test]
fn s1d4c2b_body_walk_agrees_on_context_and_the_step_guard() {
    // Two INSTANCES, so `proc_scopes` differ and `enter_body` has work to do.
    let src = "module leaf(input [7:0] i, output reg [7:0] o);\n\
                 always @(*) o = i + 8'd1;\n\
               endmodule\n\
               module t;\n\
                 reg [7:0] a; wire [7:0] x; reg [7:0] b; wire [7:0] y;\n\
                 leaf u1(.i(a), .o(x));\n\
                 leaf u2(.i(b), .o(y));\n\
                 initial begin a = 8'h10; b = 8'h20; end\n\
               endmodule\n";
    let (ir, opts) = build_with_opts(src);
    let scopes: std::collections::BTreeSet<&String> = (0..ir.processes.len())
        .filter(|&pi| crate::native::run::body_admissible(&ir, pi as u32))
        .filter_map(|pi| opts.proc_scopes.get(pi))
        .collect();
    assert!(
        scopes.len() >= 2,
        "the design must give its runnable processes DIFFERENT scopes, or \
         `enter_body` has nothing to install: {scopes:?}"
    );
    let n = s1d4c2b_body_walk(src, "two_instances", 0x4C2C_0000);
    assert!(n > 0, "produced no comparisons");

    // STEP GUARD: a bounded loop with a budget below its iteration count. Both
    // sides must reach Fatal, and by the same route.
    let loop_src = "module t;\n\
                      reg [7:0] acc; integer i;\n\
                      always @(*) begin\n\
                        acc = 8'd0;\n\
                        for (i = 0; i < 40; i = i + 1) acc = acc + 8'd1;\n\
                      end\n\
                      initial acc = 0;\n\
                    endmodule\n";
    let (ir2, opts2) = build_with_opts(loop_src);
    let arena = NetArena::build(&ir2, &opts2).expect("arena builds");
    let sink = NullSink;
    let mut st_e = fresh_state_with(&ir2, &sink, &opts2);
    let mut st_n = fresh_state_with(&ir2, &sink, &opts2);
    let empty: BTreeMap<u32, u32> = BTreeMap::new();
    let pi = (0..ir2.processes.len() as u32)
        .find(|&p| {
            crate::native::run::body_admissible(&ir2, p) && ir2.processes[p as usize].body.len() > 1
        })
        .expect("a multi-block runnable body");
    // Budget of 20 against 40 iterations: the guard MUST fire on both sides. The
    // budget sits BETWEEN one and two counts per block on purpose — counting
    // twice per iteration would fatal at a different point, and with a budget far
    // below the count both spellings fatal alike (measured: `guard += 2` survived).
    let mut sched_e = Scheduler::new(&mut st_e, 33_000, 20, None, Default::default());
    sched_e.arm_processes(); // `run_process` indexes `activities`
    let mut sched_n = Scheduler::new(&mut st_n, 33_000, 20, None, Default::default());
    let mut nk = NativeKernel::new(&ir2, arena, &mut sched_n, &empty, 20);
    let entry = ir2.processes[pi as usize].entry;
    let step_e = crate::exec::run_process(&mut sched_e, pi, entry);
    let step_n = crate::native::body::run_body(&mut nk, &ir2, pi, entry);
    assert_eq!(
        format!("{step_e:?}"),
        format!("{step_n:?}"),
        "the step guard ended the two walks differently"
    );
    assert_eq!(
        format!("{step_e:?}"),
        "Fatal",
        "the guard must actually fire"
    );
    // …and the store at the moment it fired must MATCH, which is what pins WHEN
    // it fired rather than merely that it did. Without this, counting twice per
    // block — or a hundred times — still ended in `Fatal` and survived.
    assert_stores_equal(
        sched_e.st,
        &nk.arena,
        ir2.nets.len() as u32,
        "step-guard/at-fatal",
    );
    // …and the fatal must have been REPORTED on both sides. `k_mark_fatal` on the
    // kernel used to set a local flag nothing read, so the engine emitted
    // `RunBodyStepLimit` and set the exit-class bit while the native side hit its
    // step limit in silence — and `Step::Fatal` plus a matching store said
    // nothing about that. For a correct-or-loud project the guard's whole value
    // is the report.
    assert!(
        sched_e.st.had_fatal,
        "the engine did not record the fatal — the oracle for this assertion is wrong"
    );
    assert!(
        nk.sched.st.had_fatal,
        "the tier-3 kernel hit its step limit WITHOUT reporting it: \
         `k_mark_fatal` must reach the same diagnostic the engine emits"
    );
}

#[test]
fn s1d4c2b_body_walk_agrees_on_multi_block_bodies() {
    let designs: [(&str, &str); 4] = [
        (
            "if_else_chain",
            "module t;\n\
               reg [7:0] sel; reg [7:0] y;\n\
               always @(*) begin\n\
                 if (sel < 8'd4) y = 8'h11;\n\
                 else if (sel < 8'd8) y = 8'h22;\n\
                 else if (sel[0]) y = 8'h33;\n\
                 else y = 8'h44;\n\
               end\n\
               initial sel = 8'd0;\n\
             endmodule\n",
        ),
        (
            "case_and_nested",
            "module t;\n\
               reg [7:0] op; reg [7:0] a; reg [7:0] b; reg [7:0] r;\n\
               always @(*) begin\n\
                 case (op[1:0])\n\
                   2'd0: r = a + b;\n\
                   2'd1: r = a - b;\n\
                   2'd2: begin if (a > b) r = a; else r = b; end\n\
                   default: r = 8'hff;\n\
                 endcase\n\
               end\n\
               initial begin op = 0; a = 8'h10; b = 8'h20; end\n\
             endmodule\n",
        ),
        (
            "bounded_loop",
            "module t;\n\
               reg [7:0] src; reg [7:0] acc; integer i;\n\
               always @(*) begin\n\
                 acc = 8'd0;\n\
                 for (i = 0; i < 8; i = i + 1)\n\
                   if (src[i]) acc = acc + 8'd1;\n\
               end\n\
               initial src = 8'ha5;\n\
             endmodule\n",
        ),
        (
            "loop_with_break_and_continue",
            "module t;\n\
               reg [15:0] v; reg [7:0] first; integer j;\n\
               always @(*) begin\n\
                 first = 8'hff;\n\
                 for (j = 0; j < 16; j = j + 1) begin\n\
                   if (!v[j]) continue;\n\
                   first = j[7:0];\n\
                   break;\n\
                 end\n\
               end\n\
               initial v = 16'h0100;\n\
             endmodule\n",
        ),
    ];
    let mut compared = 0usize;
    for (i, (name, src)) in designs.iter().enumerate() {
        // The design must actually have a MULTI-BLOCK runnable body, or it is
        // testing the single-block path under a name that says otherwise.
        let (ir, _) = build_with_opts(src);
        let multi =
            ir.processes.iter().enumerate().any(|(pi, p)| {
                p.body.len() > 1 && crate::native::run::body_admissible(&ir, pi as u32)
            });
        assert!(multi, "{name}: no multi-block suspend-free body");
        let n = s1d4c2b_body_walk(src, name, 0x4C2B_0000 + i as u64);
        assert!(n > 0, "{name}: produced no comparisons");
        compared += n;
    }
    assert_eq!(
        compared, 32,
        "multi-block coverage moved — re-pin deliberately"
    );
}

/// `body_is_walkable` must answer about the entry the WALK will use, not
/// about the process's declared entry.
///
/// The two coincide for every caller today, which is exactly why the bug was
/// invisible: reverting the predicate to scan from `ir.processes[proc].entry`
/// survived the whole package. They come apart when a block is unreachable from
/// the declared entry but reachable from a RESUME point — and the resume
/// parameter is the only reason `run_body` takes an entry at all.
///
/// `disable <named block>` produces exactly that shape, and it is not gate-
/// refused: `DisableKind::Scope` is deliberately outside the `disable_fork` row,
/// so the design below is `eligible: true, buildable: true`.
#[test]
fn s1d4c2b_suspend_free_scan_answers_about_the_given_entry() {
    // The unreachable block must end in a terminator the scan REFUSES, and that
    // set has shrunk TWICE: S1d-4c-2c made `Delay` walkable, S1d-4c-2d made
    // `Wait{Edge|Level|Expr}` walkable. What is left is `Fork`, `Call` and a
    // wait on a named event — none of which can appear in a design that is BOTH
    // eligible and buildable, so this one is deliberately eligible and NOT
    // buildable (a framed task call behind a `disable`).
    //
    // That does not weaken the property. The scan is what protects the walk when
    // the sidecar reasoning is defeated — `fork_modes` and `func_table` ride the
    // `.velab` trailer and a truncated one makes such a design look admissible —
    // so "does the scan answer about the entry it was given" is exactly as
    // load-bearing as before.
    let src = "module t;\n\
                 reg [7:0] y;\n\
                 task automatic slow(); begin #5; end endtask\n\
                 initial begin : blk\n\
                   y = 8'd1;\n\
                   disable blk;\n\
                   slow();\n\
                 end\n\
               endmodule\n";
    let (ir, opts) = build_with_opts(src);
    let el = crate::native::design_eligibility(&ir, &opts);
    assert!(
        el.eligible,
        "the design must be within v1's SCOPE, or the scan is not what keeps it \
         out: {:?}",
        el.refused
    );
    assert!(
        NetArena::build(&ir, &opts).is_err(),
        "…and NOT buildable, which is now the only way to get a scan-refused \
         terminator into an eligible design"
    );
    let proc = 0u32;
    let entry = ir.processes[proc as usize].entry;
    let body = &ir.processes[proc as usize].body;

    // Reachable-from-entry set, so the unreachable blocks can be named.
    let mut reachable = vec![false; body.len()];
    let mut stack = vec![entry];
    while let Some(bb) = stack.pop() {
        if reachable[bb as usize] {
            continue;
        }
        reachable[bb as usize] = true;
        match &body[bb as usize].term {
            sim_ir::Terminator::Goto { target } => stack.push(*target),
            sim_ir::Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                stack.push(*then_bb);
                stack.push(*else_bb);
            }
            _ => {}
        }
    }
    let suspending_unreachable: Vec<u32> = (0..body.len() as u32)
        .filter(|&b| !reachable[b as usize])
        .filter(|&b| {
            matches!(
                body[b as usize].term,
                sim_ir::Terminator::Call { .. } | sim_ir::Terminator::Fork { .. }
            )
        })
        .collect();
    assert!(
        !suspending_unreachable.is_empty(),
        "the design must have a REFUSED-terminator block unreachable from entry, \
         or it cannot distinguish the two spellings — blocks={}, reachable={}",
        body.len(),
        reachable.iter().filter(|b| **b).count()
    );

    // From the declared entry the body IS suspend-free …
    assert!(
        crate::native::body::body_is_walkable(&ir, proc, entry),
        "scanning from the declared entry should find no refused terminator"
    );
    // … and from the resume point it is NOT. A predicate that ignored its entry
    // would answer `true` here and the walk would then panic on a caller that
    // had asked and been told yes.
    for b in suspending_unreachable {
        assert!(
            !crate::native::body::body_is_walkable(&ir, proc, b),
            "block {b} ends in a refused terminator, but the scan called it \
             walkable — it is answering about the process entry rather than the \
             given one"
        );
    }
}

/// S2 slice 3: the SPECIALIZED offset resolver against the canonical one.
///
/// `k_resolve_lvalue_offsets` runs once per assignment — 71.9k times on the
/// tier-3 hot design, measured, every one of them through the generic
/// evaluator — so it got a fast path. This is the anchor that keeps the two
/// answering the same question: every write site of every corpus design, over
/// several random 4-state states, `fast_offsets` (when it admits) must equal
/// `eval::resolve_offsets` exactly.
///
/// The dedicated designs below the corpus carry the shapes the corpus has no
/// instance of and that the rule is most easily got wrong on: an X/Z index, a
/// negative index, an index far past any net, and a two-chunk concat lvalue.
#[test]
fn s2_specialized_offsets_match_the_canonical_resolver() {
    let sink = NullSink;
    let extra: Vec<(&str, String)> = vec![
        (
            "idx_edge_cases",
            "module top;\n\
               reg [7:0] mem [0:3];\n\
               reg [15:0] bus;\n\
               reg [7:0] i;\n\
               reg [31:0] big;\n\
               integer neg;\n\
               initial begin\n\
                 i = 8'bxxxx_xxxx; neg = -3; big = 32'hFFFF_FFFF;\n\
                 mem[i] = 8'd1;\n\
                 mem[2] = 8'd2;\n\
                 bus[neg +: 4] = 4'hF;\n\
                 bus[big +: 4] = 4'h3;\n\
                 {bus[3:0], bus[7:4]} = 8'hAB;\n\
                 mem[i[1:0]] = 8'd5;\n\
               end\n\
             endmodule\n"
                .to_string(),
        ),
        (
            "idx_const_and_three_chunk",
            "module top;\n\
               reg [7:0] mem [0:3];\n\
               reg [15:0] bus;\n\
               reg [7:0] a, b, c;\n\
               initial begin\n\
                 mem[2'bx1] = 8'd1;\n\
                 mem[64'h1_0000_0000] = 8'd2;\n\
                 bus[2'bz0 +: 2] = 2'b11;\n\
                 {a, b, c} = 24'h123456;\n\
               end\n\
             endmodule\n"
                .to_string(),
        ),
        (
            "idx_zed_and_wide",
            "module top;\n\
               reg [7:0] mem [0:7];\n\
               reg [3:0] z4;\n\
               reg [95:0] wide;\n\
               initial begin\n\
                 z4 = 4'bzzzz; wide = 96'd9;\n\
                 mem[z4] = 8'd7;\n\
                 mem[wide] = 8'd8;\n\
               end\n\
             endmodule\n"
                .to_string(),
        ),
    ];
    let designs: Vec<(String, String)> = corpus(0x5EED_F00D, 72)
        .into_iter()
        .map(|d| (d.name.to_string(), d.src))
        .chain(extra.into_iter().map(|(n, s)| (n.to_string(), s)))
        .collect();
    let mut fast = 0usize;
    let mut declined = 0usize;
    for (name, src) in &designs {
        let (ir, opts) = build_with_opts(src);
        let Ok(arena) = NetArena::build(&ir, &opts) else {
            continue; // not a flat design — the native path never sees it
        };
        let empty: BTreeMap<u32, u32> = BTreeMap::new();
        let mut st = fresh_state(&ir, &sink);
        let mut st_n = fresh_state(&ir, &sink);
        let sites = super::tests::write_sites(&ir);
        let mut rng = Rng::new(0x0FF5_E750 ^ sites.len() as u64);
        // ONE kernel across every state, so the per-ExprId index cache is
        // consulted against states it was NOT populated from. Rebuilding the
        // kernel per state — which this test did first — makes every cache
        // hit trivially fresh and hides staleness entirely (measured: freezing
        // a `Prog` result into a `Const` passed both of this slice's tests).
        let mut sched_n = Scheduler::new(&mut st_n, 10_000, 10_000, None, Default::default());
        let mut nk = NativeKernel::new(&ir, arena, &mut sched_n, &empty, 10_000);
        for state_i in 0..4 {
            {
                let arena = nk.arena_mut_for_test();
                for n in 0..ir.nets.len() as u32 {
                    for e in 0..arena.slots[n as usize].elems {
                        super::tests::mirror_random_elem(
                            &mut st,
                            arena,
                            &mut rng,
                            n,
                            e,
                            state_i == 0,
                        );
                    }
                }
            }
            for (lhs, _) in &sites {
                let canonical = crate::eval::resolve_offsets(&nk.ctx(), lhs);
                match nk.fast_offsets_for_test(lhs) {
                    Some(f) => {
                        assert_eq!(
                            f.as_slice(),
                            canonical.as_slice(),
                            "{name}: state {state_i}: specialized offsets differ"
                        );
                        fast += 1;
                    }
                    None => declined += 1,
                }
            }
        }
    }
    // Pinned: a DROP means the fast path narrowed (or the corpus shrank), and a
    // fast count of zero would make every assertion above vacuous.
    assert_eq!(
        (fast, declined),
        (2112, 56),
        "specialized-offset coverage moved. ⚠️ 4304 → (2112, 56) at §4.5.308: \
         the declared-range fix seals index expressions in a `Concat`, which the \
         W compiler does not admit, so 44 more lvalues per state take the generic \
         resolver. Correctness is unaffected and the hot benchmarks are all \
         zero-LSB descending (untouched fast path, keccak byte-identical and \
         within noise) — but a `Concat` arm in `wprog` would win them back. \
         The DECLINES are load-bearing too \
         (a >64-bit index, a part-select index and a THREE-chunk concat lvalue, \
         three per state), so a drop to zero there means the decline arm \
         stopped being exercised"
    );
}
