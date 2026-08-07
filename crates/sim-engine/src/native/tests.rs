//! S1a/S1b gates (doc-21 §5 S1 내부 분해) — the arena against the engine.
//!
//! Three properties, all differential against the EXISTING engine (measurement
//! over argument):
//!
//! 1. **S1a init parity** — a freshly built arena answers `read_net` exactly
//!    like a freshly built `SimState`, for every net, every element, and the
//!    whole-net read, across the full P6 corpus. This pins the layout AND the
//!    `expand_init` broadcast rule without re-stating either.
//! 2. **S1a roundtrip** — masked write/read at the width ladder that crosses
//!    every word boundary (1..200), plus the OOB all-X read.
//! 3. **S1b mirror differential** — the SAME evaluator (`EvalCtx` is generic)
//!    over the SAME mirrored random 4-state state, once with the engine store
//!    and once with the arena: every pure expression must produce an identical
//!    `Value` at two context widths. Divergence can only come from the read
//!    path (element indexing, OOB→X, masking) — exactly S1b's surface.
//!
//! The compared-expression count is PINNED (exact) so the purity filter cannot
//! silently shrink coverage.

use super::test_common as common;
use common::{build, corpus, Rng};
use sim_ir::{Expr, SimIr};

use crate::eval::{EvalCtx, NetReader};
use crate::native::arena::NetArena;
use crate::state::SimState;
use crate::value::top_mask;
use crate::width::WidthTable;
use crate::SimOpts;

/// A diag sink for `SimState::new` (its diagnostics are irrelevant here — the
/// designs are the validated corpus).
#[derive(Default)]
pub(super) struct NullSink;
impl diag::LogSink for NullSink {
    fn emit(&self, _e: diag::LogEvent) {}
}

/// `fresh_state` with the per-process context installed from `opts`.
///
/// `SimState::new` does NOT apply `SimOpts`; production wires them in `simulate`.
/// Without this, `proc_scopes`/`proc_multipliers` are empty, `enter_body` sets
/// every process to "top"/1, and a body walk that skipped `enter_body` entirely
/// was indistinguishable from one that called it — measured.
pub(super) fn fresh_state_with<'a>(
    ir: &'a SimIr,
    sink: &'a NullSink,
    opts: &SimOpts,
) -> SimState<'a> {
    let mut st = fresh_state(ir, sink);
    st.proc_scopes = opts.proc_scopes.clone();
    st.proc_multipliers = opts.proc_multipliers.clone();
    st.proc_prec_mults = opts.proc_prec_mults.clone();
    for &n in &opts.two_state_nets {
        st.two_state[n as usize] = true;
    }
    st
}

pub(super) fn fresh_state<'a>(ir: &'a SimIr, sink: &'a NullSink) -> SimState<'a> {
    SimState::new(
        ir,
        Box::new(std::io::sink()),
        sink,
        "1ns".to_string(),
        "test".to_string(),
        None,
    )
}

/// A full-range u64 draw (`Rng::range(0, u64::MAX)` overflows its span math).
fn r64(rng: &mut Rng) -> u64 {
    (rng.range(0, u32::MAX as u64) << 32) | rng.range(0, u32::MAX as u64)
}

/// Set bit `i` of a `BitPacked` plane pair.
pub(super) fn set_bit(bp: &mut sim_ir::BitPacked, i: u32, v: u64, u: u64) {
    let w = (i / 64) as usize;
    let b = i % 64;
    bp.val[w] = (bp.val[w] & !(1 << b)) | (v << b);
    bp.unk[w] = (bp.unk[w] & !(1 << b)) | (u << b);
}

/// Mirror ONE random 4-state value into element `e` of net `n` in BOTH stores.
/// `defined_only` forces `unk = 0` (the arithmetic-heavy profile); otherwise
/// ~25% of bits are X/Z.
pub(super) fn mirror_random_elem(
    st: &mut SimState,
    arena: &mut NetArena,
    rng: &mut Rng,
    n: u32,
    e: u32,
    defined_only: bool,
) {
    let s = arena.slots[n as usize];
    let words = s.words as usize;
    let mut vw = vec![0u64; words];
    let mut uw = vec![0u64; words];
    for k in 0..words {
        vw[k] = r64(rng);
        uw[k] = if defined_only { 0 } else { r64(rng) & r64(rng) };
    }
    let m = top_mask(s.width.max(1));
    vw[words - 1] &= m;
    uw[words - 1] &= m;
    arena.set_elem(n, e, &vw, &uw);
    // Engine side: the flat store packs elements bit-contiguously.
    let base = e * s.width;
    let cur = &mut st.nets[n as usize].cur;
    for i in 0..s.width {
        let v = (vw[(i / 64) as usize] >> (i % 64)) & 1;
        let u = (uw[(i / 64) as usize] >> (i % 64)) & 1;
        set_bit(cur, base + i, v, u);
    }
}

/// Is the expression subtree PURE state-reads only — no RNG/time/plusargs
/// (`SysFunc`), no frame call, no with-clause scratch? Only pure trees are
/// meaningful to evaluate twice.
fn pure_expr(ir: &SimIr, memo: &mut Vec<Option<bool>>, eid: u32) -> bool {
    if let Some(p) = memo[eid as usize] {
        return p;
    }
    let p = match &ir.exprs[eid as usize] {
        Expr::Const { .. } => true,
        Expr::Signal { word, .. } => word.map_or(true, |w| pure_expr(ir, memo, w)),
        Expr::ArrayItem { .. } | Expr::SysFunc { .. } | Expr::Call { .. } => false,
        Expr::Unary { operand, .. } => pure_expr(ir, memo, *operand),
        Expr::Binary { lhs, rhs, .. } => pure_expr(ir, memo, *lhs) && pure_expr(ir, memo, *rhs),
        Expr::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            pure_expr(ir, memo, *cond)
                && pure_expr(ir, memo, *then_e)
                && pure_expr(ir, memo, *else_e)
        }
        Expr::Concat { parts } => parts.iter().all(|&p| pure_expr(ir, memo, p)),
        Expr::Replicate { count, value } => {
            pure_expr(ir, memo, *count) && pure_expr(ir, memo, *value)
        }
        Expr::Select {
            base,
            offset,
            width,
            ..
        } => {
            pure_expr(ir, memo, *base)
                && pure_expr(ir, memo, *offset)
                && pure_expr(ir, memo, *width)
        }
    };
    memo[eid as usize] = Some(p);
    p
}

fn eval_with<'a, N: NetReader>(
    ir: &'a SimIr,
    wt: &'a WidthTable,
    rng: &'a crate::state::RngCells,
    nets: &'a N,
    eid: u32,
    ctx_w: u32,
    ctx_signed: bool,
) -> crate::value::Value {
    let ctx = EvalCtx {
        ir,
        nets,
        now: 0,
        wt,
        time_mult: 1,
        rng,
        plusargs: &[],
    };
    ctx.eval_ctx(eid, ctx_w, ctx_signed)
}

/// S1a — init parity over the whole corpus: arena `read_net` ≡ engine
/// `read_net` right after construction, every net × (every element + the
/// whole-net read) — plus one deliberate OOB element per array.
#[test]
fn arena_init_matches_engine_state_over_corpus() {
    let sink = NullSink;
    let mut nets_checked = 0usize;
    for d in corpus(0x5EED_F00D, 72) {
        let ir = build(&d.src);
        let arena = NetArena::build(&ir, &SimOpts::default())
            .unwrap_or_else(|e| panic!("{}: corpus design must build an arena: {e}", d.name));
        let st = fresh_state(&ir, &sink);
        // Layout partition sanity: the slots tile the buffer exactly.
        let total: usize = arena
            .slots
            .iter()
            .map(|s| 2 * s.words as usize * s.elems as usize)
            .sum();
        assert_eq!(
            total,
            arena.buf.len(),
            "{}: layout must tile the buffer",
            d.name
        );
        for n in 0..ir.nets.len() as u32 {
            let elems = arena.slots[n as usize].elems;
            for e in 0..elems {
                assert_eq!(
                    arena.read_net(n, Some(e)),
                    st.read_net(n, Some(e)),
                    "{}: init parity net {n} elem {e}",
                    d.name
                );
            }
            assert_eq!(
                arena.read_net(n, None),
                st.read_net(n, None),
                "{}: init parity net {n} whole",
                d.name
            );
            // OOB read: both sides all-X, same shape.
            assert_eq!(
                arena.read_net(n, Some(elems)),
                st.read_net(n, Some(elems)),
                "{}: OOB parity net {n}",
                d.name
            );
            nets_checked += 1;
        }
    }
    assert_eq!(
        nets_checked, 297,
        "corpus net count moved — re-pin deliberately"
    );
}

/// S1a — masked roundtrip across the word-boundary width ladder + array
/// elements + the signed flag.
#[test]
fn arena_roundtrip_across_word_boundaries() {
    let ir = build(
        "module t;\n\
           reg a;\n\
           reg [6:0] b;\n\
           reg [62:0] c;\n\
           reg [63:0] d;\n\
           reg [64:0] e;\n\
           reg [126:0] f;\n\
           reg [127:0] g;\n\
           reg [128:0] h;\n\
           reg [199:0] i;\n\
           reg signed [15:0] s;\n\
           integer n;\n\
           reg [7:0] m [0:4];\n\
           reg [65:0] wm [0:2];\n\
         endmodule\n",
    );
    let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat design");
    let mut rng = Rng::new(0x00A1_1E57);
    for n in 0..ir.nets.len() as u32 {
        let s = arena.slots[n as usize];
        assert_eq!(s.signed, ir.nets[n as usize].signed, "signed flag net {n}");
        for e in 0..s.elems {
            let words = s.words as usize;
            let raw: Vec<u64> = (0..words).map(|_| r64(&mut rng)).collect();
            let rawu: Vec<u64> = (0..words).map(|_| r64(&mut rng)).collect();
            arena.set_elem(n, e, &raw, &rawu);
            let (vp, up) = arena.planes(n, e);
            let m = top_mask(s.width.max(1));
            for k in 0..words {
                let mask = if k + 1 == words { m } else { u64::MAX };
                assert_eq!(vp[k], raw[k] & mask, "net {n} elem {e} val word {k}");
                assert_eq!(up[k], rawu[k] & mask, "net {n} elem {e} unk word {k}");
            }
        }
        // OOB element read is all-X at the element width.
        let oob = arena.read_net(n, Some(s.elems));
        assert!(oob.has_xz(), "net {n} OOB must be X");
        assert_eq!(oob.width, s.width);
    }
}

