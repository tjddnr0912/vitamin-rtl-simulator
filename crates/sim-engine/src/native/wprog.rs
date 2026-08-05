//! S2 (R2) slice 1 — width-specialized expression programs for the tier-3 walk.
//!
//! The profile that motivated this (keccak_f_flat, release, `/usr/bin/sample`):
//! ~60% of the native walk was `Value` manipulation (`mask_top` 16%, `resize`
//! 12%, `has_xz`/`from_packed`/`to_u64` ~12%) plus the generic recursive
//! evaluator (`eval_ctx` 16%, `eval_binary_ctx` 12%) and `read_net` boxing
//! (6.5%). None of that is the arithmetic itself — it is the 72-byte value
//! representation, exactly what doc-21 §2 said R2 must remove.
//!
//! ## What this slice admits, and why admission is the correctness argument
//!
//! A `WProg` is compiled for an expression tree only when EVERY node is:
//!
//! - `Const` whose self width equals the context width (4-state literals fine),
//! - `Signal` reading a whole ≤64-bit net or a CONSTANT element of a ≤64-bit-
//!   element array (bounds checked at compile time — no E4002 is reachable),
//! - `Unary` Not / `Binary` And·Or·Xor·Add·Sub whose operands' SELF widths all
//!   equal the context width, everything unsigned,
//! - `Shl`/`Shr`/`AShr` whose amount is a 2-state constant (self-determined, so
//!   its width is irrelevant; unsigned makes `AShr` ≡ `Shr`).
//!
//! Uniform width + unsigned means **no widening, no sign extension and no
//! truncation exists anywhere in the tree** — the context-sizing rules the
//! generic evaluator implements have nothing to do, so this module does not
//! restate them (the classifier-must-match-lowering trap). What remains is the
//! per-op 4-state BIT SEMANTICS, and those are pinned by an exhaustive
//! per-bit-state differential against the generic evaluator plus the corpus
//! mirror sweep (`wprog` tests) — measured equal, not argued equal.
//!
//! Anything outside the set falls back to the generic path, byte-identical by
//! construction (it IS the previous path).
//!
//! ## Representation
//!
//! `W = (val, unk)` — one net bit per plane bit, `unk=1` ⇒ x (`val=0`) or z
//! (`val=1`), the arena's own plane encoding, so `Load` is two u64 reads at
//! compile-time-resolved buffer indices (the slot invariant keeps bits above
//! `width` zero, so loads need no masking). The stack machine keeps every
//! value masked to the program width as an invariant: loads are pre-masked,
//! `Const` masks at compile time, and every op that can move bits out of range
//! (`Not`, shifts, `Add`, `Sub`) masks its result.

use sim_ir::SimIr;

use crate::native::arena::NetArena;
use crate::width::WidthTable;

/// One ≤64-bit 4-state value: the arena's two planes, one word each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct W {
    pub val: u64,
    pub unk: u64,
}

#[derive(Clone, Copy, Debug)]
enum WOp {
    /// Push a constant (pre-masked at compile time).
    Const {
        val: u64,
        unk: u64,
    },
    /// Push the net value at buffer indices `(vi, vi+1)` — val plane word,
    /// then unk plane word, resolved from the slot descriptor at compile time.
    Load {
        vi: u32,
    },
    Not,
    And,
    Or,
    Xor,
    /// Shift by a compile-time amount, already clamped: `k < width` (a shift
    /// by `>= width` is compiled to `Const{0,0}` instead — all bits leave).
    Shl {
        k: u32,
    },
    Shr {
        k: u32,
    },
    Add,
    Sub,
}

pub(crate) struct WProg {
    ops: Vec<WOp>,
    /// Mask of the (single) program width — every stack value honours it.
    mask: u64,
    width: u32,
    /// Maximum stack depth, so the executor can reserve once.
    depth: usize,
}

/// Compile `eid` for context `(w, signed)` — `None` when any node falls
/// outside the admitted set. Pure function of the IR: programs never need
/// invalidation.
pub(crate) fn compile(
    ir: &SimIr,
    wt: &WidthTable,
    arena: &NetArena,
    eid: u32,
    w: u32,
    signed: bool,
) -> Option<WProg> {
    if signed || w == 0 || w > 64 {
        return None;
    }
    let mask = if w == 64 { u64::MAX } else { (1u64 << w) - 1 };
    let mut ops = Vec::new();
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    compile_node(
        ir,
        wt,
        arena,
        eid,
        w,
        mask,
        &mut ops,
        &mut depth,
        &mut max_depth,
    )?;
    Some(WProg {
        ops,
        mask,
        width: w,
        depth: max_depth,
    })
}

