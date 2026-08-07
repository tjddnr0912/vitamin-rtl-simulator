//! S2 (R2) — width-specialized expression programs for the tier-3 walk.
//!
//! The profile that motivated this (keccak_f_flat, release, `/usr/bin/sample`):
//! ~60% of the native walk was `Value` manipulation (`mask_top` 16%, `resize`
//! 12%, `has_xz`/`from_packed`/`to_u64` ~12%) plus the generic recursive
//! evaluator (`eval_ctx` 16%, `eval_binary_ctx` 12%) and `read_net` boxing
//! (6.5%). None of that is the arithmetic itself — it is the 72-byte value
//! representation, exactly what doc-21 §2 said R2 must remove.
//!
//! ## What this module admits, and why admission is the correctness argument
//!
//! A `WProg` is compiled for an expression tree only when EVERY node is:
//!
//! - `Const` (Numeric) whose self width and sign equal the context's,
//! - `Signal` reading a whole ≤64-bit integral net, a CONSTANT in-bounds element
//!   of a ≤64-bit-element array (bounds and 2-state-ness checked at compile
//!   time), or — since S2 slice 4 — a RUNTIME element whose index expression is
//!   itself admitted,
//! - `BitNot` / `BitAnd` / `BitOr` / `BitXor` / `Add` / `Sub`,
//! - `Shl` / `Shr` / `AShr` by a 2-state constant amount — except a SIGNED
//!   `AShr`, whose sign fill is the one shift whose bits depend on the sign,
//! - `Lt` / `Le` / `Gt` / `Ge` / `Eq` / `Ne`, when both operands already share
//!   a width and a signedness (the comparison node's own context is one
//!   unsigned bit).
//!
//! Every node carries the SAME width and the SAME signedness as its context,
//! with TWO deliberate exceptions that introduce a FURTHER width rather than a
//! conversion — a comparison's operands, and a runtime array index (a comparison
//! inside an index subtree therefore gives three widths in one program) — and
//! neither mixes: the comparison yields one bit and `LoadIdx` discards the index
//! entirely. So **no widening, no sign extension and no truncation exists
//! anywhere in an admitted tree** — the context-sizing rules the generic
//! evaluator implements have nothing to do, and this module does not restate
//! them (the classifier-must-match-lowering trap). Signedness is admitted
//! because at uniform width two's complement makes every op above produce the
//! same BITS either way; that is measured by the battery, not assumed, and the
//! two places it is NOT true are handled explicitly (the signed `AShr`
//! declines, and a comparison hands its operand sign to the shared function).
//!
//! What remains is the per-op 4-state BIT SEMANTICS, and those are pinned by an
//! exhaustive per-bit-state differential against the generic evaluator plus the
//! corpus mirror sweep (`s2_wprog_*` tests) — measured equal, not argued equal.
//! The comparisons are not even restated: they call the shared
//! `eval::binops::relational` / `log_eq`.
//!
//! ## Which EVALUATIONS reach this module — narrower than "the walk"
//!
//! THREE call sites compile programs: `k_eval_for_lvalue` (the rhs of a blocking
//! assign, an NBA sample, a `force`, and a cont-assign settle), `k_truthy`
//! (branch conditions and the `wait(e)` predicate), and — since S2 slice 3 —
//! `ensure_index_kind`, which resolves an lvalue's own index expressions so
//! `fast_offsets` can answer without the generic evaluator. (That third one was
//! `index_of` until S2 slice 4 split deciding from running; both `index_of` and
//! `index_admits` reach it.) Everything else
//! still evaluates generically: a comparison inside a SYSTEM TASK argument
//! (`$display("%b", a < b)`) goes through `eval_task_arg`, and any lvalue the
//! fast path declines goes through `eval::resolve_offsets` whole.
//!
//! The count is stated because the previous version of this paragraph said
//! "two entry points … and the lvalue's own offsets go through
//! `resolve_offsets`", which slice 3 made false in the same commit that added
//! the third caller. A coverage claim that does not name its entry points is
//! not a coverage claim — and one that names the wrong number understates the
//! blast radius.
//!
//! ## Representation
//!
//! `W = (val, unk)` — one net bit per plane bit, `unk=1` ⇒ x (`val=0`) or z
//! (`val=1`), the arena's own plane encoding, so `Load` is two u64 reads at
//! compile-time-resolved buffer indices (the slot invariant keeps bits above
//! `width` zero, so loads need no masking). Every stack value stays masked to
//! ITS OWN width: loads are pre-masked, `Const` masks at compile time, and
//! each op that can move bits out of range (`Not`, `And`, `Or`, `Shl`, `Add`,
//! `Sub`) carries that operand's mask. The mask rides the OP rather than the
//! program because a program can hold two widths at once — a comparison's
//! operands are `ow` bits wide while its result is one.
//!
use sim_ir::SimIr;