/// S1b — the mirror differential: same evaluator, same mirrored state, two
/// stores. Every pure expression of every corpus design, five random states
/// (one all-defined), two context widths. The total comparison count is pinned.
#[test]
fn arena_reader_matches_engine_reader_under_shared_eval() {
    let sink = NullSink;
    let mut compared = 0usize;
    for d in corpus(0x5EED_F00D, 72) {
        let ir = build(&d.src);
        let wt = WidthTable::build(&ir, &crate::FuncTable::new());
        let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("corpus is flat");
        let mut st = fresh_state(&ir, &sink);
        let mut memo = vec![None; ir.exprs.len()];
        let pure: Vec<u32> = (0..ir.exprs.len() as u32)
            .filter(|&eid| pure_expr(&ir, &mut memo, eid))
            .collect();
        assert!(!pure.is_empty(), "{}: no pure exprs?", d.name);
        let mut rng = Rng::new(0xD1FF_0000 ^ pure.len() as u64);
        for state_i in 0..5 {
            for n in 0..ir.nets.len() as u32 {
                for e in 0..arena.slots[n as usize].elems {
                    mirror_random_elem(&mut st, &mut arena, &mut rng, n, e, state_i == 0);
                }
            }
            let rng_a = crate::state::RngCells::default();
            let rng_b = crate::state::RngCells::default();
            for &eid in &pure {
                let sw = wt.get(eid);
                for (cw, cs) in [(sw.width, sw.signed), (sw.width + 63, false)] {
                    let engine = eval_with(&ir, &wt, &rng_a, &st, eid, cw, cs);
                    let native = eval_with(&ir, &wt, &rng_b, &arena, eid, cw, cs);
                    assert_eq!(
                        engine, native,
                        "{}: eid {eid} ctx ({cw},{cs}) state {state_i}",
                        d.name
                    );
                    compared += 1;
                }
            }
        }
    }
    assert_eq!(
        compared, 18500,
        "the differential's coverage moved — re-pin deliberately (a DROP means \
         the purity filter or the corpus silently shrank)"
    );
}

// ── S1c: the write funnel ─────────────────────────────────────────────────────
//
// The gate for S1c is statement-level: from the SAME mirrored state, execute ONE
// statement's write on both stores and require (a) the `changed` verdicts agree
// and (b) the two stores still read identically, net by net, element by element.
// Because each pass keeps writing into the state it just produced, a divergence
// cannot be masked by a later re-mirror — the walk stays in lockstep or it fails
// at the first statement that breaks it.

/// lex → parse → elaborate WITH sidecars, and the `SimOpts` the engine would
/// receive. Needed because `two_state_nets` (a CORE sidecar at S0, so this
/// funnel's job) exists only on the sidecar path — a plain `build` would leave
/// both sides 4-state and the coercion arm untested.
pub(super) fn build_with_opts(src: &str) -> (SimIr, SimOpts) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = NullSink;
    let (ir, sc) = elaborate::elaborate_with_timescale(
        &su.expect("source unit"),
        &sink,
        &std::collections::BTreeMap::new(),
        -9,
    );
    // `func_table` too, not just `two_state_nets`: without it the ENGINE's
    // `frame_local` routing stays all-false, so a subroutine design would take
    // the same flat path the arena does and the mirror would agree FOR THE WRONG
    // REASON (differential-review find). With it installed, such a design is
    // refused by `NetArena::build` instead — which is the honest answer today.
    // `severities` too (S1d-4b-2): `$error`/`$fatal`/`$warning`/`$info` lower as
    // a `Display` plus an out-of-band severity entry keyed by StmtId, so WITHOUT
    // this table those tasks take the plain-display path and `run_severity_with` — a
    // render site, and the one that is not in `dispatch.rs` — is never entered.
    // Measured: dropping the reader from that arm survived every test until this
    // line existed.
    let opts = SimOpts {
        two_state_nets: sc.two_state_nets,
        func_table: sc.func_table,
        severities: sc.severities,
        // …and `timeformat_stmts`, for the same reason: `$timeformat` lowers to a
        // `Display` plus a sid, so WITHOUT the table it prints its own arguments
        // instead of applying them, and the `%t` path is never entered.
        timeformat_stmts: sc.timeformat_stmts,
        // …and the per-process context. `enter_body` installs `cur_scope` and the
        // time multipliers from these, so without them every process shares
        // "top"/1 and a body walk that skipped `enter_body` entirely was
        // indistinguishable (measured).
        proc_scopes: sc.proc_scopes,
        proc_multipliers: sc.proc_multipliers,
        proc_prec_mults: sc.proc_prec_mults,
        // …and the declaration-initializer list (S1d-4c-2c). `reg clk = 0;`
        // lowers to an ordinary process PLUS an `init_procs` entry, and the
        // entry is what makes `arm_processes` run it before arming and then drop
        // the dirt. Without the table the same source becomes a normal `initial`
        // block that hands `always @clk` an x→0 edge — so a run-loop
        // differential built on the default `SimOpts` would compare two backends
        // on a design neither was executing as written.
        init_procs: sc.init_procs,
        // …and `final_procs`, for the same reason: a `final` block lowers as an
        // ordinary Initial-shaped process PLUS this sidecar, and the sidecar is
        // the only thing that stops it being armed at t0. Without it the tier-3
        // run gate's `final` row is unreachable from any test.
        final_procs: sc.final_procs,
        // …and the wired-resolution sets. `wand`/`wor` lowers as ordinary
        // continuous assigns PLUS these sidecars, so without them the tier-3
        // run gate's wired row is unreachable from any test and a `wand` design
        // reads as an ordinary multi-driven net.
        // …and the per-transition delay table. `assign #(rise,fall)` lowers as an
        // ordinary delayed assign PLUS this sidecar, so without it every
        // transition uses the uniform delay and a test that pins rise/fall is
        // measuring something the design does not say.
        ca_delays: sc.ca_delays,
        wired_and_nets: sc.wired_and_nets,
        wired_or_nets: sc.wired_or_nets,
        // …and the class-field width sidecar (§4.5.309). `obj.f` lowers to a
        // `Signal` on the 32-bit HANDLE net plus this entry, so without it
        // `patch_class_fields` runs over an empty map and every `obj.f` in this
        // file reports the handle's 32/unsigned instead of the field's own
        // width and sign. Found by an anti-vacuity counter, not by a failure:
        // the omission made a comparison agree because BOTH sides were blind.
        class_field_widths: sc.class_field_widths,
        ..SimOpts::default()
    };
    (ir.expect("elaborate"), opts)
}

/// Every (lhs, rhs) write site in the design: procedural assigns of both kinds
/// plus continuous assigns. Taking them from the arenas (rather than walking
/// process bodies) reaches every body — a subroutine's statements live in the
/// same `ir.stmts`.
pub(super) fn write_sites(ir: &SimIr) -> Vec<(sim_ir::Lvalue, u32)> {
    let mut v: Vec<(sim_ir::Lvalue, u32)> = ir
        .stmts
        .iter()
        .filter_map(|s| match s {
            sim_ir::Stmt::BlockingAssign { lhs, rhs }
            | sim_ir::Stmt::NonblockingAssign { lhs, rhs, .. } => Some((lhs.clone(), *rhs)),
            _ => None,
        })
        .collect();
    v.extend(ir.cont_assigns.iter().map(|ca| (ca.lhs.clone(), ca.rhs)));
    v
}

/// Mirror a random state into BOTH stores. `small` biases every net to a tiny
/// value so array indices and part-select offsets land IN range (a full-range
/// 32-bit index is out of range for every array, which would test only the drop
/// arm); `xz` mixes in X/Z bits (the 2-state coercion + unknown-index arms).
pub(super) fn mirror_state(
    st: &mut SimState,
    arena: &mut NetArena,
    rng: &mut Rng,
    n_nets: u32,
    small: bool,
    xz: bool,
) {
    for n in 0..n_nets {
        for e in 0..arena.slots[n as usize].elems {
            if small {
                let s = arena.slots[n as usize];
                let words = s.words as usize;
                let mut vw = vec![0u64; words];
                let mut uw = vec![0u64; words];
                vw[0] = rng.range(0, 9);
                if xz && rng.boolean() {
                    uw[0] = rng.range(0, 3);
                }
                let m = top_mask(s.width.max(1));
                vw[words - 1] &= m;
                uw[words - 1] &= m;
                arena.set_elem(n, e, &vw, &uw);
                let base = e * s.width;
                let cur = &mut st.nets[n as usize].cur;
                for i in 0..s.width {
                    let v = (vw[(i / 64) as usize] >> (i % 64)) & 1;
                    let u = (uw[(i / 64) as usize] >> (i % 64)) & 1;
                    set_bit(cur, base + i, v, u);
                }
            } else {
                mirror_random_elem(st, arena, rng, n, e, !xz);
            }
        }
    }
}

/// Assert the two stores read identically everywhere.
pub(super) fn assert_stores_equal(st: &SimState, arena: &NetArena, n_nets: u32, what: &str) {
    for n in 0..n_nets {
        for e in 0..arena.slots[n as usize].elems {
            assert_eq!(
                arena.read_net(n, Some(e)),
                st.read_net(n, Some(e)),
                "{what}: store diverged at net {n} elem {e}"
            );
        }
    }
}

/// Walk every write site over several mirrored states, executing each write on
/// both stores. Returns the number of (statement × state) comparisons made.
fn s1c_walk(src: &str, name: &str, seed: u64) -> usize {
    let (ir, opts) = build_with_opts(src);
    let sink = NullSink;
    let mut arena = NetArena::build(&ir, &opts).unwrap_or_else(|e| panic!("{name}: arena: {e}"));
    let mut st = fresh_state(&ir, &sink);
    for &n in &opts.two_state_nets {
        if (n as usize) < st.two_state.len() {
            st.two_state[n as usize] = true;
        }
    }
    // Production shape: `Scheduler::new` bakes this, and it selects the engine's
    // whole-scalar FAST path — without it the differential would only ever
    // compare against the general funnel.
    st.build_plain_scalar();
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let sites = write_sites(&ir);
    assert!(!sites.is_empty(), "{name}: no write sites");
    let n_nets = ir.nets.len() as u32;
    let mut rng = Rng::new(seed);
    let mut compared = 0usize;
    for pass in 0..6 {
        // pass 0: defined full-range · 1: small (in-range indices) · 2: X/Z-heavy
        // · 3: small with X/Z · 4/5: full-range mixes.
        let small = pass == 1 || pass == 3;
        let xz = pass >= 2;
        mirror_state(&mut st, &mut arena, &mut rng, n_nets, small, xz);
        assert_stores_equal(&st, &arena, n_nets, &format!("{name}/pass{pass}/mirror"));
        for (i, (lhs, rhs)) in sites.iter().enumerate() {
            // Both pipelines in full: resolve offsets and evaluate the rhs on
            // each reader, then write with that reader's own results.
            let rng_a = crate::state::RngCells::default();
            let ctx_e = EvalCtx {
                ir: &ir,
                nets: &st,
                now: 0,
                wt: &wt,
                time_mult: 1,
                rng: &rng_a,
                plusargs: &[],
            };
            let sw = wt.get(*rhs);
            let ctx_w = st.lvalue_width(lhs).max(sw.width);
            let off_e = crate::eval::resolve_offsets(&ctx_e, lhs);
            let val_e = ctx_e.eval_ctx(*rhs, ctx_w, sw.signed);
            let rng_b = crate::state::RngCells::default();
            let ctx_n = EvalCtx {
                ir: &ir,
                nets: &arena,
                now: 0,
                wt: &wt,
                time_mult: 1,
                rng: &rng_b,
                plusargs: &[],
            };
            let a_ctx_w = arena.lvalue_width(&ir, lhs).max(sw.width);
            let off_n = crate::eval::resolve_offsets(&ctx_n, lhs);
            let val_n = ctx_n.eval_ctx(*rhs, a_ctx_w, sw.signed);
            assert_eq!(
                ctx_w, a_ctx_w,
                "{name}/pass{pass}/site{i}: lvalue width diverged"
            );
            assert_eq!(
                off_e.as_slice(),
                off_n.as_slice(),
                "{name}/pass{pass}/site{i}: resolved offsets diverged"
            );
            assert_eq!(
                val_e, val_n,
                "{name}/pass{pass}/site{i}: rhs value diverged"
            );
            let ch_e = st.write_lvalue(lhs, val_e, &off_e);
            let ch_n = arena.write_lvalue(&ir, lhs, val_n, &off_n);
            assert_eq!(
                ch_e, ch_n,
                "{name}/pass{pass}/site{i}: `changed` verdict diverged"
            );
            assert_stores_equal(
                &st,
                &arena,
                n_nets,
                &format!("{name}/pass{pass}/site{i} (after write)"),
            );
            compared += 1;
        }
    }
    compared
}

