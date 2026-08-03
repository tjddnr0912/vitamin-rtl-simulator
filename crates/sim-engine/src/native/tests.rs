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
struct NullSink;
impl diag::LogSink for NullSink {
    fn emit(&self, _e: diag::LogEvent) {}
}

fn fresh_state<'a>(ir: &'a SimIr, sink: &'a NullSink) -> SimState<'a> {
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
fn set_bit(bp: &mut sim_ir::BitPacked, i: u32, v: u64, u: u64) {
    let w = (i / 64) as usize;
    let b = i % 64;
    bp.val[w] = (bp.val[w] & !(1 << b)) | (v << b);
    bp.unk[w] = (bp.unk[w] & !(1 << b)) | (u << b);
}

/// Mirror ONE random 4-state value into element `e` of net `n` in BOTH stores.
/// `defined_only` forces `unk = 0` (the arithmetic-heavy profile); otherwise
/// ~25% of bits are X/Z.
fn mirror_random_elem(
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
        compared, 17940,
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
fn build_with_opts(src: &str) -> (SimIr, SimOpts) {
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
    let opts = SimOpts {
        two_state_nets: sc.two_state_nets,
        func_table: sc.func_table,
        ..SimOpts::default()
    };
    (ir.expect("elaborate"), opts)
}

/// Every (lhs, rhs) write site in the design: procedural assigns of both kinds
/// plus continuous assigns. Taking them from the arenas (rather than walking
/// process bodies) reaches every body — a subroutine's statements live in the
/// same `ir.stmts`.
fn write_sites(ir: &SimIr) -> Vec<(sim_ir::Lvalue, u32)> {
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
fn mirror_state(
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
fn assert_stores_equal(st: &SimState, arena: &NetArena, n_nets: u32, what: &str) {
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

/// The frame-local lane, pinned as a REFUSAL rather than left to a mirror that
/// agrees only because the engine's routing was never installed.
///
/// User calls are CORE at S0 (revision 4: S3 absorbs them), so a subroutine
/// design is `eligible: true` — but its locals live in the activation's frame
/// window, and BOTH the arena's read path and its write funnel are frame-blind.
/// `NetArena::build` therefore refuses, and this test states all three facts
/// together so the next slice cannot mistake the gap for coverage.
#[test]
fn a_subroutine_design_is_eligible_but_the_arena_refuses_it() {
    let src = "module t;\n\
           function automatic integer f(input integer x);\n\
             integer loc;\n\
             begin loc = x + 1; f = loc; end\n\
           endfunction\n\
           integer r;\n\
           initial begin r = f(3); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n";
    let (ir, opts) = build_with_opts(src);
    assert!(
        !opts.func_table.is_empty(),
        "the design must actually produce a frame table"
    );
    // S0 says yes — calls are core, and this is the documented design-level
    // UPPER BOUND (`native.eligible` is not a promise the executor exists).
    assert!(
        crate::native::design_eligibility(&ir, &opts).eligible,
        "calls are CORE at S0 (rev-4)"
    );
    // The storage says no, structurally.
    assert_eq!(
        NetArena::build(&ir, &opts).err(),
        Some("frame-local storage: S3 (subroutine frames)"),
        "the arena must refuse rather than give frame locals ordinary slots"
    );
}
