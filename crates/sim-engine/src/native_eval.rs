//! [C4-lite] Native expression evaluator — a VM-only fast path that compiles a
//! `sim_ir::Expr` tree into a flat post-order register program and evaluates it
//! WITHOUT building a `Value` per node, removing the recursive `eval_ctx` dispatch
//! and per-operator `Value` construction that dominate expression-bound designs
//! (doc-18 §실측: eval is 55–82% of runtime once expressions are wide).
//!
//! ## Scope (intentionally bounded; everything else falls back to the kernel)
//! [`try_compile`] returns `Some` ONLY when the WHOLE tree is in this subset, and
//! EVERY node's context-determined eval width is at most 64 bits (one plane word):
//! leaves `Const` (non-real) and scalar `Signal` (`word == None`); binary
//! `Add`/`Sub`/`Mul`/`Div`/`Mod`, the four bitwise ops, all eight comparisons
//! (`<`/`<=`/`>`/`>=`/`==`/`!=`/`===`/`!==`), shifts (`<<`/`>>`/`>>>`), logical
//! `&&`/`||`; unary `BitNot`/`Plus`/`Minus`, the six reductions, `!`; the
//! ternary `?:` (X-cond branch merge included); and the structural trio —
//! bit/part `Select` (dynamic offset, X/Z-offset and out-of-range → X),
//! `Concat`, `Replicate` (const count) — all unsigned zero-extended into their
//! context exactly as the oracle's `resize_keep_sign(w, false)`. Any other
//! variant (`**`, sysfunc, call), a real const, an array-indexed signal, or a
//! node wider than 64 bits makes the whole expression return `None`, so the VM
//! delegates to the kernel's tree-walk `eval_ctx` (the differential ORACLE). The
//! over-64-bit lane and real stay deferred follow-ons.
//!
//! ## Why it is byte-identical (the P5 gate proves it end-to-end on top)
//! The interpreter's `eval_ctx` stays the oracle; native eval reproduces it EXACTLY
//! for the supported nodes (width at most 64). Width/sign context propagates DOWN
//! exactly as `eval_ctx` does (`w = self.max(ctx)`, `eff_signed = self_signed &&
//! ctx_signed`) — the SAME recursion, so per-node widths match. Leaves reuse the
//! EXACT oracle path: `read_net(net,None)` / `eval_const`-equiv, then
//! `resize_keep_sign(w, eff_signed)` — net read + context resize verbatim. Arith
//! (oracle `arith()`): if EITHER operand has any X/Z the whole result is X; else a
//! 128-bit lane masked to `w`, and for `w` at most 64 the low-`w` bits of a `u64`
//! wrapping op equal that lane (two's-complement, hence sign-independent), so
//! `(a OP b) & low_mask(w)` matches both signed and unsigned. Bitwise/not reuse the
//! SAME `value::{and_w,or_w,xor_w,xnor_w,not_w}` 4-state word primitives the oracle
//! calls, masked to `w`. A register is the single-word 4-state pair `(val, unk)`,
//! masked to its node width on production, so the program rebuilds exactly one
//! `Value` at the end.

use sim_ir::{BinOp, ConstRepr, Expr, SelKind, SimIr, UnOp};

use crate::eval::NetReader;
use crate::value::{and_w, low_mask, not_w, or_w, xnor_w, xor_w, Value};
use crate::width::WidthTable;

#[derive(Clone, Copy)]
enum ArithKind {
    Add,
    Sub,
    Mul,
}

#[derive(Clone, Copy)]
enum BitKind {
    And,
    Or,
    Xor,
    Xnor,
}

#[derive(Clone, Copy, PartialEq)]
enum CmpKind {
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy)]
enum DivKind {
    Div,
    Mod,
}

#[derive(Clone, Copy)]
enum RedK {
    And,
    Or,
    Xor,
}

/// One native eval instruction (post-order; operates on a value stack of single-word
/// 4-state pairs `(val, unk)`). `Plus` needs no opcode — its operand is simply
/// compiled at this node's `(w, eff_signed)` and left on the stack (passthrough).
#[derive(Clone, Copy)]
enum NOp {
    /// push a compile-time constant, already resized to its node width and masked.
    Const {
        val: u64,
        unk: u64,
    },
    /// push `read_net(net, None).resize_keep_sign(w, signed)`, masked to `w`.
    LoadScalar {
        net: u32,
        w: u32,
        signed: bool,
    },
    /// pop b, pop a → push the `w`-wide arith result (all-X if either has any X/Z).
    Arith {
        kind: ArithKind,
        w: u32,
    },
    /// pop b, pop a → push the `w`-wide 4-state bitwise result.
    Bitwise {
        kind: BitKind,
        w: u32,
    },
    /// pop a → push the `w`-wide 4-state complement.
    Not {
        w: u32,
    },
    /// pop a → push the `w`-wide two's-complement negate (all-X if any X/Z).
    Neg {
        w: u32,
    },
    /// pop b, pop a → push the 1-bit relational result at pair width `w`
    /// (signed iff BOTH operands signed; any X/Z → 1-bit X). Mirrors oracle
    /// `relational` (single-word case).
    Cmp {
        kind: CmpKind,
        w: u32,
        signed: bool,
    },
    /// pop b, pop a → push 1-bit `==`/`!=` (any X/Z in the compared bits → X).
    EqNe {
        ne: bool,
        w: u32,
    },
    /// pop b, pop a → push 1-bit `===`/`!==` (exact 4-state plane compare; never X).
    CaseEqNe {
        ne: bool,
        w: u32,
    },
    /// pop amount (self-determined), pop l (`w`-wide) → push `l << amt` at `w`
    /// (X/Z amount → all-X; amt ≥ w shifts everything out). Oracle `shift_left`
    /// grow-then-truncate ≡ direct masked shift for w ≤ 64.
    Shl {
        w: u32,
    },
    /// pop amount, pop l → push `l >> amt` at `w`. `arith` (the lhs OWN sign for
    /// `>>>`) fills vacated top bits with l's MSB pair (which may be X/Z).
    Shr {
        w: u32,
        arith: bool,
    },
    /// pop b, pop a → push `w`-wide div/mod (X/Z or divide-by-zero → all-X;
    /// signed = truncating toward zero, exactly oracle `arith`'s i128 lane).
    DivMod {
        kind: DivKind,
        w: u32,
        signed: bool,
    },
    /// pop else, pop then, pop cond → push the selected branch at `w`; an X/Z
    /// cond merges the branches bit-wise (agree → through, differ → X).
    Ternary {
        w: u32,
        cond_w: u32,
    },
    /// pop a (self-determined, `opw` wide) → push the 1-bit reduction
    /// (negated for the N-forms; X stays X under negation).
    Reduce {
        kind: RedK,
        neg: bool,
        opw: u32,
    },
    /// pop a → push 1-bit `!a` via tri-valued truthiness.
    LogNot {
        opw: u32,
    },
    /// pop r, pop l (each self-determined) → push 1-bit `&&`/`||` (tri-valued).
    LogBin {
        and: bool,
        lw: u32,
        rw: u32,
    },
    /// pop offset (self-determined), pop base (`src_w` self bits) → push the
    /// `sel_w` gathered bits zero-extended to node width (oracle `eval_select`
    /// then unsigned `resize_keep_sign`): X/Z offset ⇒ `sel_w` X bits; an
    /// out-of-range source bit reads X.
    Select {
        kind: SelKind,
        sel_w: u32,
        src_w: u32,
    },
    /// pop lo (`lo_w` self bits), pop hi → push `(hi << lo_w) | lo` masked to
    /// the running natural concat width `w` (4-state planes shift+or alike).
    ConcatPair {
        lo_w: u32,
        w: u32,
    },
    /// pop part (`part_w` self bits) → push it repeated `count` times (`w` =
    /// part_w × count natural bits; X/Z bits repeat with the pattern).
    Repl {
        part_w: u32,
        count: u32,
        w: u32,
    },
    /// pop index (self-determined, narrow), then push
    /// `read_net(net, Some(idx)).resize_keep_sign(w, signed)` — the oracle's
    /// array-word read: an X/Z index or one beyond u32 maps to the `u32::MAX`
    /// out-of-range sentinel (`net_word_packed` then reads all-X).
    LoadIndexed {
        net: u32,
        w: u32,
        signed: bool,
    },

    // ── C6 wide lane: 65..=128-bit values on a SEPARATE u128 pair stack.
    //    Same oracle contracts, two-word registers. `lower` guarantees a node's
    //    register is wide IFF its eval width w > 64 (`Promote` bridges narrow
    //    producers feeding wide contexts; the reverse never occurs). ──
    /// pop a narrow register → push it on the wide stack (zero-extend; narrow
    /// registers keep bits ≥ their node width 0, so the pass-through is exact).
    Promote,
    /// push a compile-time wide constant (resized + masked at compile).
    WConst {
        val: u128,
        unk: u128,
    },
    /// wide `read_net(net, None).resize_keep_sign(w, signed)`.
    WLoadScalar {
        net: u32,
        w: u32,
        signed: bool,
    },
    /// pop narrow index → wide array-word read (same sentinel rules).
    WLoadIndexed {
        net: u32,
        w: u32,
        signed: bool,
    },
    /// pop b, pop a (wide) → `w`-wide UNSIGNED arith (oracle's u128 lane;
    /// signed >64-bit arith X-poisons in the oracle and stays oracle-bound).
    WArith {
        kind: ArithKind,
        w: u32,
    },
    /// pop b, pop a (wide) → 4-state bitwise at `w` (same `*_w` formulas on u128).
    WBitwise {
        kind: BitKind,
        w: u32,
    },
    WNot {
        w: u32,
    },
    /// any X/Z → all-X; else `(!a + 1) & mask` ≡ the oracle's word-carry chain.
    WNeg {
        w: u32,
    },
    /// pop b, pop a (wide) → 1-bit relational on the NARROW stack.
    WCmp {
        kind: CmpKind,
        w: u32,
        signed: bool,
    },
    WEqNe {
        ne: bool,
        w: u32,
    },
    WCaseEqNe {
        ne: bool,
        w: u32,
    },
    /// pop narrow amount, pop wide l → wide shift at `w` (guards at 128).
    WShl {
        w: u32,
    },
    WShr {
        w: u32,
        arith: bool,
    },
    /// UNSIGNED wide div/mod (signed >64 stays oracle-bound).
    WDivMod {
        kind: DivKind,
        w: u32,
    },
    /// pop wide else, wide then, then the cond (wide or narrow stack by
    /// `cond_wide`) → wide select/merge at `w`.
    WTernary {
        w: u32,
        cond_wide: bool,
        cond_w: u32,
    },
    /// pop a wide operand → 1-bit reduction on the NARROW stack.
    WReduce {
        kind: RedK,
        neg: bool,
        opw: u32,
    },
    WLogNot {
        opw: u32,
    },
    /// v6 ④ wide structural trio. pop narrow offset, pop base (wide stack iff
    /// `base_wide`) → gather `sel_w` bits (same OOB→X / X-offset rules as the
    /// narrow `Select`), result on the wide stack iff `out_wide` (sel_w > 64).
    WSelect {
        kind: SelKind,
        sel_w: u32,
        src_w: u32,
        base_wide: bool,
        out_wide: bool,
    },
    /// One >64-bit concat fold step: pop part (lo; wide iff `part_wide`), pop
    /// the running acc (hi; wide iff `acc_wide`) → push `(hi << lo_w) | lo`
    /// masked to `w` (65..=128) on the wide stack.
    WConcatPair {
        lo_w: u32,
        w: u32,
        acc_wide: bool,
        part_wide: bool,
    },
    /// pop narrow part (`part_w` ≤ 64) → push it repeated `count` times on the
    /// wide stack (`w` = part_w × count ∈ 65..=128).
    WRepl {
        part_w: u32,
        count: u32,
        w: u32,
    },
}

/// A compiled expression. `root_w`/`root_signed` stamp the final `Value` so it is
/// byte-identical to `eval_ctx`'s return for this `(ExprId, ctx)`; the kernel hands
/// that `Value` straight to `k_write_lvalue`/`k_schedule_nba`.
pub(crate) struct NativeProg {
    ops: Vec<NOp>,
    root_w: u32,
    root_signed: bool,
    /// VM-WIDEZERO: true iff the program uses the wide (u128-pair) stack
    /// (`wmax > 0`). A narrow-only program skips the wide-stack zero-init in
    /// `run`, which is otherwise a per-eval tax on every narrow expression.
    needs_wide: bool,
}

/// P3-5: the run-time value stacks are FIXED arrays (zero per-call heap
/// allocation). Max live depth == the expression's right-leaning nesting depth
/// (a linear `a+b+c+…` chain peaks at 2–3); `try_compile` verifies the compiled
/// program never exceeds either cap and bails to the oracle otherwise.
const NATIVE_STACK: usize = 64;
/// Wide (u128 pair) stack cap. Deliberately SMALL: the buffer is stack-zeroed
/// on every `run` call (no-unsafe policy ⇒ no MaybeUninit), so its size is a
/// per-eval tax on NARROW programs too. Real wide expressions peak at depth
/// 2–3 (post-order left-fold); anything deeper bails to the oracle at compile.
const WIDE_STACK: usize = 8;

/// `low_mask` at 128 bits.
#[inline]
fn wmask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

/// Stack effect of one op on (narrow, wide): (npop, npush, wpop, wpush).
fn arity(op: &NOp) -> (u32, u32, u32, u32) {
    match op {
        NOp::Const { .. } | NOp::LoadScalar { .. } => (0, 1, 0, 0),
        NOp::LoadIndexed { .. } => (1, 1, 0, 0),
        NOp::Not { .. }
        | NOp::Neg { .. }
        | NOp::Reduce { .. }
        | NOp::LogNot { .. }
        | NOp::Repl { .. } => (1, 1, 0, 0),
        NOp::Ternary { .. } => (3, 1, 0, 0),
        // wide lane
        NOp::Promote => (1, 0, 0, 1),
        NOp::WConst { .. } | NOp::WLoadScalar { .. } => (0, 0, 0, 1),
        NOp::WLoadIndexed { .. } => (1, 0, 0, 1),
        NOp::WArith { .. } | NOp::WBitwise { .. } | NOp::WDivMod { .. } => (0, 0, 2, 1),
        NOp::WNot { .. } | NOp::WNeg { .. } => (0, 0, 1, 1),
        NOp::WCmp { .. } | NOp::WEqNe { .. } | NOp::WCaseEqNe { .. } => (0, 1, 2, 0),
        NOp::WShl { .. } | NOp::WShr { .. } => (1, 0, 1, 1),
        NOp::WTernary { cond_wide, .. } => {
            if *cond_wide {
                (0, 0, 3, 1)
            } else {
                (1, 0, 2, 1)
            }
        }
        NOp::WReduce { .. } | NOp::WLogNot { .. } => (0, 1, 1, 0),
        // v6 ④ wide structural trio
        NOp::WSelect {
            base_wide,
            out_wide,
            ..
        } => (
            1 + u32::from(!base_wide),
            u32::from(!out_wide),
            u32::from(*base_wide),
            u32::from(*out_wide),
        ),
        NOp::WConcatPair {
            acc_wide,
            part_wide,
            ..
        } => (
            u32::from(!acc_wide) + u32::from(!part_wide),
            0,
            u32::from(*acc_wide) + u32::from(*part_wide),
            1,
        ),
        NOp::WRepl { .. } => (1, 0, 0, 1),
        // remaining narrow binaries (Arith/Bitwise/Cmp/EqNe/CaseEqNe/Shl/Shr/
        // DivMod/LogBin/Select/ConcatPair)
        _ => (2, 1, 0, 0),
    }
}