/// S1c over the P6 corpus — the same 72 designs the S1a/S1b gates use, now
/// EXECUTING their writes on both stores.
///
/// What this number does NOT cover (measured census, differential review): the
/// corpus contains **no** 2-state net, **no** subroutine and **no** real value,
/// so the funnel's three VALUE-sensitive arms (2-state coercion, frame lane,
/// real→int round) get nothing from these 3,150 comparisons — they are covered
/// by the adversarial designs below and by the frame-refusal pin. It DOES cover
/// the geometry broadly: 21 of the 72 have arrays whose elements are unaligned
/// in the engine's packed store, which is the seam the arena's aligned layout
/// collapses.
#[test]
fn s1c_write_funnel_matches_engine_over_corpus() {
    let mut compared = 0usize;
    for (i, d) in corpus(0x5EED_F00D, 72).into_iter().enumerate() {
        compared += s1c_walk(&d.src, &d.name, 0x5157_0000 + i as u64);
    }
    assert_eq!(
        compared, 3150,
        "S1c corpus coverage moved — re-pin deliberately"
    );
}

/// S1c over the lvalue SHAPES the corpus does not contain: part-selects (const,
/// `+:`, `-:` — including offsets that underflow below bit 0), bit-selects,
/// array-element writes with out-of-range and X indices, a concat LHS, 2-state
/// destinations receiving X/Z, and multi-word nets whose part-writes straddle a
/// word boundary.
#[test]
fn s1c_write_funnel_matches_engine_on_adversarial_lvalues() {
    let designs: [(&str, &str); 9] = [
        (
            "part_selects",
            "module t;\n\
               reg [31:0] y; reg [31:0] src; integer i;\n\
               reg [7:0] narrow;\n\
               initial begin\n\
                 y[5:2] = src[3:0];\n\
                 y[i+:4] = src[7:4];\n\
                 y[i-:4] = src[11:8];\n\
                 y[i] = src[0];\n\
                 narrow[i+:3] = src[2:0];\n\
                 narrow[i-:3] = src[5:3];\n\
               end\n\
             endmodule\n",
        ),
        (
            "array_elems",
            "module t;\n\
               reg [7:0] m [0:3]; reg [63:0] wide [0:2]; integer k; reg [7:0] v;\n\
               initial begin\n\
                 m[k] = v;\n\
                 m[k][3:1] = v[2:0];\n\
                 m[k][k] = v[0];\n\
                 wide[k] = {v, v, v, v, v, v, v, v};\n\
                 wide[k][k+:8] = v;\n\
               end\n\
             endmodule\n",
        ),
        (
            "concat_lhs",
            "module t;\n\
               reg [7:0] a; reg [3:0] b; reg [15:0] c; reg [31:0] s; integer i;\n\
               initial begin\n\
                 {a, b} = s[11:0];\n\
                 {c, a, b} = s[27:0];\n\
                 {a[i], b[2:1]} = s[2:0];\n\
                 {a, a} = s[15:0];\n\
               end\n\
             endmodule\n",
        ),
        (
            "two_state",
            "module t;\n\
               bit [7:0] b8; int ii; byte by; logic [7:0] l8; integer k;\n\
               bit [3:0] barr [0:2];\n\
               initial begin\n\
                 b8 = l8;\n\
                 ii = {l8, l8, l8, l8};\n\
                 by = l8;\n\
                 b8[k] = l8[0];\n\
                 b8[k+:3] = l8[2:0];\n\
                 barr[k] = l8[3:0];\n\
               end\n\
             endmodule\n",
        ),
        (
            "wide_nets",
            "module t;\n\
               reg [128:0] w129; reg [64:0] w65; reg [199:0] w200; reg [63:0] src;\n\
               integer k;\n\
               initial begin\n\
                 w129 = {src, src};\n\
                 w129[k+:64] = src;\n\
                 w129[63:0] = src;\n\
                 w65[k] = src[0];\n\
                 w65[k-:32] = src[31:0];\n\
                 w200[k+:65] = {src, src[0]};\n\
                 w200[127:64] = src;\n\
               end\n\
             endmodule\n",
        ),
        (
            "real_valued_rhs",
            "module t;\n\
               reg [31:0] x; reg [7:0] y; reg signed [15:0] sg; reg [31:0] src;\n\
               initial begin\n\
                 x = $itor(src) / 2.0;\n\
                 y = $bitstoreal(64'h3FF8000000000000);\n\
                 sg = -$itor(src);\n\
                 x[3] = $itor(src);\n\
                 x[7:4] = $itor(src) + 0.5;\n\
               end\n\
             endmodule\n",
        ),
        (
            "wide_2state_and_down_select",
            "module t;\n\
               bit [127:0] b128; bit [95:0] b96; reg [127:0] l128; integer k;\n\
               reg [199:0] w200; reg [63:0] src;\n\
               initial begin\n\
                 b128 = l128;\n\
                 b128[k+:70] = l128[69:0];\n\
                 b96[k-:40] = l128[39:0];\n\
                 w200[k-:96] = {src, src[31:0]};\n\
                 w200[159:64] = {src, src[31:0]};\n\
               end\n\
             endmodule\n",
        ),
        (
            "real_into_concat",
            "module t;\n\
               reg [15:0] hi; reg [15:0] lo; reg [31:0] src;\n\
               initial begin\n\
                 {hi, lo} = $itor(src) + 0.5;\n\
                 {hi[3:0], lo} = $itor(src) / 4.0;\n\
               end\n\
             endmodule\n",
        ),
        (
            "signed_and_ranges",
            "module t;\n\
               reg signed [15:0] s16; reg [7:2] hi; reg signed [31:0] s32; integer k;\n\
               initial begin\n\
                 s16 = s32;\n\
                 s32 = s16;\n\
                 hi[k] = s16[0];\n\
                 hi[k+:2] = s16[1:0];\n\
                 s16[k-:4] = s32[3:0];\n\
               end\n\
             endmodule\n",
        ),
    ];
    // The 2-state coercion arm must not be VACUOUS: a design that produced no
    // `two_state_nets` entry would run the whole walk without ever entering it.
    let (_, ts_opts) = build_with_opts(designs[3].1);
    assert!(
        !ts_opts.two_state_nets.is_empty(),
        "the 2-state design must actually produce 2-state nets"
    );
    let mut compared = 0usize;
    for (i, (name, src)) in designs.iter().enumerate() {
        compared += s1c_walk(src, name, 0x5AD0_0000 + i as u64);
    }
    assert_eq!(
        compared, 270,
        "S1c shape coverage moved — re-pin deliberately"
    );
}

/// Set one element of BOTH stores to an exact value (deterministic twin of
/// `mirror_random_elem`).
fn mirror_set(st: &mut SimState, arena: &mut NetArena, n: u32, e: u32, val: u64, unk: u64) {
    let s = arena.slots[n as usize];
    let words = s.words as usize;
    let mut vw = vec![0u64; words];
    let mut uw = vec![0u64; words];
    vw[0] = val;
    uw[0] = unk;
    let m = top_mask(s.width.max(1));
    vw[words - 1] &= m;
    uw[words - 1] &= m;
    arena.set_elem(n, e, &vw, &uw);
    let base = e * s.width;
    let cur = &mut st.nets[n as usize].cur;
    for i in 0..s.width {
        let v = (vw[(i / 64) as usize] >> (i % 64)) & 1;
        let u = (uw[(i / 64) as usize] >> (i % 64)) & 1;
        set_bit(cur, base + i, v, u);
    }
}