#[allow(clippy::too_many_arguments)]
fn compile_node(
    ir: &SimIr,
    wt: &WidthTable,
    arena: &NetArena,
    eid: u32,
    w: u32,
    mask: u64,
    ops: &mut Vec<WOp>,
    depth: &mut usize,
    max_depth: &mut usize,
) -> Option<()> {
    // UNIFORM WIDTH, UNSIGNED — the admission that makes sizing rules moot.
    let sw = wt.get(eid);
    if sw.width != w || sw.signed {
        return None;
    }
    let push = |ops: &mut Vec<WOp>, depth: &mut usize, max_depth: &mut usize, op: WOp| {
        ops.push(op);
        *depth += 1;
        *max_depth = (*max_depth).max(*depth);
    };
    match ir.exprs.get(eid as usize)? {
        sim_ir::Expr::Const { val } => {
            let (cv, cu) = const_planes(ir, *val, w)?;
            push(
                ops,
                depth,
                max_depth,
                WOp::Const {
                    val: cv & mask,
                    unk: cu & mask,
                },
            );
            Some(())
        }
        sim_ir::Expr::Signal { net, word } => {
            let slot = arena.slots.get(*net as usize)?;
            if slot.width != w || slot.words != 1 {
                return None;
            }
            // `word` is the INDEX EXPRESSION's id, not an element number —
            // admitted only when it is a 2-state Numeric constant in bounds
            // (an out-of-bounds or dynamic index stays on the generic path,
            // which is where the E4002 machinery lives).
            let e = match word {
                None => {
                    if slot.elems != 1 {
                        return None;
                    }
                    0u64
                }
                Some(weid) => {
                    let idx = match ir.exprs.get(*weid as usize)? {
                        sim_ir::Expr::Const { val } => {
                            let (iv, iu) = const_planes(ir, *val, 64)?;
                            if iu != 0 {
                                return None;
                            }
                            iv
                        }
                        _ => return None,
                    };
                    if idx >= u64::from(slot.elems) {
                        return None;
                    }
                    idx
                }
            };
            let vi = slot.off + (e as u32) * 2; // words == 1: [val, unk] adjacent
            push(ops, depth, max_depth, WOp::Load { vi });
            Some(())
        }
        sim_ir::Expr::Unary { op, operand } => {
            if !matches!(op, sim_ir::UnOp::BitNot) {
                return None;
            }
            compile_node(ir, wt, arena, *operand, w, mask, ops, depth, max_depth)?;
            ops.push(WOp::Not);
            Some(())
        }
        sim_ir::Expr::Binary { op, lhs, rhs } => {
            use sim_ir::BinOp as B;
            match op {
                B::BitAnd | B::BitOr | B::BitXor | B::Add | B::Sub => {
                    compile_node(ir, wt, arena, *lhs, w, mask, ops, depth, max_depth)?;
                    compile_node(ir, wt, arena, *rhs, w, mask, ops, depth, max_depth)?;
                    *depth -= 1; // binary: two pops, one push
                    ops.push(match op {
                        B::BitAnd => WOp::And,
                        B::BitOr => WOp::Or,
                        B::BitXor => WOp::Xor,
                        B::Add => WOp::Add,
                        _ => WOp::Sub,
                    });
                    Some(())
                }
                B::Shl | B::Shr | B::AShr => {
                    // Amount: self-determined 2-state CONSTANT only. Unsigned
                    // is already required, so `AShr` is `Shr`.
                    let k = match ir.exprs.get(*rhs as usize)? {
                        sim_ir::Expr::Const { val } => {
                            let aw = wt.get(*rhs).width.min(64);
                            let (av, au) = const_planes(ir, *val, aw)?;
                            if au != 0 {
                                return None;
                            }
                            av
                        }
                        _ => return None,
                    };
                    // The LHS is compiled FIRST in every case — including
                    // `k >= w`. The first spelling short-circuited that case to
                    // `Const{0,0}` without visiting the lhs, which admitted
                    // trees whose lhs would have DECLINED: a dynamic OOB index
                    // lost its E4002 (loud → silent, exit class flipped) and a
                    // `$urandom` operand lost its draw (the whole subsequent
                    // RNG stream shifted — silent-wrong at exit 0). Both were
                    // measured by the soundness review; the admission IS the
                    // correctness argument, so no arm may skip it.
                    compile_node(ir, wt, arena, *lhs, w, mask, ops, depth, max_depth)?;
                    if k >= u64::from(w) {
                        // Every bit leaves. The compiled lhs is pure (SysFunc
                        // never admits), so its value is simply annihilated:
                        // AND with definite-0 is definite-0 for every 4-state
                        // input, x/z included — the same all-0 the generic
                        // path produces by actually shifting.
                        push(ops, depth, max_depth, WOp::Const { val: 0, unk: 0 });
                        *depth -= 1;
                        ops.push(WOp::And);
                        return Some(());
                    }
                    ops.push(match op {
                        B::Shl => WOp::Shl { k: k as u32 },
                        _ => WOp::Shr { k: k as u32 },
                    });
                    Some(())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// A `Const` expression's two planes at width `w` (≤64) — `None` when the
/// literal is wider than one word.
fn const_planes(ir: &SimIr, val_idx: u32, w: u32) -> Option<(u64, u64)> {
    if w > 64 {
        return None;
    }
    let cv = ir.consts.get(val_idx as usize)?;
    // Numeric only: a string or real literal has a different value domain and
    // its own sizing rules — generic path.
    if !matches!(cv.repr, sim_ir::ConstRepr::Numeric) || cv.width > 64 {
        return None;
    }
    let v = *cv.bits.val.first().unwrap_or(&0);
    let u = *cv.bits.unk.first().unwrap_or(&0);
    Some((v, u))
}

impl WProg {
    /// Execute over the arena buffer. `scratch` is the caller's reusable
    /// stack; cleared here, capacity grown once to the compiled depth.
    pub(crate) fn run(&self, buf: &[u64], scratch: &mut Vec<W>) -> W {
        scratch.clear();
        scratch.reserve(self.depth);
        let m = self.mask;
        for op in &self.ops {
            match *op {
                WOp::Const { val, unk } => scratch.push(W { val, unk }),
                WOp::Load { vi } => scratch.push(W {
                    val: buf[vi as usize],
                    unk: buf[vi as usize + 1],
                }),
                WOp::Not => {
                    let a = scratch.last_mut().expect("wprog stack");
                    // 0→1, 1→0, x/z→x: val flips only where definite.
                    a.val = !a.val & m & !a.unk;
                }
                WOp::And => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    // definite-1 = val&!unk (excludes z); definite-0 = !val&!unk.
                    let d1 = (a.val & !a.unk) & (b.val & !b.unk);
                    let d0 = (!a.val & !a.unk) | (!b.val & !b.unk);
                    a.val = d1;
                    a.unk = m & !(d1 | d0);
                }
                WOp::Or => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    let d1 = (a.val & !a.unk) | (b.val & !b.unk);
                    let d0 = (!a.val & !a.unk) & (!b.val & !b.unk);
                    a.val = d1;
                    a.unk = m & !(d1 | d0);
                }
                WOp::Xor => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    let unk = a.unk | b.unk;
                    a.val = (a.val ^ b.val) & !unk;
                    a.unk = unk;
                }
                WOp::Shl { k } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    a.val = (a.val << k) & m;
                    a.unk = (a.unk << k) & m;
                }
                WOp::Shr { k } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    a.val >>= k;
                    a.unk >>= k;
                }
                WOp::Add => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    if (a.unk | b.unk) != 0 {
                        a.val = 0;
                        a.unk = m; // any x/z operand ⇒ all-X (IEEE arithmetic)
                    } else {
                        a.val = a.val.wrapping_add(b.val) & m;
                    }
                }
                WOp::Sub => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    if (a.unk | b.unk) != 0 {
                        a.val = 0;
                        a.unk = m;
                    } else {
                        a.val = a.val.wrapping_sub(b.val) & m;
                    }
                }
            }
        }
        scratch.pop().expect("wprog result")
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }
}