/// `eval_const` minus the real lane (we bail on real). Returns the const's natural
/// `Value` (pre-resize); `None` for a real const.
fn const_value(ir: &SimIr, cid: u32) -> Option<Value> {
    let c = &ir.consts[cid as usize];
    if matches!(c.repr, ConstRepr::Real) {
        return None;
    }
    let signed = matches!(c.repr, ConstRepr::Numeric) && c.signed;
    Some(Value::from_packed(&c.bits, c.width, signed))
}

/// Extract a `Value`'s low 128 bits as a masked u128 pair (wide registers).
fn wide_pair(v: &Value, w: u32) -> (u128, u128) {
    let m = wmask(w);
    let vlo = v.val.first().copied().unwrap_or(0) as u128;
    let vhi = v.val.get(1).copied().unwrap_or(0) as u128;
    let ulo = v.unk.first().copied().unwrap_or(0) as u128;
    let uhi = v.unk.get(1).copied().unwrap_or(0) as u128;
    ((vlo | (vhi << 64)) & m, (ulo | (uhi << 64)) & m)
}

/// Bridge a narrow 1-bit/natural producer into a wide context (plain
/// zero-extend — narrow registers keep bits ≥ node width 0).
fn promote_if(wide: bool, ops: &mut Vec<NOp>) {
    if wide {
        ops.push(NOp::Promote);
    }
}

/// `BinOp` → its native class, or `None` if unsupported (Div/Mod/Pow/compare/shift/
/// logical — each has subtler semantics, deferred to a later increment).
enum BinClass {
    Arith(ArithKind),
    Bit(BitKind),
}
fn classify_binop(op: BinOp) -> Option<BinClass> {
    Some(match op {
        BinOp::Add => BinClass::Arith(ArithKind::Add),
        BinOp::Sub => BinClass::Arith(ArithKind::Sub),
        BinOp::Mul => BinClass::Arith(ArithKind::Mul),
        BinOp::BitAnd => BinClass::Bit(BitKind::And),
        BinOp::BitOr => BinClass::Bit(BitKind::Or),
        BinOp::BitXor => BinClass::Bit(BitKind::Xor),
        BinOp::BitXnor => BinClass::Bit(BitKind::Xnor),
        _ => return None,
    })
}

/// POW-LANE: the largest exponent we expand into a native Mul chain.
const POW_MAX: u128 = 16;

/// POW-LANE: `Some(n)` iff `eid` is a `Const` exponent that is X-free and a small
/// integer `2..=POW_MAX`. A non-const, X-bearing, <2, negative, or too-large
/// exponent returns `None` (the `**` node then stays oracle-bound). Exponent 1 is
/// excluded on purpose: `a**1` is `a` natively, but the oracle routes it through
/// `arith(Pow,…)` which X-poisons ANY X bit to all-X — only a chain with an actual
/// Mul (n>=2) reproduces that, so n==1 is left to the oracle.
fn const_pow_exponent(ir: &SimIr, eid: u32) -> Option<u32> {
    let Expr::Const { val } = ir.exprs.get(eid as usize)? else {
        return None;
    };
    let v = const_value(ir, *val)?; // None for a real const
    if v.has_xz() {
        return None;
    }
    let n = v.to_u128()?; // a negative signed const reads huge here → rejected below
    if (2..=POW_MAX).contains(&n) {
        Some(n as u32)
    } else {
        None
    }
}

/// Try to compile `eid` evaluated in context `(ctx_width, ctx_signed)` — the SAME
/// context `eval_for_lvalue` passes (`ctx_width = max(lvalue_w, self_w(rhs))`,
/// `ctx_signed = rhs self-sign`). `None` ⇒ outside the supported subset ⇒ fall back.
pub(crate) fn try_compile(
    ir: &SimIr,
    wt: &WidthTable,
    eid: u32,
    ctx_width: u32,
    ctx_signed: bool,
) -> Option<NativeProg> {
    let self_sw = wt.get(eid);
    let root_w = self_sw.width.max(ctx_width);
    if root_w == 0 || root_w > 128 {
        return None;
    }
    let mut ops = Vec::new();
    lower(ir, wt, eid, ctx_width, ctx_signed, &mut ops)?;
    // P3-5: verify the post-order program fits BOTH fixed run-time stacks.
    let (mut nbal, mut wbal): (u32, u32) = (0, 0);
    let (mut nmax, mut wmax): (u32, u32) = (0, 0);
    for op in &ops {
        let (npop, npush, wpop, wpush) = arity(op);
        nbal = nbal.checked_sub(npop)?; // malformed program defends as a bail
        wbal = wbal.checked_sub(wpop)?;
        nbal += npush;
        wbal += wpush;
        nmax = nmax.max(nbal);
        wmax = wmax.max(wbal);
    }
    if nmax as usize > NATIVE_STACK || wmax as usize > WIDE_STACK {
        return None; // absurdly right-leaning nesting: leave it to the oracle
    }
    // At the root, `ctx_signed` IS `rhs` self-sign (eval_for_lvalue), so
    // `eff_signed = self_signed && ctx_signed == ctx_signed`, and the arith/
    // bitwise/compare arms stamp the result `signed = eff_signed` = `ctx_signed`.
    // The STRUCTURAL arms are the exception: the oracle finishes them with
    // `resize_keep_sign(w, false)` (select/concat/replicate are unsigned by
    // definition), which stamps `signed = false` REGARDLESS of context — mirror
    // that so a structural root matches the oracle for any caller-provided ctx.
    let root_signed = match ir.exprs.get(eid as usize) {
        Some(Expr::Select { .. } | Expr::Concat { .. } | Expr::Replicate { .. }) => false,
        _ => ctx_signed,
    };
    Some(NativeProg {
        ops,
        root_w,
        root_signed,
        needs_wide: wmax > 0, // VM-WIDEZERO
    })
}