/// TEETH for the two drop arms, made deterministic rather than left to the
/// random walk: an out-of-range array-word write and an X-indexed one must both
/// be dropped — no store change, `changed == false` — on BOTH sides. A funnel
/// that clamped instead (writing the last element) would still pass a
/// value-equality sweep if the engine clamped too; here the engine is the
/// oracle AND the "nothing moved" assertion is explicit.
#[test]
fn s1c_out_of_range_and_x_index_writes_are_dropped() {
    let (ir, opts) = build_with_opts(
        "module t;\n\
           reg [7:0] m [0:3]; integer k; reg [7:0] v;\n\
           initial begin m[k] = v; end\n\
         endmodule\n",
    );
    let sink = NullSink;
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let sites = write_sites(&ir);
    assert_eq!(sites.len(), 1, "one write site");
    let (lhs, rhs) = &sites[0];
    // Net ids by declaration order: m = 0, k = 1, v = 2.
    let (m, k, v) = (0u32, 1u32, 2u32);
    assert_eq!(ir.nets[m as usize].array_len, 4, "m is the 4-element array");

    for (label, kval, kunk) in [
        ("in range", 2u64, 0u64),
        ("out of range", 9, 0),
        ("far out of range", 0xFFFF_FFFF, 0),
        ("X index", 0, 0xF),
    ] {
        let mut arena = NetArena::build(&ir, &opts).expect("arena");
        let mut st = fresh_state(&ir, &sink);
        st.build_plain_scalar();
        for e in 0..4 {
            mirror_set(&mut st, &mut arena, m, e, 0x11 * (e as u64 + 1), 0);
        }
        mirror_set(&mut st, &mut arena, v, 0, 0xA5, 0);
        mirror_set(&mut st, &mut arena, k, 0, kval, kunk);
        let before: Vec<_> = (0..4).map(|e| st.read_net(m, Some(e))).collect();

        let rng_a = crate::state::RngCells::default();
        let ctx_e = EvalCtx {
            ir: &ir,
            nets: &st,
            now: 0,
            wt: &wt,
            time_mult: 1,
            rng: &rng_a,
            plusargs: &[],
        };
        let sw = wt.get(*rhs);
        let ctx_w = st.lvalue_width(lhs).max(sw.width);
        let off_e = crate::eval::resolve_offsets(&ctx_e, lhs);
        let val_e = ctx_e.eval_ctx(*rhs, ctx_w, sw.signed);
        let rng_b = crate::state::RngCells::default();
        let ctx_n = EvalCtx {
            ir: &ir,
            nets: &arena,
            now: 0,
            wt: &wt,
            time_mult: 1,
            rng: &rng_b,
            plusargs: &[],
        };
        let off_n = crate::eval::resolve_offsets(&ctx_n, lhs);
        let val_n = ctx_n.eval_ctx(*rhs, ctx_w, sw.signed);
        let ch_e = st.write_lvalue(lhs, val_e, &off_e);
        let ch_n = arena.write_lvalue(&ir, lhs, val_n, &off_n);

        assert_eq!(ch_e, ch_n, "{label}: `changed` verdict");
        for e in 0..4 {
            assert_eq!(
                arena.read_net(m, Some(e)),
                st.read_net(m, Some(e)),
                "{label}: element {e}"
            );
        }
        if label == "in range" {
            assert!(ch_e, "{label}: the write must land");
            assert_eq!(
                st.read_net(m, Some(2)).val[0],
                0xA5,
                "{label}: landed value"
            );
        } else {
            assert!(!ch_e, "{label}: the write must be dropped");
            for e in 0..4 {
                assert_eq!(
                    st.read_net(m, Some(e)),
                    before[e as usize],
                    "{label}: element {e} must be untouched (a clamp would move one)"
                );
            }
        }
    }
}

/// The frame-local lane, pinned as the SPLIT S3a drew rather than as the blanket
/// refusal it replaced.
///
/// User calls are CORE at S0 (revision 4: S3 absorbs them), so a subroutine
/// design is `eligible: true`. What changed is the storage half: a subroutine
/// whose body names only its own frame slots is now buildable, because the
/// tier-3 kernel answers `Expr::Call` by delegating to the engine's frame
/// executor — and a frame window is not part of either net store. One that
/// names a MODULE net is still refused, because THAT read would come from the
/// flat store a native run never writes.
///
/// Both halves are asserted here, on the two designs that differ by one
/// identifier, so neither can go vacuous alone.
#[test]
fn a_subroutine_design_builds_only_when_its_body_stays_in_its_frame() {
    let contained = "module t;\n\
           function automatic integer f(input integer x);\n\
             integer loc;\n\
             begin loc = x + 1; f = loc; end\n\
           endfunction\n\
           integer r;\n\
           initial begin r = f(3); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n";
    let reads_module = "module t;\n\
           integer g;\n\
           function automatic integer f(input integer x);\n\
             integer loc;\n\
             begin loc = x + g; f = loc; end\n\
           endfunction\n\
           integer r;\n\
           initial begin g = 1; r = f(3); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n";
    for (label, src, want) in [
        ("frame-contained", contained, None),
        (
            "reads a module net",
            reads_module,
            Some("a subroutine that names a net outside its own frame: S3b"),
        ),
    ] {
        let (ir, opts) = build_with_opts(src);
        assert!(
            !opts.func_table.is_empty(),
            "{label}: the design must actually produce a frame table"
        );
        // S0 says yes for both — calls are core, and this is the documented
        // design-level UPPER BOUND (`native.eligible` is not a promise the
        // executor exists).
        assert!(
            crate::native::design_eligibility(&ir, &opts).eligible,
            "{label}: calls are CORE at S0 (rev-4)"
        );
        assert_eq!(
            NetArena::build(&ir, &opts).err(),
            want,
            "{label}: storage verdict"
        );
    }
}

// ── S1d-2: the dirty/edge channel ─────────────────────────────────────────────
//
// The gate for S1c compared VALUES. This one compares what a value comparison
// structurally cannot see: which nets the write funnel reported as changed, in
// what order, with which intra-slot edge mask. A channel that stored every bit
// correctly and dropped a `posedge` would pass every S1a/S1b/S1c gate and freeze
// the design — so the engine's channel is the oracle here, sampled at the same
// two store points.

/// Execute one write on BOTH stores WITHOUT taking the channel — so a batch of
/// them accumulates, which is what a real delta does and what makes the ORDER
/// and the same-slot GLITCH mask observable at all.
fn write_both_stores(
    st: &mut SimState,
    arena: &mut NetArena,
    ir: &SimIr,
    wt: &WidthTable,
    lhs: &sim_ir::Lvalue,
    rhs: u32,
) -> (bool, bool) {
    let rng_a = crate::state::RngCells::default();
    let ctx_e = EvalCtx {
        ir,
        nets: &*st,
        now: 0,
        wt,
        time_mult: 1,
        rng: &rng_a,
        plusargs: &[],
    };
    let sw = wt.get(rhs);
    let ctx_w = st.lvalue_width(lhs).max(sw.width);
    let off_e = crate::eval::resolve_offsets(&ctx_e, lhs);
    let val_e = ctx_e.eval_ctx(rhs, ctx_w, sw.signed);
    let rng_b = crate::state::RngCells::default();
    let ctx_n = EvalCtx {
        ir,
        nets: &*arena,
        now: 0,
        wt,
        time_mult: 1,
        rng: &rng_b,
        plusargs: &[],
    };
    let off_n = crate::eval::resolve_offsets(&ctx_n, lhs);
    let val_n = ctx_n.eval_ctx(rhs, ctx_w, sw.signed);
    let ch_e = st.write_lvalue(lhs, val_e, &off_e);
    let ch_n = arena.write_lvalue(ir, lhs, val_n, &off_n);
    (ch_e, ch_n)
}

/// Take the engine's channel exactly as `propagate_changes` does.
fn engine_take(st: &mut SimState) -> Vec<(u32, u8, u32)> {
    let mut cand = std::mem::take(&mut st.dirty);
    cand.sort_unstable();
    let out: Vec<(u32, u8, u32)> = cand
        .iter()
        .map(|&n| {
            st.dirty_flag[n as usize] = false;
            (
                n,
                st.slot_edge[n as usize],
                st.last_blocking_writer[n as usize],
            )
        })
        .collect();
    cand.clear();
    st.dirty = cand;
    out
}