use crate::native::arena::NetArena;
use crate::value::Value;
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
    /// S2 slice 4: push the ELEMENT the top of stack names. Pops the index
    /// (already computed at ITS own self-width by the ops just emitted) and
    /// pushes `[val, unk]` at `off + idx*2`, or all-X when the index is not a
    /// clean in-range element — which also COUNTS a deferred out-of-range
    /// report, because that is the arm `read_net` reports from.
    ///
    /// `words == 1` is an admission precondition, so the stride is 2.
    LoadIdx {
        off: u32,
        elems: u32,
        /// The ELEMENT width's mask — what an out-of-range read fills with x.
        m: u64,
    },
    // Every op that can move bits out of its operand's range carries THAT
    // operand's mask, not the program's: since S2 slice 2 a program can hold
    // two widths (a comparison's operands are `ow` bits wide, its result is 1).
    Not {
        m: u64,
    },
    And {
        m: u64,
    },
    Or {
        m: u64,
    },
    Xor,
    Shl {
        k: u32,
        m: u64,
    },
    Shr {
        k: u32,
    },
    Add {
        m: u64,
    },
    Sub {
        m: u64,
    },
    /// The ordered / equality comparisons. Pops two operand-width values,
    /// pushes a 1-bit result computed by the SHARED `eval::binops` free
    /// functions — the 4-state rules there (an ambiguous compare is x, but a
    /// definite mismatch decides `==` even with x elsewhere) are exactly the
    /// kind this module must never restate.
    Cmp {
        op: sim_ir::BinOp,
        ow: u32,
        osigned: bool,
    },
}

pub(crate) struct WProg {
    ops: Vec<WOp>,
    /// The RESULT's width and signedness — what the caller stamps on the
    /// `Value` it builds. Operand widths live in the ops that need them.
    width: u32,
    signed: bool,
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
    if w == 0 || w > 64 {
        return None;
    }
    let mut ops = Vec::new();
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    compile_node(
        ir,
        wt,
        arena,
        eid,
        w,
        signed,
        &mut ops,
        &mut depth,
        &mut max_depth,
    )?;
    Some(WProg {
        ops,
        width: w,
        signed,
        depth: max_depth,
    })
}

#[inline]
fn mask_of(w: u32) -> u64 {
    if w >= 64 {
        u64::MAX
    } else {
        (1u64 << w) - 1
    }
}