/// Post-order lowering mirroring `eval_ctx`'s context propagation. Returns `None`
/// (bailing the WHOLE expression) on any unsupported node or any node width > 64.
fn lower(
    ir: &SimIr,
    wt: &WidthTable,
    eid: u32,
    ctx_width: u32,
    ctx_signed: bool,
    ops: &mut Vec<NOp>,
) -> Option<()> {
    let self_sw = wt.get(eid);
    let w = self_sw.width.max(ctx_width);
    let eff_signed = self_sw.signed && ctx_signed;
    if w == 0 || w > 128 {
        return None;
    }
    // C6: a node's register lives on the wide stack IFF its eval width > 64.
    let wide = w > 64;
    // P2-8: a same-schema corrupted `.velab` can carry an out-of-range ExprId;
    // bail to the interpreter (which raises its own diagnostics) over panicking.
    let expr = ir.exprs.get(eid as usize)?;
    match expr {
        Expr::Const { val } => {
            let r = const_value(ir, *val)?.resize_keep_sign(w, eff_signed);
            if wide {
                let (cv, cu) = wide_pair(&r, w);
                ops.push(NOp::WConst { val: cv, unk: cu });
            } else {
                let m = low_mask(w);
                ops.push(NOp::Const {
                    val: r.val.first().copied().unwrap_or(0) & m,
                    unk: r.unk.first().copied().unwrap_or(0) & m,
                });
            }
            Some(())
        }
        Expr::Signal { net, word } => {
            if let Some(weid) = word {
                // v5 ⑤/v6: assoc keys are SIGNED-i64 (or byte-string) domain
                // — the u32 LoadIndexed funnel cannot carry them (a negative
                // key would sentinel to X while the interpreter reads the
                // element). Stay oracle-bound (eval_ctx fallback).
                if matches!(
                    ir.nets.get(*net as usize).map(|n| n.kind),
                    Some(sim_ir::NetKind::Assoc | sim_ir::NetKind::AssocStr)
                ) {
                    return None;
                }
                // dynamic array-word read: index is SELF-determined (oracle
                // `self.eval(weid)`); a wide index stays oracle-bound.
                let iw = wt.get(*weid).width;
                if iw == 0 || iw > 64 {
                    return None;
                }
                lower(ir, wt, *weid, 0, true, ops)?;
                ops.push(if wide {
                    NOp::WLoadIndexed {
                        net: *net,
                        w,
                        signed: eff_signed,
                    }
                } else {
                    NOp::LoadIndexed {
                        net: *net,
                        w,
                        signed: eff_signed,
                    }
                });
            } else {
                ops.push(if wide {
                    NOp::WLoadScalar {
                        net: *net,
                        w,
                        signed: eff_signed,
                    }
                } else {
                    NOp::LoadScalar {
                        net: *net,
                        w,
                        signed: eff_signed,
                    }
                });
            }
            Some(())
        }
        Expr::Unary { op, operand } => match op {
            // context-determined unary: propagate (w, eff_signed) into the operand.
            UnOp::Plus => lower(ir, wt, *operand, w, eff_signed, ops), // passthrough
            UnOp::Minus => {
                lower(ir, wt, *operand, w, eff_signed, ops)?;
                ops.push(if wide {
                    NOp::WNeg { w }
                } else {
                    NOp::Neg { w }
                });
                Some(())
            }
            UnOp::BitNot => {
                lower(ir, wt, *operand, w, eff_signed, ops)?;
                ops.push(if wide {
                    NOp::WNot { w }
                } else {
                    NOp::Not { w }
                });
                Some(())
            }
            // reductions / lognot: 1-bit result over a SELF-DETERMINED operand
            // (lower with ctx_width 0 / ctx_signed true ⇒ (self_w, self_signed),
            // exactly the oracle's `self.eval(operand)`), then zero-extend (free:
            // the register's upper bits are already 0 and parents mask).
            UnOp::LogNot => {
                let opw = wt.get(*operand).width;
                lower(ir, wt, *operand, 0, true, ops)?;
                ops.push(if opw > 64 {
                    NOp::WLogNot { opw }
                } else {
                    NOp::LogNot { opw }
                });
                promote_if(wide, ops);
                Some(())
            }
            UnOp::RedAnd
            | UnOp::RedNand
            | UnOp::RedOr
            | UnOp::RedNor
            | UnOp::RedXor
            | UnOp::RedXnor => {
                let opw = wt.get(*operand).width;
                lower(ir, wt, *operand, 0, true, ops)?;
                let (kind, neg) = match op {
                    UnOp::RedAnd => (RedK::And, false),
                    UnOp::RedNand => (RedK::And, true),
                    UnOp::RedOr => (RedK::Or, false),
                    UnOp::RedNor => (RedK::Or, true),
                    UnOp::RedXor => (RedK::Xor, false),
                    _ => (RedK::Xor, true), // RedXnor
                };
                ops.push(if opw > 64 {
                    NOp::WReduce { kind, neg, opw }
                } else {
                    NOp::Reduce { kind, neg, opw }
                });
                promote_if(wide, ops);
                Some(())
            }
        },
        Expr::Binary { op, lhs, rhs } => {
            use BinOp as B;
            match op {
                // COMPARISONS: operands mutually context-determined at
                // (max(self_w(L), self_w(R)), bothsigned); 1-bit result.
                B::Lt | B::Le | B::Gt | B::Ge | B::Eq | B::Ne | B::CaseEq | B::CaseNe => {
                    let cmp_w = wt.get(*lhs).width.max(wt.get(*rhs).width);
                    if cmp_w == 0 || cmp_w > 128 {
                        return None;
                    }
                    let cmp_wide = cmp_w > 64;
                    let pair_signed = wt.get(*lhs).signed && wt.get(*rhs).signed;
                    lower(ir, wt, *lhs, cmp_w, pair_signed, ops)?;
                    lower(ir, wt, *rhs, cmp_w, pair_signed, ops)?;
                    let cmp = |kind| {
                        if cmp_wide {
                            NOp::WCmp {
                                kind,
                                w: cmp_w,
                                signed: pair_signed,
                            }
                        } else {
                            NOp::Cmp {
                                kind,
                                w: cmp_w,
                                signed: pair_signed,
                            }
                        }
                    };
                    let eq = |ne| {
                        if cmp_wide {
                            NOp::WEqNe { ne, w: cmp_w }
                        } else {
                            NOp::EqNe { ne, w: cmp_w }
                        }
                    };
                    let ceq = |ne| {
                        if cmp_wide {
                            NOp::WCaseEqNe { ne, w: cmp_w }
                        } else {
                            NOp::CaseEqNe { ne, w: cmp_w }
                        }
                    };
                    ops.push(match op {
                        B::Lt => cmp(CmpKind::Lt),
                        B::Le => cmp(CmpKind::Le),
                        B::Gt => cmp(CmpKind::Gt),
                        B::Ge => cmp(CmpKind::Ge),
                        B::Eq => eq(false),
                        B::Ne => eq(true),
                        B::CaseEq => ceq(false),
                        _ => ceq(true), // CaseNe
                    });
                    promote_if(wide, ops);
                    Some(())
                }
                // LOGICAL: each operand self-determined, tri-valued combine
                // (wide operands stay oracle-bound — `c = a128 && b` is rare).
                B::LogAnd | B::LogOr => {
                    let lw = wt.get(*lhs).width;
                    let rw = wt.get(*rhs).width;
                    if lw > 64 || rw > 64 {
                        return None;
                    }
                    lower(ir, wt, *lhs, 0, true, ops)?;
                    lower(ir, wt, *rhs, 0, true, ops)?;
                    ops.push(NOp::LogBin {
                        and: matches!(op, B::LogAnd),
                        lw,
                        rw,
                    });
                    promote_if(wide, ops);
                    Some(())
                }
                // SHIFTS: LEFT context-determined; amount SELF-determined
                // (a >64-bit amount register stays oracle-bound).
                B::Shl | B::AShl => {
                    if wt.get(*rhs).width > 64 {
                        return None;
                    }
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    lower(ir, wt, *rhs, 0, true, ops)?;
                    ops.push(if wide {
                        NOp::WShl { w }
                    } else {
                        NOp::Shl { w }
                    });
                    Some(())
                }
                B::Shr => {
                    if wt.get(*rhs).width > 64 {
                        return None;
                    }
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    lower(ir, wt, *rhs, 0, true, ops)?;
                    ops.push(if wide {
                        NOp::WShr { w, arith: false }
                    } else {
                        NOp::Shr { w, arith: false }
                    });
                    Some(())
                }
                // `>>>`: fill governed by the LEFT operand's OWN sign (oracle
                // evaluates lhs at (w, own-sign); the final ctx re-stamp only
                // changes the sign FLAG, never the bits — registers carry bits).
                B::AShr => {
                    if wt.get(*rhs).width > 64 {
                        return None;
                    }
                    let lhs_signed = wt.get(*lhs).signed;
                    lower(ir, wt, *lhs, w, lhs_signed, ops)?;
                    lower(ir, wt, *rhs, 0, true, ops)?;
                    ops.push(if wide {
                        NOp::WShr {
                            w,
                            arith: lhs_signed,
                        }
                    } else {
                        NOp::Shr {
                            w,
                            arith: lhs_signed,
                        }
                    });
                    Some(())
                }
                // DIV/MOD: context-determined like Add (X/Z or /0 → all-X).
                // SIGNED wide div/mod X-poisons in the oracle — oracle-bound.
                B::Div | B::Mod => {
                    if wide && eff_signed {
                        return None;
                    }
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    lower(ir, wt, *rhs, w, eff_signed, ops)?;
                    let kind = if matches!(op, B::Div) {
                        DivKind::Div
                    } else {
                        DivKind::Mod
                    };
                    ops.push(if wide {
                        NOp::WDivMod { kind, w }
                    } else {
                        NOp::DivMod {
                            kind,
                            w,
                            signed: eff_signed,
                        }
                    });
                    Some(())
                }
                // POWER: native only for a small const exponent n>=2, expanded to a
                // Mul chain (a*a*…). For n>=2 the X-poison matches the oracle's Pow
                // exactly (any X bit → all-X, identical to repeated Mul's arith
                // X-guard; n==1 is excluded above because it would skip the Mul).
                // Two guards keep VALUES byte-identical to the oracle:
                //  - bail when eff_signed (signed uses ipow_signed, a different path),
                //  - require w*n <= 128 so a^n < 2^128 — the oracle computes a^n in a
                //    u128 and returns 0 on u128 OVERFLOW (`checked_pow().unwrap_or(0)`)
                //    rather than wrapping, a quirk a per-step mod-2^w chain can't mimic.
                B::Pow => {
                    let n = const_pow_exponent(ir, *rhs)?;
                    if eff_signed || (w as u128) * (n as u128) > 128 {
                        return None;
                    }
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    for _ in 1..n {
                        lower(ir, wt, *lhs, w, eff_signed, ops)?;
                        ops.push(if wide {
                            NOp::WArith {
                                kind: ArithKind::Mul,
                                w,
                            }
                        } else {
                            NOp::Arith {
                                kind: ArithKind::Mul,
                                w,
                            }
                        });
                    }
                    Some(())
                }
                _ => {
                    let class = classify_binop(*op)?;
                    // SIGNED wide arith: the oracle X-poisons (its i128 sign lane
                    // gates at 64) — conservatively oracle-bound.
                    if wide && eff_signed && matches!(class, BinClass::Arith(_)) {
                        return None;
                    }
                    // ARITHMETIC + BITWISE: BOTH operands at (w, eff_signed).
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    lower(ir, wt, *rhs, w, eff_signed, ops)?;
                    ops.push(match (class, wide) {
                        (BinClass::Arith(kind), false) => NOp::Arith { kind, w },
                        (BinClass::Arith(kind), true) => NOp::WArith { kind, w },
                        (BinClass::Bit(kind), false) => NOp::Bitwise { kind, w },
                        (BinClass::Bit(kind), true) => NOp::WBitwise { kind, w },
                    });
                    Some(())
                }
            }
        }
        // ternary: cond self-determined truthiness; branches context-determined.
        Expr::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            let cond_w = wt.get(*cond).width;
            // a wide cond steering NARROW branches stays oracle-bound (the
            // narrow Ternary op reads a single-word cond register).
            if !wide && cond_w > 64 {
                return None;
            }
            lower(ir, wt, *cond, 0, true, ops)?;
            lower(ir, wt, *then_e, w, eff_signed, ops)?;
            lower(ir, wt, *else_e, w, eff_signed, ops)?;
            ops.push(if wide {
                NOp::WTernary {
                    w,
                    cond_wide: cond_w > 64,
                    cond_w,
                }
            } else {
                NOp::Ternary { w, cond_w }
            });
            Some(())
        }
        // ── structural ops: SELF-determined natural value, then UNSIGNED
        //    zero-extend to the node width (oracle: `eval_select`/`eval_concat`/
        //    `eval_replicate` + `resize_keep_sign(w, false)` — and unsigned
        //    `resize` is a plain zero-extend, which is FREE here because every
        //    register keeps its upper bits 0). ──
        Expr::Select {
            base,
            offset,
            width,
            kind,
        } => {
            // `width` is a const-expr edge — fold it exactly as the oracle does
            // (`unwrap_or(1)`); `Bit` forces a 1-bit select regardless.
            let folded = crate::width::const_u32_of_expr(ir, *width).unwrap_or(1);
            let sel_w = match kind {
                SelKind::Bit => 1,
                _ => folded,
            };
            let src_w = wt.get(*base).width;
            // v6 ④: the trio runs to 128 bits (two-word gather); only a wide
            // OFFSET register still bails.
            if sel_w == 0 || sel_w > 128 || src_w == 0 || src_w > 128 {
                return None;
            }
            if wt.get(*offset).width > 64 {
                return None;
            }
            lower(ir, wt, *base, 0, true, ops)?; // oracle: self.eval(base)
            lower(ir, wt, *offset, 0, true, ops)?; // oracle: self.eval(offset)
            let base_wide = src_w > 64;
            let out_wide = sel_w > 64;
            if !base_wide && !out_wide {
                ops.push(NOp::Select {
                    kind: *kind,
                    sel_w,
                    src_w,
                });
            } else {
                ops.push(NOp::WSelect {
                    kind: *kind,
                    sel_w,
                    src_w,
                    base_wide,
                    out_wide,
                });
            }
            // a narrow result feeding a wide context still bridges; a wide
            // result is already in place (zero-extend beyond sel_w is free).
            promote_if(wide && !out_wide, ops);
            Some(())
        }
        Expr::Concat { parts } => {
            // parts[0] is MSB-most; left-fold `(hi << lo_w) | lo` reproduces the
            // oracle's top-down fill. Natural width = Σ self widths ≤ node w ≤ 64.
            let (&first, rest) = parts.split_first()?;
            let mut tot = wt.get(first).width;
            if tot == 0 || tot > 128 {
                return None;
            }
            lower(ir, wt, first, 0, true, ops)?;
            for &p in rest {
                let pw = wt.get(p).width;
                if pw == 0 || pw > 128 {
                    return None;
                }
                let acc_wide = tot > 64; // where the RUNNING acc lives
                tot = tot.checked_add(pw).filter(|&t| t <= 128)?;
                lower(ir, wt, p, 0, true, ops)?;
                if tot <= 64 {
                    ops.push(NOp::ConcatPair { lo_w: pw, w: tot });
                } else {
                    ops.push(NOp::WConcatPair {
                        lo_w: pw,
                        w: tot,
                        acc_wide,
                        part_wide: pw > 64,
                    });
                }
            }
            // single-part `{x}` (or an all-narrow fold) may end narrow in a
            // wide context — bridge; a >64 fold already sits on the wide stack.
            promote_if(wide && tot <= 64, ops);
            Some(())
        }
        Expr::Replicate { count, value } => {
            // `count` is a const-expr edge (oracle folds with `unwrap_or(0)`);
            // a zero count is the degenerate width-0 case — leave it to the oracle.
            let count = crate::width::const_u32_of_expr(ir, *count).unwrap_or(0);
            if count == 0 {
                return None;
            }
            let part_w = wt.get(*value).width;
            // v6 ④: total runs to 128 (the PART stays narrow — a >64-bit part
            // with count ≥ 2 exceeds the wide lane anyway).
            if part_w == 0 || part_w > 64 {
                return None;
            }
            let total = part_w.checked_mul(count).filter(|&t| t <= 128)?;
            lower(ir, wt, *value, 0, true, ops)?;
            if total <= 64 {
                ops.push(NOp::Repl {
                    part_w,
                    count,
                    w: total,
                });
                promote_if(wide, ops);
            } else {
                ops.push(NOp::WRepl {
                    part_w,
                    count,
                    w: total,
                });
            }
            Some(())
        }
        // B1 frame-call: the VM never compiles a Call-bearing body
        // (`is_codegen_able`'s `expr_has_call` exclusion), so the native path
        // must NEVER reach a user `Expr::Call`. Assert the contract in debug;
        // bail to the oracle (kernel `eval_ctx`, which runs the real frame
        // evaluator) in release — safe either way.
        Expr::Call { .. } => {
            debug_assert!(
                false,
                "is_codegen_able must keep Expr::Call off the native/VM path"
            );
            None
        }
        // sysfunc / array-indexed signal: deferred increment.
        _ => None,
    }
}