/// S1d-2 over the corpus + the clock-shaped designs the corpus lacks: after every
/// write, the two channels must agree on the changed set, its ORDER, each net's
/// edge mask and each net's authoring writer.
#[test]
fn s1d2_dirty_and_edge_channel_matches_engine() {
    let mut designs: Vec<(String, String)> = corpus(0x5EED_F00D, 72)
        .into_iter()
        .map(|d| (d.name, d.src))
        .collect();
    // The corpus has clocked designs, but none that re-writes a clock TWICE in
    // one slot — and the glitch mask exists precisely for that. These do.
    designs.push((
        "glitch_clock".to_string(),
        ("module t;\n\
           reg clk; reg [7:0] q; reg a, b;\n\
           always @(posedge clk) q <= q + 1;\n\
           always @(negedge a) q <= q + 2;\n\
           always @(b) q <= q + 3;\n\
           initial begin\n\
             clk = 0; q = 0; a = 1; b = 0;\n\
             clk = 1; clk = 0; clk = 1;\n\
             a = 0; a = 1; a = 0;\n\
             b = 1; b = 1'bx; b = 1'bz; b = 0;\n\
             #1 $display(\"q=%0d\", q); $finish;\n\
           end\n\
         endmodule\n")
            .to_string(),
    ));
    // ⭐ The BIT-SERIAL store point. Measured by the S1d-2 review: without a
    // bit-/part-select lvalue that arm is NEVER ENTERED (`ser_enter = 0`), so
    // deleting its entire channel block left the whole package green — the exact
    // defect this slice exists to prevent, at half its surface. The 7-behaviour
    // teeth list had been derived from `dirty.rs`'s three concepts instead of
    // from the two STORE POINTS `write.rs` enumerates.
    designs.push((
        "bit_select_clock".to_string(),
        ("module t;\n\
           reg [7:0] bus; reg [7:0] q;\n\
           always @(posedge bus[0]) q <= q + 1;\n\
           always @(negedge bus[0]) q <= q + 2;\n\
           initial begin\n\
             bus = 0; q = 0;\n\
             bus[0] = 1; bus[0] = 0; bus[0] = 1;\n\
             bus[3:1] = 3'b101; bus[7] = 1;\n\
             #1 $display(\"q=%0d\", q); $finish;\n\
           end\n\
         endmodule\n")
            .to_string(),
    ));
    designs.push((
        "wide_edge_target".to_string(),
        ("module t;\n\
           reg [63:0] wclk; reg [7:0] q; reg [3:0] m [0:2];\n\
           always @(posedge wclk[0]) q <= q + 1;\n\
           initial begin\n\
             wclk = 0; q = 0;\n\
             wclk = 64'h1; wclk = 64'h0; wclk = 64'hFFFF_FFFF_FFFF_FFFF;\n\
             m[0] = 4'h5; m[1] = 4'h6; m[0] = 4'h5;\n\
             #1 $display(\"q=%0d\", q); $finish;\n\
           end\n\
         endmodule\n")
            .to_string(),
    ));

    let sink = NullSink;
    let mut compared = 0usize;
    let mut saw_edge_mask = 0usize;
    let mut saw_multi = 0usize;
    let mut batches = 0usize;
    let mut saw_writer = 0usize;
    let mut saw_negedge = 0usize;
    let mut clocked = 0usize;
    for (i, (name, src)) in designs.iter().enumerate() {
        let (ir, opts) = build_with_opts(src);
        let Ok(mut arena) = NetArena::build(&ir, &opts) else {
            continue;
        };
        let mut st = fresh_state(&ir, &sink);
        for &n in &opts.two_state_nets {
            if (n as usize) < st.two_state.len() {
                st.two_state[n as usize] = true;
            }
        }
        st.build_plain_scalar();
        // The two channels must start from the SAME notion of which nets are
        // edge targets — that set is built by one shared scan, and this asserts
        // the arena actually received it.
        assert_eq!(
            arena.ch.is_edge_target, st.is_edge_target,
            "{name}: edge-target sets differ"
        );
        let wt = WidthTable::build(&ir, &crate::FuncTable::new());
        let sites = write_sites(&ir);
        let n_nets = ir.nets.len() as u32;
        let mut rng = Rng::new(0x0D1E_0000 + i as u64);
        for pass in 0..4 {
            mirror_state(
                &mut st,
                &mut arena,
                &mut rng,
                n_nets,
                pass % 2 == 1,
                pass >= 2,
            );
            // A mirror writes the stores directly, so neither channel saw it —
            // clear both so the comparison starts from a common empty state.
            st.dirty
                .iter()
                .for_each(|&n| st.dirty_flag[n as usize] = false);
            st.dirty.clear();
            let mut drop = Vec::new();
            arena.take_changed(&mut drop);
            // BATCH the whole design's writes before taking the channel — one
            // take per write would leave every dirty list length 1, which makes
            // the ORDER contract and the same-slot glitch accumulator vacuous
            // (measured: with per-write takes, deliberately dropping the sort
            // and the posedge bit both still passed).
            // The AUTHOR tag: `blocking_writer` is what stops a process being
            // re-fired on a net it wrote itself. Set BEFORE the batch — set
            // after, the tag a batch observes is a carry-over from the previous
            // pass, so the arm is non-vacuous only by accident (review find).
            let writer = if pass % 2 == 0 {
                Some(pass as u32)
            } else {
                None
            };
            st.blocking_writer = writer;
            arena.ch.blocking_writer = writer;
            for (si, (lhs, rhs)) in sites.iter().enumerate() {
                let (ce, cn) = write_both_stores(&mut st, &mut arena, &ir, &wt, lhs, *rhs);
                assert_eq!(ce, cn, "{name}/pass{pass}/site{si}: changed verdict");
                compared += 1;
            }
            // The AUTHOR tag: `blocking_writer` is what stops a process being
            // re-fired on a net it wrote itself. Left `None` it degenerates to
            // `u32::MAX` and the arm is untested (measured), so drive it.
            let writer = if pass % 2 == 0 {
                Some(pass as u32)
            } else {
                None
            };
            st.blocking_writer = writer;
            arena.ch.blocking_writer = writer;
            let eng = engine_take(&mut st);
            let mut nat = Vec::new();
            arena.take_changed(&mut nat);
            assert_eq!(
                eng, nat,
                "{name}/pass{pass}: dirty/edge channel diverged after the batch"
            );
            saw_edge_mask += eng.iter().filter(|(_, m, _)| *m & 1 != 0).count();
            saw_multi += usize::from(eng.len() > 1);
            saw_writer += eng.iter().filter(|(_, _, w)| *w != u32::MAX).count();
            batches += 1;

            // CROSS-BATCH: the mask must be per-SLOT, not cumulative. Drive each
            // edge-target net 0 → 1 → 0 through the funnel, taking the channel
            // between each, so successive batches see DIFFERENT masks (posedge
            // then negedge). Without the first-dirty reset the second batch would
            // report the OR of both — and with one take per batch that is the
            // only way to see it (measured: dropping the reset otherwise passed).
            for net in 0..n_nets {
                if !arena.ch.is_edge_target[net as usize] {
                    continue;
                }
                let w = arena.slots[net as usize].width.max(1);
                let lhs = sim_ir::Lvalue {
                    chunks: vec![sim_ir::LvalChunk {
                        net,
                        word: None,
                        offset: None,
                        width: None,
                        kind: sim_ir::SelKind::Bit,
                    }],
                };
                let offs = crate::exec::Offsets::Inline {
                    buf: [(0, 0); 2],
                    len: 1,
                };
                for step in [0u64, 1, 0, 1] {
                    let mut v = crate::value::Value::zeros(w, false);
                    if step == 1 {
                        v.set_vu(0, 1, 0);
                    }
                    let ce = st.write_lvalue(&lhs, v.clone(), &offs);
                    let cn = arena.write_lvalue(&ir, &lhs, v, &offs);
                    assert_eq!(ce, cn, "{name}/pass{pass}/net{net}: clock changed verdict");
                    let e2 = engine_take(&mut st);
                    let mut n2 = Vec::new();
                    arena.take_changed(&mut n2);
                    assert_eq!(
                        e2, n2,
                        "{name}/pass{pass}/net{net}/step{step}: per-slot mask diverged"
                    );
                    saw_edge_mask += e2.iter().filter(|(_, m, _)| *m & 1 != 0).count();
                    saw_negedge += e2.iter().filter(|(_, m, _)| *m & 2 != 0).count();
                    clocked += 1;
                }
            }
        }
    }
    assert_eq!(compared, 2240, "S1d-2 coverage moved — re-pin deliberately");
    // NON-VACUITY, verified by deliberately breaking each behaviour and
    // watching this test fail: a POSEDGE must actually have been accumulated
    // (else `m |= 1` is untested), and some batch must have had TWO OR MORE
    // changed nets (else the ascending-order contract is untested — with one
    // net, sorting is a no-op).
    assert!(
        saw_edge_mask > 0,
        "no posedge was ever accumulated — the edge arm is untested"
    );
    assert!(
        saw_multi > 0,
        "no batch changed 2+ nets — the ascending-order contract is untested"
    );
    assert!(saw_negedge > 0, "no negedge was accumulated");
    assert!(
        saw_writer > 0,
        "the blocking-writer tag was never non-default"
    );
    assert!(batches > 0 && clocked > 0);
}

// ── S1d-3: the wake decision ──────────────────────────────────────────────────
//
// S1d-2 compared the changed SET; this compares what the scheduler DECIDES from
// it — which processes go ready, in which order. The engine's
// `propagate_changes` pass (a) is the oracle: the same writes are applied to
// both stores, the engine propagates, and the delta of its Active queue is
// compared against `WakeTable::wake`'s output.

/// Designs whose clocked processes the wake decision is about. The corpus has
/// clocked designs but the interesting rules (multi-net dedup, self-write
/// suppression, glitch pulses, both edge kinds on one net) need shapes it lacks.
fn wake_designs() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = corpus(0x5EED_F00D, 24)
        .into_iter()
        .map(|d| (d.name, d.src))
        .collect();
    for (n, src) in [
        (
            "multi_edge_one_proc",
            "module t;\n\
               reg c1, c2, rst; reg [7:0] q;\n\
               always @(posedge c1 or posedge c2 or negedge rst) q <= q + 1;\n\
               always @(posedge c1) q <= q + 2;\n\
               always @(negedge c1) q <= q + 3;\n\
               initial begin\n\
                 c1 = 0; c2 = 0; rst = 1; q = 0;\n\
                 c1 = 1; c2 = 1; rst = 0;\n\
                 c1 = 0; c1 = 1; c1 = 0;\n\
                 #1 $display(\"q=%0d\", q); $finish;\n\
               end\n\
             endmodule\n",
        ),
        (
            "anyedge_and_xz",
            "module t;\n\
               reg a; reg [7:0] q;\n\
               always @(a) q <= q + 1;\n\
               always @(posedge a) q <= q + 2;\n\
               initial begin\n\
                 a = 0; q = 0;\n\
                 a = 1'bx; a = 1'bz; a = 1; a = 0;\n\
                 #1 $display(\"q=%0d\", q); $finish;\n\
               end\n\
             endmodule\n",
        ),
        // ORDER + LEVEL SELF-WRITE: a LOW-index level process and a HIGH-index
        // edge process both sensitive to the same net. The edge loop pushes the
        // high index first and the level loop the low one after, so without the
        // final sort the order is wrong — and with the author cycling over
        // processes, the level watcher eventually authors a change to its own
        // watched net. Both rules were untested before this design (measured).
        (
            "level_before_edge",
            "module t;\n\
               reg a, b; reg [7:0] q0, q1;\n\
               always @(a) q0 <= q0 + 1;\n\
               always @(posedge a) q1 <= q1 + 1;\n\
               initial begin\n\
                 a = 0; b = 0; q0 = 0; q1 = 0;\n\
                 a = 1; a = 0; a = 1; b = 1; b = 0; a = 0; a = 1;\n\
                 #1 $display(\"q0=%0d q1=%0d\", q0, q1); $finish;\n\
               end\n\
             endmodule\n",
        ),
        // CONSUME + ORDER: a LEVEL process on TWO nets, indexed BELOW an edge
        // process on one of them. Two nets changing in one delta must wake the
        // level process ONCE (consume), and the level push lands after the edge
        // push so the final sort is what restores tie order. Neither is reachable
        // unless a delta carries 2+ changed nets — which is why the writes below
        // are batched (the S1d-2 lesson, hit again).
        (
            "level_two_nets",
            "module t;\n\
               reg a, b; reg [7:0] q0, q1;\n\
               always @(a or b) q0 <= q0 + 1;\n\
               always @(posedge a) q1 <= q1 + 1;\n\
               initial begin\n\
                 a = 0; b = 0; q0 = 0; q1 = 0;\n\
                 a = 1; b = 1; a = 0; b = 0; a = 1; b = 1;\n\
                 #1 $display(\"q0=%0d q1=%0d\", q0, q1); $finish;\n\
               end\n\
             endmodule\n",
        ),
        (
            "self_write_clock",
            "module t;\n\
               reg clk; reg [7:0] q;\n\
               always @(posedge clk) begin q <= q + 1; clk = 1'b0; end\n\
               initial begin clk = 0; q = 0; clk = 1; #1 clk = 1; #1 $finish; end\n\
             endmodule\n",
        ),
    ] {
        v.push((n.to_string(), src.to_string()));
    }
    v
}