#[allow(clippy::too_many_arguments)]
fn compile_node(
    ir: &SimIr,
    wt: &WidthTable,
    arena: &NetArena,
    eid: u32,
    w: u32,
    signed: bool,
    ops: &mut Vec<WOp>,
    depth: &mut usize,
    max_depth: &mut usize,
) -> Option<()> {
    // UNIFORM WIDTH AND SIGN inside one node's subtree — the admission that
    // makes the sizing rules moot. Signedness was excluded outright until S2
    // slice 2; at uniform width it is inert for every op admitted below
    // (two's complement makes the BITS identical, and there is no widening to
    // sign-extend), which the exhaustive battery measures rather than assumes.
    // The two places sign is NOT inert are handled explicitly: an arithmetic
    // right shift declines when signed, and a comparison passes the operand
    // sign to the shared comparison functions.
    let sw = wt.get(eid);
    if sw.width != w || sw.signed != signed {
        return None;
    }
    let mask = mask_of(w);
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
            // KIND first, and locally rather than by inheritance. Today the
            // arena refuses `NetKind::Real` at build ("real: S2 width class"),
            // so a real net cannot reach here — but this arm's checks are all
            // about SHAPE (`width`/`words`/`elems`), and a real slot has the
            // shape of a 64-bit integer while its bits are an f64. Whoever
            // lifts that arena row would otherwise silently hand `-0.0` to the
            // integer path, where `truthiness` reads a set sign bit as TRUE —
            // the exact trap that function's own doc names. The guard is one
            // `matches!`; the precondition it replaces was three files away.
            let kind = ir.nets.get(*net as usize)?.kind;
            if !matches!(
                kind,
                sim_ir::NetKind::Wire
                    | sim_ir::NetKind::Reg
                    | sim_ir::NetKind::Logic
                    | sim_ir::NetKind::Integer
            ) {
                return None;
            }
            let slot = arena.slots.get(*net as usize)?;
            if slot.width != w || slot.words != 1 {
                return None;
            }
            // `word` is the INDEX EXPRESSION's id, not an element number. A
            // 2-state Numeric constant in bounds folds at compile time; anything
            // else is admitted as a RUNTIME index since S2 slice 4, which is why
            // the E4002 machinery had to come with it (`LoadIdx` reports).
            let e = match word {
                None => {
                    if slot.elems != 1 {
                        return None;
                    }
                    0u64
                }
                Some(weid) => {
                    match ir.exprs.get(*weid as usize)? {
                        sim_ir::Expr::Const { val } => {
                            let (iv, iu) = const_planes(ir, *val, 64)?;
                            if iu != 0 {
                                return None;
                            }
                            if iv >= u64::from(slot.elems) {
                                return None;
                            }
                            iv
                        }
                        // RUNTIME INDEX (S2 slice 4). Compiled INLINE at its OWN
                        // self-determined width and sign — which is what the
                        // generic path evaluates it at (`self.eval(weid)`), and
                        // which a program is already allowed to hold alongside the
                        // value's width (slice 2 put the mask on the op, not the
                        // program). If the index will not compile, the whole tree
                        // declines exactly as before.
                        //
                        // The reason the constant arm above still exists is not
                        // symmetry: a constant in-bounds index proves at COMPILE
                        // time that no E4002 is reachable, so it emits a plain
                        // `Load` with no bounds test in the loop.
                        _ => {
                            // The width guard is NOT decoration: this is the one
                            // place a NEW context width enters the recursion, so it
                            // restates `compile`'s own entry precondition. Without
                            // it `mask_of(0)` is 0 and a >64-bit index would be
                            // truncated into the one-word `W` silently.
                            //
                            // ⚠️ An earlier version of this note claimed the seal
                            // normalises every array-word index to exactly 32 bits,
                            // so "no index of width != 32 arrives here". The round-2
                            // soundness review MEASURED that false — a 2-bit
                            // `Concat` arrives from the packed-element domain, and
                            // `packed.rs` has three more carve-outs that return an
                            // index unsealed. Those all decline at the `Concat`, so
                            // there is no consequence today; what is retracted is
                            // the reason, not the guard. Deleting this guard or
                            // `slot.words != 1` survives the suite, which is a
                            // statement about which widths reach here, not about
                            // whether they may.
                            let iw = wt.get(*weid);
                            if iw.width == 0 || iw.width > 64 {
                                return None;
                            }
                            compile_node(
                                ir, wt, arena, *weid, iw.width, iw.signed, ops, depth, max_depth,
                            )?;
                            // net 0: pops the index, pushes the element.
                            ops.push(WOp::LoadIdx {
                                off: slot.off,
                                elems: slot.elems,
                                m: mask,
                            });
                            return Some(());
                        }
                    }
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
            compile_node(ir, wt, arena, *operand, w, signed, ops, depth, max_depth)?;
            ops.push(WOp::Not { m: mask });
            Some(())
        }
        sim_ir::Expr::Binary { op, lhs, rhs } => {
            use sim_ir::BinOp as B;
            match op {
                B::BitAnd | B::BitOr | B::BitXor | B::Add | B::Sub => {
                    compile_node(ir, wt, arena, *lhs, w, signed, ops, depth, max_depth)?;
                    compile_node(ir, wt, arena, *rhs, w, signed, ops, depth, max_depth)?;
                    *depth -= 1; // binary: two pops, one push
                    ops.push(match op {
                        B::BitAnd => WOp::And { m: mask },
                        B::BitOr => WOp::Or { m: mask },
                        B::BitXor => WOp::Xor,
                        B::Add => WOp::Add { m: mask },
                        _ => WOp::Sub { m: mask },
                    });
                    Some(())
                }
                // ORDERED / EQUALITY comparisons (S2 slice 2). The result is
                // ONE bit while the operands are `ow` bits — one of the two nodes
                // whose subtree width differs from its own (the other is a runtime
                // array index, S2 slice 4) — admitted only
                // when both operands already share a width AND a signedness, so
                // no §11.8.1 mixed-sign or widening question arises here
                // either. The comparison itself is the shared free function.
                B::Lt | B::Le | B::Gt | B::Ge | B::Eq | B::Ne => {
                    if w != 1 || signed {
                        return None; // a comparison's own self-width IS 1, unsigned
                    }
                    let lw = wt.get(*lhs);
                    let rw = wt.get(*rhs);
                    if lw.width != rw.width || lw.signed != rw.signed {
                        return None;
                    }
                    if lw.width == 0 || lw.width > 64 {
                        return None;
                    }
                    compile_node(
                        ir, wt, arena, *lhs, lw.width, lw.signed, ops, depth, max_depth,
                    )?;
                    compile_node(
                        ir, wt, arena, *rhs, lw.width, lw.signed, ops, depth, max_depth,
                    )?;
                    *depth -= 1;
                    ops.push(WOp::Cmp {
                        op: *op,
                        ow: lw.width,
                        osigned: lw.signed,
                    });
                    Some(())
                }
                B::Shl | B::Shr | B::AShr => {
                    // An ARITHMETIC right shift fills with the left operand's
                    // own sign bit; at uniform admission that sign is this
                    // node's, so a signed `>>>` is the one shift whose bits
                    // differ from the logical one. Decline it rather than
                    // restate the fill (`Shr` is logical for both signs, and
                    // `Shl` moves bits the same way either way).
                    if signed && matches!(op, B::AShr) {
                        return None;
                    }
                    // Amount: self-determined 2-state CONSTANT only.
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
                    compile_node(ir, wt, arena, *lhs, w, signed, ops, depth, max_depth)?;
                    if k >= u64::from(w) {
                        // Every bit leaves. The compiled lhs is pure (SysFunc
                        // never admits), so its value is simply annihilated:
                        // AND with definite-0 is definite-0 for every 4-state
                        // input, x/z included — the same all-0 the generic
                        // path produces by actually shifting.
                        push(ops, depth, max_depth, WOp::Const { val: 0, unk: 0 });
                        *depth -= 1;
                        ops.push(WOp::And { m: mask });
                        return Some(());
                    }
                    ops.push(match op {
                        B::Shl => WOp::Shl {
                            k: k as u32,
                            m: mask,
                        },
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
    pub(crate) fn run(&self, arena: &NetArena, scratch: &mut Vec<W>) -> W {
        let buf = &arena.buf;
        scratch.clear();
        scratch.reserve(self.depth);
        for op in &self.ops {
            match *op {
                WOp::Const { val, unk } => scratch.push(W { val, unk }),
                WOp::Load { vi } => scratch.push(W {
                    val: buf[vi as usize],
                    unk: buf[vi as usize + 1],
                }),
                WOp::LoadIdx { off, elems, m } => {
                    let i = scratch.last_mut().expect("wprog stack");
                    // The index rule is the SHARED one — `word_index_of` is what
                    // `eval_core`'s `Expr::Signal` arm calls, and it owns the
                    // "x/z or beyond-u32 is not a wrap" decision. Reaching it from
                    // planes rather than a `Value` is the whole point of this op.
                    let clean = if i.unk != 0 { None } else { Some(i.val) };
                    let idx = crate::eval::word_index_of(clean);
                    if idx >= elems {
                        arena.note_bad_index(idx == crate::eval::WORD_UNKNOWN);
                        i.val = 0;
                        i.unk = m;
                    } else {
                        let vi = (off + idx * 2) as usize;
                        i.val = buf[vi];
                        i.unk = buf[vi + 1];
                    }
                }
                WOp::Not { m } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    // 0→1, 1→0, x/z→x: val flips only where definite.
                    a.val = !a.val & m & !a.unk;
                }
                WOp::And { m } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    // definite-1 = val&!unk (excludes z); definite-0 = !val&!unk.
                    let d1 = (a.val & !a.unk) & (b.val & !b.unk);
                    let d0 = (!a.val & !a.unk) | (!b.val & !b.unk);
                    a.val = d1;
                    a.unk = m & !(d1 | d0);
                }
                WOp::Or { m } => {
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
                WOp::Shl { k, m } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    a.val = (a.val << k) & m;
                    a.unk = (a.unk << k) & m;
                }
                WOp::Shr { k } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    a.val >>= k;
                    a.unk >>= k;
                }
                WOp::Add { m } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    if (a.unk | b.unk) != 0 {
                        a.val = 0;
                        a.unk = m; // any x/z operand ⇒ all-X (IEEE arithmetic)
                    } else {
                        a.val = a.val.wrapping_add(b.val) & m;
                    }
                }
                WOp::Sub { m } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    if (a.unk | b.unk) != 0 {
                        a.val = 0;
                        a.unk = m;
                    } else {
                        a.val = a.val.wrapping_sub(b.val) & m;
                    }
                }
                WOp::Cmp { op, ow, osigned } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    // Hand both operands to the SHARED comparison functions as
                    // ordinary values (≤64 bits ⇒ `Words::Inline`, no
                    // allocation). What is specialized is reaching this point,
                    // not what happens at it.
                    let av = one_word_value(a.val, a.unk, ow, osigned);
                    let bv = one_word_value(b.val, b.unk, ow, osigned);
                    let r = if matches!(op, sim_ir::BinOp::Eq | sim_ir::BinOp::Ne) {
                        crate::eval::binops::log_eq(op, &av, &bv)
                    } else {
                        crate::eval::binops::relational(op, &av, &bv)
                    };
                    a.val = r.val.first().copied().unwrap_or(0);
                    a.unk = r.unk.first().copied().unwrap_or(0);
                }
            }
        }
        scratch.pop().expect("wprog result")
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn signed(&self) -> bool {
        self.signed
    }

    /// Instruction count — read by the battery to assert that COMPOUND
    /// comparison operands are actually present (a suite in which every
    /// comparison operand is one `Load` cannot see the per-op masks).
    #[cfg(test)]
    pub(crate) fn op_count(&self) -> usize {
        self.ops.len()
    }
}

/// A ≤64-bit 4-state `Value` from the two plane words — the bridge to code
/// that speaks `Value` (the comparison functions today).
fn one_word_value(val: u64, unk: u64, w: u32, signed: bool) -> Value {
    let mut v = Value::zeros(w, signed);
    v.val[0] = val;
    v.unk[0] = unk;
    v
}