/// Run a compiled program against `nets`, producing the single `Value` the oracle's
/// `eval_ctx` would return for the same `(ExprId, ctx)`.
pub(crate) fn run(prog: &NativeProg, nets: &dyn NetReader) -> Value {
    // P3-5: fixed arrays + manual sp — no heap allocation per evaluation.
    // `try_compile` guaranteed the program's max depth fits BOTH stacks.
    let mut buf = [(0u64, 0u64); NATIVE_STACK];
    let mut sp = 0usize;
    let mut stack = FixedStack {
        buf: &mut buf,
        sp: &mut sp,
    };
    // VM-WIDEZERO: only zero-init the 256 B wide stack for programs that use it;
    // a narrow-only program (wmax==0) never executes a W* opcode, so leave `wbuf`
    // uninitialized and hand the wide stack an empty slice (never indexed).
    let mut wbuf: [(u128, u128); WIDE_STACK];
    let wbuf_slice: &mut [(u128, u128)] = if prog.needs_wide {
        wbuf = [(0u128, 0u128); WIDE_STACK];
        &mut wbuf
    } else {
        &mut []
    };
    let mut wsp = 0usize;
    let mut wstack = FixedStack {
        buf: wbuf_slice,
        sp: &mut wsp,
    };
    for op in &prog.ops {
        // VM-ARITY-ASSERT: verify each op's actual stack movement matches arity()
        // (which try_compile trusts for the fixed-stack cap check). Debug-only, so
        // release is byte-identical with zero overhead. Catches both a wrong
        // explicit arity entry and a NEW NOp variant silently routed to the
        // `_ => (2,1,0,0)` catchall.
        #[cfg(debug_assertions)]
        let (sp_dbg, wsp_dbg) = (*stack.sp, *wstack.sp);
        match *op {
            NOp::Const { val, unk } => stack.push((val, unk)),
            NOp::LoadScalar { net, w, signed } => {
                let v = nets.read_net(net, None).resize_keep_sign(w, signed);
                let m = low_mask(w);
                stack.push((
                    v.val.first().copied().unwrap_or(0) & m,
                    v.unk.first().copied().unwrap_or(0) & m,
                ));
            }
            NOp::Arith { kind, w } => {
                let (bv, bu) = stack.pop().expect("native arith: missing rhs");
                let (av, au) = stack.pop().expect("native arith: missing lhs");
                let m = low_mask(w);
                // Oracle `arith`: ANY X/Z in EITHER operand poisons the whole result to
                // X. An X bit is `(val=0, unk=1)` (matching `Value::xs`), so all-X is
                // `(0, m)` — NOT `(m, m)`.
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, m)
                } else {
                    let rv = match kind {
                        ArithKind::Add => av.wrapping_add(bv),
                        ArithKind::Sub => av.wrapping_sub(bv),
                        ArithKind::Mul => av.wrapping_mul(bv),
                    };
                    (rv & m, 0)
                };
                stack.push(res);
            }
            NOp::Bitwise { kind, w } => {
                let (bv, bu) = stack.pop().expect("native bitwise: missing rhs");
                let (av, au) = stack.pop().expect("native bitwise: missing lhs");
                let m = low_mask(w);
                let (rv, ru) = match kind {
                    BitKind::And => and_w(av, au, bv, bu),
                    BitKind::Or => or_w(av, au, bv, bu),
                    BitKind::Xor => xor_w(av, au, bv, bu),
                    BitKind::Xnor => xnor_w(av, au, bv, bu),
                };
                stack.push((rv & m, ru & m));
            }
            NOp::Not { w } => {
                let (av, au) = stack.pop().expect("native not: missing operand");
                let m = low_mask(w);
                let (rv, ru) = not_w(av, au);
                stack.push((rv & m, ru & m));
            }
            NOp::Neg { w } => {
                let (av, au) = stack.pop().expect("native neg: missing operand");
                let m = low_mask(w);
                let res = if (au & m) != 0 {
                    (0, m) // oracle `negate`: any X/Z poisons to X (val=0, unk=1)
                } else {
                    ((!av).wrapping_add(1) & m, 0)
                };
                stack.push(res);
            }
            NOp::Cmp { kind, w, signed } => {
                let (bv, bu) = stack.pop().expect("native cmp: missing rhs");
                let (av, au) = stack.pop().expect("native cmp: missing lhs");
                let m = low_mask(w);
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, 1) // oracle: any X/Z → 1-bit X
                } else {
                    use std::cmp::Ordering::*;
                    let ord = if signed {
                        let sx = |x: u64| ((x << (64 - w)) as i64) >> (64 - w);
                        sx(av & m).cmp(&sx(bv & m))
                    } else {
                        (av & m).cmp(&(bv & m))
                    };
                    let b = matches!(
                        (kind, ord),
                        (CmpKind::Lt, Less)
                            | (CmpKind::Le, Less)
                            | (CmpKind::Le, Equal)
                            | (CmpKind::Gt, Greater)
                            | (CmpKind::Ge, Greater)
                            | (CmpKind::Ge, Equal)
                    );
                    (b as u64, 0)
                };
                stack.push(res);
            }
            NOp::EqNe { ne, w } => {
                let (bv, bu) = stack.pop().expect("native eq: missing rhs");
                let (av, au) = stack.pop().expect("native eq: missing lhs");
                let m = low_mask(w);
                let u = (au | bu) & m;
                // §11.4.5: a both-known differing bit decides (definite 0/1);
                // X only for an AMBIGUOUS compare (mirrors eval.rs log_eq).
                let res = if ((av ^ bv) & !u & m) != 0 {
                    (ne as u64, 0)
                } else if u != 0 {
                    (0, 1)
                } else {
                    ((!ne) as u64, 0)
                };
                stack.push(res);
            }
            NOp::CaseEqNe { ne, w } => {
                let (bv, bu) = stack.pop().expect("native caseeq: missing rhs");
                let (av, au) = stack.pop().expect("native caseeq: missing lhs");
                let m = low_mask(w);
                let eq = (av & m) == (bv & m) && (au & m) == (bu & m);
                stack.push(((eq ^ ne) as u64, 0));
            }
            NOp::Shl { w } => {
                let (rv, ru) = stack.pop().expect("native shl: missing amount");
                let (lv, lu) = stack.pop().expect("native shl: missing lhs");
                let m = low_mask(w);
                let res = if ru != 0 {
                    (0, m) // X/Z amount → all-X at w (oracle xs(l.width))
                } else if rv >= 64 {
                    (0, 0) // everything shifted out
                } else {
                    ((lv << rv) & m, (lu << rv) & m)
                };
                stack.push(res);
            }
            NOp::Shr { w, arith } => {
                let (rv, ru) = stack.pop().expect("native shr: missing amount");
                let (lv, lu) = stack.pop().expect("native shr: missing lhs");
                let m = low_mask(w);
                let res = if ru != 0 {
                    (0, m)
                } else if !arith {
                    if rv >= 64 {
                        (0, 0)
                    } else {
                        ((lv >> rv) & m, (lu >> rv) & m)
                    }
                } else {
                    // sign fill from l's MSB pair (which may itself be X/Z).
                    let fv = (lv >> (w - 1)) & 1;
                    let fu = (lu >> (w - 1)) & 1;
                    let body = if rv >= 64 {
                        (0, 0)
                    } else {
                        ((lv >> rv) & m, (lu >> rv) & m)
                    };
                    let fill_n = rv.min(w as u64) as u32; // top bits to fill
                    let fill_mask = if fill_n == 0 {
                        0
                    } else {
                        m & !low_mask(w - fill_n)
                    };
                    (
                        (body.0 & !fill_mask) | (if fv == 1 { fill_mask } else { 0 }),
                        (body.1 & !fill_mask) | (if fu == 1 { fill_mask } else { 0 }),
                    )
                };
                stack.push(res);
            }
            NOp::DivMod { kind, w, signed } => {
                let (bv, bu) = stack.pop().expect("native divmod: missing rhs");
                let (av, au) = stack.pop().expect("native divmod: missing lhs");
                let m = low_mask(w);
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, m)
                } else if signed {
                    let sx = |x: u64| ((x << (64 - w)) as i64) >> (64 - w);
                    let b = sx(bv & m);
                    if b == 0 {
                        (0, m) // divide by zero → all-X
                    } else {
                        let a = sx(av & m);
                        let r = match kind {
                            DivKind::Div => a.wrapping_div(b),
                            DivKind::Mod => a.wrapping_rem(b),
                        };
                        ((r as u64) & m, 0)
                    }
                } else {
                    let b = bv & m;
                    if b == 0 {
                        (0, m)
                    } else {
                        let a = av & m;
                        let r = match kind {
                            DivKind::Div => a / b,
                            DivKind::Mod => a % b,
                        };
                        (r & m, 0)
                    }
                };
                stack.push(res);
            }
            NOp::Ternary { w, cond_w } => {
                let (ev, eu) = stack.pop().expect("native ternary: missing else");
                let (tv, tu) = stack.pop().expect("native ternary: missing then");
                let (cv, cu) = stack.pop().expect("native ternary: missing cond");
                let mc = low_mask(cond_w);
                let m = low_mask(w);
                // truthiness: any definite-1 → True; else any unknown → Unknown;
                // else False (matches oracle `Tri`).
                let res = if (cv & !cu & mc) != 0 {
                    (tv & m, tu & m)
                } else if (cu & mc) != 0 {
                    // merge_x: identical (val,unk) pairs pass through, else X.
                    let ident = !((tv ^ ev) | (tu ^ eu)) & m;
                    ((tv & ident), (tu & ident) | (m & !ident))
                } else {
                    (ev & m, eu & m)
                };
                stack.push(res);
            }
            NOp::Reduce { kind, neg, opw } => {
                let (av, au) = stack.pop().expect("native reduce: missing operand");
                let m = low_mask(opw);
                let known1 = av & !au & m;
                let known0 = !au & !av & m;
                let unk = au & m;
                let (v, u): (u64, u64) = match kind {
                    RedK::And if known0 != 0 => (0, 0),
                    RedK::And if unk != 0 => (0, 1),
                    RedK::And => (1, 0),
                    RedK::Or if known1 != 0 => (1, 0),
                    RedK::Or if unk != 0 => (0, 1),
                    RedK::Or => (0, 0),
                    RedK::Xor if unk != 0 => (0, 1),
                    RedK::Xor => ((known1.count_ones() & 1) as u64, 0),
                };
                let out = if neg && u == 0 { (v ^ 1, 0) } else { (v, u) };
                stack.push(out);
            }
            NOp::LogNot { opw } => {
                let (av, au) = stack.pop().expect("native lognot: missing operand");
                let m = low_mask(opw);
                let res = if (av & !au & m) != 0 {
                    (0, 0) // truthy → !a = 0
                } else if (au & m) != 0 {
                    (0, 1) // unknown → X
                } else {
                    (1, 0) // falsy → 1
                };
                stack.push(res);
            }
            NOp::LogBin { and, lw, rw } => {
                let (bv, bu) = stack.pop().expect("native logbin: missing rhs");
                let (av, au) = stack.pop().expect("native logbin: missing lhs");
                let tri = |v: u64, u: u64, w: u32| {
                    let m = low_mask(w);
                    if (v & !u & m) != 0 {
                        Some(true)
                    } else if (u & m) != 0 {
                        None
                    } else {
                        Some(false)
                    }
                };
                let (l, r) = (tri(av, au, lw), tri(bv, bu, rw));
                let res = if and {
                    match (l, r) {
                        (Some(false), _) | (_, Some(false)) => (0, 0),
                        (Some(true), Some(true)) => (1, 0),
                        _ => (0, 1),
                    }
                } else {
                    match (l, r) {
                        (Some(true), _) | (_, Some(true)) => (1, 0),
                        (Some(false), Some(false)) => (0, 0),
                        _ => (0, 1),
                    }
                };
                stack.push(res);
            }
            NOp::Select { kind, sel_w, src_w } => {
                let (off_v, off_u) = stack.pop().expect("native select: missing offset");
                let (sv, su) = stack.pop().expect("native select: missing base");
                let m = low_mask(sel_w);
                // Oracle: X/Z offset (or one beyond the i64 lane) ⇒ the whole
                // select reads X at its natural width (upper bits stay 0 — the
                // unsigned resize is a zero-extend).
                let res = match (off_u == 0)
                    .then_some(off_v)
                    .and_then(|v| i64::try_from(v).ok())
                {
                    None => (0, m),
                    Some(off) => {
                        let lsb = match kind {
                            SelKind::Bit | SelKind::PartConst | SelKind::PartIdxUp => off,
                            SelKind::PartIdxDown => off - (sel_w as i64) + 1,
                        };
                        let (mut rv, mut ru) = (0u64, 0u64);
                        for i in 0..sel_w as i64 {
                            let si = lsb + i;
                            if si >= 0 && (si as u32) < src_w {
                                rv |= ((sv >> si) & 1) << i;
                                ru |= ((su >> si) & 1) << i;
                            } else {
                                ru |= 1 << i; // out-of-range read → X (val=0)
                            }
                        }
                        (rv, ru)
                    }
                };
                stack.push(res);
            }
            NOp::ConcatPair { lo_w, w } => {
                let (lo_v, lo_u) = stack.pop().expect("native concat: missing lo");
                let (hi_v, hi_u) = stack.pop().expect("native concat: missing hi");
                let m = low_mask(w);
                // lo_w ≤ 63 here: with ≥2 parts of ≥1 bit each and w ≤ 64.
                stack.push((((hi_v << lo_w) | lo_v) & m, ((hi_u << lo_w) | lo_u) & m));
            }
            NOp::Repl { part_w, count, w } => {
                let (pv, pu) = stack.pop().expect("native repl: missing part");
                let (mut rv, mut ru) = (0u64, 0u64);
                for c in 0..count {
                    let sh = c * part_w; // (count-1)·part_w < w ≤ 64
                    rv |= pv << sh;
                    ru |= pu << sh;
                }
                let m = low_mask(w);
                stack.push((rv & m, ru & m));
            }
            NOp::LoadIndexed { net, w, signed } => {
                let (iv, iu) = stack.pop().expect("native loadidx: missing index");
                let v = nets
                    .read_net(net, Some(word_index(iv, iu)))
                    .resize_keep_sign(w, signed);
                let m = low_mask(w);
                stack.push((
                    v.val.first().copied().unwrap_or(0) & m,
                    v.unk.first().copied().unwrap_or(0) & m,
                ));
            }

            // ── C6 wide lane ──
            NOp::Promote => {
                let (v, u) = stack.pop().expect("native promote: missing operand");
                wstack.push((v as u128, u as u128));
            }
            NOp::WConst { val, unk } => wstack.push((val, unk)),
            NOp::WLoadScalar { net, w, signed } => {
                let v = nets.read_net(net, None).resize_keep_sign(w, signed);
                wstack.push(wide_pair(&v, w));
            }
            NOp::WLoadIndexed { net, w, signed } => {
                let (iv, iu) = stack.pop().expect("native wloadidx: missing index");
                let v = nets
                    .read_net(net, Some(word_index(iv, iu)))
                    .resize_keep_sign(w, signed);
                wstack.push(wide_pair(&v, w));
            }
            NOp::WArith { kind, w } => {
                let (bv, bu) = wstack.pop().expect("native warith: missing rhs");
                let (av, au) = wstack.pop().expect("native warith: missing lhs");
                let m = wmask(w);
                // oracle `arith` unsigned lane: u128 wrapping ops, X/Z poisons.
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, m)
                } else {
                    let rv = match kind {
                        ArithKind::Add => av.wrapping_add(bv),
                        ArithKind::Sub => av.wrapping_sub(bv),
                        ArithKind::Mul => av.wrapping_mul(bv),
                    };
                    (rv & m, 0)
                };
                wstack.push(res);
            }
            NOp::WBitwise { kind, w } => {
                let (bv, bu) = wstack.pop().expect("native wbitwise: missing rhs");
                let (av, au) = wstack.pop().expect("native wbitwise: missing lhs");
                let m = wmask(w);
                // the same `value::*_w` plane formulas, bit-parallel at u128.
                let (rv, ru) = match kind {
                    BitKind::And => {
                        let known0 = (!au & !av) | (!bu & !bv);
                        let known1 = (!au & av) & (!bu & bv);
                        (known1, !known0 & !known1)
                    }
                    BitKind::Or => {
                        let known1 = (!au & av) | (!bu & bv);
                        let known0 = (!au & !av) & (!bu & !bv);
                        (known1, !known1 & !known0)
                    }
                    BitKind::Xor => {
                        let ru = au | bu;
                        ((av ^ bv) & !ru, ru)
                    }
                    BitKind::Xnor => {
                        let ru = au | bu;
                        (!(av ^ bv) & !ru, ru)
                    }
                };
                wstack.push((rv & m, ru & m));
            }
            NOp::WNot { w } => {
                let (av, au) = wstack.pop().expect("native wnot: missing operand");
                let m = wmask(w);
                wstack.push(((!av & !au) & m, au & m));
            }
            NOp::WNeg { w } => {
                let (av, au) = wstack.pop().expect("native wneg: missing operand");
                let m = wmask(w);
                let res = if (au & m) != 0 {
                    (0, m) // oracle `negate`: any X/Z poisons to X
                } else {
                    // (!x + 1) at u128 ≡ the oracle's per-word carry chain.
                    ((!av).wrapping_add(1) & m, 0)
                };
                wstack.push(res);
            }
            NOp::WCmp { kind, w, signed } => {
                let (bv, bu) = wstack.pop().expect("native wcmp: missing rhs");
                let (av, au) = wstack.pop().expect("native wcmp: missing lhs");
                let m = wmask(w);
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, 1) // oracle: any X/Z → 1-bit X
                } else {
                    use std::cmp::Ordering::*;
                    let ord = if signed {
                        // sign-extend from w (65..=128; w==128 shifts by 0).
                        let sx = |x: u128| ((x << (128 - w)) as i128) >> (128 - w);
                        sx(av & m).cmp(&sx(bv & m))
                    } else {
                        (av & m).cmp(&(bv & m))
                    };
                    let b = matches!(
                        (kind, ord),
                        (CmpKind::Lt, Less)
                            | (CmpKind::Le, Less)
                            | (CmpKind::Le, Equal)
                            | (CmpKind::Gt, Greater)
                            | (CmpKind::Ge, Greater)
                            | (CmpKind::Ge, Equal)
                    );
                    (b as u64, 0)
                };
                stack.push(res);
            }
            NOp::WEqNe { ne, w } => {
                let (bv, bu) = wstack.pop().expect("native weq: missing rhs");
                let (av, au) = wstack.pop().expect("native weq: missing lhs");
                let m = wmask(w);
                let u = (au | bu) & m;
                // §11.4.5 definite-mismatch rule — mirrors EqNe / log_eq.
                let res = if ((av ^ bv) & !u & m) != 0 {
                    (ne as u64, 0)
                } else if u != 0 {
                    (0, 1)
                } else {
                    ((!ne) as u64, 0)
                };
                stack.push(res);
            }
            NOp::WCaseEqNe { ne, w } => {
                let (bv, bu) = wstack.pop().expect("native wcaseeq: missing rhs");
                let (av, au) = wstack.pop().expect("native wcaseeq: missing lhs");
                let m = wmask(w);
                let eq = (av & m) == (bv & m) && (au & m) == (bu & m);
                stack.push(((eq ^ ne) as u64, 0));
            }
            NOp::WShl { w } => {
                let (rv, ru) = stack.pop().expect("native wshl: missing amount");
                let (lv, lu) = wstack.pop().expect("native wshl: missing lhs");
                let m = wmask(w);
                let res = if ru != 0 {
                    (0, m)
                } else if rv >= 128 {
                    (0, 0)
                } else {
                    ((lv << rv) & m, (lu << rv) & m)
                };
                wstack.push(res);
            }
            NOp::WShr { w, arith } => {
                let (rv, ru) = stack.pop().expect("native wshr: missing amount");
                let (lv, lu) = wstack.pop().expect("native wshr: missing lhs");
                let m = wmask(w);
                let res = if ru != 0 {
                    (0, m)
                } else if !arith {
                    if rv >= 128 {
                        (0, 0)
                    } else {
                        ((lv >> rv) & m, (lu >> rv) & m)
                    }
                } else {
                    let fv = (lv >> (w - 1)) & 1;
                    let fu = (lu >> (w - 1)) & 1;
                    let body = if rv >= 128 {
                        (0, 0)
                    } else {
                        ((lv >> rv) & m, (lu >> rv) & m)
                    };
                    let fill_n = rv.min(w as u64) as u32;
                    let fill_mask = if fill_n == 0 {
                        0
                    } else {
                        m & !wmask(w - fill_n)
                    };
                    (
                        (body.0 & !fill_mask) | (if fv == 1 { fill_mask } else { 0 }),
                        (body.1 & !fill_mask) | (if fu == 1 { fill_mask } else { 0 }),
                    )
                };
                wstack.push(res);
            }
            NOp::WDivMod { kind, w } => {
                let (bv, bu) = wstack.pop().expect("native wdivmod: missing rhs");
                let (av, au) = wstack.pop().expect("native wdivmod: missing lhs");
                let m = wmask(w);
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, m)
                } else {
                    let b = bv & m;
                    if b == 0 {
                        (0, m)
                    } else {
                        let a = av & m;
                        let r = match kind {
                            DivKind::Div => a / b,
                            DivKind::Mod => a % b,
                        };
                        (r & m, 0)
                    }
                };
                wstack.push(res);
            }
            NOp::WTernary {
                w,
                cond_wide,
                cond_w,
            } => {
                let (ev, eu) = wstack.pop().expect("native wternary: missing else");
                let (tv, tu) = wstack.pop().expect("native wternary: missing then");
                // truthiness fold is OR-equivalent across words, so the wide
                // cond uses the same rule on u128.
                let (c1, cx) = if cond_wide {
                    let (cv, cu) = wstack.pop().expect("native wternary: missing cond");
                    let mc = wmask(cond_w);
                    ((cv & !cu & mc) != 0, (cu & mc) != 0)
                } else {
                    let (cv, cu) = stack.pop().expect("native wternary: missing cond");
                    let mc = low_mask(cond_w);
                    ((cv & !cu & mc) != 0, (cu & mc) != 0)
                };
                let m = wmask(w);
                let res = if c1 {
                    (tv & m, tu & m)
                } else if cx {
                    let ident = !((tv ^ ev) | (tu ^ eu)) & m;
                    ((tv & ident), (tu & ident) | (m & !ident))
                } else {
                    (ev & m, eu & m)
                };
                wstack.push(res);
            }
            NOp::WReduce { kind, neg, opw } => {
                let (av, au) = wstack.pop().expect("native wreduce: missing operand");
                let m = wmask(opw);
                let known1 = av & !au & m;
                let known0 = !au & !av & m;
                let unk = au & m;
                let (v, u): (u64, u64) = match kind {
                    RedK::And if known0 != 0 => (0, 0),
                    RedK::And if unk != 0 => (0, 1),
                    RedK::And => (1, 0),
                    RedK::Or if known1 != 0 => (1, 0),
                    RedK::Or if unk != 0 => (0, 1),
                    RedK::Or => (0, 0),
                    RedK::Xor if unk != 0 => (0, 1),
                    RedK::Xor => ((known1.count_ones() & 1) as u64, 0),
                };
                let out = if neg && u == 0 { (v ^ 1, 0) } else { (v, u) };
                stack.push(out);
            }
            // ── v6 ④ wide structural trio ──
            NOp::WSelect {
                kind,
                sel_w,
                src_w,
                base_wide,
                out_wide,
            } => {
                let (off_v, off_u) = stack.pop().expect("native wselect: missing offset");
                let (sv, su): (u128, u128) = if base_wide {
                    wstack.pop().expect("native wselect: missing base")
                } else {
                    let (v, u) = stack.pop().expect("native wselect: missing base");
                    (v as u128, u as u128)
                };
                let m = wmask(sel_w);
                let res: (u128, u128) = match (off_u == 0)
                    .then_some(off_v)
                    .and_then(|v| i64::try_from(v).ok())
                {
                    None => (0, m), // X/Z offset → sel_w X bits (oracle)
                    Some(off) => {
                        let lsb = match kind {
                            SelKind::Bit | SelKind::PartConst | SelKind::PartIdxUp => off,
                            SelKind::PartIdxDown => off - (sel_w as i64) + 1,
                        };
                        // fully in-range: one two-word shift (oracle fast path)
                        if lsb >= 0 && (lsb as u64) + sel_w as u64 <= src_w as u64 {
                            (((sv >> lsb) & m), ((su >> lsb) & m))
                        } else {
                            let (mut rv, mut ru) = (0u128, 0u128);
                            for i in 0..sel_w as i64 {
                                let si = lsb + i;
                                if si >= 0 && (si as u32) < src_w {
                                    rv |= ((sv >> si) & 1) << i;
                                    ru |= ((su >> si) & 1) << i;
                                } else {
                                    ru |= 1 << i; // out-of-range read → X
                                }
                            }
                            (rv, ru)
                        }
                    }
                };
                if out_wide {
                    wstack.push(res);
                } else {
                    stack.push((res.0 as u64, res.1 as u64));
                }
            }
            NOp::WConcatPair {
                lo_w,
                w,
                acc_wide,
                part_wide,
            } => {
                let (lo_v, lo_u): (u128, u128) = if part_wide {
                    wstack.pop().expect("native wconcat: missing lo")
                } else {
                    let (v, u) = stack.pop().expect("native wconcat: missing lo");
                    (v as u128, u as u128)
                };
                let (hi_v, hi_u): (u128, u128) = if acc_wide {
                    wstack.pop().expect("native wconcat: missing hi")
                } else {
                    let (v, u) = stack.pop().expect("native wconcat: missing hi");
                    (v as u128, u as u128)
                };
                // lo_w ≤ 127 here: tot = acc_w + lo_w ≤ 128 with acc_w ≥ 1.
                let m = wmask(w);
                wstack.push((((hi_v << lo_w) | lo_v) & m, ((hi_u << lo_w) | lo_u) & m));
            }
            NOp::WRepl { part_w, count, w } => {
                let (pv, pu) = stack.pop().expect("native wrepl: missing part");
                let (pv, pu) = (pv as u128, pu as u128);
                let (mut rv, mut ru) = (0u128, 0u128);
                for c in 0..count {
                    let sh = c * part_w; // (count-1)·part_w < w ≤ 128
                    rv |= pv << sh;
                    ru |= pu << sh;
                }
                let m = wmask(w);
                wstack.push((rv & m, ru & m));
            }
            NOp::WLogNot { opw } => {
                let (av, au) = wstack.pop().expect("native wlognot: missing operand");
                let m = wmask(opw);
                let res = if (av & !au & m) != 0 {
                    (0, 0)
                } else if (au & m) != 0 {
                    (0, 1)
                } else {
                    (1, 0)
                };
                stack.push(res);
            }
        }
        #[cfg(debug_assertions)]
        {
            let (np, npu, wp, wpu) = arity(op);
            debug_assert_eq!(
                *stack.sp as i64,
                sp_dbg as i64 - np as i64 + npu as i64,
                "VM-ARITY-ASSERT: narrow-stack drift (arity npop={np} npush={npu})"
            );
            debug_assert_eq!(
                *wstack.sp as i64,
                wsp_dbg as i64 - wp as i64 + wpu as i64,
                "VM-ARITY-ASSERT: wide-stack drift (arity wpop={wp} wpush={wpu})"
            );
        }
    }
    let mut out = Value::zeros(prog.root_w, prog.root_signed);
    if prog.root_w > 64 {
        let (fv, fu) = wstack.pop().expect("native eval produced no wide result");
        out.val[0] = fv as u64;
        out.val[1] = (fv >> 64) as u64;
        out.unk[0] = fu as u64;
        out.unk[1] = (fu >> 64) as u64;
    } else {
        let (fv, fu) = stack.pop().expect("native eval produced no result");
        out.val[0] = fv;
        out.unk[0] = fu;
    }
    out
}

