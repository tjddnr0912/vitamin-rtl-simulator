//! S1a/S1b adversarial probes — AUTHORED BY the differential review of the
//! slice and absorbed upstream on its own recommendation: the shipped corpus
//! mirror (tests.rs) only reaches array element widths 4-16, so the seam its
//! own doc names highest-risk (the engine's bit-serial UNALIGNED element path
//! vs the arena's aligned copy) was unmeasured beyond 16 bits. These close it:
//!
//! A. Adversarial widths 1/7/8/33/63/64/65/95/127/128/129/191/192/193 as
//!    ARRAYS (elements unaligned in the ENGINE flat store, aligned in the
//!    arena), compared 3-WAY: arena read vs engine read vs the ground-truth
//!    Value the mirrored words define. Deterministic patterns (all-0/1/X/Z,
//!    boundary single-bits, SHORT + DIRTY set_elem inputs) + many random
//!    states. Includes phase-sweep arrays (64 elements of width 63/65 → every
//!    bit-phase 0..63 of the engine's bit-serial path).
//! B. X/Z/OOB-driven index reads through the SHARED evaluator: `m[idx]` where
//!    idx is driven to in-range / OOB / u32-overflow / u64-overflow / X / Z /
//!    negative values by the mirrored state; every pure expr evaluated on both
//!    readers at two context widths.
//! C. Signed + context resize matrix: signed/unsigned nets (incl. signed
//!    UNALIGNED array elements) read under ctx widths w..w+65 × ctx signed
//!    {false,true}, with the sign bit forced to 0/1/X/Z.
//! D. Whole-net read on ARRAY nets with DISTINCT element values (folded into
//!    probe A: every pass asserts read_net(n, None) == read_net(n, Some(0)) on
//!    both readers, on states where elements differ).

use super::test_common as common;
use common::{build, Rng};
use sim_ir::{Expr, NetKind, SimIr};

use crate::eval::{EvalCtx, NetReader};
use crate::native::arena::NetArena;
use crate::state::SimState;
use crate::value::{top_mask, Value, Words};
use crate::width::WidthTable;
use crate::SimOpts;

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

fn r64(rng: &mut Rng) -> u64 {
    (rng.range(0, u32::MAX as u64) << 32) | rng.range(0, u32::MAX as u64)
}

fn set_bit(bp: &mut sim_ir::BitPacked, i: u32, v: u64, u: u64) {
    let w = (i / 64) as usize;
    let b = i % 64;
    bp.val[w] = (bp.val[w] & !(1 << b)) | (v << b);
    bp.unk[w] = (bp.unk[w] & !(1 << b)) | (u << b);
}

/// Write `(vw, uw)` into element `e` of net `n` in BOTH stores and return the
/// ground-truth `Value` those words define. The RAW (possibly dirty-top,
/// possibly short) slices go to `set_elem` — its masking/padding is under
/// test; the engine mirror and the ground truth use the canonical masked form.
fn mirror_set(
    st: &mut SimState,
    arena: &mut NetArena,
    n: u32,
    e: u32,
    vw: &[u64],
    uw: &[u64],
) -> Value {
    let s = arena.slots[n as usize];
    let words = s.words as usize;
    let m = top_mask(s.width.max(1));
    let mut vm: Vec<u64> = (0..words)
        .map(|k| vw.get(k).copied().unwrap_or(0))
        .collect();
    let mut um: Vec<u64> = (0..words)
        .map(|k| uw.get(k).copied().unwrap_or(0))
        .collect();
    vm[words - 1] &= m;
    um[words - 1] &= m;
    arena.set_elem(n, e, vw, uw);
    let base = e * s.width;
    let cur = &mut st.nets[n as usize].cur;
    for i in 0..s.width {
        let v = (vm[(i / 64) as usize] >> (i % 64)) & 1;
        let u = (um[(i / 64) as usize] >> (i % 64)) & 1;
        set_bit(cur, base + i, v, u);
    }
    Value {
        val: Words::from_slice_padded(&vm, words),
        unk: Words::from_slice_padded(&um, words),
        width: s.width,
        signed: s.signed,
        is_real: false,
        is_str: false,
    }
}

/// The single net whose NetVar satisfies `pred` (asserts uniqueness so a
/// lowering change cannot silently retarget a probe).
fn find_net(ir: &SimIr, what: &str, pred: impl Fn(&sim_ir::NetVar) -> bool) -> u32 {
    let hits: Vec<u32> = (0..ir.nets.len() as u32)
        .filter(|&n| pred(&ir.nets[n as usize]))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "{what}: expected exactly one net, got {hits:?}"
    );
    hits[0]
}

