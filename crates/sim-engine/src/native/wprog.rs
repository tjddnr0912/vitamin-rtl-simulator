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
//! - `Lt` / `Le` / `Gt` / `Ge` / `Eq` / `Ne` / `CaseEq` / `CaseNe`, when both
//!   operands already share a width and a signedness (the comparison node's
//!   own context is one unsigned bit),
//! - `LogAnd` / `LogOr`, whose operands are SELF-determined and need not share
//!   a width with each other or with the result,
//! - `LogNot` and the six reductions, over a self-determined operand.
//! - `Concat` and `Replicate`, whose parts are self-determined and TILE the
//!   result exactly (S2 slice 5),
//! - `Select` over a self-determined base at a CONSTANT, provably in-range
//!   offset (S2 slice 6) — one shift and one mask,
//! - `Ternary` whose branches CANNOT REPORT (S2 slice 7) — the one admission in
//!   this module that is about evaluation order rather than width.
//!
//! Every node carries the SAME width and the SAME signedness as its context,
//! with SIX deliberate exceptions that introduce a FURTHER width rather than a
//! conversion — a comparison's operands, a runtime array index, the operands
//! of `&&`/`||` and of the one-bit unaries, a concat/replicate part, and a
//! select's base.
//! Four widths in ONE program is an
//! ordinary shape, not a corner: `(sa < sb) && m[idx]` gives 1 / 8 / 8 / 4.
//! The first four DISCARD their operand — every one of those nodes yields a
//! single bit and `LoadIdx` discards the index entirely — so the further width
//! dies where it is introduced. The fifth does not: a concat part's bits SURVIVE
//! into a wider result, and the sentence that used to stand here ("no value is
//! ever moved between widths") stopped being true when slice 5 admitted it.
//! What is still true, and is the whole correctness argument, is the narrower
//! claim: **no widening, no sign extension and no truncation exists
//! anywhere in an admitted tree.** A part contributes EXACTLY its own bits at a
//! compile-time offset, the offsets tile the result (`Σ pw == w`, checked), and
//! the accumulator they merge into is definite zero — so there is no room for a
//! fill and no bit is written twice. A select is the mirror image and the same
//! claim holds for the same reason: it takes a proven-in-range window of its
//! base's bits and discards the rest, so nothing is extended and nothing is
//! silently cut (a cut the SOURCE did not authorise is what `bits != w` and the
//! range test refuse). The context-sizing rules the generic
//! evaluator implements therefore still have nothing to do, and this module does
//! not restate them (the classifier-must-match-lowering trap). Signedness is admitted
//! because at uniform width two's complement makes every op above produce the
//! same BITS either way; that is measured by the battery, not assumed, and the
//! two places it is NOT true are handled explicitly (the signed `AShr`
//! declines, and a comparison hands its operand sign to the shared function).
//!
//! What remains is the per-op 4-state BIT SEMANTICS, and those are pinned by an
//! exhaustive per-bit-state differential against the generic evaluator plus the
//! corpus mirror sweep (`s2_wprog_*` tests) — measured equal, not argued equal.
//! Nothing here is restated: the comparisons call the shared
//! `eval::binops::{relational, log_eq, case_eq}`, `&&`/`||` call
//! `eval::binops::{log_and, log_or}`, and `!` plus the six reductions call
//! `eval::unary_self_of` — the same function the generic `eval_unary_self`
//! reaches. Those are free functions extracted for this; the pre-existing
//! `EvalCtx` methods delegate, so the generic path is unchanged.
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
    /// One-bit-result LOGICAL binary (`&&`, `||`). Unlike a comparison, the two
    /// operands are SELF-determined and need not share a width, so each carries
    /// its own — the value on the stack was masked to it when it was compiled.
    LogBin {
        op: sim_ir::BinOp,
        lw: u32,
        ls: bool,
        rw: u32,
        rs: bool,
    },
    /// One-bit-result unary over a SELF-determined operand: `!` or one of the six
    /// reductions. The mapping is not restated here — `unary_self_of` is the same
    /// function the generic evaluator reaches through `eval_unary_self`.
    Unary1 {
        op: sim_ir::UnOp,
        ow: u32,
        os: bool,
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
    /// S2 slice 7: `?:`. Pops the ELSE value, the THEN value and the COND
    /// (pushed in that order) and pushes the branch the condition selects, or
    /// the bitwise X-merge when the condition is unknown — the same three
    /// outcomes `eval_ctx`'s `Ternary` arm has, through the same two shared
    /// functions (`truthiness`, `merge_x_word`).
    ///
    /// ⚠️ Both branches have ALREADY been evaluated when this runs. That is the
    /// one place this module departs from the generic path's SHAPE, and the
    /// compiler pays for it with an admission check rather than an argument —
    /// see the compile arm.
    Tern {
        cw: u32,
        cs: bool,
        m: u64,
    },
    /// S2 slice 6: the top of stack's bits `[k +: w]`, i.e. one shift and one
    /// mask on both planes. `copy_bits(out, 0, src, k, w)` into a zeroed `w`-bit
    /// destination is exactly this, and the compiler only emits it after proving
    /// the range lies wholly inside the source — so the generic path's X-filling
    /// arms have no counterpart here because they are unreachable.
    Slice {
        k: u32,
        m: u64,
    },
    /// S2 slice 5: merge the popped value into the accumulator BENEATH it —
    /// `count` copies of its `stride`-spaced bits starting at bit `off`, masked
    /// to the result width. One op serves both a concatenation part
    /// (`count: 1`) and a replication (`count: n, stride: operand width`).
    ///
    /// The merge is a plane-wise OR rather than a 4-state one because the
    /// destination range is definite ZERO on entry — the accumulator is a
    /// `Const{0,0}` and the ranges tile without overlap. That is the identical
    /// contract `eval::eval_core::copy_bits` states for its own destination.
    Splice {
        off: u32,
        stride: u32,
        count: u32,
        m: u64,
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
            // `!` and the six reductions: SELF-determined operand, one unsigned
            // result bit. Same shape as the `&&`/`||` arm BELOW — the operand is
            // compiled at its own width so the shared mapping sees the bits it
            // expects, and the result discards it.
            if matches!(
                op,
                sim_ir::UnOp::LogNot
                    | sim_ir::UnOp::RedAnd
                    | sim_ir::UnOp::RedNand
                    | sim_ir::UnOp::RedOr
                    | sim_ir::UnOp::RedNor
                    | sim_ir::UnOp::RedXor
                    | sim_ir::UnOp::RedXnor
            ) {
                // Insurance, not a live check: `compile_node`'s own entry
                // precondition already requires the node's self width and sign to
                // equal the context's, and `sim_ir::selfwidth` gives every op in
                // this list `{width: 1, signed: false}` — so an unreachable
                // combination. Measured: replacing this `return` with a `panic!`
                // is never hit across the suite or the real designs. It stays
                // because the width the op RECORDS must be one bit whatever a
                // future width rule says, and that is cheaper to keep than to
                // re-derive.
                if w != 1 || signed {
                    return None; // the result IS one unsigned bit
                }
                let ow = wt.get(*operand);
                if ow.width == 0 || ow.width > 64 {
                    return None;
                }
                compile_node(
                    ir, wt, arena, *operand, ow.width, ow.signed, ops, depth, max_depth,
                )?;
                ops.push(WOp::Unary1 {
                    op: *op,
                    ow: ow.width,
                    os: ow.signed,
                });
                return Some(());
            }
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
                B::Lt | B::Le | B::Gt | B::Ge | B::Eq | B::Ne | B::CaseEq | B::CaseNe => {
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
                // `&&` / `||` — SELF-determined operands, each reduced to a
                // truth value independently, one unsigned result bit. Two facts
                // make this admissible without restating anything:
                //
                // (1) the generic evaluator does NOT short-circuit — it writes
                //     `let l = self.eval(lhs); let r = self.eval(rhs);` and only
                //     then folds. So evaluating both eagerly here is not a
                //     behavioural choice; it is the same order, which matters
                //     because an admitted subtree can still COUNT an
                //     out-of-range element read (`LoadIdx`).
                // (2) each operand is compiled at ITS OWN width and sign, so the
                //     stack value is already masked to that width and the
                //     truthiness scan sees exactly the bits the generic one sees.
                //
                // The operands may differ in width from each other and from this
                // node — that is why the op carries both, and why this does not
                // violate the module's uniform-width admission: like a comparison,
                // it introduces a further width rather than a conversion, and the
                // result discards the operands entirely.
                B::LogAnd | B::LogOr => {
                    // Insurance, not a live check — see the identical note on the
                    // unary arm above.
                    if w != 1 || signed {
                        return None; // the result IS one unsigned bit
                    }
                    let lw = wt.get(*lhs);
                    let rw = wt.get(*rhs);
                    if lw.width == 0 || lw.width > 64 || rw.width == 0 || rw.width > 64 {
                        return None;
                    }
                    compile_node(
                        ir, wt, arena, *lhs, lw.width, lw.signed, ops, depth, max_depth,
                    )?;
                    compile_node(
                        ir, wt, arena, *rhs, rw.width, rw.signed, ops, depth, max_depth,
                    )?;
                    *depth -= 1;
                    ops.push(WOp::LogBin {
                        op: *op,
                        lw: lw.width,
                        ls: lw.signed,
                        rw: rw.width,
                        rs: rw.signed,
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
        // ── CONCATENATION / REPLICATION (S2 slice 5) ───────────────────────
        //
        // The first nodes this module admits whose operands live at a DIFFERENT
        // width from the result and whose bits SURVIVE into it. The four earlier
        // exceptions (comparison operands, a runtime array index, `&&`/`||` and
        // the one-bit unaries) all discard their operand, so the module doc could
        // say no value is ever moved between widths. That sentence is no longer
        // true and has been corrected there; what is still true — and is the
        // whole correctness argument — is that no value is ever WIDENED,
        // SIGN-EXTENDED or TRUNCATED:
        //
        // - IEEE §11.8.1: a concatenation's operands are SELF-DETERMINED. Each
        //   part is compiled at its own `(width, signed)`, which is exactly what
        //   `eval_concat`'s `self.eval(p)` does.
        // - Every part contributes EXACTLY its own `pw` bits at a compile-time
        //   offset. A part's signedness never causes a fill, because there is no
        //   room to fill: the offsets are laid end to end and `Σ pw == w` is
        //   checked below, so the ranges tile the result exactly.
        // - The accumulator starts as definite ZERO and each splice writes a
        //   DISJOINT range, so merging is a plane-wise OR — the same contract
        //   `eval::eval_core::copy_bits` documents for its zeroed destination.
        //   That is why this is not a 4-state `Or` (which would have to decide
        //   `1|x`): no bit is ever written twice.
        //
        // ORDER, not just value: an admitted subtree can COUNT an out-of-range
        // element read (`LoadIdx`), and those reports are ORDERED (the arena's
        // `pending_range` is a Vec precisely because two counters could not carry
        // order). `eval_concat` evaluates `parts` in SOURCE order — parts[0], the
        // MSB-most, first — so the parts are emitted in source order here too and
        // each carries its own precomputed offset. Emitting LSB-first would have
        // produced identical bits and reversed two diagnostics.
        // ── CONDITIONAL (S2 slice 7) ───────────────────────────────────────
        //
        // ⚠️ THE ADMISSION IS ABOUT LAZINESS, NOT WIDTH. `eval_ctx`'s `Ternary`
        // arm evaluates the condition and then ONLY THE TAKEN BRANCH; it
        // evaluates both solely when the condition is unknown. This module has
        // no control flow, so it evaluates both always — which produces the same
        // VALUE (every admitted op is pure) but not necessarily the same
        // DIAGNOSTICS, because `LoadIdx` COUNTS an out-of-range element read.
        // An untaken branch carrying one would report an access the generic path
        // never performs: a diagnostic APPEARING is a divergence exactly as much
        // as one going missing.
        //
        // So the check is not syntactic and not conservative-by-shape — it asks
        // the compiled OPS whether either branch can report, which is the one
        // question that matters and is exact. `LoadIdx` is the only op in this
        // module that touches `note_bad_index`.
        //
        // The branches are CONTEXT-determined at `(w, signed)` — the same
        // (w, eff_signed) the generic passes down — so uniform admission applies
        // to them unchanged, and a branch narrower than the context declines
        // rather than being silently widened here.
        sim_ir::Expr::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            let cw = wt.get(*cond);
            if cw.width == 0 || cw.width > 64 {
                return None;
            }
            compile_node(
                ir, wt, arena, *cond, cw.width, cw.signed, ops, depth, max_depth,
            )?;
            let branches_at = ops.len();
            compile_node(ir, wt, arena, *then_e, w, signed, ops, depth, max_depth)?;
            compile_node(ir, wt, arena, *else_e, w, signed, ops, depth, max_depth)?;
            if ops[branches_at..]
                .iter()
                .any(|o| matches!(o, WOp::LoadIdx { .. }))
            {
                return None;
            }
            *depth -= 2; // three on the stack, one left
            ops.push(WOp::Tern {
                cw: cw.width,
                cs: cw.signed,
                m: mask,
            });
            Some(())
        }
        // ── BIT / PART SELECT (S2 slice 6) ─────────────────────────────────
        //
        // ADMISSION: a 2-state CONSTANT offset whose range lies WHOLLY inside
        // the base. That is not convenience, it is the correctness argument.
        // `eval_select` has THREE outcomes — all-X when the offset is unknown or
        // outside the i64 lane, one `copy_bits` when the range is fully inside
        // the source, and a per-bit loop that X-fills the overhang otherwise —
        // and proving the range at COMPILE time means only the middle one is
        // reachable. `copy_bits(out, 0, src, lsb, w)` into a zeroed `w`-bit
        // destination IS a shift and a mask, which is the whole `Slice` op.
        //
        // The base is SELF-determined (`eval_select` calls `self.eval(base)`),
        // and the select's own result is UNSIGNED — the entry gate above already
        // required the context to agree, so nothing here re-decides either.
        //
        // What this does NOT admit is an indexed select with a RUNTIME offset
        // (`x[i +: 4]`), which is most of what `+:`/`-:` exist for. That needs a
        // runtime bounds test and the X-fill arm; the constant forms are the
        // ones a `[15:0]` costs, and those are the 73 declines measured on
        // picorv32.
        sim_ir::Expr::Select {
            base,
            offset,
            width,
            kind,
        } => {
            // `width` is a const-expr EDGE (`Add(Sub(msb,lsb),1)`), folded by
            // the same helper `eval_select` and the self-width table both use.
            let sel_w = crate::width::const_u32_of_expr(ir, *width).unwrap_or(1);
            let os = wt.get(*offset);
            let off = match ir.exprs.get(*offset as usize)? {
                sim_ir::Expr::Const { val } => {
                    if os.width == 0 || os.width > 64 {
                        return None;
                    }
                    let (ov, ou) = const_planes(ir, *val, os.width)?;
                    // An x/z offset is the generic path's all-X arm.
                    if ou != 0 {
                        return None;
                    }
                    // The SAME two calls the generic makes, over the same value —
                    // `to_u64` deliberately ignores signedness, so a negative
                    // constant reads as a large positive one and then fails the
                    // in-range test below, exactly as it does there.
                    one_word_value(ov, ou, os.width, os.signed)
                        .to_u64()
                        .and_then(|o| i64::try_from(o).ok())?
                }
                _ => return None,
            };
            let (lsb, bits) = crate::eval::binops::select_lsb_width(*kind, off, sel_w);
            // The folded width and the width table must agree; they are two
            // readings of the same const-expr edge and this module must not pick
            // one when they differ.
            if bits != w {
                return None;
            }
            let bs = wt.get(*base);
            if bs.width == 0 || bs.width > 64 {
                return None;
            }
            // `eval_select`'s own fully-in-range condition, verbatim.
            if !(lsb >= 0 && (lsb as u64) + (w as u64) <= bs.width as u64) {
                return None;
            }
            compile_node(
                ir, wt, arena, *base, bs.width, bs.signed, ops, depth, max_depth,
            )?;
            // `lsb + w <= bs.width <= 64` and `w >= 1`, so the shift is ≤ 63.
            ops.push(WOp::Slice {
                k: lsb as u32,
                m: mask,
            });
            Some(())
        }
        sim_ir::Expr::Concat { parts } => {
            // `Σ pw == w` is the tiling proof AND the agreement check between
            // this module's two sources of width (the table, and `eval_concat`'s
            // sum over evaluated part widths). If they disagree — a clamp, a
            // saturating add, an empty concat's `.max(1)` — decline rather than
            // guess which one the generic path will use.
            let mut total: u64 = 0;
            for &p in parts.iter() {
                let pw = wt.get(p).width;
                // A ZERO-width part is legal (`{0{x}}`, IEEE §11.4.12.1) and is
                // declined rather than skipped: `eval_concat` still EVALUATES it,
                // so skipping could drop an E4002 the generic path reports.
                if pw == 0 {
                    return None;
                }
                total += u64::from(pw);
            }
            if total != u64::from(w) {
                return None;
            }
            push(ops, depth, max_depth, WOp::Const { val: 0, unk: 0 });
            let mut pos = w;
            for &p in parts.iter() {
                let ps = wt.get(p);
                pos -= ps.width; // parts[0] is MSB-most (`eval_concat`'s `pos -= v.width`)
                compile_node(ir, wt, arena, p, ps.width, ps.signed, ops, depth, max_depth)?;
                ops.push(WOp::Splice {
                    off: pos,
                    stride: 0,
                    count: 1,
                    m: mask,
                });
                *depth -= 1; // splice: two on the stack, one left
            }
            Some(())
        }
        sim_ir::Expr::Replicate { count, value } => {
            // `count` is a const-expr EDGE, not a literal — folded by the same
            // function `eval_replicate` and the self-width table both use.
            let n = crate::width::const_u32_of_expr(ir, *count)?;
            let vs = wt.get(*value);
            // `{0{x}}` has width 0 and would make this node's own width 0, which
            // `compile`'s entry already refuses at the root; declining here too
            // keeps a NESTED one from reaching `mask_of(0)`, and matches the
            // zero-width concat part above (the operand is still evaluated by
            // `eval_replicate`, so skipping is not equivalent).
            if n == 0 || vs.width == 0 {
                return None;
            }
            if u64::from(n) * u64::from(vs.width) != u64::from(w) {
                return None;
            }
            push(ops, depth, max_depth, WOp::Const { val: 0, unk: 0 });
            // ONE compile, `count` splices — `eval_replicate` evaluates the
            // operand ONCE and copies it, so compiling it `count` times would
            // multiply any `LoadIdx` report by `count`.
            compile_node(
                ir, wt, arena, *value, vs.width, vs.signed, ops, depth, max_depth,
            )?;
            ops.push(WOp::Splice {
                off: 0,
                stride: vs.width,
                count: n,
                m: mask,
            });
            *depth -= 1;
            Some(())
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
                WOp::LogBin { op, lw, ls, rw, rs } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    // Each operand at ITS OWN width — see the compile-side note.
                    let av = one_word_value(a.val, a.unk, lw, ls);
                    let bv = one_word_value(b.val, b.unk, rw, rs);
                    let r = if matches!(op, sim_ir::BinOp::LogAnd) {
                        crate::eval::binops::log_and(&av, &bv)
                    } else {
                        crate::eval::binops::log_or(&av, &bv)
                    };
                    a.val = r.val.first().copied().unwrap_or(0);
                    a.unk = r.unk.first().copied().unwrap_or(0);
                }
                WOp::Unary1 { op, ow, os } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    let av = one_word_value(a.val, a.unk, ow, os);
                    let r = crate::eval::unary_self_of(op, &av);
                    a.val = r.val.first().copied().unwrap_or(0);
                    a.unk = r.unk.first().copied().unwrap_or(0);
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
                    let r = match op {
                        sim_ir::BinOp::CaseEq | sim_ir::BinOp::CaseNe => {
                            crate::eval::binops::case_eq(op, &av, &bv)
                        }
                        sim_ir::BinOp::Eq | sim_ir::BinOp::Ne => {
                            crate::eval::binops::log_eq(op, &av, &bv)
                        }
                        _ => crate::eval::binops::relational(op, &av, &bv),
                    };
                    a.val = r.val.first().copied().unwrap_or(0);
                    a.unk = r.unk.first().copied().unwrap_or(0);
                }
                WOp::Tern { cw, cs, m } => {
                    let e = scratch.pop().expect("wprog stack");
                    let t = scratch.pop().expect("wprog stack");
                    let c = scratch.last_mut().expect("wprog stack");
                    // The condition is SELF-determined and asked with the same
                    // free function every other truth question in this module
                    // uses (`&&`, `||`, `!`, a branch condition).
                    let cv = one_word_value(c.val, c.unk, cw, cs);
                    *c = match crate::eval::truthiness(&cv) {
                        crate::eval::Tri::True => t,
                        crate::eval::Tri::False => e,
                        crate::eval::Tri::Unknown => {
                            let (v, u) =
                                crate::eval::binops::merge_x_word(t.val, t.unk, e.val, e.unk);
                            // `merge_x_word` X-poisons every DIFFERING bit,
                            // including above `w`; the generic path masks with
                            // `top_mask`, this one with the op's own mask.
                            W {
                                val: v & m,
                                unk: u & m,
                            }
                        }
                    };
                }
                WOp::Slice { k, m } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    a.val = (a.val >> k) & m;
                    a.unk = (a.unk >> k) & m;
                }
                WOp::Splice {
                    off,
                    stride,
                    count,
                    m,
                } => {
                    let p = scratch.pop().expect("wprog stack");
                    let acc = scratch.last_mut().expect("wprog stack");
                    // `off + i*stride < w <= 64` holds by construction (the
                    // compiler checked that the ranges tile a ≤64-bit result and
                    // refused a zero-width part), so no shift here can reach 64.
                    for i in 0..count {
                        let sh = off + i * stride;
                        acc.val |= (p.val << sh) & m;
                        acc.unk |= (p.unk << sh) & m;
                    }
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