#[test]
fn s1d3_wake_decision_matches_engine() {
    let sink = NullSink;
    let mut compared = 0usize;
    let mut saw_wake = 0usize;
    let mut saw_multi_wake = 0usize;
    let mut saw_dedup = 0usize;
    let mut saw_authored = 0usize;
    let mut saw_multi_net = 0usize;
    let mut comparisons = 0usize;
    let mut saw_comb = 0usize;
    for (i, (name, src)) in wake_designs().iter().enumerate() {
        let (ir, opts) = build_with_opts(src);
        let Ok(mut arena) = NetArena::build(&ir, &opts) else {
            continue;
        };
        let mut wake = crate::native::wake::WakeTable::new(&ir);
        let mut st = fresh_state(&ir, &sink);
        for &n in &opts.two_state_nets {
            if (n as usize) < st.two_state.len() {
                st.two_state[n as usize] = true;
            }
        }
        st.build_plain_scalar();
        let wt = WidthTable::build(&ir, &crate::FuncTable::new());
        let sites = write_sites(&ir);
        let n_nets = ir.nets.len() as u32;
        let mut rng = Rng::new(0x3A5E_0000 + i as u64);
        // The engine needs a live scheduler for `propagate_changes`, and arming
        // is what BUILDS `net_to_edge` — without it pass (a) has nothing
        // registered and the comparison would be vacuous on both sides.
        let mut sched = crate::sched::Scheduler::new(
            &mut st,
            1_000_000,
            100_000_000,
            None,
            opts.fork_modes.clone(),
        );
        sched.arm_processes();
        for pass in 0..4 {
            // OBSERVATION GRANULARITY IS AN AXIS, NOT A CHOICE. Measured both
            // ways: propagating after EVERY write leaves one changed net per
            // delta, so a level and an edge process can never co-fire (the ORDER
            // contract and the level CONSUME are unreachable); batching every
            // write first lets a repeatedly-written clock accumulate a mask with
            // all three bits, so "always fire" becomes indistinguishable from
            // "fires" (the MASK rule goes untested). Each granularity hides what
            // the other exposes, so the sweep runs both.
            // Two INDEPENDENT axes. They shared one predicate before, so batched
            // observation was only ever taken with in-range indices and per-write
            // only with full-range — half the matrix went unswept.
            let batched = pass % 2 == 1;
            let small = (pass / 2) % 2 == 1;
            {
                let stx = sched.state_mut();
                mirror_state(stx, &mut arena, &mut rng, n_nets, small, pass >= 2);
                stx.dirty
                    .iter()
                    .for_each(|&n| stx.dirty_flag[n as usize] = false);
                stx.dirty.clear();
            }
            let mut drop = Vec::new();
            arena.take_changed(&mut drop);

            let mut before = sched.active_ready_procs();
            for (si, (lhs, rhs)) in sites.iter().enumerate() {
                // SELF-WRITE suppression needs an author that is actually
                // registered on the net — with `blocking_writer` left `None` the
                // tag is always the sentinel and the rule is untested (measured).
                // Every third write is UNAUTHORED (`None` → the `u32::MAX`
                // sentinel), which in production is the COMMON case — an NBA
                // apply, a cont-assign settle and a clocking commit all write
                // with no blocking author. Forcing an author on every site
                // inverted that distribution and left the sentinel arm
                // differentially uncompared (measured).
                let author = if si % 3 == 2 {
                    None
                } else {
                    Some((si % ir.processes.len().max(1)) as u32)
                };
                {
                    let stx = sched.state_mut();
                    stx.blocking_writer = author;
                    let rng_a = crate::state::RngCells::default();
                    let ctx = EvalCtx {
                        ir: &ir,
                        nets: &*stx,
                        now: 0,
                        wt: &wt,
                        time_mult: 1,
                        rng: &rng_a,
                        plusargs: &[],
                    };
                    let sw = wt.get(*rhs);
                    let ctx_w = stx.lvalue_width(lhs).max(sw.width);
                    let off = crate::eval::resolve_offsets(&ctx, lhs);
                    let val = ctx.eval_ctx(*rhs, ctx_w, sw.signed);
                    stx.write_lvalue(lhs, val, &off);
                    stx.blocking_writer = None;
                }
                let rng_b = crate::state::RngCells::default();
                let ctx_n = EvalCtx {
                    ir: &ir,
                    nets: &arena,
                    now: 0,
                    wt: &wt,
                    time_mult: 1,
                    rng: &rng_b,
                    plusargs: &[],
                };
                let sw = wt.get(*rhs);
                let ctx_w = arena.lvalue_width(&ir, lhs).max(sw.width);
                let off_n = crate::eval::resolve_offsets(&ctx_n, lhs);
                let val_n = ctx_n.eval_ctx(*rhs, ctx_w, sw.signed);
                arena.ch.blocking_writer = author;
                arena.write_lvalue(&ir, lhs, val_n, &off_n);
                arena.ch.blocking_writer = None;
                compared += 1;
                if !batched {
                    compare_wake(
                        &mut sched,
                        &mut arena,
                        &mut wake,
                        &ir,
                        &mut before,
                        name,
                        pass,
                        &mut comparisons,
                        &mut saw_wake,
                        &mut saw_multi_wake,
                        &mut saw_authored,
                        &mut saw_multi_net,
                    );
                }
            }
            if batched {
                compare_wake(
                    &mut sched,
                    &mut arena,
                    &mut wake,
                    &ir,
                    &mut before,
                    name,
                    pass,
                    &mut comparisons,
                    &mut saw_wake,
                    &mut saw_multi_wake,
                    &mut saw_authored,
                    &mut saw_multi_net,
                );
            }
            // A new event cluster: both sides drop their timestep dedup markers.
            sched.reset_edge_seen_marks();
            wake.reset_edge_seen();
            saw_dedup += 1;
            // `arm_processes` QUEUES Comb/Latch into Active at t0 rather than
            // ARMING them; they arm when that first run completes. Bodies never
            // run here, so model that completion — but only AFTER pass 0, so the
            // sweep observes BOTH states. Re-arming immediately made the t0
            // distinction unobservable (measured: the wrong arm state passed).
            if pass == 0 {
                for p in 0..ir.processes.len() as u32 {
                    if matches!(
                        ir.processes[p as usize].sensitivity.kind,
                        sim_ir::SensKind::Comb | sim_ir::SensKind::Latch
                    ) {
                        sched.arm_sensitivity(p);
                        wake.rearm_level(p);
                        saw_comb += 1;
                    }
                }
            }
        }
    }
    // TWO numbers, because they measure different things and only the second is
    // coverage: `compared` counts WRITES driven, `comparisons` counts wake
    // decisions actually compared (a batched pass drives many writes and yields
    // one decision). The pin message used to say "coverage" over the write count.
    assert_eq!(
        compared, 920,
        "S1d-3 write count moved — re-pin deliberately"
    );
    assert_eq!(
        comparisons, 518,
        "S1d-3 wake-comparison coverage moved — re-pin deliberately"
    );
    assert!(
        saw_wake > 0,
        "no process was ever woken — the gate is vacuous"
    );
    assert!(
        saw_multi_wake > 0,
        "no delta woke 2+ processes — the ORDER contract is untested"
    );
    assert!(
        saw_authored > 0,
        "no change was ever authored by a named process — self-write suppression is untested"
    );
    assert!(
        saw_multi_net > 0,
        "no delta changed 2+ nets — consume and ORDER are untested (batch the writes)"
    );
    assert!(saw_dedup > 0);
    assert!(
        saw_comb > 0,
        "no Comb/Latch process exists in the sweep — the class that was missing entirely"
    );
}

/// One wake comparison: propagate on the engine, take the arena's changed set,
/// and require the two decisions to agree. Factored out because the sweep makes
/// it at two different granularities (per-write and per-batch) and a second copy
/// would be the place they silently drift.
#[allow(clippy::too_many_arguments)]
fn compare_wake(
    sched: &mut crate::sched::Scheduler,
    arena: &mut NetArena,
    wake: &mut crate::native::wake::WakeTable,
    ir: &SimIr,
    before: &mut Vec<u32>,
    name: &str,
    pass: usize,
    comparisons: &mut usize,
    saw_wake: &mut usize,
    saw_multi_wake: &mut usize,
    saw_authored: &mut usize,
    saw_multi_net: &mut usize,
) {
    sched.propagate_changes();
    let after = sched.active_ready_procs();
    let mut engine_woken = after.clone();
    for b in before.iter() {
        if let Some(p) = engine_woken.iter().position(|x| x == b) {
            engine_woken.remove(p);
        }
    }
    let mut changed = Vec::new();
    arena.take_changed(&mut changed);
    let mut native_woken = Vec::new();
    wake.wake(&changed, &mut native_woken);
    assert_eq!(
        engine_woken, native_woken,
        "{name}/pass{pass}: wake decision diverged (changed={changed:?})"
    );
    // STEADY STATE: a woken LEVEL process runs to completion and re-arms.
    // Without modelling that, its waiter is consumed once and the ORDER contract
    // (level index < edge index) is never reached again.
    for &p in &engine_woken {
        if ir.processes[p as usize].sensitivity.kind == sim_ir::SensKind::Level {
            sched.arm_sensitivity(p);
            wake.rearm_level(p);
        }
    }
    *comparisons += 1;
    *saw_wake += usize::from(!engine_woken.is_empty());
    *saw_multi_wake += usize::from(engine_woken.len() > 1);
    *saw_authored += usize::from(changed.iter().any(|&(_, _, w)| w != u32::MAX));
    *saw_multi_net += usize::from(changed.len() > 1);
    *before = after;
    // A comparison is one event cluster, and the run loop resets the timestep
    // dedup markers at EVERY cluster boundary (#0 batch, NBA region, time
    // advance). Resetting once per pass instead left each edge process able to
    // fire at most once per pass, so a level and an edge process never co-fired
    // — which is exactly what the two designs written for the ORDER contract
    // were supposed to produce (measured: `lvl+edge = 0` before this).
    sched.reset_edge_seen_marks();
    wake.reset_edge_seen();
}

// ── S2 slice 1: the width-specialized programs ───────────────────────────────