/// Shipped purity filter, duplicated verbatim (the probe must not depend on
/// the module under review for its own instrumentation).
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

fn pure_exprs(ir: &SimIr) -> Vec<u32> {
    let mut memo = vec![None; ir.exprs.len()];
    (0..ir.exprs.len() as u32)
        .filter(|&eid| pure_expr(ir, &mut memo, eid))
        .collect()
}

fn eval_with<'a, N: NetReader>(
    ir: &'a SimIr,
    wt: &'a WidthTable,
    rng: &'a crate::state::RngCells,
    nets: &'a N,
    eid: u32,
    ctx_w: u32,
    ctx_signed: bool,
) -> Value {
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

// ─────────────────────────────────────────────────────────────────────────────
// Probe A + D: adversarial-width arrays, direct 3-way read comparison.
// ─────────────────────────────────────────────────────────────────────────────

const A_SRC: &str = "module t;\n\
  reg [0:0] a1 [0:6];\n\
  reg [6:0] a7 [0:6];\n\
  reg [7:0] a8 [0:7];\n\
  reg [32:0] a33 [0:4];\n\
  reg [62:0] a63 [0:4];\n\
  reg [63:0] a64 [0:4];\n\
  reg [64:0] a65 [0:4];\n\
  reg [94:0] a95 [0:3];\n\
  reg [126:0] a127 [0:2];\n\
  reg [127:0] a128 [0:2];\n\
  reg [128:0] a129 [0:2];\n\
  reg [190:0] a191 [0:2];\n\
  reg [191:0] a192 [0:2];\n\
  reg [192:0] a193 [0:2];\n\
  reg signed [62:0] s63 [0:3];\n\
  reg signed [63:0] s64 [0:3];\n\
  reg signed [64:0] s65 [0:3];\n\
  reg signed [128:0] s129 [0:2];\n\
  integer iarr [0:3];\n\
  reg [62:0] ph63 [0:63];\n\
  reg [64:0] ph65 [0:63];\n\
endmodule\n";

#[test]
fn probe_a_adversarial_width_arrays_direct_read() {
    let sink = NullSink;
    let ir = build(A_SRC);
    let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat design");
    let mut st = fresh_state(&ir, &sink);

    // Anti-vacuity: the design must actually contain the unaligned seam — at
    // least one array whose element width is NOT a multiple of 64.
    assert!(
        arena.slots.iter().any(|s| s.elems > 1 && s.width % 64 != 0),
        "no unaligned-element array in the probe design"
    );

    // Init parity FIRST (X-broadcast across multi-word wide arrays).
    for n in 0..ir.nets.len() as u32 {
        let elems = arena.slots[n as usize].elems;
        for e in 0..elems {
            assert_eq!(
                arena.read_net(n, Some(e)),
                st.read_net(n, Some(e)),
                "init parity net {n} elem {e}"
            );
        }
        assert_eq!(
            arena.read_net(n, None),
            st.read_net(n, None),
            "init parity net {n} whole"
        );
    }

    let mut elem_points = 0usize;
    let mut oob_points = 0usize;
    let mut whole_points = 0usize;
    for seed in [0xBAD_5EEDu64, 0x0DDC_0FFE, 0x7EA5_ED42] {
        let mut rng = Rng::new(seed);
        for pass in 0..24 {
            for n in 0..ir.nets.len() as u32 {
                let s = arena.slots[n as usize];
                let words = s.words as usize;
                for e in 0..s.elems {
                    let (vw, uw): (Vec<u64>, Vec<u64>) = match pass {
                        0 => (vec![0; words], vec![0; words]), // all-0
                        // all-1 with DIRTY top word — set_elem must mask
                        1 => (vec![u64::MAX; words], vec![0; words]),
                        2 => (vec![0; words], vec![u64::MAX; words]), // all-X (dirty top)
                        3 => (vec![u64::MAX; words], vec![u64::MAX; words]), // all-Z
                        4 => {
                            // single bits at every word-boundary position in range
                            let mut v = vec![0u64; words];
                            for b in [0u32, 6, 31, 62, 63, 64, 65, 126, 127, 128, 190, 191, 192] {
                                if b < s.width {
                                    v[(b / 64) as usize] |= 1 << (b % 64);
                                }
                            }
                            let u = v.iter().map(|w| w >> 1).collect(); // X just below each
                            (v, u)
                        }
                        5 => (vec![r64(&mut rng)], vec![]), // SHORT slices: missing words = 0
                        _ => (
                            (0..words).map(|_| r64(&mut rng)).collect(),
                            (0..words)
                                .map(|_| {
                                    if pass % 3 == 0 {
                                        r64(&mut rng) // dense X/Z
                                    } else {
                                        r64(&mut rng) & r64(&mut rng) & r64(&mut rng)
                                        // sparse
                                    }
                                })
                                .collect(),
                        ),
                    };
                    let expect = mirror_set(&mut st, &mut arena, n, e, &vw, &uw);
                    let ar = arena.read_net(n, Some(e));
                    let en = st.read_net(n, Some(e));
                    assert_eq!(
                    ar, expect,
                    "ARENA diverges from ground truth: net {n} (w={}, elems={}) elem {e} pass {pass}",
                    s.width, s.elems
                );
                    assert_eq!(
                    en, expect,
                    "ENGINE diverges from ground truth: net {n} (w={}, elems={}) elem {e} pass {pass}",
                    s.width, s.elems
                );
                    elem_points += 1;
                }
                // OOB element reads: first-OOB, far-OOB, and the u32::MAX sentinel.
                for oob in [s.elems, s.elems + 7, u32::MAX] {
                    assert_eq!(
                        arena.read_net(n, Some(oob)),
                        st.read_net(n, Some(oob)),
                        "OOB parity net {n} word {oob} pass {pass}"
                    );
                    oob_points += 1;
                }
                // Whole-net read on the array (elements now DISTINCT under random
                // passes): must equal element 0 on BOTH readers.
                let wa = arena.read_net(n, None);
                let we = st.read_net(n, None);
                assert_eq!(wa, we, "whole-net parity net {n} pass {pass}");
                assert_eq!(
                    wa,
                    arena.read_net(n, Some(0)),
                    "arena whole-net must be element 0: net {n} pass {pass}"
                );
                assert_eq!(
                    we,
                    st.read_net(n, Some(0)),
                    "engine whole-net must be element 0: net {n} pass {pass}"
                );
                whole_points += 1;
            }
        }
    }
    eprintln!(
        "probe A: nets={} elem_points={elem_points} oob_points={oob_points} whole_points={whole_points}",
        ir.nets.len()
    );
    assert_eq!(elem_points, 15192, "probe A elem coverage moved");
    assert_eq!(oob_points, 4536, "probe A OOB coverage moved");
    assert_eq!(whole_points, 1512, "probe A whole-net coverage moved");
}

// ─────────────────────────────────────────────────────────────────────────────
// Probe B: X/Z/OOB/overflow-driven index reads through the shared evaluator.
// ─────────────────────────────────────────────────────────────────────────────

const B_SRC: &str = "module t;\n\
  reg [7:0] m [0:2];\n\
  reg [7:0] idx;\n\
  reg [95:0] wi;\n\
  reg [63:0] qi;\n\
  integer si;\n\
  wire [7:0] y = m[idx];\n\
  wire [7:0] z = m[wi];\n\
  wire [7:0] p = m[qi];\n\
  wire [7:0] ns = m[si];\n\
  wire [16:0] cat = {m[idx], 1'b0, m[wi]};\n\
endmodule\n";

#[test]
fn probe_b_xz_oob_overflow_index_reads() {
    let sink = NullSink;
    let ir = build(B_SRC);
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat design");
    let mut st = fresh_state(&ir, &sink);

    let m = find_net(&ir, "m", |nv| nv.array_len == 3);
    let idx = find_net(&ir, "idx", |nv| {
        nv.kind == NetKind::Reg && nv.width == 8 && nv.array_len <= 1
    });
    let wi = find_net(&ir, "wi", |nv| nv.width == 96);
    let qi = find_net(&ir, "qi", |nv| nv.kind == NetKind::Reg && nv.width == 64);
    let si = find_net(&ir, "si", |nv| nv.kind == NetKind::Integer);

    // Anti-vacuity: the lowering must contain indexed Signal reads of `m`.
    let word_signals = ir
        .exprs
        .iter()
        .filter(|e| matches!(e, Expr::Signal { net, word: Some(_) } if *net == m))
        .count();
    assert!(
        word_signals >= 5,
        "expected >=5 Signal-with-word reads of m, got {word_signals}"
    );

    let pure = pure_exprs(&ir);
    assert!(!pure.is_empty());

    // Two element-fill profiles for m: distinct DEFINED values, then X/Z-heavy
    // (a valid index must return the stored X pattern identically).
    let m_fills: [[(u64, u64); 3]; 2] = [
        [(0x11, 0), (0x22, 0), (0x33, 0)],
        [(0x0F, 0xF0), (0xAA, 0xAA), (0x00, 0xFF)],
    ];
    // Index states: (net, val words, unk words, label).
    type S = (u32, Vec<u64>, Vec<u64>, &'static str);
    let states: Vec<S> = vec![
        (idx, vec![0], vec![0], "idx=0"),
        (idx, vec![1], vec![0], "idx=1"),
        (idx, vec![2], vec![0], "idx=2"),
        (idx, vec![3], vec![0], "idx=3 first-OOB"),
        (idx, vec![4], vec![0], "idx=4 OOB"),
        (idx, vec![255], vec![0], "idx=255 OOB"),
        (idx, vec![0x00], vec![0xFF], "idx=all-X"),
        (idx, vec![0xFF], vec![0xFF], "idx=all-Z"),
        (idx, vec![0x02], vec![0x01], "idx bit0 X, rest in-range"),
        (idx, vec![0x01], vec![0x80], "idx high-bit X, low clean"),
        (wi, vec![0, 0], vec![0, 0], "wi=0"),
        (wi, vec![1, 0], vec![0, 0], "wi=1"),
        (wi, vec![2, 0], vec![0, 0], "wi=2"),
        (wi, vec![3, 0], vec![0, 0], "wi=3 first-OOB"),
        (
            wi,
            vec![0xFFFF_FFFF, 0],
            vec![0, 0],
            "wi=u32::MAX (fits u32, OOB)",
        ),
        (
            wi,
            vec![0x1_0000_0000, 0],
            vec![0, 0],
            "wi=2^32 (u32 overflow → sentinel)",
        ),
        (
            wi,
            vec![0, 1],
            vec![0, 0],
            "wi=2^64 (to_u64 None → sentinel)",
        ),
        (
            wi,
            vec![1, 0],
            vec![0, 0x8000_0000],
            "wi bit95 X, low=1 (X poisons index)",
        ),
        (
            wi,
            vec![1, 0x8000_0000],
            vec![0, 0x8000_0000],
            "wi bit95 Z, low=1",
        ),
        (qi, vec![2], vec![0], "qi=2"),
        (qi, vec![0xFFFF_FFFF], vec![0], "qi=u32::MAX (fits, OOB)"),
        (
            qi,
            vec![0x1_0000_0000],
            vec![0],
            "qi=2^32 (u32 overflow → sentinel)",
        ),
        (
            qi,
            vec![u64::MAX],
            vec![0],
            "qi=u64::MAX (u32 overflow → sentinel)",
        ),
        (qi, vec![5], vec![1 << 40], "qi mid-bit X"),
        (si, vec![1], vec![0], "si=1"),
        (
            si,
            vec![0xFFFF_FFFF],
            vec![0],
            "si=-1 (reads as huge unsigned → OOB)",
        ),
        (si, vec![0x8000_0000], vec![0], "si=INT_MIN (OOB)"),
    ];

    let rng_a = crate::state::RngCells::default();
    let rng_b = crate::state::RngCells::default();
    let mut compared = 0usize;
    for (fi, fill) in m_fills.iter().enumerate() {
        for (e, &(v, u)) in fill.iter().enumerate() {
            mirror_set(&mut st, &mut arena, m, e as u32, &[v], &[u]);
        }
        for (net, vw, uw, label) in &states {
            mirror_set(&mut st, &mut arena, *net, 0, vw, uw);
            for &eid in &pure {
                let sw = wt.get(eid);
                for (cw, cs) in [(sw.width, sw.signed), (sw.width + 63, false)] {
                    let engine = eval_with(&ir, &wt, &rng_a, &st, eid, cw, cs);
                    let native = eval_with(&ir, &wt, &rng_b, &arena, eid, cw, cs);
                    assert_eq!(
                        engine, native,
                        "fill {fi} state [{label}] eid {eid} ctx ({cw},{cs})"
                    );
                    compared += 1;
                }
            }
            // Direct read of every element + sentinel while in this state.
            for w in [0, 1, 2, 3, u32::MAX] {
                assert_eq!(
                    arena.read_net(m, Some(w)),
                    st.read_net(m, Some(w)),
                    "fill {fi} state [{label}] direct m[{w}]"
                );
                compared += 1;
            }
        }
    }
    eprintln!(
        "probe B: pure_exprs={} states={} compared={compared}",
        pure.len(),
        states.len()
    );
    // 2 fills × 27 states × (37 pure exprs × 2 ctxs + 5 direct reads) = 4266.
    // (§4.5.308 re-pinned this without re-deriving the formula — its `14` gave
    // 1782, not the 2862 it asserted. §4.5.310 raised the pure-expr count by
    // giving the array-word index funnel a Select/Mul/Concat form, so the
    // derivation is restated here rather than the number bumped again.)
    assert_eq!(compared, 4266, "probe B coverage moved");
}

// ─────────────────────────────────────────────────────────────────────────────
// Probe C: signed + context-resize matrix, sign bit forced to 0/1/X/Z.
// ─────────────────────────────────────────────────────────────────────────────

const C_SRC: &str = "module t;\n\
  reg signed [3:0] s4;\n\
  reg signed [6:0] s7;\n\
  reg signed [31:0] s32;\n\
  reg signed [62:0] s63;\n\
  reg signed [63:0] s64;\n\
  reg signed [64:0] s65;\n\
  reg signed [126:0] s127;\n\
  integer i32;\n\
  reg [3:0] u4;\n\
  reg [63:0] u64x;\n\
  reg [64:0] u65;\n\
  reg signed [64:0] sa65 [0:2];\n\
  reg signed [62:0] sa63 [0:2];\n\
  wire signed [3:0] c0 = s4;\n\
  wire signed [6:0] c1 = s7;\n\
  wire signed [31:0] c2 = s32;\n\
  wire signed [62:0] c3 = s63;\n\
  wire signed [63:0] c4 = s64;\n\
  wire signed [64:0] c5 = s65;\n\
  wire signed [126:0] c6 = s127;\n\
  wire signed [31:0] c7 = i32;\n\
  wire [3:0] c8 = u4;\n\
  wire [63:0] c9 = u64x;\n\
  wire [64:0] c10 = u65;\n\
  wire signed [64:0] c11 = sa65[1];\n\
  wire signed [62:0] c12 = sa63[2];\n\
endmodule\n";

#[test]
fn probe_c_signed_context_resize_matrix() {
    let sink = NullSink;
    let ir = build(C_SRC);
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    let mut arena = NetArena::build(&ir, &SimOpts::default()).expect("flat design");
    let mut st = fresh_state(&ir, &sink);

    let pure = pure_exprs(&ir);
    // Anti-vacuity: there must be signed Signal reads, including an unaligned
    // signed ARRAY element read (sa65[1], base bit 65 in the engine store).
    let signed_sigs = pure
        .iter()
        .filter(|&&eid| {
            matches!(&ir.exprs[eid as usize], Expr::Signal { net, .. }
                if ir.nets[*net as usize].signed)
        })
        .count();
    assert!(signed_sigs >= 10, "signed Signal reads: {signed_sigs}");

    let rng_a = crate::state::RngCells::default();
    let rng_b = crate::state::RngCells::default();
    let mut rng = Rng::new(0x5163_ED00);
    let mut compared = 0usize;
    // Sign-bit shapes: (v, u) of the net's top bit — 0, 1, X, Z.
    for &(sv, su) in &[(0u64, 0u64), (1, 0), (0, 1), (1, 1)] {
        for _seed in 0..6 {
            for n in 0..ir.nets.len() as u32 {
                let s = arena.slots[n as usize];
                let words = s.words as usize;
                for e in 0..s.elems {
                    let mut vw: Vec<u64> = (0..words).map(|_| r64(&mut rng)).collect();
                    let mut uw: Vec<u64> =
                        (0..words).map(|_| r64(&mut rng) & r64(&mut rng)).collect();
                    // Force the sign bit of this element to the pass shape.
                    let top = s.width - 1;
                    let (tw, tb) = ((top / 64) as usize, top % 64);
                    vw[tw] = (vw[tw] & !(1 << tb)) | (sv << tb);
                    uw[tw] = (uw[tw] & !(1 << tb)) | (su << tb);
                    mirror_set(&mut st, &mut arena, n, e, &vw, &uw);
                }
            }
            for &eid in &pure {
                let sw = wt.get(eid);
                for dw in [0u32, 1, 31, 63, 64, 65] {
                    for cs in [false, true] {
                        let cw = sw.width + dw;
                        let engine = eval_with(&ir, &wt, &rng_a, &st, eid, cw, cs);
                        let native = eval_with(&ir, &wt, &rng_b, &arena, eid, cw, cs);
                        assert_eq!(engine, native, "sign ({sv},{su}) eid {eid} ctx ({cw},{cs})");
                        compared += 1;
                    }
                }
            }
        }
    }
    eprintln!("probe C: pure_exprs={} compared={compared}", pure.len());
    // 4 sign shapes × 6 seeds × 15 pure exprs × 6 widths × 2 signs = 4320.
    assert_eq!(compared, 4320, "probe C coverage moved");
}
