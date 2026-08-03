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
        let arena = NetArena::build(&ir)
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
    let mut arena = NetArena::build(&ir).expect("flat design");
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
        let mut arena = NetArena::build(&ir).expect("corpus is flat");
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