/// EXHAUSTIVE per-op differential: every admitted op over EVERY 4-state value
/// pair at width 4 — 256×256 plane combinations × 9 expression shapes, WProg
/// vs the generic evaluator over the SAME arena. Width 4 makes exhaustion
/// affordable while still exercising cross-bit carry (`+`/`-`) and shift
/// boundary bits; the per-bit tables (and/or/xor/not) are fully enumerated
/// many times over. This is the anchor the §4.5.302 rule demands for the
/// specialized spellings of the 4-state tables — measured equal against the
/// canonical evaluator, not derived equal.
#[test]
fn s2_wprog_matches_generic_eval_exhaustively_at_width_4() {
    // Each continuous assign is one shape; every one is compiled at its OWN
    // (self width, self signedness) so unsigned, SIGNED and one-bit comparison
    // results can share a single design. `sa`/`sb` mirror `a`/`b` bit for bit,
    // so one (a,b) sweep drives the signed rows too.
    //
    // ⚠️ Rows are added here whenever an admission BRANCH is added — S2 slice
    // 1's `k >= w` shift arm shipped with a comment claiming this battery
    // covered it while every shift here had `k < w`, and a constant mutant
    // passed the whole suite. "Exhaustive" is a property of the input set, not
    // of the word.
    let src = "module top;\n\
       reg [3:0] a, b;\n\
       reg signed [3:0] sa, sb;\n\
       wire [3:0] x1; assign x1 = a ^ b;\n\
       wire [3:0] x2; assign x2 = a & b;\n\
       wire [3:0] x3; assign x3 = a | b;\n\
       wire [3:0] x4; assign x4 = ~a;\n\
       wire [3:0] x5; assign x5 = a + b;\n\
       wire [3:0] x6; assign x6 = a - b;\n\
       wire [3:0] x7; assign x7 = a << 1;\n\
       wire [3:0] x8; assign x8 = a >> 1;\n\
       wire [3:0] x9; assign x9 = (a << 3) | (a >> 1);\n\
       wire [3:0] xa; assign xa = a >> 4;\n\
       wire [3:0] xb; assign xb = a << 5;\n\
       wire signed [3:0] y1; assign y1 = sa ^ sb;\n\
       wire signed [3:0] y2; assign y2 = sa & sb;\n\
       wire signed [3:0] y3; assign y3 = ~sa;\n\
       wire signed [3:0] y4; assign y4 = sa + sb;\n\
       wire signed [3:0] y5; assign y5 = sa - sb;\n\
       wire signed [3:0] y6; assign y6 = sa << 1;\n\
       wire signed [3:0] y7; assign y7 = sa >> 1;\n\
       wire signed [3:0] y8; assign y8 = sa >>> 1;\n\
       wire c1; assign c1 = a < b;\n\
       wire c2; assign c2 = a <= b;\n\
       wire c3; assign c3 = a > b;\n\
       wire c4; assign c4 = a >= b;\n\
       wire c5; assign c5 = a == b;\n\
       wire c6; assign c6 = a != b;\n\
       wire d1; assign d1 = sa < sb;\n\
       wire d2; assign d2 = sa <= sb;\n\
       wire d3; assign d3 = sa > sb;\n\
       wire d4; assign d4 = sa >= sb;\n\
       wire d5; assign d5 = sa == sb;\n\
       wire d6; assign d6 = sa != sb;\n\
       wire e1; assign e1 = (a + b) < (a ^ b);\n\
       wire e2; assign e2 = (a & b) <= (a | b);\n\
       wire e3; assign e3 = (~a) > (b >> 1);\n\
       wire e4; assign e4 = (a << 1) == (b << 1);\n\
       wire e5; assign e5 = (sa + sb) < (sa - sb);\n\
       wire e6; assign e6 = (sa ^ sb) != (~sa);\n\
       initial begin a = 4'd0; b = 4'd0; sa = 4'sd0; sb = 4'sd0; end\n\
       endmodule\n";
    let ir = build(src);
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat");
    // The four stimulus regs by SHAPE: the first assign is `a ^ b`, the first
    // signed one is `sa ^ sb` (names are a sidecar, shapes are the IR).
    let sig = |eid: u32| match ir.exprs.get(eid as usize) {
        Some(sim_ir::Expr::Signal { net, .. }) => *net,
        other => panic!("op battery shape moved: {other:?}"),
    };
    let ops2 = |ca: &sim_ir::ContAssign| match ir.exprs.get(ca.rhs as usize) {
        Some(sim_ir::Expr::Binary { lhs, rhs, .. }) => (sig(*lhs), sig(*rhs)),
        other => panic!("op battery shape moved: {other:?}"),
    };
    let (na, nb) = ops2(&ir.cont_assigns[0]);
    let (nsa, nsb) = ops2(&ir.cont_assigns[11]);
    let mut progs = Vec::new();
    let mut declined = 0usize;
    for ca in &ir.cont_assigns {
        let sw = wt.get(ca.rhs);
        match crate::native::wprog::compile(&ir, &wt, &arena, ca.rhs, sw.width, sw.signed) {
            Some(p) => progs.push((ca.rhs, sw, p)),
            None => declined += 1,
        }
    }
    assert_eq!(progs.len(), 36, "op battery coverage moved");
    // COMPOUND comparison operands specifically: the per-op masks this slice
    // introduced exist because a program can hold two widths at once, and the
    // soundness review measured that EVERY comparison operand in the whole
    // sim-engine suite was a single `Load`/`Const` — so rewriting the operand
    // masks back to slice 1's single program mask survived the entire suite.
    // A comparison whose operands are computed is the only shape that can see
    // the difference; these rows are that shape.
    assert!(
        progs.iter().filter(|(_, _, p)| p.op_count() > 3).count() >= 6,
        "no compound-operand comparison left — the per-op masks lose their teeth"
    );
    assert_eq!(
        declined, 1,
        "exactly one row must DECLINE — `sa >>> 1` is the one shift whose bits \
         depend on the sign (arithmetic fill), and a battery where everything \
         admits cannot show that the decline still happens"
    );
    let rng = crate::state::RngCells::default();
    let mut scratch = Vec::new();
    let set = |arena: &mut NetArena, net: u32, val: u64, unk: u64| {
        let s = arena.slots[net as usize];
        arena.buf[s.off as usize] = val;
        arena.buf[s.off as usize + 1] = unk;
    };
    let mut compared = 0usize;
    // all 4-state values of a 4-bit net: (val,unk) pairs where val ⊆ mask.
    for pa in 0u64..256 {
        let (av, au) = (pa & 0xF, pa >> 4);
        for pb in 0u64..256 {
            let (bv, bu) = (pb & 0xF, pb >> 4);
            set(&mut arena, na, av, au);
            set(&mut arena, nb, bv, bu);
            set(&mut arena, nsa, av, au);
            set(&mut arena, nsb, bv, bu);
            for (rhs, sw, prog) in &progs {
                let w = prog.run(&arena, &mut scratch);
                let generic = eval_with(&ir, &wt, &rng, &arena, *rhs, sw.width, sw.signed);
                assert_eq!(
                    (w.val, w.unk),
                    (generic.val[0], generic.unk[0]),
                    "rhs {rhs} ({}b signed={}): a=({av:#x},{au:#x}) b=({bv:#x},{bu:#x})",
                    sw.width,
                    sw.signed
                );
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 256 * 256 * 36);
}

/// The corpus + a keccak-shaped design, swept: every pure expression that the
/// W-compiler ADMITS must evaluate identically to the generic path over five
/// random 4-state states. The admitted count is pinned — a silent shrink of
/// the admission (or of the corpus) reads as coverage that no longer exists.
#[test]
fn s2_wprog_matches_generic_eval_on_admitted_corpus_trees() {
    let sink = NullSink;
    let mut admitted_total = 0usize;
    let mut designs: Vec<(String, String)> = corpus(0x5EED_F00D, 72)
        .into_iter()
        .map(|d| (d.name.to_string(), d.src))
        .collect();
    // keccak's shape, miniaturised: const-index array elements, rot idiom,
    // chi's and-not, uniform 64-bit — the exact hot set of the real design.
    designs.push((
        "keccak_shaped".to_string(),
        "module top;\n\
           reg [63:0] st [0:4];\n\
           wire [63:0] c0; assign c0 = st[0] ^ st[1] ^ st[2] ^ st[3] ^ st[4];\n\
           wire [63:0] d0; assign d0 = st[4] ^ ((st[1] << 1) | (st[1] >> 63));\n\
           wire [63:0] b0; assign b0 = st[0] ^ (~st[1] & st[2]);\n\
           wire [63:0] i0; assign i0 = st[0] ^ 64'h8000000080008008;\n\
           initial begin st[0]=64'd1; st[1]=64'd2; st[2]=64'd3; st[3]=64'd4; st[4]=64'd5; end\n\
         endmodule\n"
            .to_string(),
    ));
    for (name, src) in &designs {
        let ir = build(src);
        let wt = WidthTable::build(&ir, &crate::FuncTable::new());
        let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat");
        let mut st = fresh_state(&ir, &sink);
        let mut memo = vec![None; ir.exprs.len()];
        let pure: Vec<u32> = (0..ir.exprs.len() as u32)
            .filter(|&eid| pure_expr(&ir, &mut memo, eid))
            .collect();
        let mut rng = Rng::new(0x57A7_0000 ^ pure.len() as u64);
        let mut scratch = Vec::new();
        let rng_cells = crate::state::RngCells::default();
        for state_i in 0..5 {
            for n in 0..ir.nets.len() as u32 {
                for e in 0..arena.slots[n as usize].elems {
                    mirror_random_elem(&mut st, &mut arena, &mut rng, n, e, state_i == 0);
                }
            }
            for &eid in &pure {
                let sw = wt.get(eid);
                let Some(prog) =
                    crate::native::wprog::compile(&ir, &wt, &arena, eid, sw.width, sw.signed)
                else {
                    continue;
                };
                let w = prog.run(&arena, &mut scratch);
                let generic = eval_with(&ir, &wt, &rng_cells, &arena, eid, sw.width, sw.signed);
                assert_eq!(
                    (w.val, w.unk),
                    (generic.val[0], generic.unk[0]),
                    "{name}: eid {eid} state {state_i}"
                );
                admitted_total += 1;
            }
        }
    }
    assert_eq!(
        admitted_total, 7715,
        "the admitted-tree coverage moved — re-pin deliberately (a DROP means \
         the admission or the corpus silently shrank)"
    );
}

/// CENSUS: which assignment right-hand sides the W-compiler admits, and what
/// the declined ones are, for the tier-3 hot design. A profile says where time
/// goes; this says which shapes are still on the generic path and why.
///
/// The numbers are ASSERTED, not printed. The first version only printed them
/// with `assert!(admitted > 0)` under a doc claiming to be "a measurement
/// pinned as a test" — the soundness review's word for that is the right one:
/// it was not a pin, and any admission change was invisible to it. It also
/// carried a "bench/ may be absent" skip, which is false for this path —
/// `.gitignore` un-ignores `/bench/keccak/` and the file is tracked.
#[test]
fn s2_admission_census_on_the_hot_design() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../bench/keccak/keccak_f_flat.sv"
    ))
    .expect("bench/keccak is the committed first-party bench (see .gitignore)");
    let ir = build(&src);
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let arena = NetArena::build(&ir, &SimOpts::default()).expect("flat");
    let mut admitted = 0usize;
    let mut declined: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    let mut visit = |lhs: &sim_ir::Lvalue, rhs: u32| {
        let lw = arena.lvalue_width(&ir, lhs);
        let sw = wt.get(rhs);
        let w = lw.max(sw.width);
        if crate::native::wprog::compile(&ir, &wt, &arena, rhs, w, sw.signed).is_some() {
            admitted += 1;
        } else {
            let kind = match ir.exprs.get(rhs as usize) {
                Some(sim_ir::Expr::Signal { word: Some(_), .. }) => "signal[dynamic-or-wide]",
                Some(sim_ir::Expr::Signal { .. }) => "signal[whole]",
                Some(sim_ir::Expr::Const { .. }) => "const",
                Some(sim_ir::Expr::Binary { .. }) => "binary",
                Some(sim_ir::Expr::Unary { .. }) => "unary",
                Some(sim_ir::Expr::Select { .. }) => "select",
                Some(sim_ir::Expr::Concat { .. }) => "concat",
                Some(sim_ir::Expr::Ternary { .. }) => "ternary",
                _ => "other",
            };
            *declined.entry(kind).or_default() += 1;
        }
    };
    // `blocks[..].stmts` holds STATEMENT IDS into the flat `ir.stmts` arena.
    for st in &ir.stmts {
        match st {
            sim_ir::Stmt::BlockingAssign { lhs, rhs, .. }
            | sim_ir::Stmt::NonblockingAssign { lhs, rhs, .. } => visit(lhs, *rhs),
            _ => {}
        }
    }
    // Pinned. A DROP in `admitted` means the admission narrowed; a rise means
    // it widened — either is a deliberate act that re-pins this line.
    //
    // 129/3 → 131/1 in S2 slice 4: the two DYNAMIC-INDEX array reads this line
    // used to call "the next slice's target" are now admitted, which is that
    // slice. What is left is one `select` (a part-select), still generic.
    let declined_total: usize = declined.values().sum();
    assert_eq!(
        (admitted, declined_total),
        (131, 1),
        "admission census moved: {declined:?}"
    );
    assert_eq!(
        declined
            .get("signal[dynamic-or-wide]")
            .copied()
            .unwrap_or(0),
        0,
        "a dynamic array index is admitted since S2 slice 4; a decline here means \
         the runtime element load narrowed: {declined:?}"
    );
}

