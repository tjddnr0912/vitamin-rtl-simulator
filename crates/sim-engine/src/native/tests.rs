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
        // `net_at_off` inverts the layout by SCANNING for a matching `off`,
        // which is exact only because `build` advances the cursor by
        // `words*2*elems` with both factors >= 1 — so offsets are STRICTLY
        // increasing and therefore unique. That is the whole proof, and it is
        // one `+= 0` away from being false, at which point an out-of-range
        // diagnostic would name the WRONG array with no other symptom.
        for w in arena.slots.windows(2) {
            assert!(
                w[1].off > w[0].off,
                "{}: slot offsets must strictly increase for `net_at_off` to be \
                 a bijection (got {} then {})",
                d.name,
                w[0].off,
                w[1].off
            );
        }
        for (n, s) in arena.slots.iter().enumerate() {
            assert_eq!(
                arena.net_at_off(s.off),
                n as u32,
                "{}: net_at_off must invert the layout",
                d.name
            );
        }
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
        // #10 stmt_locs: threaded for production parity. Always EMPTY here
        // (engine tests elaborate with no SpanResolver), so it changes nothing
        // today — the field exists so a future resolver-installing harness
        // cannot silently diverge from the production construction.
        stmt_locs: sc.stmt_locs,
        stmt_scopes: sc.stmt_scopes,
        expr_scopes: sc.expr_scopes,
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
        // …and the four ASSERTION side tables (V1 slice 1). Same failure mode as
        // every entry above, and it bit this file the day the tables became
        // reachable: SVA desugars to ordinary IR plus StmtId tables, so WITHOUT
        // them a `$assertoff` is a plain `Display` that PRINTS instead of
        // flipping `st.assert_disabled`, and the `assert_fire` suppression path
        // is never entered — an assertion-control row in the differential corpus
        // would then compare two backends on a design neither was executing as
        // written, and agree.
        //
        // `defer_marks`/`defer_acts` for the mirror-image reason: they are what
        // makes an `assert #0 (…)` design a DEFERRED assertion, and therefore what
        // makes tier-3's `deferred_assert` refusal row reachable from any test at
        // all. Without them that row is unreachable and a test asserting it fires
        // reads `Ok(())` — which is exactly how this omission was found.
        assert_fire: sc.assert_fire,
        assert_ctl: sc.assert_ctl,
        defer_marks: sc.defer_marks,
        defer_acts: sc.defer_acts,
        // …and the two QUEUE-OPERATION tables (V1 slice 2c). Same failure mode
        // as every entry above, and it made a row of this file's own corpus
        // vacuous ON THE DAY IT WAS WRITTEN: `r = q[a:b]` lowers to an ordinary
        // marker plus a `queue_slice_stmts` entry and `int bq[$:2]` to an
        // ordinary queue plus a `queue_bounds` entry, so WITHOUT the tables the
        // slice is not a slice and the bound is not a bound — and the row meant
        // to test them compares two backends on a design neither is executing
        // as written.
        //
        // ⚠️ How it was found is the part worth keeping: the mutation that
        // reverts `run_queue_slice`'s reader threading was killed ONLY by the
        // source-scan pin, never by the corpus row written for exactly that
        // defect. A source scan is a CHANGE DETECTOR; it cannot stand in for a
        // behaviour test, and a mutation battery that reports "killed" without
        // saying BY WHAT will hide the difference.
        queue_slice_stmts: sc.queue_slice_stmts,
        queue_bounds: sc.queue_bounds,
        // …and the WHOLE-HANDLE COPY markers (A8-a). Sixth entry in this list,
        // and it would have made this slice's test measure nothing: `d2 = d1`
        // between two handles lowers to a no-op `Display` PLUS this table, so
        // without it the statement prints nothing and copies nothing — on both
        // backends. The design would then agree about a deep copy neither
        // performed, which is the exact shape of the four notes above.
        handle_copy_stmts: sc.handle_copy_stmts,
        // …and the COVERAGE manifest (A7). Seventh, and the failure mode is the
        // one that makes this list worth keeping: a covergroup's SAMPLING is
        // ordinary IR either way, so a design without this table still runs and
        // still sets its bitmap bits — it just produces `SimResult.coverage ==
        // None`. A test asserting the summary would then compare `None` with
        // `None` and pass while the store-routing it exists to check never ran.
        coverage_manifest: sc.coverage_manifest,
        // …and the two heap ELEMENT refinements (V1 slice 3b). THIRD time this
        // file has been caught by the same omission (`assert_ctl` in slice 1,
        // `queue_slice_stmts` in 2c), and this one showed both failure modes at
        // once: without `string_elem_dyn_nets` a `string s[]` element is not a
        // byte string, so BOTH backends printed `[ ][\u{1}][ ] len=0` — agreeing
        // about a design neither executes as written — and without
        // `real_elem_dyn_nets` the two backends actually DIVERGED (1.5 vs 2.0),
        // because the handle is not `is_real` and the element coercion falls to a
        // bit resize the two paths reach differently.
        //
        // ⭐ The general rule this file keeps paying for: a sidecar is not
        // optional context, it is part of what the SOURCE MEANS. Anything the
        // corpus can spell must have its table here.
        real_elem_dyn_nets: sc.real_elem_dyn_nets,
        string_elem_dyn_nets: sc.string_elem_dyn_nets,
        // …and the two CALL-SITE binding tables (A3-i). FOURTH time this file has
        // been caught by the same omission, and this one is the starkest: a
        // `Terminator::Call` carries only `{target, ret_bb}`, so the argument↔formal
        // mapping IS this sidecar. Without it every call statement in every design
        // built here binds nothing — `run_process` falls straight through to
        // `bb = ret_bb` and so would the tier-3 arm, so `r = f(4, o)` leaves `o`
        // untouched and BOTH backends agree about a design neither performs.
        //
        // Measured, not reasoned: `s3a_a_call_statement_is_refused_by_the_executor_layer`
        // built exactly that design and its `task_calls_proc` came out EMPTY, which
        // is why the A3-i gate reported the call unrunnable for a reason that had
        // nothing to do with the callee.
        task_calls_proc: sc.task_calls_proc,
        task_calls_func: sc.task_calls_func,
        // …and the CLASS/OOP tables (A2-i). ⚠️ **FIFTH time**, and this one was
        // caught by reading the four notes above rather than by a failure — which
        // is the only reason it is not a sixth entry in that list. `class_field_widths`
        // was already here (§4.5.309 threaded it alone), so the file looked
        // covered while `class_handle_nets` — the table that drives BOTH
        // `SimState::class_is_handle` and the arena's `class` bitmap — was
        // missing. Without it a `class C; int f; endclass … o.f = 7` is not a
        // field access at all: `write_chunk`'s class lane never fires, the write
        // lands in the handle net's own slot, and BOTH backends do the same
        // wrong thing and agree.
        //
        // `class_layouts` carries the field widths AND the `new` defaults, so
        // `class_alloc` without it mints an object with ZERO fields and every
        // read is the stale/short warn; `class_new_sites` is what makes a
        // `BlockingAssign` a `ClassNew` effect rather than an ordinary const-0
        // assignment (so `k_class_alloc` is never entered); `class_vtable` /
        // `class_calls` are method dispatch.
        class_handle_nets: sc.class_handle_nets,
        class_new_sites: sc.class_new_sites,
        class_layouts: sc.class_layouts,
        class_field_inits: sc.class_field_inits,
        class_vtable: sc.class_vtable,
        class_calls: sc.class_calls,
        // The CRV half too. `class_crv`/`class_virtual` are refusal rows now, and
        // a row whose table is not installed is a row no test can reach — the
        // `defer_marks` lesson three entries up, applied before it bites.
        // …and the three CLOCKING tables (slice #1). ⚠️ **EIGHTH entry**, and it
        // has the list's worst failure mode: a `clocking` block lowers to an
        // ordinary holding net plus a marked `always @(clk);` handler whose body
        // is NULL, so WITHOUT these tables the handler is a process that does
        // nothing and `cb.sig` sits at its X init forever — on BOTH backends,
        // agreeing about a sample neither takes. Found by reading this list
        // before writing the slice's tests, as `class_handle_nets` was.
        // …and the ASSIGN RANK table (slice #2). ⚠️ **NINTH entry**, and it has
        // this list's signature failure mode: `force`/`assign` and
        // `release`/`deassign` lower to the SAME two IR statements, and this
        // sidecar is the ONLY thing that says which. Without it a procedural
        // `assign v = e;` is executed as a strong FORCE and `deassign` as a
        // `release` — by BOTH backends, so a differential row about §9.3.1
        // priority would agree about a design neither performs as written.
        // …and the FILE-DIRECTED marker set (slice #4). ⚠️ **TENTH entry.**
        // `$fmonitor`/`$fstrobe` REUSE the frozen `Monitor`/`Strobe` task ids —
        // this sidecar is the only thing that makes `args[0]` a descriptor —
        // so without it a `$fmonitor(fd, …)` is a plain `$monitor` that PRINTS
        // the fd as a value to stdout, on both backends, agreeing about a
        // design neither performs.
        // …and the STAGE marker set (slice #6). ELEVENTH entry: `$vita_stage`
        // lowers to a no-op `Display` plus this StmtId set, so without it the
        // call PRINTS its label and values to stdout instead of recording a
        // `stage.jsonl` line — on both backends.
        stage_stmts: sc.stage_stmts,
        file_directed_stmts: sc.file_directed_stmts,
        assign_ranks: sc.assign_ranks,
        clocking_inputs: sc.clocking_inputs,
        clocking_commit: sc.clocking_commit,
        clocking_outputs: sc.clocking_outputs,
        class_rand: sc.class_rand,
        class_constraints: sc.class_constraints,
        class_dist: sc.class_dist,
        class_randc: sc.class_randc,
        randomize_with: sc.randomize_with,
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
            let ch_n = arena.write_lvalue(&ir, lhs, val_n, &off_n, &[]);
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
        let ch_n = arena.write_lvalue(&ir, lhs, val_n, &off_n, &[]);

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
    let writes_module = "module t;\n\
           class C; int v; endclass\n\
           C c;\n\
           function automatic integer f(input integer x);\n\
             begin c.v = x; f = x; end\n\
           endfunction\n\
           integer r;\n\
           initial begin c = new(); r = f(3); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n";
    for (label, src, want) in [
        ("frame-contained", contained, None),
        // ⚠️ A3-iii: a READ no longer refuses — the delegated executor takes the
        // caller's store. The row narrowed to WRITES, and the only out-of-window
        // destination a subroutine body may have is a class field (a plain
        // module net is E3009 a phase earlier).
        ("reads a module net", reads_module, None),
        (
            "writes a module-scope class field",
            writes_module,
            // ⚠️⚠️ A3-iii-b INVERTED this row. A class FIELD write is admitted
            // now: its destination is `SimState::class_heap`, which both kernels
            // borrow, so it never needed routing — only the HANDLE READ did, and
            // that is threaded through `HeapRouted` (the caller's store BARE was
            // measured wrong: a `this` in a frame slot read back as null).
            //
            // The row still refuses a FLAT out-of-window write and has no
            // reachable design for it: elaborate rejects a function assigning a
            // module net (E3009 — measured on three spellings: direct, via a
            // local, and `c = new()`), and a non-`automatic` task is inlined, so
            // it has no frame at all. `writes_module` is kept under its name as
            // the POSITIVE case, because the shape it names is the one this
            // slice made work.
            None,
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
    let ch_n = arena.write_lvalue(ir, lhs, val_n, &off_n, &[]);
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
                    let cn = arena.write_lvalue(&ir, &lhs, v, &offs, &[]);
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

/// THE LEAF FAST-PATH LOCK — for BOTH stores that now offer one.
///
/// ⚠️⚠️ `state/netread.rs` said this test existed ("Locked by
/// `leaf_fast_path_matches_read_net`") and it did not. The engine's fast path
/// had been unlocked since it was written, and the D4 pre-census then gave
/// tier-3's arena the same fast path — the shortcut that took `struct-heavy`
/// from 0.88× the VM to 0.60× (ROADMAP §5.1-az). Two unlocked copies of a
/// sign-extension rule is precisely how two backends come to disagree about one
/// net, so this asserts the property both claim.
///
/// The property: whenever `read_scalar_words` answers `Some`, that answer is
/// bit-identical to what the slow path produces —
/// `read_net(net, None).resize_keep_sign(w, ctx_signed)` truncated to one word.
/// Swept over every net shape in the design × every 4-state value of a 4-bit
/// stimulus × both context signednesses × narrowing, equal and WIDENING context
/// widths, because sign extension only happens when widening and only for a
/// signed value whose sign bit is 1 or x.
///
/// The DECLINES are asserted too and each names a different reason: an array
/// (the offset arithmetic this path does not do), a `real` (whose stored bits
/// are an IEEE-754 double and must arrive as a stamped `Value`), and a >64-bit
/// net (two words).
#[test]
fn the_leaf_fast_path_matches_the_slow_path_on_both_stores() {
    let src = "module top;\n\
       reg [3:0] u4;\n\
       reg signed [3:0] s4;\n\
       reg u1;\n\
       reg signed [7:0] s8;\n\
       reg [63:0] u64n;\n\
       reg [99:0] wide;\n\
       real r;\n\
       reg [3:0] arr [0:3];\n\
       wire [3:0] keep; assign keep = u4 ^ s4;\n\
       initial begin u4 = 4'd0; s4 = 4'sd0; u1 = 1'b0; s8 = 8'sd0;\n\
                     u64n = 64'd0; wide = 100'd0; r = 0.0; arr[0] = 4'd0; end\n\
       endmodule\n";
    let ir = build(src);
    let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat");
    let sink = NullSink;
    let mut st = fresh_state(&ir, &sink);

    // Nets by SHAPE, not by name: every net the arena gave a slot to.
    let mut checked = 0usize;
    let mut declined = 0usize;
    let mut widened_signed = 0usize;

    for net in 0..arena.slots.len() as u32 {
        let sl = arena.slots[net as usize];
        if sl.width == 0 {
            continue;
        }
        // Sweep the low four bits through all 16 definite values and all 16
        // unknown patterns; wider nets get the same bits in their low word,
        // which is where the sign bit of the narrow shapes lives.
        for pv in 0u64..16 {
            for pu in 0u64..16 {
                let i = sl.off as usize;
                arena.buf[i] = pv;
                arena.buf[i + 1] = pu;
                if let Some(ns) = st.nets.get_mut(net as usize) {
                    if !ns.cur.val.is_empty() {
                        ns.cur.val[0] = pv;
                        ns.cur.unk[0] = pu;
                    }
                }
                for &w in &[1u32, 3, 4, 5, 8, 16, 33, 64] {
                    for &cs in &[false, true] {
                        // ── store 1: tier-3's arena ──
                        let fast = crate::eval::NetReader::read_scalar_words(&arena, net, w, cs);
                        let slow = crate::eval::NetReader::read_net(&arena, net, None)
                            .resize_keep_sign(w, cs);
                        match fast {
                            None => declined += 1,
                            Some((fv, fu)) => {
                                let m = crate::value::top_mask(w);
                                assert_eq!(
                                    (fv, fu),
                                    (
                                        slow.val.first().copied().unwrap_or(0) & m,
                                        slow.unk.first().copied().unwrap_or(0) & m
                                    ),
                                    "arena leaf fast path diverged: net={net} w={w} \
                                     ctx_signed={cs} val={pv:#x} unk={pu:#x}"
                                );
                                checked += 1;
                                if w > sl.width && sl.signed {
                                    widened_signed += 1;
                                }
                            }
                        }
                        // ── store 2: the engine, whose doc claimed this lock ──
                        let efast = crate::eval::NetReader::read_scalar_words(&st, net, w, cs);
                        if let Some((fv, fu)) = efast {
                            let eslow = crate::eval::NetReader::read_net(&st, net, None)
                                .resize_keep_sign(w, cs);
                            let m = crate::value::top_mask(w);
                            assert_eq!(
                                (fv, fu),
                                (
                                    eslow.val.first().copied().unwrap_or(0) & m,
                                    eslow.unk.first().copied().unwrap_or(0) & m
                                ),
                                "ENGINE leaf fast path diverged: net={net} w={w} \
                                 ctx_signed={cs} val={pv:#x} unk={pu:#x}"
                            );
                        }
                    }
                }
            }
        }
    }

    assert!(
        checked > 5_000,
        "only {checked} comparisons — the fast path is declining almost everything \
         and this test would pass while locking nothing"
    );
    assert!(
        declined > 0,
        "nothing declined — the array/real/wide rows stopped being present"
    );
    // ⚠️ ANTI-VACUITY on the one arm that can be wrong in a way narrowing hides:
    // sign extension happens ONLY when widening a signed net. Without rows in
    // that quadrant the whole extension block is untested and a mutant that
    // deletes it survives.
    assert!(
        widened_signed > 500,
        "only {widened_signed} widening-signed comparisons — the sign-extension \
         arm is the one this test exists for"
    );
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
        let mut st = fresh_state(&ir, &sink);
        let mut wake = crate::native::wake::WakeTable::new(&ir, &st);
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
                arena.write_lvalue(&ir, lhs, val_n, &off_n, &[]);
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
    // The clocking diversion list. Empty for every design in this corpus (no
    // `clocking` block spells one), which is why it is asserted rather than
    // dropped: a diversion that started firing here would silently REMOVE
    // processes from the compared list and the comparison would still pass.
    let mut native_clocked = Vec::new();
    wake.wake(&changed, &mut native_woken, &mut native_clocked);
    assert!(
        native_clocked.is_empty(),
        "{name}/pass{pass}: a clocking handler was diverted in a corpus with no clocking block"
    );
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
/// D2 TEETH — the 2-state lane, asked directly.
///
/// ⚠️⚠️ The exhaustive battery below ALREADY compares the lane against the
/// generic evaluator for every definite input, because `run` dispatches into
/// it. What that battery cannot see is the lane NOT FIRING: make `two_state`
/// always false and every differential in this repository still passes, because
/// the canonical loop is the fallback and the fallback is correct. A speedup
/// that silently stops happening is exactly the failure this slice can have,
/// so the first assertion here is a vacuity assertion.
///
/// The three rows are the three claims the design rests on:
///   1. on definite leaves the lane FIRES and agrees with the canonical loop,
///   2. on an unknown leaf it DECLINES (so the canonical loop answers), and
///   3. an x-carrying `Const` is refused at COMPILE time, because that leaf is
///      unknown on every evaluation and a per-run bail would never clear.
#[test]
fn d2_two_state_lane_fires_and_agrees_with_the_canonical_loop() {
    let src = "module top;\n\
       reg [3:0] a, b;\n\
       reg signed [3:0] sa, sb;\n\
       wire [3:0] o1; assign o1 = (a ^ b) + (a & ~b) - (a | b);\n\
       wire [3:0] o2; assign o2 = {a[1:0], b[3:2]};\n\
       wire [3:0] o3; assign o3 = (a < b) ? (a << 1) : (b >> 1);\n\
       wire       o4; assign o4 = (|a) && (&b) || (^a);\n\
       wire       o5; assign o5 = (sa >= sb) == (a != b);\n\
       wire [3:0] o6; assign o6 = a ^ 4'bxx01;\n\
       initial begin a = 4'd0; b = 4'd0; sa = 4'sd0; sb = 4'sd0; end\n\
       endmodule\n";
    let ir = build(src);
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat");
    let sig = |eid: u32| match ir.exprs.get(eid as usize) {
        Some(sim_ir::Expr::Signal { net, .. }) => *net,
        other => panic!("shape moved: {other:?}"),
    };
    // `o1`'s rhs is `((a^b) + (a&~b)) - (a|b)`; walk to the leftmost leaves.
    let (na, nb) = {
        let mut e = ir.cont_assigns[0].rhs;
        loop {
            match ir.exprs.get(e as usize) {
                Some(sim_ir::Expr::Binary { lhs, rhs, .. }) => {
                    if let Some(sim_ir::Expr::Binary {
                        lhs: l2, rhs: r2, ..
                    }) = ir.exprs.get(*lhs as usize)
                    {
                        if matches!(
                            ir.exprs.get(*l2 as usize),
                            Some(sim_ir::Expr::Signal { .. })
                        ) && matches!(
                            ir.exprs.get(*r2 as usize),
                            Some(sim_ir::Expr::Signal { .. })
                        ) {
                            break (sig(*l2), sig(*r2));
                        }
                    }
                    let _ = rhs;
                    e = *lhs;
                }
                other => panic!("shape moved: {other:?}"),
            }
        }
    };
    let mut progs = Vec::new();
    for ca in &ir.cont_assigns {
        let sw = wt.get(ca.rhs);
        if let Some(p) =
            crate::native::wprog::compile(&ir, &wt, &arena, ca.rhs, sw.width, sw.signed)
        {
            progs.push((ca.rhs, sw, p));
        }
    }
    assert!(progs.len() >= 6, "lane battery lost its programs");

    let set = |arena: &mut NetArena, net: u32, val: u64, unk: u64| {
        let s = arena.slots[net as usize];
        arena.buf[s.off as usize] = val;
        arena.buf[s.off as usize + 1] = unk;
    };
    // ⚠️ Every net starts at its CONSTRUCTION value, which is all-x — so a
    // program reading a net this sweep does not drive (`sa`, `sb`) would bail
    // for a reason that has nothing to do with what is being tested. Zeroing
    // the unknown plane once is what puts the whole design in the definite
    // world the first row is about.
    for i in 0..arena.slots.len() {
        let sl = arena.slots[i];
        assert_eq!(sl.words, 1, "this battery assumes one-word slots");
        arena.buf[sl.off as usize + 1] = 0;
    }
    let mut s2 = Vec::new();
    let mut s4 = crate::native::wprog::WScratch::default();

    // ROW 3 first: the x-carrying constant must be refused at COMPILE time.
    let x_const = progs.iter().filter(|(_, _, p)| !p.two_state_flag()).count();
    assert_eq!(
        x_const, 1,
        "exactly one row ({{a ^ 4'bxx01}}) carries an x constant; if this is 0 the \
         compile-time refusal stopped working, if it is >1 a row changed shape"
    );

    // ROW 1: definite leaves — the lane fires, and its answer IS the canonical
    // one. Swept over all 256 definite (a,b) pairs so no single value can
    // happen to agree.
    let mut fired = 0usize;
    for av in 0u64..16 {
        for bv in 0u64..16 {
            set(&mut arena, na, av, 0);
            set(&mut arena, nb, bv, 0);
            for (_, _, p) in &progs {
                let two = p.run_2s_for_test(&arena, &mut s2);
                let four = p.run_4s_for_test(&arena, &mut s4.w);
                if p.two_state_flag() {
                    let got = two.expect("definite leaves must not bail");
                    assert_eq!(
                        (got, 0u64),
                        (four.val, four.unk),
                        "2-state lane disagrees with the canonical loop at a={av} b={bv}"
                    );
                    fired += 1;
                }
            }
        }
    }
    assert!(
        fired >= 256 * 5,
        "the 2-state lane fired {fired} times — a lane that does not run is not a lane"
    );

    // ROW 2: one unknown leaf and the lane must DECLINE, so the canonical loop
    // (which `run` falls back to) is what answers.
    set(&mut arena, na, 0b0101, 0b0010);
    set(&mut arena, nb, 0b0011, 0);
    let mut declined = 0usize;
    for (_, _, p) in &progs {
        if p.two_state_flag() && p.run_2s_for_test(&arena, &mut s2).is_none() {
            declined += 1;
        }
        // `run` is still right either way, and that is the point of the design:
        // the fallback is the canonical implementation, not an approximation.
        let via_run = p.run(&arena, &mut s4);
        let direct = p.run_4s_for_test(&arena, &mut s4.w);
        assert_eq!(
            (via_run.val, via_run.unk),
            (direct.val, direct.unk),
            "dispatch changed the answer on an unknown input"
        );
    }
    assert!(
        declined >= 4,
        "only {declined} rows declined on an unknown leaf — the bail is not working"
    );
}

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
       wire f1; assign f1 = a && b;\n\
       wire f2; assign f2 = a || b;\n\
       wire f3; assign f3 = !a;\n\
       wire f4; assign f4 = &a;\n\
       wire f5; assign f5 = |a;\n\
       wire f6; assign f6 = ^a;\n\
       wire f7; assign f7 = ~&a;\n\
       wire f8; assign f8 = ~|a;\n\
       wire f9; assign f9 = ~^a;\n\
       wire g1; assign g1 = a === b;\n\
       wire g2; assign g2 = a !== b;\n\
       wire g3; assign g3 = sa === sb;\n\
       wire g4; assign g4 = (a + b) && (a ^ b);\n\
       wire g5; assign g5 = (a & b) === (a | b);\n\
       wire g6; assign g6 = !(a ^ b);\n\
       wire g7; assign g7 = a && (a < b);\n\
       wire g8; assign g8 = (a > b) || !sa;\n\
       wire g9; assign g9 = (a < b) && b;\n\
       wire ga; assign ga = !a || b;\n\
       wire [7:0] h1; assign h1 = {a, b};\n\
       wire [7:0] h2; assign h2 = {sa, b};\n\
       wire [7:0] h3; assign h3 = {4'b0, a};\n\
       wire [7:0] h4; assign h4 = {2{a}};\n\
       wire [11:0] h5; assign h5 = {{2{a}}, b};\n\
       wire [15:0] h6; assign h6 = {{a,b},{b,a}};\n\
       wire [7:0] h7; assign h7 = {a,b} + {b,a};\n\
       wire h8; assign h8 = {a,b} < {b,a};\n\
       wire [7:0] h9; assign h9 = ~{a,b};\n\
       wire i1; assign i1 = &{a,b};\n\
       wire [7:0] i2; assign i2 = {a,b} >> 2;\n\
       wire [11:0] i4; assign i4 = {a,sa,b};\n\
       wire       j1; assign j1 = a[2];\n\
       wire [1:0] j2; assign j2 = a[3:2];\n\
       wire [3:0] j3; assign j3 = {a,b}[5:2];\n\
       wire [3:0] j4; assign j4 = {a,b}[7:4];\n\
       wire       j5; assign j5 = sa[3];\n\
       wire [2:0] j6; assign j6 = a[2:0] ^ b[2:0];\n\
       wire [3:0] j7; assign j7 = {a[1:0],b[3:2]};\n\
       wire       j8; assign j8 = a[0] & b[3];\n\
       wire [1:0] j9; assign j9 = {2{a[1]}};\n\
       wire [3:0] ja; assign ja = a[3:0];\n\
       wire [3:0] k1; assign k1 = a[0] ? a : b;\n\
       wire signed [3:0] k2; assign k2 = sa[3] ? sa : sb;\n\
       wire [3:0] k3; assign k3 = a ? a : b;\n\
       wire [3:0] k4; assign k4 = (a < b) ? (a + b) : (a - b);\n\
       wire [3:0] k5; assign k5 = a[0] ? (a[1] ? a : b) : (a[2] ? b : a);\n\
       /* There is deliberately NO row dedicated to the unknown-condition X
          merge. Two were tried (identical arms; arms agreeing on half their
          bits) and MEASURED redundant - over 65,536 states the rows above
          already reach every agreement pattern, so both a give-up-whole-value
          mutant and an always-X one die without them. A row that decides
          nothing is worse than no row: it reads as coverage. */\n\
       wire [1:0] jc; assign jc = a[1 +: 2];\n\
       wire [1:0] jd; assign jd = a[3 -: 2];\n\
       wire [2:0] je; assign je = {a,b}[5 -: 3];\n\
       wire [3:0] jb; assign jb = a[4:1];\n\
       /* ⚠️ i4's signed part is in the MIDDLE on purpose. `{sa, b}` (h2) is
          structurally immune to a sign-extension bug: the fill lands ABOVE the
          result width and the mask deletes it. MEASURED — a mutant that
          sign-extends a signed part before splicing survives the whole battery
          without i4 and dies with it. */\n\
       wire [8:0]  i5; assign i5 = {a,1'b1,b};\n\
       wire [7:0] i3; assign i3 = {4'ha, {0{a}}, b};\n\
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
    assert_eq!(progs.len(), 86, "op battery coverage moved");
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
        declined, 3,
        "exactly three rows must DECLINE — `sa >>> 1` is the one shift whose bits \
         depend on the sign (arithmetic fill), the zero-replicate row \
         carries a \
         ZERO-WIDTH part, which slice 5 refuses rather than skips (the generic \
         path still EVALUATES that part, so skipping it could drop a diagnostic), \
         and `a[4:1]` runs one bit PAST a 4-bit source — slice 6 admits only a \
         window it can prove lies wholly inside the base, because the overhang \
         is the generic path's per-bit X-filling arm and this module has no \
         counterpart for it. \
         A battery where everything admits cannot show that the declines happen"
    );
    let rng = crate::state::RngCells::default();
    let mut scratch = crate::native::wprog::WScratch::default();
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
    assert_eq!(compared, 256 * 256 * 86);
}

/// The corpus + a keccak-shaped design, swept: every pure expression that the
/// W-compiler ADMITS must evaluate identically to the generic path over five
/// random 4-state states. The admitted count is pinned — a silent shrink of
/// the admission (or of the corpus) reads as coverage that no longer exists.
#[test]
fn s2_wprog_matches_generic_eval_on_admitted_corpus_trees() {
    let sink = NullSink;
    let mut admitted_total = 0usize;
    let mut widened_total = 0usize;
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
        let mut scratch = crate::native::wprog::WScratch::default();
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
            // ⭐ THE WIDENING SWEEP. The loop above compiles every tree at its OWN
            // width, so it reaches the widening admission only through inner nodes.
            // This asks the same trees in a WIDER context — which is the whole of
            // what that admission added, and the only place its two halves can
            // disagree with the generic path:
            //
            //   * a SELF-determined tree must be computed at its own width and then
            //     extended (sign-filled iff it and the context are both signed);
            //   * a CONTEXT-determined one must be computed at the WIDER width —
            //     `v[8:11] + 4'd1` is 16 at eight bits and 0 at four, and the first
            //     version of the admission got exactly that backwards.
            //
            // `eval_with` at the same `(w, signed)` is the oracle, so a wrong choice
            // of half shows up as a value mismatch rather than as a coverage number.
            for &eid in &pure {
                let sw = wt.get(eid);
                for extra in [1u32, 7, 24] {
                    let w = sw.width + extra;
                    if w > 64 {
                        continue;
                    }
                    for signed in [false, true] {
                        let Some(prog) =
                            crate::native::wprog::compile(&ir, &wt, &arena, eid, w, signed)
                        else {
                            continue;
                        };
                        let got = prog.run(&arena, &mut scratch);
                        let generic = eval_with(&ir, &wt, &rng_cells, &arena, eid, w, signed);
                        assert_eq!(
                            (got.val, got.unk),
                            (generic.val[0], generic.unk[0]),
                            "{name}: eid {eid} state {state_i} widened {} -> {w} signed {signed}",
                            sw.width
                        );
                        widened_total += 1;
                    }
                }
            }
        }
    }
    assert_eq!(
        admitted_total, 8225,
        "the admitted-tree coverage moved — re-pin deliberately (a DROP means \
         the admission or the corpus silently shrank). 7715 → 7890 at S2 slice 5 \
         (`Concat`/`Replicate`) → 7960 at S2 slice 6 (`Select`) → 8225 when a \
         narrower node stopped declining in a wider context (the widening admission)"
    );
    // ⚠️ A sweep that never fires passes every assertion in it. This is the teeth:
    // the widening admission MUST produce compiled programs, and the number is
    // pinned for the same reason the one above is.
    assert_eq!(
        widened_total, 45180,
        "the WIDENING sweep's coverage moved — re-pin deliberately; a drop to 0 \
         would mean the admission stopped firing and every value assertion above \
         it became vacuous"
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
    //
    // 131/1 → 132/1 when the §12.5 case-scrutinee capture landed: this design
    // has exactly ONE `case` (`case (rnd)`, line 116), and the capture assign it
    // now emits is an admitted RHS. The rise is the whole delta — `declined` is
    // unmoved, still the one part-select.
    let declined_total: usize = declined.values().sum();
    assert_eq!(
        (admitted, declined_total),
        (132, 1),
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
    let mut scratch = crate::native::wprog::WScratch::default();
    let (mut compared, mut saw_oob, mut saw_inrange) = (0usize, 0usize, 0usize);
    for (kv, ku) in cases {
        {
            let s = arena.slots[kn as usize];
            arena.buf[s.off as usize] = kv;
            arena.buf[s.off as usize + 1] = ku;
        }
        for (rhs, w, signed, prog) in &progs {
            let _ = arena.take_deferred_range_kinds().len(); // start from zero
            let got = prog.run(&arena, &mut scratch);
            let got_reports = arena.take_deferred_range_kinds().len();

            let want = eval_with(&ir, &wt, &rng, &arena, *rhs, *w, *signed);
            let want_reports = arena.take_deferred_range_kinds().len();

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

/// S2 slice 8 — the ≤64-bit truthiness fast path is the loop, exhaustively.
///
/// `truthiness` answers in three mask tests for a value that fits one word and
/// falls back to a per-bit loop above that. Those two are the SAME rule and this
/// row proves it rather than asserting it — and it does so WITHOUT a restatement
/// of the loop in the test: the same bit pattern is asked twice, once as a 4-bit
/// value (fast path) and once as a 128-bit value whose upper bits are zero (loop
/// path), so the loop itself is the reference.
///
/// Exhaustive over the 4-bit 4-state space: all 256 `(val, unk)` pairs, which
/// covers every combination of definite-1, definite-0, x and z — including the
/// three that decide the answer (a definite 1 anywhere wins; otherwise any x or
/// z at all is unknown; `z` is `val=1, unk=1` and must NOT count as a definite
/// 1).
#[test]
fn s2_truthiness_word_and_loop_agree_exhaustively() {
    let mut seen_true = 0usize;
    let mut seen_false = 0usize;
    let mut seen_unknown = 0usize;
    for p in 0u64..256 {
        let (v, u) = (p & 0xF, p >> 4);
        let mut narrow = crate::value::Value::zeros(4, false);
        narrow.val[0] = v;
        narrow.unk[0] = u;
        // The SAME bits at a width the fast path declines, so `truthiness`
        // takes the per-bit loop and is the oracle for this row.
        let mut wide = crate::value::Value::zeros(128, false);
        wide.val[0] = v;
        wide.unk[0] = u;
        let fast = crate::eval::truthiness(&narrow);
        let loopy = crate::eval::truthiness(&wide);
        let word = crate::eval::truthiness_word(v, u, crate::value::low_mask(4));
        assert_eq!(fast, loopy, "val={v:#x} unk={u:#x}: fast path vs loop");
        assert_eq!(word, loopy, "val={v:#x} unk={u:#x}: word entry vs loop");
        match loopy {
            crate::eval::Tri::True => seen_true += 1,
            crate::eval::Tri::False => seen_false += 1,
            crate::eval::Tri::Unknown => seen_unknown += 1,
        }
    }
    // Non-vacuity: all three verdicts must actually occur, or the row would
    // pass against an implementation that always returned one of them.
    assert!(
        seen_true > 0 && seen_false > 0 && seen_unknown > 0,
        "one verdict never occurred: true={seen_true} false={seen_false} unknown={seen_unknown}"
    );
    // Derived, not recorded: each of the four bits is one of 0/1/x/z, so
    // TRUE = "at least one bit is a definite 1" = 256 - 3^4 = 175, FALSE = "all
    // four are definite 0" = 1, and UNKNOWN is the rest = 80.
    assert_eq!((seen_true, seen_false, seen_unknown), (175, 1, 80));
}

/// A full snapshot of everything a write can move: the store, the dirty/edge
/// channel, the deferred VCD records and the deferred range diagnostics.
///
/// Comparing only `buf` is what made the S1d-2 gate blind to the edge KIND, and
/// comparing only the returned `changed` flag is what made an earlier offset
/// gate blind to a duplicated report. The word entry touches all four, so all
/// four are compared.
#[cfg(test)]
fn arena_snapshot(a: &NetArena) -> String {
    format!(
        "buf={:?} dirty={:?} edge={:?} lbw={:?} vcd={:?} range={:?}",
        a.buf,
        a.ch.dirty,
        a.ch.slot_edge,
        a.ch.last_blocking_writer,
        a.ch.vcd_pending,
        a.pending_range.borrow(),
    )
}

/// §4.5.332 — the ONE-WORD write entry against the general funnel, over every
/// lvalue shape a single-word destination can take, swept across offsets that
/// reach outside the net on both ends, array words that are in range / out of
/// range / unknown, and source values that are narrower, equal and wider than
/// the destination in both sign domains.
///
/// The two sides are the SAME function with the entry turned off, so what is
/// compared is exactly the delegation — and the comparison is the full snapshot
/// (store + dirty + edge + VCD + deferred diagnostics), because the entry is a
/// store point and a store point owns all four.
#[test]
fn s2_word_write_entry_matches_the_general_funnel() {
    // The `always @(posedge a)` is LOAD-BEARING, not decoration: `is_edge_target`
    // is derived from Edge sensitivities, so without one in the design every net
    // has `track_edge == false` and the glitch capture plus `accumulate_edge` —
    // half of what this store point owns — is never entered. Reviewing my own
    // sweep is what found that; the first version of this design had only
    // `initial` blocks.
    let src = "module t;\n\
                 reg [7:0] a; reg [0:0] b; reg [63:0] w; reg signed [7:0] sg;\n\
                 bit [7:0] tw; reg [70:0] wide;\n\
                 reg [7:0] m [0:3]; integer k; reg [7:0] v;\n\
                 always @(posedge a) begin end\n\
                 initial begin\n\
                   a = v; b = v[0]; w = v; sg = v; tw = v; wide = v;\n\
                   a[3] = v[0]; a[5:2] = v[3:0]; a[k+:3] = v[2:0]; a[k-:3] = v[2:0];\n\
                   m[k] = v; m[k][3] = v[0]; m[k][5:2] = v[3:0];\n\
                   {a, b} = v;\n\
                 end\n\
               endmodule\n";
    let (ir, opts) = build_with_opts(src);
    let sites = write_sites(&ir);
    assert_eq!(sites.len(), 14, "one site per assignment in the design");

    // Offsets that land inside, straddle the top, straddle bit 0 (a NEGATIVE
    // offset arrives as a 32-bit two's complement) and miss entirely.
    let offs: [u32; 8] = [0, 1, 5, 7, 8, 63, (-1i32) as u32, (-9i32) as u32];
    // In range, out of range, and the unknown-index sentinel.
    let words: [u32; 4] = [0, 3, 9, crate::eval::OFF_UNKNOWN];
    // (val, unk) plane pairs: defined, all-X, all-Z, and mixed.
    let planes: [(u64, u64); 5] = [
        (0x0000_0000_0000_0000, 0),
        (0xA5A5_A5A5_A5A5_A5A5, 0),
        (0, u64::MAX),
        (u64::MAX, u64::MAX),
        (0x0F0F_0F0F_0F0F_0F0F, 0x00FF_00FF_00FF_00FF),
    ];
    // Source widths that resize DOWN, exactly and UP (the last exercises the
    // sign-extension branch, which is why `signed` is swept with them), plus the
    // two FLAG domains the entry's admission reasons about but a plain
    // `Value::zeros` never sets: a REAL value (which the coercion above the entry
    // must have turned into an int — this row is what tests that claim rather
    // than asserting it) and a STRING value (whose flag `resize` drops, so the
    // entry is right to ignore it). `flag`: 0 = plain, 1 = real, 2 = string.
    let src_shapes: [(u32, bool, u8); 9] = [
        (1, false, 0),
        (4, true, 0),
        (8, false, 0),
        (8, true, 0),
        (32, true, 0),
        (64, false, 0),
        (64, false, 1),
        (64, true, 1),
        (24, false, 2),
    ];

    let mut compared = 0usize;
    let mut entered = 0usize;
    let mut landed = 0usize;
    for (si, (lhs, _rhs)) in sites.iter().enumerate() {
        for &off in &offs {
            for &word in &words {
                for &(pv, pu) in &planes {
                    for &(sw, ss, flag) in &src_shapes {
                        let mut fast = NetArena::build(&ir, &opts).expect("arena");
                        let mut slow = NetArena::build(&ir, &opts).expect("arena");
                        // The VCD capture inside `note_change` is a store-point
                        // effect too; with `vcd_on` false the snapshot would be
                        // blind to it.
                        fast.ch.vcd_on = true;
                        slow.ch.vcd_on = true;
                        fast.ch.blocking_writer = Some(7);
                        slow.ch.blocking_writer = Some(7);
                        let mut value = crate::value::Value::zeros(sw, ss);
                        value.val[0] = pv & crate::value::low_mask(sw);
                        value.unk[0] = pu & crate::value::low_mask(sw);
                        if flag == 1 {
                            // A real carries its f64 in `val[0]`; the planes above
                            // would be a nonsense payload, so give it a real one.
                            value.is_real = true;
                            value.unk[0] = 0;
                            value.val[0] = f64::to_bits(if pv == 0 { -3.25 } else { 7.5 });
                        } else if flag == 2 {
                            value.is_str = true;
                        }
                        let o = crate::exec::Offsets::Heap(
                            lhs.chunks.iter().map(|_| (off, word)).collect(),
                        );
                        let cf = fast.write_lvalue(&ir, lhs, value.clone(), &o, &[]);
                        let cs = slow.write_lvalue_general_for_test(&ir, lhs, value, &o);
                        let ctx = format!(
                            "site{si} off={off:#x} word={word:#x} \
                             planes=({pv:#x},{pu:#x}) src=({sw},{ss},flag={flag})"
                        );
                        assert_eq!(cf, cs, "{ctx}: `changed` verdict diverged");
                        assert_eq!(
                            arena_snapshot(&fast),
                            arena_snapshot(&slow),
                            "{ctx}: arena state diverged"
                        );
                        if lhs.chunks.len() == 1
                            && fast.slots[lhs.chunks[0].net as usize].words == 1
                        {
                            entered += 1;
                        }
                        if cf {
                            landed += 1;
                        }
                        compared += 1;
                    }
                }
            }
        }
    }
    assert_eq!(compared, 14 * 8 * 4 * 5 * 9, "sweep size");
    // Non-vacuity, three ways: the entry has to be TAKEN for most rows (otherwise
    // the sweep proves nothing about it), some row has to declare a real change
    // (otherwise every comparison is "nothing happened"), and at least one WRITTEN
    // net has to be an edge target (otherwise the glitch/edge half of the store
    // point is never entered — the gap that reviewing this test found).
    assert_eq!(entered, 12 * 8 * 4 * 5 * 9, "the word entry's row count");
    assert!(landed > 500, "too few rows actually stored: {landed}");
    let arena = NetArena::build(&ir, &opts).expect("arena");
    let edge_targets = arena.ch.is_edge_target.iter().filter(|&&b| b).count();
    assert!(
        edge_targets > 0,
        "no edge target: the glitch path is dead code"
    );
}

/// `resize_word`'s width-0 guard, against the ONLY oracle that can still answer:
/// `Value::resize`'s general (>64-bit) path, which the fast path no longer reaches.
///
/// A width-0 source copies `nwords(min(from,to)) == 0` words there, so the result is
/// zero no matter what the source words hold. The guard is what reproduces that.
/// PRODUCTION cannot tell the two apart — every constructor masks to `top_mask(0) == 0`,
/// so a width-0 `Value` has no set bits to leak — which is exactly why the row is here:
/// it is the only thing that distinguishes them, and without it a mutation removing the
/// guard survives the whole suite (measured).
#[test]
fn resize_word_zero_width_source_matches_the_general_path() {
    for &(v, u) in &[(0xFFu64, 0xFFu64), (u64::MAX, 0), (0, u64::MAX)] {
        for &signed in &[false, true] {
            // The general path: > 64 bits, so `resize` cannot delegate to the word form.
            let mut src = crate::value::Value::zeros(0, signed);
            src.val[0] = v;
            src.unk[0] = u;
            let general = src.resize(65);
            let (wv, wu) = crate::value::resize_word(v, u, 0, 64, signed);
            assert_eq!(
                (wv, wu),
                (general.val[0], general.unk[0]),
                "v={v:#x} u={u:#x} signed={signed}: word form vs general path"
            );
        }
    }
}

/// ⭐⭐ **B4b TEETH — in a product-shape build a gate refusal is FATAL, and this
/// test only exists there.**
///
/// ⚠️⚠️ It pins a hole that B2' opened. Gating the fall-back arm removed the only
/// consumer of the gate's verdict in this build, and `simulate` then ran the
/// design on tier-3 anyway: measured with a forced refusal in a
/// `--no-default-features` binary, the design RAN, exit 0, no diagnostic. The
/// gate said "out of scope" and nothing listened.
///
/// Fatal rather than a warning is the same ladder argument B4a used, read the
/// other way: with the VM compiled out there is no correct-support option, so
/// the choice is loud-or-wrong instead of loud-or-correct. In the default build
/// the same refusal is `W4030` plus a VM fall-back — both are pinned, in the two
/// builds where each is the right answer.
///
/// The refusal is a corrupted sidecar for the reason every B-phase test uses
/// one: Phase A closed every row a compiler can produce input for, so no `.sv`
/// reaches this.
#[cfg(not(feature = "oracle"))]
#[test]
fn b4b_a_refused_design_is_fatal_when_there_is_no_fallback() {
    use diag::{LogEvent, MsgCode};
    use std::cell::RefCell;

    #[derive(Default)]
    struct DiagSink {
        diags: RefCell<Vec<(MsgCode, String)>>,
    }
    impl diag::LogSink for DiagSink {
        fn emit(&self, event: LogEvent) {
            if let LogEvent::Diagnostic(d) = event {
                self.diags.borrow_mut().push((d.code, d.message));
            }
        }
    }

    let src = "module t;\n\
                 function automatic integer inc(input integer x);\n\
                   integer loc; begin loc = x + 1; inc = loc; end\n\
                 endfunction\n\
                 integer r;\n\
                 initial begin r = inc(3); #1 $finish; end\n\
               endmodule\n";
    let (ir, base) = build_with_opts(src);

    // CONTROL: nothing refuses it, so it runs and says nothing.
    let sink = DiagSink::default();
    let res = crate::simulate(&ir, &sink, base.clone());
    assert_eq!(
        res.exit_class,
        crate::ExitClass::Ok,
        "the control design must run: {:?}",
        sink.diags.borrow()
    );

    // …and the same design with a frame window that cannot exist.
    let mut broken = base;
    assert!(!broken.func_table.is_empty(), "no frame table to corrupt");
    broken.func_table[0].locals_len = u32::MAX;
    let sink = DiagSink::default();
    let res = crate::simulate(&ir, &sink, broken);
    assert_ne!(
        res.exit_class,
        crate::ExitClass::Ok,
        "a refused design must NOT run silently: {:?}",
        sink.diags.borrow()
    );
    let diags = sink.diags.borrow();
    let hit = diags
        .iter()
        .find(|(c, _)| *c == MsgCode::RunFatal)
        .unwrap_or_else(|| panic!("the refusal was silent: {diags:?}"));
    assert!(
        hit.1.contains("frame window out of range") && hit.1.contains("oracle"),
        "the message must name the refusing row AND why there is no fall-back: {}",
        hit.1
    );
}

/// ⭐ THE SINGLE-LEAF SHORT CIRCUIT, and why it needs its own test.
///
/// `WProg::run` answers a one-`Load`/one-`Const` program directly instead of
/// entering an executor. That is a pure performance change, which means the
/// whole differential apparatus in this file is BLIND to it: the fast arm
/// returns the same value the ops return, so a lane that never fires and a lane
/// that always fires are indistinguishable by output. The same anti-vacuity
/// problem `two_state_flag` exists for.
///
/// So this asserts both halves separately — which programs take the short
/// circuit (classification), and that the value is the one the executor would
/// have produced (equivalence), over both planes.
#[test]
fn a_single_leaf_program_short_circuits_and_agrees_with_the_executor() {
    let src = "module m;\n\
       reg [7:0] a, b;\n\
       wire [7:0] leaf   = a;          // one Load  -> fast kind 1\n\
       wire [7:0] lit    = 8'hA5;      // one Const -> fast kind 2\n\
       wire [7:0] xlit   = 8'bxx01xx01; // an x-carrying Const, still one op\n\
       wire [7:0] two    = a ^ b;      // more than one op -> kind 0\n\
       initial begin a = 8'd0; b = 8'd0; end\n\
       endmodule\n";
    let ir = build(src);
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat");
    for i in 0..arena.slots.len() {
        let sl = arena.slots[i];
        arena.buf[sl.off as usize + 1] = 0;
    }
    let progs: Vec<_> = ir
        .cont_assigns
        .iter()
        .map(|ca| {
            let sw = wt.get(ca.rhs);
            crate::native::wprog::compile(&ir, &wt, &arena, ca.rhs, sw.width, sw.signed)
                .expect("every row of this design compiles")
        })
        .collect();
    assert_eq!(progs.len(), 4, "design shape moved");

    // CLASSIFICATION — the anti-vacuity half. If these all became 0 the short
    // circuit would be dead code and every other test would still pass.
    let kinds: Vec<u8> = progs.iter().map(|p| p.fast_kind()).collect();
    assert_eq!(kinds, vec![1, 2, 2, 0], "got {kinds:?}");

    // EQUIVALENCE — over a sweep, on both planes, against the executor the fast
    // arm skips. `run_ops_for_test` is `run` with the short circuit bypassed.
    let mut sc = crate::native::wprog::WScratch::default();
    let a_net = match ir.exprs.get(ir.cont_assigns[0].rhs as usize) {
        Some(sim_ir::Expr::Signal { net, .. }) => *net,
        other => panic!("shape moved: {other:?}"),
    };
    let slot = arena.slots[a_net as usize];
    for val in [0u64, 1, 0x5a, 0xff] {
        for unk in [0u64, 0x0f, 0xff] {
            arena.buf[slot.off as usize] = val & !unk;
            arena.buf[slot.off as usize + 1] = unk;
            for p in &progs {
                let fast = p.run(&arena, &mut sc);
                let slow = p.run_ops_for_test(&arena, &mut sc);
                assert_eq!(
                    (fast.val, fast.unk),
                    (slow.val, slow.unk),
                    "kind {} diverged at val={val:#x} unk={unk:#x}",
                    p.fast_kind()
                );
            }
        }
    }
}

/// ⭐ THE SIGN SEAL, and why it needs a test the differential cannot give.
///
/// `expr_cast` wraps every size cast's result in `$signed`/`$unsigned`, and this
/// module used to decline the node — so `8'(e)` fell to the generic evaluator.
/// An external reviewer's workload took 14.6 million of those from a source that
/// never writes `$signed`; a cast loop measured 0.678 s sealed against 0.480 s
/// unsealed, and after the arm the two are equal.
///
/// The seal emits NO OPS, which is exactly why a value differential is blind to
/// it: sealed and unsealed compute the same thing whether or not the arm fired.
/// So this asserts the two halves separately — that the sealed form COMPILES
/// (before, it did not) and that it compiles to the SAME PROGRAM as the
/// unsealed one.
#[test]
fn a_size_casts_sign_seal_compiles_to_the_same_program_as_its_operand() {
    let src = "module m;\n\
       logic [7:0] a, b;\n\
       wire [7:0] sealed   = 8'((a << 4) | b);   // cast → `$signed`/`$unsigned` seal\n\
       wire [7:0] unsealed = (a << 4) | b;       // same expression, no seal\n\
       initial begin a = 8'h3c; b = 8'h5a; end\n\
     endmodule\n";
    let ir = build(src);
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let arena = NetArena::build(&ir, &SimOpts::default()).expect("flat");

    let prog = |i: usize| {
        let ca = &ir.cont_assigns[i];
        let sw = wt.get(ca.rhs);
        crate::native::wprog::compile(&ir, &wt, &arena, ca.rhs, sw.width, sw.signed)
    };
    // ANTI-VACUITY: the sealed row must compile at all. This is the assertion
    // that fails if the arm is deleted — every value test stays green.
    let sealed = prog(0).expect("the sealed cast must reach the compiled lane");
    let unsealed = prog(1).expect("control row");
    assert_eq!(
        sealed.ops_len(),
        unsealed.ops_len(),
        "the seal is a stamp, not a computation — it must add no ops"
    );
}
