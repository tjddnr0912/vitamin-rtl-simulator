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

// ---- split parts (mechanical refactor) ----
pub(crate) use exec_vm::*;
mod compile;
mod exec_vm;
pub(crate) use compile::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub(crate) enum ArithKind {
    Add,
    Sub,
    Mul,
}

#[derive(Clone, Copy)]
pub(crate) enum BitKind {
    And,
    Or,
    Xor,
    Xnor,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum CmpKind {
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy)]
pub(crate) enum DivKind {
    Div,
    Mod,
}

#[derive(Clone, Copy)]
pub(crate) enum RedK {
    And,
    Or,
    Xor,
}

/// One native eval instruction (post-order; operates on a value stack of single-word
/// 4-state pairs `(val, unk)`). `Plus` needs no opcode — its operand is simply
/// compiled at this node's `(w, eff_signed)` and left on the stack (passthrough).
#[derive(Clone, Copy)]
pub(crate) enum NOp {
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

/// `BinOp` → its native class, or `None` if unsupported (Div/Mod/Pow/compare/shift/
/// logical — each has subtler semantics, deferred to a later increment).
pub(crate) enum BinClass {
    Arith(ArithKind),
    Bit(BitKind),
}

/// POW-LANE: the largest exponent we expand into a native Mul chain.
const POW_MAX: u128 = 16;

/// Minimal push/pop facade over a fixed buffer so the op arms read identically
/// to the previous `Vec` form (instantiated per stack: narrow u64 / wide u128).
/// The two evaluation stacks, owned by the CALLER and reused across runs.
///
/// `run` used to declare both as local arrays. The narrow one is
/// `[(u64, u64); NATIVE_STACK]` = 1 KiB, zero-initialised on entry — and on a real
/// design `run` is called 6,509,189 times for programs averaging **4.2 ops**, whose peak
/// stack depth is 2–3. That is a kilobyte zeroed and a kilobyte of stack touched, per
/// four-instruction expression. Measured on picorv32 + testbench: shrinking the array
/// alone took the run from 0.71 s to 0.65 s.
///
/// Shrinking the array is not the fix — `NATIVE_STACK` is the DEPTH CAP that decides
/// which expressions `try_compile` accepts, so lowering it pushes deep expressions back
/// onto the interpreter. Hoisting the buffer out keeps the cap and pays its cost once.
/// Same move as VM-REGPOOL made for the VM's register and offset files.
pub(crate) struct NativeScratch {
    narrow: [(u64, u64); NATIVE_STACK],
    wide: [(u128, u128); WIDE_STACK],
}

impl Default for NativeScratch {
    fn default() -> Self {
        NativeScratch {
            narrow: [(0, 0); NATIVE_STACK],
            wide: [(0, 0); WIDE_STACK],
        }
    }
}

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