/// The oracle's array-word index conversion: `to_u64` (None on X/Z) then
/// `u32::try_from`, both failures mapping to the `u32::MAX` OOR sentinel.
#[inline]
fn word_index(iv: u64, iu: u64) -> u32 {
    if iu != 0 {
        u32::MAX
    } else {
        u32::try_from(iv).unwrap_or(u32::MAX)
    }
}

/// Minimal push/pop facade over a fixed buffer so the op arms read identically
/// to the previous `Vec` form (instantiated per stack: narrow u64 / wide u128).
struct FixedStack<'a, T: Copy> {
    buf: &'a mut [T],
    sp: &'a mut usize,
}
impl<T: Copy> FixedStack<'_, T> {
    #[inline]
    fn push(&mut self, v: T) {
        self.buf[*self.sp] = v;
        *self.sp += 1;
    }
    #[inline]
    fn pop(&mut self) -> Option<T> {
        if *self.sp == 0 {
            return None;
        }
        *self.sp -= 1;
        Some(self.buf[*self.sp])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_ir::SimIr;

    /// A `NetReader` returning a fixed `Value` per NetId (and all-X for any other).
    struct FakeNets(Vec<Value>);
    impl NetReader for FakeNets {
        fn read_net(&self, net: u32, _word: Option<u32>) -> Value {
            self.0
                .get(net as usize)
                .cloned()
                .unwrap_or_else(|| Value::xs(1, false))
        }
    }

    fn nv(width: u32, signed: bool) -> sim_ir::NetVar {
        sim_ir::NetVar {
            kind: sim_ir::NetKind::Reg,
            width,
            msb: width.saturating_sub(1),
            lsb: 0,
            signed,
            array_len: 1,
            dir: sim_ir::PortDir::Internal,
            init: sim_ir::BitPacked {
                val: vec![0],
                unk: vec![0],
            },
        }
    }

    /// Minimal `SimIr` carrying only the arenas width inference + native eval read:
    /// `exprs`, `consts`, `nets`. Everything else is empty.
    fn ir_of(exprs: Vec<Expr>, consts: Vec<sim_ir::ConstVal>, nets: Vec<sim_ir::NetVar>) -> SimIr {
        SimIr {
            instances: vec![],
            nets,
            processes: vec![],
            cont_assigns: vec![],
            funcs: vec![],
            exprs,
            stmts: vec![],
            blocks: vec![],
            consts,
        }
    }

    /// Cross-check native `try_compile + run` against the interpreter oracle
    /// `EvalCtx::eval_ctx` for `eid` in context `(ctx_w, ctx_signed)` over a set of
    /// net `Value`s. Asserts the produced `Value`s are byte-identical (val, unk,
    /// width, signed) — the same equality the P5 gate enforces end-to-end.
    fn assert_matches_oracle(ir: &SimIr, eid: u32, ctx_w: u32, ctx_signed: bool, nets: &[Value]) {
        assert_matches_oracle_on(ir, eid, ctx_w, ctx_signed, &FakeNets(nets.to_vec()));
    }

    /// Generic core: same byte-identity contrast over ANY `NetReader` (the
    /// array-indexed lane needs word-indexed fakes).
    fn assert_matches_oracle_on(
        ir: &SimIr,
        eid: u32,
        ctx_w: u32,
        ctx_signed: bool,
        fake: &impl NetReader,
    ) {
        let wt = WidthTable::build(ir, &crate::FuncTable::new());
        let oracle = {
            let rng = crate::state::RngCells::default();
            let ctx = crate::eval::EvalCtx {
                ir,
                nets: fake,
                now: 0,
                wt: &wt,
                time_mult: 1,
                rng: &rng,
                plusargs: &[],
            };
            ctx.eval_ctx(eid, ctx_w, ctx_signed)
        };
        let prog = try_compile(ir, &wt, eid, ctx_w, ctx_signed)
            .expect("expression must be native-compilable in this test");
        let native = run(&prog, fake);
        assert_eq!(
            native, oracle,
            "native eval diverged from oracle for eid {eid} ctx ({ctx_w},{ctx_signed})"
        );
    }

    fn sig(net: u32) -> Expr {
        Expr::Signal { net, word: None }
    }
    fn bin(op: BinOp, lhs: u32, rhs: u32) -> Expr {
        Expr::Binary { op, lhs, rhs }
    }

    // ── a 64-bit clean Value with given low word ──
    fn v64(x: u64) -> Value {
        let mut v = Value::zeros(64, false);
        v.val[0] = x;
        v
    }
    // ── an X/Z-bearing Value: `xmask` marks unknown bits ──
    fn v64_xz(val: u64, xmask: u64) -> Value {
        let mut v = Value::zeros(64, false);
        v.val[0] = val & !xmask;
        v.unk[0] = xmask;
        v
    }

    #[test]
    fn add_two_signals_matches_oracle_incl_wrap() {
        // exprs: 0=sig(0), 1=sig(1), 2 = 0 + 1
        let ir = ir_of(
            vec![sig(0), sig(1), bin(BinOp::Add, 0, 1)],
            vec![],
            vec![nv(64, false), nv(64, false)],
        );
        for (a, b) in [(7u64, 35u64), (u64::MAX, 1), (0, 0), (1 << 63, 1 << 63)] {
            assert_matches_oracle(&ir, 2, 64, false, &[v64(a), v64(b)]);
        }
    }

    #[test]
    fn arith_xz_operand_poisons_whole_result() {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(BinOp::Add, 0, 1)],
            vec![],
            vec![nv(64, false), nv(64, false)],
        );
        // one X bit anywhere in an operand ⇒ the oracle poisons the whole add to X.
        assert_matches_oracle(&ir, 2, 64, false, &[v64_xz(5, 0b10), v64(9)]);
        assert_matches_oracle(&ir, 2, 64, false, &[v64(9), v64_xz(0, 1 << 40)]);
    }

    #[test]
    fn sub_and_mul_match_oracle() {
        for op in [BinOp::Sub, BinOp::Mul] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(32, false), nv(32, false)],
            );
            for (a, b) in [(3u64, 9u64), (0xFFFF_FFFF, 2), (123456, 654321)] {
                assert_matches_oracle(&ir, 2, 32, false, &[v64(a), v64(b)]);
            }
        }
    }

    #[test]
    fn bitwise_4state_matches_oracle() {
        for op in [BinOp::BitAnd, BinOp::BitOr, BinOp::BitXor, BinOp::BitXnor] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(16, false), nv(16, false)],
            );
            // include X/Z so the 4-state truth tables (not just 2-state) are exercised.
            let a = v64_xz(0b1010_0101, 0b0000_1100);
            let b = v64_xz(0b1100_0011, 0b0011_0000);
            assert_matches_oracle(&ir, 2, 16, false, &[a, b]);
        }
    }

    // POW-LANE: a small positive const exponent lowers to a native Mul chain;
    // for every (w, n) with w*n <= 128 the result is byte-identical to the oracle
    // (clean values AND X-poison), so backend selection stays transparent.
    #[test]
    fn pow_const_small_exponent_matches_oracle() {
        // (ctx width, exponent) pairs: n>=2 and w*n <= 128.
        for (w, n) in [
            (16u32, 2u64),
            (16, 3),
            (16, 4),
            (16, 8),
            (8, 16),
            (64, 2),
            (32, 4),
        ] {
            let ir = ir_of(
                vec![sig(0), Expr::Const { val: 0 }, bin(BinOp::Pow, 0, 1)],
                vec![cnum(8, n)],
                vec![nv(w, false)],
            );
            for a in [0u64, 1, 2, 3, 7, 255, 0x1234, 0xFFFF] {
                assert_matches_oracle(&ir, 2, w, false, &[v64(a)]);
            }
            // X anywhere in the base must X-poison exactly like the oracle's Pow.
            assert_matches_oracle(&ir, 2, w, false, &[v64_xz(0x12, 0x0F)]);
        }
    }

    // POW-LANE bail boundary: exponent 0 / over POW_MAX / w*n>128 / non-const must
    // NOT compile natively (they stay oracle-bound), or values would diverge from
    // the oracle's u128-`checked_pow().unwrap_or(0)` overflow quirk.
    #[test]
    fn pow_uncompilable_cases_bail_to_oracle() {
        let wt_none = crate::FuncTable::new();
        let compiles = |consts: Vec<sim_ir::ConstVal>, rhs: Expr, w: u32, signed: bool| {
            // two nets so a non-const `rhs = sig(1)` resolves (net 1 unused otherwise).
            let ir = ir_of(
                vec![sig(0), rhs, bin(BinOp::Pow, 0, 1)],
                consts,
                vec![nv(w, signed), nv(w, false)],
            );
            let wt = WidthTable::build(&ir, &wt_none);
            try_compile(&ir, &wt, 2, w, signed).is_some()
        };
        // a ** 0 : oracle X-poisons an X base, but a Const-1 would say 1.
        assert!(
            !compiles(vec![cnum(8, 0)], Expr::Const { val: 0 }, 16, false),
            "a**0 must bail"
        );
        // a ** 1 : native passthrough keeps an X base, but the oracle X-poisons it.
        assert!(
            !compiles(vec![cnum(8, 1)], Expr::Const { val: 0 }, 16, false),
            "a**1 must bail"
        );
        // a ** 17 : over POW_MAX.
        assert!(
            !compiles(vec![cnum(8, 17)], Expr::Const { val: 0 }, 16, false),
            "a**17 must bail"
        );
        // a ** 4 at w=64 : w*n = 256 > 128 (oracle overflow-to-0 quirk reachable).
        assert!(
            !compiles(vec![cnum(8, 4)], Expr::Const { val: 0 }, 64, false),
            "wide w*n>128 must bail"
        );
        // a ** b (non-const exponent).
        assert!(
            !compiles(vec![], sig(1), 16, false),
            "a**b (non-const) must bail"
        );
        // signed Pow uses ipow_signed — bail.
        assert!(
            !compiles(vec![cnum(8, 2)], Expr::Const { val: 0 }, 16, true),
            "signed a**2 must bail"
        );
    }

    #[test]
    fn bitnot_and_negate_match_oracle() {
        // ~sig0
        let ir_not = ir_of(
            vec![
                sig(0),
                Expr::Unary {
                    op: UnOp::BitNot,
                    operand: 0,
                },
            ],
            vec![],
            vec![nv(8, false)],
        );
        assert_matches_oracle(&ir_not, 1, 8, false, &[v64(0b1011_0010)]);
        assert_matches_oracle(&ir_not, 1, 8, false, &[v64_xz(0b1011_0010, 0b0000_1111)]);

        // -sig0 (two's complement); X/Z poisons.
        let ir_neg = ir_of(
            vec![
                sig(0),
                Expr::Unary {
                    op: UnOp::Minus,
                    operand: 0,
                },
            ],
            vec![],
            vec![nv(8, true)],
        );
        assert_matches_oracle(&ir_neg, 1, 8, true, &[v64(5)]);
        assert_matches_oracle(&ir_neg, 1, 8, true, &[v64_xz(5, 0b10)]);
    }

    #[test]
    fn chained_adds_match_oracle() {
        // (((s0 + s1) + s2) + s3) — the EXPR_HEAVY shape, all 64-bit.
        let ir = ir_of(
            vec![
                sig(0),
                sig(1),
                bin(BinOp::Add, 0, 1),
                sig(2),
                bin(BinOp::Add, 2, 3),
                sig(3),
                bin(BinOp::Add, 4, 5),
            ],
            vec![],
            vec![nv(64, false), nv(64, false), nv(64, false), nv(64, false)],
        );
        assert_matches_oracle(&ir, 6, 64, false, &[v64(11), v64(22), v64(33), v64(44)]);
    }

    #[test]
    fn over_128_bits_is_not_native_compilable() {
        // beyond the two-word wide lane (>128) the whole tree must bail (None)
        // → the VM keeps interpreting it. (65..=128 now compiles — C6 lane.)
        let ir = ir_of(
            vec![sig(0), sig(1), bin(BinOp::Add, 0, 1)],
            vec![],
            vec![nv(200, false), nv(200, false)],
        );
        let wt = WidthTable::build(&ir, &crate::FuncTable::new());
        assert!(try_compile(&ir, &wt, 2, 200, false).is_none());
    }

    #[test]
    fn unsupported_operator_bails() {
        // SysFunc stays outside the subset → None (Concat/Select/Replicate
        // joined in the structural increment).
        let ir = ir_of(
            vec![
                sig(0),
                Expr::SysFunc {
                    which: sim_ir::SysFuncId::Time,
                    args: vec![],
                },
                bin(BinOp::Add, 0, 1),
            ],
            vec![],
            vec![nv(32, false)],
        );
        let wt = WidthTable::build(&ir, &crate::FuncTable::new());
        assert!(try_compile(&ir, &wt, 2, 64, false).is_none());
    }

    // ── follow-on increment: comparisons / shifts / div-mod / ternary /
    //    reductions / logical (REMAINING_WORK "native-eval follow-on") ──

    fn un(op: UnOp, operand: u32) -> Expr {
        Expr::Unary { op, operand }
    }
    fn vw(w: u32, x: u64) -> Value {
        let mut v = Value::zeros(w, false);
        v.val[0] = x & low_mask(w);
        v
    }
    fn vws(w: u32, x: u64) -> Value {
        let mut v = Value::zeros(w, true);
        v.val[0] = x & low_mask(w);
        v
    }
    fn vw_xz(w: u32, x: u64, xm: u64) -> Value {
        let mut v = Value::zeros(w, false);
        let m = low_mask(w);
        v.val[0] = x & !xm & m;
        v.unk[0] = xm & m;
        v
    }

    #[test]
    fn relational_signed_pair_matches_oracle() {
        for op in [BinOp::Lt, BinOp::Le, BinOp::Gt, BinOp::Ge] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(8, true), nv(8, true)],
            );
            // -8 vs 3, 3 vs -8, equal, MIN vs MAX — signed ordering.
            for (a, b) in [(0xF8u64, 3u64), (3, 0xF8), (5, 5), (0x80, 0x7F)] {
                assert_matches_oracle(&ir, 2, 32, false, &[vws(8, a), vws(8, b)]);
            }
            // any X/Z → 1-bit X (zero-extended into the context).
            assert_matches_oracle(&ir, 2, 32, false, &[vw_xz(8, 5, 0b100), vws(8, 9)]);
        }
    }

    #[test]
    fn relational_mixed_sign_is_unsigned_compare() {
        // one unsigned operand ⇒ pair compares UNSIGNED (0xF8 > 3, not -8 < 3).
        let ir = ir_of(
            vec![sig(0), sig(1), bin(BinOp::Lt, 0, 1)],
            vec![],
            vec![nv(8, true), nv(8, false)],
        );
        assert_matches_oracle(&ir, 2, 8, false, &[vws(8, 0xF8), vw(8, 3)]);
        assert_matches_oracle(&ir, 2, 8, false, &[vws(8, 3), vw(8, 0xF8)]);
    }

    #[test]
    fn equality_and_case_equality_match_oracle() {
        for op in [BinOp::Eq, BinOp::Ne] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(8, false), nv(8, false)],
            );
            assert_matches_oracle(&ir, 2, 8, false, &[vw(8, 0xAB), vw(8, 0xAB)]);
            assert_matches_oracle(&ir, 2, 8, false, &[vw(8, 0xAB), vw(8, 0xAC)]);
            // any X in either ⇒ X result for ==/!=.
            assert_matches_oracle(&ir, 2, 8, false, &[vw_xz(8, 0xAB, 1), vw(8, 0xAB)]);
        }
        for op in [BinOp::CaseEq, BinOp::CaseNe] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(8, false), nv(8, false)],
            );
            // === compares X positions literally: matching X ⇒ equal, never X.
            assert_matches_oracle(
                &ir,
                2,
                8,
                false,
                &[vw_xz(8, 0xA0, 0xF), vw_xz(8, 0xA0, 0xF)],
            );
            assert_matches_oracle(&ir, 2, 8, false, &[vw_xz(8, 0xA0, 0xF), vw(8, 0xA0)]);
        }
    }

    #[test]
    fn shifts_match_oracle() {
        for op in [BinOp::Shl, BinOp::Shr, BinOp::AShr] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(16, true), nv(8, false)],
            );
            for (x, amt) in [
                (0x8001u64, 0u64),
                (0x8001, 3),
                (0x8001, 15),
                (0x8001, 16), // == width: everything out / full sign fill
                (0x8001, 200),
                (0x7001, 4),
            ] {
                assert_matches_oracle(&ir, 2, 16, false, &[vws(16, x), vw(8, amt)]);
                // signed enclosing context too (AShr fill follows lhs OWN sign).
                assert_matches_oracle(&ir, 2, 16, true, &[vws(16, x), vw(8, amt)]);
            }
            // X/Z amount poisons; X MSB on AShr fills X.
            assert_matches_oracle(&ir, 2, 16, false, &[vws(16, 0x8001), vw_xz(8, 2, 1)]);
            assert_matches_oracle(&ir, 2, 16, false, &[vw_xz(16, 1, 1 << 15), vw(8, 3)]);
        }
    }

    #[test]
    fn div_mod_match_oracle() {
        for op in [BinOp::Div, BinOp::Mod] {
            // UNSIGNED
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(16, false), nv(16, false)],
            );
            for (a, b) in [(100u64, 7u64), (7, 100), (0xFFFF, 3), (5, 0)] {
                assert_matches_oracle(&ir, 2, 16, false, &[vw(16, a), vw(16, b)]);
            }
            assert_matches_oracle(&ir, 2, 16, false, &[vw_xz(16, 9, 1), vw(16, 3)]);
            // SIGNED: truncating toward zero (-7/2 = -3, -7%2 = -1).
            let irs = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(16, true), nv(16, true)],
            );
            for (a, b) in [(0xFFF9u64, 2u64), (7, 0xFFFE), (0xFFF9, 0xFFFE), (5, 0)] {
                assert_matches_oracle(&irs, 2, 16, true, &[vws(16, a), vws(16, b)]);
            }
        }
    }

    #[test]
    fn ternary_matches_oracle() {
        // exprs: 0=cond sig2, 1=sig0, 2=sig1, 3 = cond ? sig0 : sig1
        let ir = ir_of(
            vec![
                sig(2),
                sig(0),
                sig(1),
                Expr::Ternary {
                    cond: 0,
                    then_e: 1,
                    else_e: 2,
                },
            ],
            vec![],
            vec![nv(8, false), nv(8, false), nv(1, false)],
        );
        let (t, e) = (vw(8, 0xAA), vw(8, 0xAC));
        assert_matches_oracle(&ir, 3, 8, false, &[t.clone(), e.clone(), vw(1, 1)]);
        assert_matches_oracle(&ir, 3, 8, false, &[t.clone(), e.clone(), vw(1, 0)]);
        // X cond ⇒ bitwise merge: agreeing bits pass, differing become X.
        assert_matches_oracle(&ir, 3, 8, false, &[t.clone(), e.clone(), vw_xz(1, 0, 1)]);
        // merge where branches carry X themselves.
        assert_matches_oracle(
            &ir,
            3,
            8,
            false,
            &[vw_xz(8, 0xA0, 3), vw_xz(8, 0xA0, 3), vw_xz(1, 0, 1)],
        );
    }

    #[test]
    fn reductions_and_lognot_match_oracle() {
        for op in [
            UnOp::RedAnd,
            UnOp::RedNand,
            UnOp::RedOr,
            UnOp::RedNor,
            UnOp::RedXor,
            UnOp::RedXnor,
            UnOp::LogNot,
        ] {
            let ir = ir_of(vec![sig(0), un(op, 0)], vec![], vec![nv(8, false)]);
            for v in [
                vw(8, 0xFF),
                vw(8, 0x00),
                vw(8, 0b1010_0110),
                vw_xz(8, 0xFF, 0b1), // X with otherwise-all-1 (AND → X, OR → 1)
                vw_xz(8, 0x00, 0b1000), // X with otherwise-all-0 (OR → X, AND → 0)
            ] {
                assert_matches_oracle(&ir, 1, 8, false, &[v.clone()]);
            }
        }
    }

    #[test]
    fn logical_and_or_match_oracle() {
        for op in [BinOp::LogAnd, BinOp::LogOr] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(8, false), nv(8, false)],
            );
            for (a, b) in [
                (vw(8, 5), vw(8, 9)),             // T,T
                (vw(8, 0), vw(8, 9)),             // F,T
                (vw(8, 5), vw(8, 0)),             // T,F
                (vw(8, 0), vw(8, 0)),             // F,F
                (vw_xz(8, 0, 1), vw(8, 9)),       // X,T
                (vw_xz(8, 0, 1), vw(8, 0)),       // X,F
                (vw_xz(8, 0, 1), vw_xz(8, 0, 2)), // X,X
                (vw_xz(8, 2, 1), vw(8, 0)),       // definite-1 + X bit = TRUE, F
            ] {
                assert_matches_oracle(&ir, 2, 8, false, &[a.clone(), b.clone()]);
            }
        }
    }

    #[test]
    fn comparison_of_arith_results_matches_oracle() {
        // (s0 + s1) < (s2 * s3) — comparison over native sub-trees.
        let ir = ir_of(
            vec![
                sig(0),
                sig(1),
                bin(BinOp::Add, 0, 1),
                sig(2),
                sig(3),
                bin(BinOp::Mul, 3, 4),
                bin(BinOp::Lt, 2, 5),
            ],
            vec![],
            vec![nv(16, false); 4],
        );
        assert_matches_oracle(
            &ir,
            6,
            8,
            false,
            &[vw(16, 100), vw(16, 200), vw(16, 20), vw(16, 14)],
        );
        assert_matches_oracle(
            &ir,
            6,
            8,
            false,
            &[vw(16, 1000), vw(16, 2000), vw(16, 2), vw(16, 3)],
        );
    }

    // ── follow-on increment 2: select / concat / replicate (REMAINING_WORK
    //    "native-eval >64bit/real/select/concat lane" — the ≤64-bit half) ──

    fn cnum(w: u32, x: u64) -> sim_ir::ConstVal {
        sim_ir::ConstVal {
            width: w,
            signed: false,
            repr: sim_ir::ConstRepr::Numeric,
            bits: sim_ir::BitPacked {
                val: vec![x],
                unk: vec![0],
            },
        }
    }

    #[test]
    fn bit_select_matches_oracle() {
        // exprs: 0=sig0(16b), 1=Const#0 (offset), 2=Const#1 (width edge, =1),
        // 3 = sig0[off]. Sweep in-range bits (0/1/X at the picked bit) + OOR.
        for off in [0u64, 5, 15, 16, 200] {
            let ir = ir_of(
                vec![
                    sig(0),
                    Expr::Const { val: 0 },
                    Expr::Const { val: 1 },
                    Expr::Select {
                        base: 0,
                        offset: 1,
                        width: 2,
                        kind: SelKind::Bit,
                    },
                ],
                vec![cnum(32, off), cnum(32, 1)],
                vec![nv(16, false)],
            );
            assert_matches_oracle(
                &ir,
                3,
                8,
                false,
                &[vw_xz(16, 0b1010_0101_1100_0011, 0b10_0000)],
            );
        }
    }

    #[test]
    fn part_selects_match_oracle_incl_oor_and_xz_src() {
        // [11:4] as PartConst(off=4,w=8); s[4 +: 8]; s[11 -: 8]; plus a select
        // whose window hangs off the top (off=12 ⇒ upper bits OOR→X) and one off
        // the bottom (IdxDown off=3 ⇒ lsb=-4 ⇒ low bits OOR→X).
        for (kind, off) in [
            (SelKind::PartConst, 4u64),
            (SelKind::PartIdxUp, 4),
            (SelKind::PartIdxDown, 11),
            (SelKind::PartConst, 12),
            (SelKind::PartIdxDown, 3),
        ] {
            let ir = ir_of(
                vec![
                    sig(0),
                    Expr::Const { val: 0 },
                    Expr::Const { val: 1 },
                    Expr::Select {
                        base: 0,
                        offset: 1,
                        width: 2,
                        kind,
                    },
                ],
                vec![cnum(32, off), cnum(32, 8)],
                vec![nv(16, false)],
            );
            // ctx wider than the select (32) — proves the unsigned zero-extend;
            // ctx_signed=true proves a select stays unsigned in a signed context.
            assert_matches_oracle(&ir, 3, 32, true, &[vw_xz(16, 0xA5C3, 0x0420)]);
        }
    }

    #[test]
    fn dynamic_select_offset_from_net_matches_oracle() {
        // offset comes from a NET (exprs: 1=sig1) — in-range, OOR, and X-offset.
        let ir = ir_of(
            vec![
                sig(0),
                sig(1),
                Expr::Const { val: 0 },
                Expr::Select {
                    base: 0,
                    offset: 1,
                    width: 2,
                    kind: SelKind::PartIdxUp,
                },
            ],
            vec![cnum(32, 4)],
            vec![nv(16, false), nv(8, false)],
        );
        let src = vw_xz(16, 0xA5C3, 0x0420);
        assert_matches_oracle(&ir, 3, 16, false, &[src.clone(), vw(8, 6)]);
        assert_matches_oracle(&ir, 3, 16, false, &[src.clone(), vw(8, 14)]); // window OOR top
                                                                             // X/Z offset ⇒ the whole select reads X (zero-extended into ctx).
        assert_matches_oracle(&ir, 3, 16, false, &[src, vw_xz(8, 6, 0b1)]);
    }

    #[test]
    fn concat_matches_oracle() {
        // {sig0(8b), sig1(4b), sig2(4b)} = 16 natural bits; parts[0] is MSB-most.
        let ir = ir_of(
            vec![
                sig(0),
                sig(1),
                sig(2),
                Expr::Concat {
                    parts: vec![0, 1, 2],
                },
            ],
            vec![],
            vec![nv(8, false), nv(4, false), nv(4, false)],
        );
        // clean + X/Z-bearing parts; ctx 32 (zero-extend) and ctx_signed=true
        // (concat is unsigned regardless of context).
        assert_matches_oracle(&ir, 3, 32, true, &[vw(8, 0xAB), vw(4, 0x5), vw(4, 0xC)]);
        assert_matches_oracle(
            &ir,
            3,
            32,
            true,
            &[vw_xz(8, 0xAB, 0x0F), vw_xz(4, 0x5, 0b1000), vw(4, 0xC)],
        );
    }

    #[test]
    fn replicate_matches_oracle() {
        // {3{sig0(5b)}} = 15 natural bits, X bits repeat with the pattern.
        let ir = ir_of(
            vec![
                sig(0),
                Expr::Const { val: 0 },
                Expr::Replicate { count: 1, value: 0 },
            ],
            vec![cnum(32, 3)],
            vec![nv(5, false)],
        );
        assert_matches_oracle(&ir, 2, 16, false, &[vw(5, 0b10110)]);
        assert_matches_oracle(&ir, 2, 16, false, &[vw_xz(5, 0b10110, 0b00100)]);
    }

    #[test]
    fn select_of_concat_composes() {
        // {sig0, sig1}[6 +: 4] — structural ops compose inside one program.
        let ir = ir_of(
            vec![
                sig(0),
                sig(1),
                Expr::Concat { parts: vec![0, 1] },
                Expr::Const { val: 0 },
                Expr::Const { val: 1 },
                Expr::Select {
                    base: 2,
                    offset: 3,
                    width: 4,
                    kind: SelKind::PartIdxUp,
                },
            ],
            vec![cnum(32, 6), cnum(32, 4)],
            vec![nv(8, false), nv(8, false)],
        );
        assert_matches_oracle(&ir, 5, 8, false, &[vw(8, 0x3C), vw_xz(8, 0xF0, 0x0F)]);
    }

    // ── C6 lane: array-indexed Signal + the 65..=128-bit two-word wide lane ──

    /// Per-net fake honoring the array word index (mirrors `net_word_packed`'s
    /// OOR ⇒ all-X-at-element-width contract; the contrast only needs both
    /// sides to see the SAME reader).
    enum FakeNet {
        Scalar(Value),
        Array(Vec<Value>, u32), // elements, element width
    }
    struct FakeMem(Vec<FakeNet>);
    impl NetReader for FakeMem {
        fn read_net(&self, net: u32, word: Option<u32>) -> Value {
            match (&self.0[net as usize], word) {
                (FakeNet::Scalar(v), None) => v.clone(),
                (FakeNet::Array(els, ew), Some(i)) => els
                    .get(i as usize)
                    .cloned()
                    .unwrap_or_else(|| Value::xs(*ew, false)),
                (FakeNet::Scalar(v), Some(_)) => v.clone(),
                (FakeNet::Array(_, ew), None) => Value::xs(*ew, false),
            }
        }
    }

    /// 2-word Value builders for the wide lane.
    fn vwide(w: u32, lo: u64, hi: u64) -> Value {
        let mut v = Value::zeros(w, false);
        v.val[0] = lo;
        if v.val.len() > 1 {
            v.val[1] = hi;
        }
        v.mask_top();
        v
    }
    fn vwide_s(w: u32, lo: u64, hi: u64) -> Value {
        let mut v = vwide(w, lo, hi);
        v.signed = true;
        v
    }
    fn vwide_xz(w: u32, lo: u64, hi: u64, xlo: u64, xhi: u64) -> Value {
        let mut v = vwide(w, lo & !xlo, hi & !xhi);
        v.unk[0] = xlo;
        if v.unk.len() > 1 {
            v.unk[1] = xhi;
        }
        v.mask_top();
        v
    }
    /// Two-word numeric const.
    fn cnum2(w: u32, lo: u64, hi: u64) -> sim_ir::ConstVal {
        sim_ir::ConstVal {
            width: w,
            signed: false,
            repr: sim_ir::ConstRepr::Numeric,
            bits: sim_ir::BitPacked {
                val: vec![lo, hi],
                unk: vec![0, 0],
            },
        }
    }

    #[test]
    fn indexed_load_matches_oracle() {
        // mem[idx] where idx comes from a net: exprs 0=sig(1) (index), 1=mem read.
        let ir = ir_of(
            vec![
                sig(1),
                Expr::Signal {
                    net: 0,
                    word: Some(0),
                },
            ],
            vec![],
            vec![nv(8, false), nv(8, false)],
        );
        let mem = |idx: Value| {
            FakeMem(vec![
                FakeNet::Array(vec![vw(8, 0x11), vw(8, 0x22), vw_xz(8, 0x30, 0xF)], 8),
                FakeNet::Scalar(idx),
            ])
        };
        assert_matches_oracle_on(&ir, 1, 16, false, &mem(vw(8, 0))); // first
        assert_matches_oracle_on(&ir, 1, 16, false, &mem(vw(8, 2))); // X-bearing element
        assert_matches_oracle_on(&ir, 1, 16, false, &mem(vw(8, 200))); // OOR → all-X
        assert_matches_oracle_on(&ir, 1, 16, false, &mem(vw_xz(8, 1, 0b1))); // X idx → all-X
    }

    #[test]
    fn indexed_load_in_arith_matches_oracle() {
        // mem[i] + 1 — indexed read composing with the arith lane.
        let ir = ir_of(
            vec![
                sig(1),
                Expr::Signal {
                    net: 0,
                    word: Some(0),
                },
                Expr::Const { val: 0 },
                bin(BinOp::Add, 1, 2),
            ],
            vec![cnum(32, 1)],
            vec![nv(16, false), nv(4, false)],
        );
        let mem = FakeMem(vec![
            FakeNet::Array(vec![vw(16, 0xFFFF), vw(16, 7)], 16),
            FakeNet::Scalar(vw(4, 0)),
        ]);
        assert_matches_oracle_on(&ir, 3, 32, false, &mem);
    }

    #[test]
    fn wide_indexed_load_matches_oracle() {
        // a 100-bit element array read lands on the WIDE stack.
        let ir = ir_of(
            vec![
                sig(1),
                Expr::Signal {
                    net: 0,
                    word: Some(0),
                },
            ],
            vec![],
            vec![nv(100, false), nv(8, false)],
        );
        let mem = |idx: Value| {
            FakeMem(vec![
                FakeNet::Array(
                    vec![vwide(100, u64::MAX, 0xABC), vwide(100, 5, 1 << 35)],
                    100,
                ),
                FakeNet::Scalar(idx),
            ])
        };
        assert_matches_oracle_on(&ir, 1, 100, false, &mem(vw(8, 1)));
        assert_matches_oracle_on(&ir, 1, 100, false, &mem(vw(8, 99))); // OOR
        assert_matches_oracle_on(&ir, 1, 100, false, &mem(vw_xz(8, 0, 1))); // X idx
    }

    #[test]
    fn wide_arith_matches_oracle() {
        for op in [BinOp::Add, BinOp::Sub, BinOp::Mul] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(100, false), nv(100, false)],
            );
            for (a, b) in [
                (vwide(100, u64::MAX, 0), vwide(100, 1, 0)), // carry crosses word 0→1
                (vwide(100, 0, 1), vwide(100, 1, 0)),        // borrow crosses back
                (vwide(100, u64::MAX, 0xF_FFFF_FFFF), vwide(100, 3, 7)), // wrap at 100
                (vwide(100, 1 << 60, 0), vwide(100, 1 << 60, 0)), // mul carries past 63
            ] {
                assert_matches_oracle(&ir, 2, 100, false, &[a.clone(), b.clone()]);
            }
            // X anywhere (here: only in word 1) poisons the whole result.
            assert_matches_oracle(
                &ir,
                2,
                100,
                false,
                &[vwide_xz(100, 5, 0, 0, 1 << 10), vwide(100, 9, 0)],
            );
        }
    }

    #[test]
    fn wide_bitwise_not_neg_match_oracle() {
        for op in [BinOp::BitAnd, BinOp::BitOr, BinOp::BitXor, BinOp::BitXnor] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(96, false), nv(96, false)],
            );
            let a = vwide_xz(96, 0xA5A5, 0xFF00, 0x0F, 0x3);
            let b = vwide_xz(96, 0x5A5A, 0x00FF, 0xF0, 0xC);
            assert_matches_oracle(&ir, 2, 96, false, &[a, b]);
        }
        let ir_not = ir_of(
            vec![sig(0), un(UnOp::BitNot, 0)],
            vec![],
            vec![nv(96, false)],
        );
        assert_matches_oracle(&ir_not, 1, 96, false, &[vwide_xz(96, 0xA5, 0x10, 0xF, 0x1)]);
        let ir_neg = ir_of(
            vec![sig(0), un(UnOp::Minus, 0)],
            vec![],
            vec![nv(100, false)],
        );
        assert_matches_oracle(&ir_neg, 1, 100, false, &[vwide(100, 0, 1)]); // borrow chain
        assert_matches_oracle(&ir_neg, 1, 100, false, &[vwide(100, 5, 7)]);
        assert_matches_oracle(&ir_neg, 1, 100, false, &[vwide_xz(100, 5, 0, 1, 0)]);
        // X poison
    }

    #[test]
    fn wide_cmp_and_equality_match_oracle() {
        // signed 100-bit: sign bit is bit 99 (word-1 bit 35).
        let neg = vwide_s(100, 5, 1 << 35 | 3); // negative (bit 99 set)
        let pos = vwide_s(100, u64::MAX, 0x3_FFFF_FFFF); // large positive
        for op in [BinOp::Lt, BinOp::Le, BinOp::Gt, BinOp::Ge] {
            let irs = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(100, true), nv(100, true)],
            );
            assert_matches_oracle(&irs, 2, 8, false, &[neg.clone(), pos.clone()]);
            assert_matches_oracle(&irs, 2, 8, false, &[pos.clone(), neg.clone()]);
            assert_matches_oracle(&irs, 2, 8, false, &[neg.clone(), neg.clone()]);
            // unsigned pair: same bits compare the other way around.
            let iru = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(100, false), nv(100, false)],
            );
            let (a, b) = (
                vwide(100, 5, 1 << 35 | 3),
                vwide(100, u64::MAX, 0x3_FFFF_FFFF),
            );
            assert_matches_oracle(&iru, 2, 8, false, &[a.clone(), b.clone()]);
            // X in word 1 only → 1-bit X.
            assert_matches_oracle(&iru, 2, 8, false, &[vwide_xz(100, 0, 0, 0, 1), b]);
        }
        for op in [BinOp::Eq, BinOp::Ne, BinOp::CaseEq, BinOp::CaseNe] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(128, false), nv(128, false)],
            );
            let same = vwide(128, 0xDEAD_BEEF, 0xFEED_F00D);
            let diff_hi = vwide(128, 0xDEAD_BEEF, 0xFEED_F00E); // differs only in word 1
            assert_matches_oracle(&ir, 2, 8, false, &[same.clone(), same.clone()]);
            assert_matches_oracle(&ir, 2, 8, false, &[same.clone(), diff_hi]);
            // matching X positions: ==/!= → X, ===/!== → equal.
            let x1 = vwide_xz(128, 0xA0, 0xB0, 0xF, 0xF0);
            assert_matches_oracle(&ir, 2, 8, false, &[x1.clone(), x1.clone()]);
            assert_matches_oracle(&ir, 2, 8, false, &[x1, same]);
        }
    }

    #[test]
    fn wide_shifts_match_oracle() {
        for op in [BinOp::Shl, BinOp::Shr, BinOp::AShr] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(100, true), nv(8, false)],
            );
            let x = vwide_s(100, 0xDEAD_BEEF_CAFE_F00D, 1 << 35 | 0x123); // bit 99 set
            for amt in [0u64, 1, 37, 63, 64, 65, 99, 100, 127, 200] {
                assert_matches_oracle(&ir, 2, 100, false, &[x.clone(), vw(8, amt)]);
                assert_matches_oracle(&ir, 2, 100, true, &[x.clone(), vw(8, amt)]);
            }
            // X amount → all-X; X MSB on >>> fills X.
            assert_matches_oracle(&ir, 2, 100, false, &[x.clone(), vw_xz(8, 2, 1)]);
            assert_matches_oracle(
                &ir,
                2,
                100,
                false,
                &[vwide_xz(100, 1, 0, 0, 1 << 35), vw(8, 3)],
            );
        }
    }

    #[test]
    fn wide_divmod_match_oracle() {
        for op in [BinOp::Div, BinOp::Mod] {
            let ir = ir_of(
                vec![sig(0), sig(1), bin(op, 0, 1)],
                vec![],
                vec![nv(128, false), nv(128, false)],
            );
            let big = vwide(128, 0x1234_5678_9ABC_DEF0, 0xFFFF_0000_1111_2222);
            assert_matches_oracle(&ir, 2, 128, false, &[big.clone(), vwide(128, 7, 0)]);
            assert_matches_oracle(&ir, 2, 128, false, &[big.clone(), vwide(128, 0, 3)]);
            assert_matches_oracle(&ir, 2, 128, false, &[vwide(128, 7, 0), big.clone()]);
            assert_matches_oracle(&ir, 2, 128, false, &[big, vwide(128, 0, 0)]);
            // /0 → X
        }
    }

    #[test]
    fn wide_ternary_matches_oracle() {
        // 1-bit cond steering 100-bit branches (clean, X-cond merge).
        let ir = ir_of(
            vec![
                sig(2),
                sig(0),
                sig(1),
                Expr::Ternary {
                    cond: 0,
                    then_e: 1,
                    else_e: 2,
                },
            ],
            vec![],
            vec![nv(100, false), nv(100, false), nv(1, false)],
        );
        let (t, e) = (vwide(100, 0xAAAA, 0xA), vwide(100, 0xAAAC, 0xC));
        assert_matches_oracle(&ir, 3, 100, false, &[t.clone(), e.clone(), vw(1, 1)]);
        assert_matches_oracle(&ir, 3, 100, false, &[t.clone(), e.clone(), vw(1, 0)]);
        assert_matches_oracle(&ir, 3, 100, false, &[t, e, vw_xz(1, 0, 1)]);
        // WIDE cond whose only definite-1 lives in word 1.
        let ir_wc = ir_of(
            vec![
                sig(2),
                sig(0),
                sig(1),
                Expr::Ternary {
                    cond: 0,
                    then_e: 1,
                    else_e: 2,
                },
            ],
            vec![],
            vec![nv(100, false), nv(100, false), nv(100, false)],
        );
        for cond in [
            vwide(100, 0, 1 << 20),         // true via word 1
            vwide(100, 0, 0),               // false
            vwide_xz(100, 0, 0, 0, 1 << 5), // unknown via word 1
        ] {
            assert_matches_oracle(
                &ir_wc,
                3,
                100,
                false,
                &[vwide(100, 1, 2), vwide(100, 3, 4), cond],
            );
        }
    }

    #[test]
    fn wide_reductions_and_lognot_match_oracle() {
        for op in [
            UnOp::RedAnd,
            UnOp::RedNand,
            UnOp::RedOr,
            UnOp::RedNor,
            UnOp::RedXor,
            UnOp::RedXnor,
            UnOp::LogNot,
        ] {
            let ir = ir_of(vec![sig(0), un(op, 0)], vec![], vec![nv(128, false)]);
            for v in [
                vwide(128, u64::MAX, u64::MAX),     // all ones
                vwide(128, u64::MAX, u64::MAX - 1), // single 0 in word 1
                vwide(128, 0, 0),                   // all zeros
                vwide(128, 1, 1 << 40),             // parity across words
                vwide_xz(128, u64::MAX, 0, 0, 1),   // X only in word 1
                vwide_xz(128, 0, 0, 0, 1 << 63),    // X with otherwise-all-0
            ] {
                assert_matches_oracle(&ir, 1, 8, false, &[v.clone()]);
            }
        }
    }

    #[test]
    fn wide_const_matches_oracle() {
        // 128-bit const + 128-bit signal — WConst materialization.
        let ir = ir_of(
            vec![Expr::Const { val: 0 }, sig(0), bin(BinOp::Add, 0, 1)],
            vec![cnum2(128, u64::MAX, 0x7)],
            vec![nv(128, false)],
        );
        assert_matches_oracle(&ir, 2, 128, false, &[vwide(128, 1, 0)]);
    }

    #[test]
    fn wide_compare_feeds_narrow_context() {
        // (a & b) != 0 over 100 bits → 1-bit result zero-extended into 8-bit ctx.
        let ir = ir_of(
            vec![
                sig(0),
                sig(1),
                bin(BinOp::BitAnd, 0, 1),
                Expr::Const { val: 0 },
                bin(BinOp::Ne, 2, 3),
            ],
            vec![cnum2(100, 0, 0)],
            vec![nv(100, false), nv(100, false)],
        );
        assert_matches_oracle(
            &ir,
            4,
            8,
            false,
            &[vwide(100, 0, 1 << 30), vwide(100, 0, 1 << 30)],
        );
        assert_matches_oracle(
            &ir,
            4,
            8,
            false,
            &[vwide(100, 0, 1 << 30), vwide(100, 1, 0)],
        );
    }

    #[test]
    fn wide_lane_bails_outside_subset() {
        let wt_of = |ir: &SimIr| WidthTable::build(ir, &crate::FuncTable::new());
        // SIGNED >64-bit arith: the oracle X-poisons via a different route —
        // conservatively out of the native subset.
        let ir = ir_of(
            vec![sig(0), sig(1), bin(BinOp::Add, 0, 1)],
            vec![],
            vec![nv(100, true), nv(100, true)],
        );
        assert!(try_compile(&ir, &wt_of(&ir), 2, 100, true).is_none());
        // shift AMOUNT wider than 64 bits.
        let ir = ir_of(
            vec![sig(0), sig(1), bin(BinOp::Shl, 0, 1)],
            vec![],
            vec![nv(32, false), nv(100, false)],
        );
        assert!(try_compile(&ir, &wt_of(&ir), 2, 32, false).is_none());
        // select over a >128-bit source stays oracle-bound (the v6 ④ wide
        // structural trio runs to 128; beyond it the whole tree bails).
        let ir = ir_of(
            vec![
                sig(0),
                Expr::Const { val: 0 },
                Expr::Const { val: 1 },
                Expr::Select {
                    base: 0,
                    offset: 1,
                    width: 2,
                    kind: SelKind::PartConst,
                },
            ],
            vec![cnum(32, 4), cnum(32, 8)],
            vec![nv(200, false)],
        );
        assert!(try_compile(&ir, &wt_of(&ir), 3, 8, false).is_none());
        // select OFFSET wider than 64 bits (base narrow).
        let ir = ir_of(
            vec![
                sig(0),
                sig(1),
                Expr::Const { val: 0 },
                Expr::Select {
                    base: 0,
                    offset: 1,
                    width: 2,
                    kind: SelKind::PartIdxUp,
                },
            ],
            vec![cnum(32, 4)],
            vec![nv(16, false), nv(100, false)],
        );
        assert!(try_compile(&ir, &wt_of(&ir), 3, 16, false).is_none());
        // logical &&/|| over a wide operand.
        let ir = ir_of(
            vec![sig(0), sig(1), bin(BinOp::LogAnd, 0, 1)],
            vec![],
            vec![nv(100, false), nv(8, false)],
        );
        assert!(try_compile(&ir, &wt_of(&ir), 2, 8, false).is_none());
        // narrow-result ternary steered by a WIDE cond.
        let ir = ir_of(
            vec![
                sig(2),
                sig(0),
                sig(1),
                Expr::Ternary {
                    cond: 0,
                    then_e: 1,
                    else_e: 2,
                },
            ],
            vec![],
            vec![nv(8, false), nv(8, false), nv(100, false)],
        );
        assert!(try_compile(&ir, &wt_of(&ir), 3, 8, false).is_none());
        // array index expr wider than 64 bits.
        let ir = ir_of(
            vec![
                sig(1),
                Expr::Signal {
                    net: 0,
                    word: Some(0),
                },
            ],
            vec![],
            vec![nv(8, false), nv(100, false)],
        );
        assert!(try_compile(&ir, &wt_of(&ir), 1, 8, false).is_none());
    }

    // ── v6 ④ wide structural trio (select/concat/replicate to 128 bits) ──

    #[test]
    fn wide_select_from_wide_base_matches_oracle() {
        // 100-bit base; windows crossing the word-0/1 boundary, hanging off the
        // top (OOR→X), and an X/Z offset — narrow result (sel_w 16).
        for off in [0u64, 56, 60, 90, 96, 200] {
            let ir = ir_of(
                vec![
                    sig(0),
                    Expr::Const { val: 0 },
                    Expr::Const { val: 1 },
                    Expr::Select {
                        base: 0,
                        offset: 1,
                        width: 2,
                        kind: SelKind::PartConst,
                    },
                ],
                vec![cnum(32, off), cnum(32, 16)],
                vec![nv(100, false)],
            );
            assert_matches_oracle(
                &ir,
                3,
                32,
                true,
                &[vwide_xz(
                    100,
                    0xA5C3_1234_DEAD_BEEF,
                    0x9_ABCD,
                    1 << 62,
                    0b1010,
                )],
            );
        }
    }

    #[test]
    fn wide_select_wide_result_matches_oracle() {
        // sel_w 100 from a 120-bit base (wide → wide) AND from a 32-bit base
        // (narrow base, wide result: everything beyond bit 31 reads X).
        for (base_w, src) in [
            (120u32, vwide_xz(120, 77, 0xFFFF_0000_0000_0001, 0, 1 << 50)),
            (32u32, vw_xz(32, 0xA5C3_0F0F, 0x10)),
        ] {
            let ir = ir_of(
                vec![
                    sig(0),
                    Expr::Const { val: 0 },
                    Expr::Const { val: 1 },
                    Expr::Select {
                        base: 0,
                        offset: 1,
                        width: 2,
                        kind: SelKind::PartConst,
                    },
                ],
                vec![cnum(32, 4), cnum(32, 100)],
                vec![nv(base_w, false)],
            );
            assert_matches_oracle(&ir, 3, 100, false, &[src]);
        }
    }

    #[test]
    fn wide_select_idxdown_negative_lsb_matches_oracle() {
        // s[3 -: 8] on a 100-bit base ⇒ lsb = −4 ⇒ low half OOR→X.
        let ir = ir_of(
            vec![
                sig(0),
                Expr::Const { val: 0 },
                Expr::Const { val: 1 },
                Expr::Select {
                    base: 0,
                    offset: 1,
                    width: 2,
                    kind: SelKind::PartIdxDown,
                },
            ],
            vec![cnum(32, 3), cnum(32, 8)],
            vec![nv(100, false)],
        );
        assert_matches_oracle(&ir, 3, 8, false, &[vwide(100, 0xCAFE, 0x3)]);
    }

    #[test]
    fn wide_concat_folds_match_oracle() {
        // {a(64), b(36)} = 100: the fold CROSSES 64 on the second part
        // (acc narrow + part narrow → wide).
        let ir = ir_of(
            vec![sig(0), sig(1), Expr::Concat { parts: vec![0, 1] }],
            vec![],
            vec![nv(64, false), nv(36, false)],
        );
        assert_matches_oracle(
            &ir,
            2,
            100,
            true,
            &[
                vw_xz(64, 0xDEAD_BEEF_A5C3_1234, 1 << 40),
                vw(36, 0xF_F00F_F00F),
            ],
        );
        // {w(100), n(20)} = 120: acc already wide + narrow part.
        let ir2 = ir_of(
            vec![sig(0), sig(1), Expr::Concat { parts: vec![0, 1] }],
            vec![],
            vec![nv(100, false), nv(20, false)],
        );
        assert_matches_oracle(
            &ir2,
            2,
            120,
            false,
            &[vwide_xz(100, 1, 0xF_0000_0001, 0, 1 << 35), vw(20, 0xABCDE)],
        );
        // {n(20), w(100)} = 120: narrow acc + WIDE part.
        let ir3 = ir_of(
            vec![sig(0), sig(1), Expr::Concat { parts: vec![1, 0] }],
            vec![],
            vec![nv(100, false), nv(20, false)],
        );
        assert_matches_oracle(
            &ir3,
            2,
            120,
            false,
            &[vwide(100, u64::MAX, 0x9_9999), vw_xz(20, 0x12345, 0b100)],
        );
    }

    #[test]
    fn wide_replicate_matches_oracle() {
        // {3{s(40)}} = 120 natural bits — X bits repeat with the pattern.
        let ir = ir_of(
            vec![sig(0), Expr::Replicate { count: 1, value: 0 }],
            vec![],
            vec![nv(40, false)],
        );
        // count edge is a const-expr edge: build via cnum like the width edges.
        let ir = {
            let mut ir = ir;
            ir.consts.push(cnum(32, 3));
            ir.exprs[1] = Expr::Replicate { count: 2, value: 0 };
            ir.exprs.push(Expr::Const { val: 0 });
            ir
        };
        assert_matches_oracle(&ir, 1, 128, false, &[vw_xz(40, 0xAB_CD12_3456, 0xF0)]);
    }
}