/// The two DRIVERS of the shared self-width rule agree (§4.5.309).
///
/// `sim_ir::selfwidth` is now the only spelling of IEEE §5.4.1/§5.5, but it is
/// driven two ways: the engine fills the whole arena in one forward pass, and
/// `elaborate` fills a PREFIX on demand while it is still pushing expressions,
/// with the class-field sidecar applied inline rather than as a later patch.
/// Those two drivers are what can drift now, so that is what this asserts —
/// prefix-by-prefix equality with the full build, class-field designs included.
///
/// It replaces a test that asserted a hand-written conservative predicate was a
/// subset of the canonical rule. (That predicate still exists, but §4.5.309
/// demoted it to one job — freezing the UNSIGNED half of the index seal at its
/// pre-§4.5.309 decision — so it is no longer a second answer to the signedness
/// question and the subset property is no longer the thing to assert.)
///
/// ⚠️ What this does NOT test, measured: the RULE. Both sides here call
/// `self_width_of`, so any mutation inside `sim_ir::selfwidth` moves both and
/// passes — flipping the `**` arm to "signed if BOTH operands are" survives this
/// and is killed by `decl_range_norm::pow_and_class_field_indices_keep_their_sign`,
/// which is the anchor for the rule's content. Nor does it see any of the three
/// things `elaborate`'s driver actually carries: it runs on FINISHED IR, with no
/// `Elaborator`, no `selfw_cache` and no placeholders (`cli/tests/hier_index_seal.rs`
/// owns those). What it does own is the class-field sidecar reaching the pass
/// rather than a later patch — it is the test that fails if `build_with` goes
/// back to sweeping the finished table.
#[test]
fn s2_incremental_and_full_selfwidth_drivers_agree() {
    let cls = "class C; int si; bit [7:0] bu; endclass\n\
       module top;\n\
         C c; reg [-2:-33] dn; reg signed [7:0] s8; byte bb;\n\
         function automatic signed [7:0] fs(input [7:0] x); fs = -8'sd6; endfunction\n\
         initial begin\n\
           c = new(); c.si = -5; c.bu = 8'd5; dn = 0; s8 = -8'sd6; bb = -8'sd5;\n\
           dn[c.si] = 1'b1; dn[~c.si] = 1'b1; dn[c.bu] = 1'b1;\n\
           dn[s8] = 1'b1; dn[bb] = 1'b1; dn[fs(8'd6)] = 1'b1;\n\
           dn[s8 ** 32'd3] = 1'b1;\n\
         end\n\
       endmodule\n";
    let mut srcs: Vec<String> = corpus(0x5EED_F00D, 72).into_iter().map(|d| d.src).collect();
    srcs.push(cls.to_string());
    let (mut compared, mut signed_seen, mut cf_seen) = (0usize, 0usize, 0usize);
    for src in &srcs {
        let (ir, opts) = build_with_opts(src);
        let full = WidthTable::build_with(&ir, &opts.func_table, &opts.class_field_widths);
        let ctx = sim_ir::selfwidth::ExprCtx::of(&ir);
        let call_ret = |f: u32| {
            opts.func_table
                .get(f as usize)
                .map(|m| (m.ret_width, m.ret_signed))
        };
        // Elaborate's shape: a growing prefix, class fields applied as they go.
        let mut sw: Vec<sim_ir::selfwidth::SelfWidth> = Vec::new();
        for i in 0..ir.exprs.len() as u32 {
            let s = if let Some(&(w, sg)) = opts.class_field_widths.get(&i) {
                sim_ir::selfwidth::SelfWidth {
                    width: w.max(1),
                    signed: sg,
                }
            } else {
                sim_ir::selfwidth::self_width_of(ctx, &call_ret, &sw, i)
            };
            sw.push(s);
            let f = full.get(i);
            assert_eq!(
                (sw[i as usize].width, sw[i as usize].signed),
                (f.width, f.signed),
                "expr {i} differs between the incremental and full drivers: {:?}",
                ir.exprs[i as usize]
            );
            compared += 1;
            signed_seen += usize::from(f.signed);
        }
        cf_seen += opts.class_field_widths.len();
    }
    // A floor, not a pin: the value is coverage, and an exact count would churn
    // on any IR-shape change. The two anti-vacuity counters are the real teeth —
    // without them this passes when every expression is unsigned (the corpus
    // alone very nearly is) and the sign axis, which is the whole reason the
    // rule was shared, would never be compared at all.
    assert!(compared > 1_500, "too few exprs compared: {compared}");
    assert!(signed_seen > 0, "no signed expression compared");
    assert!(cf_seen > 0, "no class field compared");
}

/// S2 slice 4 — the RUNTIME element load, against the generic evaluator,
/// **including the deferred out-of-range report count**.
///
/// The value is the easy half. The hard half is that this branch owns a
/// DIAGNOSTIC: `read_net`'s out-of-range arm counts an E4002 (`Severity::Error`
/// → exit 1), and a specialized load that returned the right all-X while
/// forgetting to count would turn a design whose index walks past a memory from
/// FAIL into PASS with every printed byte identical. So the comparison is
/// `(val, unk, reports)`, not `(val, unk)` — which is why both sides drain the
/// counter around each evaluation.
///
/// COVERAGE, stated precisely rather than called "exhaustive": the index net is
/// 32-bit, so its 4-state space is 2^64 and no sweep is exhaustive over it. What
/// is exhaustive is the LOW NIBBLE — all 256 `(val, unk)` pairs on bits 3:0 with
/// the rest zero — which covers every in-range element (0..5), every clean
/// out-of-range one (6..15), and every partial/complete unknown in the bits that
/// decide the element. The classes the nibble cannot reach are then enumerated:
/// bits set above the nibble, exactly `u32::MAX` (which is also what a negative
/// `integer` index is in bits), `2^31`, an unknown confined to the HIGH bits, z,
/// and x. Those cover every REACHABLE arm of `word_index_of` and of
/// `idx >= elems` — `u32::try_from` FAILING is not among them, because the seal
/// caps an admitted index at 32 bits.
///
/// The array has SIX elements on purpose — a power of two would let a masking
/// bug read as correct.
///
/// ⚠️ The index is `integer` (32-bit) because a NARROWER one does not reach this
/// branch at all: the §4.5.310 index seal wraps it in a `Concat` to widen it, and
/// `wprog` has no `Concat` arm (a separate, already-queued S2 item). So this test
/// covers the shape real RTL uses — a loop variable — and not every shape a user
/// can write; the narrow ones decline to the generic path, which is correct.
#[test]
fn s2_wprog_runtime_element_load_matches_generic_eval() {
    let src = "module top;\n\
       reg [3:0] m [0:5];\n\
       integer k;\n\
       wire [3:0] e1; assign e1 = m[k];\n\
       wire [3:0] e2; assign e2 = m[k] ^ m[k];\n\
       wire [3:0] e3; assign e3 = m[k] & 4'hF;\n\
       wire       c1; assign c1 = m[k] == 4'h3;\n\
     endmodule\n";
    let ir = build(src);
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat");

    let (mut arr, mut kn) = (u32::MAX, u32::MAX);
    for (i, nv) in ir.nets.iter().enumerate() {
        if nv.array_len == 6 && arr == u32::MAX {
            arr = i as u32;
        } else if nv.array_len <= 1 && nv.width == 32 && kn == u32::MAX {
            kn = i as u32;
        }
    }
    assert!(arr != u32::MAX && kn != u32::MAX, "design shape changed");
    for e in 0..6u32 {
        arena.set_elem(arr, e, &[u64::from(e) + 9], &[0]);
    }

    // Every rhs of the design, compiled. All four MUST admit, or the sweep is
    // measuring the generic path against itself.
    let mut progs = Vec::new();
    for ca in &ir.cont_assigns {
        let lw = arena.lvalue_width(&ir, &ca.lhs);
        let sw = wt.get(ca.rhs);
        let w = lw.max(sw.width);
        let p = crate::native::wprog::compile(&ir, &wt, &arena, ca.rhs, w, sw.signed)
            .expect("a runtime element load must be admitted since S2 slice 4");
        progs.push((ca.rhs, w, sw.signed, p));
    }
    assert_eq!(progs.len(), 4, "design shape changed");

    // The exhaustive nibble, then the classes it cannot reach.
    let mut cases: Vec<(u64, u64)> = (0u64..256).map(|p| (p & 0xF, p >> 4)).collect();
    cases.extend([
        (0x10, 0), // clean, above the nibble
        // u32::MAX — the sentinel's own value, and also what a NEGATIVE
        // `integer` index is in bits. One case, not two: an earlier version
        // listed the same pair twice and called the set "seven".
        (0xFFFF_FFFF, 0),
        (0x8000_0000, 0),           // 2^31
        (0, 0xFFFF_FFF0),           // unknown confined to the HIGH bits
        (0xFFFF_FFFF, 0xFFFF_FFFF), // all z
        (0, 0xFFFF_FFFF),           // all x
    ]);

    let rng = crate::state::RngCells::default();
    let mut scratch = Vec::new();
    let (mut compared, mut saw_oob, mut saw_inrange) = (0usize, 0usize, 0usize);
    for (kv, ku) in cases {
        {
            let s = arena.slots[kn as usize];
            arena.buf[s.off as usize] = kv;
            arena.buf[s.off as usize + 1] = ku;
        }
        for (rhs, w, signed, prog) in &progs {
            let _ = arena.take_deferred_range_reports(); // start from zero
            let got = prog.run(&arena, &mut scratch);
            let got_reports = arena.take_deferred_range_reports();

            let want = eval_with(&ir, &wt, &rng, &arena, *rhs, *w, *signed);
            let want_reports = arena.take_deferred_range_reports();

            assert_eq!(
                (got.val, got.unk, got_reports),
                (want.val[0], want.unk[0], want_reports),
                "rhs {rhs}: k=({kv:#x},{ku:#x})"
            );
            compared += 1;
            if want_reports > 0 {
                saw_oob += 1;
            } else {
                saw_inrange += 1;
            }
        }
    }
    assert_eq!(compared, (256 + 6) * 4);
    // ANTI-VACUITY on BOTH arms: a sweep that never went out of range would not
    // test the report at all, and one that never stayed in range would not test
    // the load.
    assert!(
        saw_oob > 0 && saw_inrange > 0,
        "{saw_oob} oob / {saw_inrange} in"
    );
}
