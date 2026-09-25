//! Single-mention IR builders for the cast and formal-bind lanes.
//!
//! The engine walks the expression DAG as a TREE, so every node that names an
//! operand evaluates it again. A user call runs its body (and its `$display`s)
//! once per mention, and `$random` draws once per mention. The builders here
//! name their operand exactly ONCE, so an operand that may not be repeated can
//! still be sign-extended or coerced to 2-state.

use super::*;

impl Elaborator<'_> {
    /// Sign-extend `e` to `n` bits, naming `e` once and preserving 4-state bits:
    /// `$signed(1'b1 ? $signed(e) : <n-bit signed 0>)`.
    ///
    /// The engine's `Ternary` arm evaluates the TAKEN branch at the wider arm's
    /// width, signed iff both arms are signed, so `$signed(e)` is sign-extended
    /// to `n` (an x or z MSB extends as x or z, like a plain signed assignment).
    /// The outer `$signed` seals the result against an enclosing context. `n` is
    /// the TARGET width. ⚠️ The caller must know that `e`'s own width is a
    /// DECLARED fact and narrower than `n`: the ternary evaluates at
    /// max(runtime width of `e`, `n`) and never cuts to `n`, so over a fabricated
    /// width it can answer wider than the cast (`40'(q48.sum())` gave
    /// `ffff800000000001` for `0000800000000001`). The unsigned twin needs no
    /// builder: `extend_with_fill` with a constant zero fill is already
    /// single-mention.
    pub(crate) fn extend_signed_once(&mut self, e: u32, n: u32) -> u32 {
        let cond = self.const_u32_expr(1, 1);
        let then_e = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Signed,
            args: vec![e],
        });
        let zero = self.intern_const(make_const_i64(0, n.max(1), true));
        let else_e = self.push_expr(ir::Expr::Const { val: zero });
        let t = self.push_expr(ir::Expr::Ternary {
            cond,
            then_e,
            else_e,
        });
        self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Signed,
            args: vec![t],
        })
    }

    /// 2-state coercion of `e` (IEEE §6.11.1): every x/z bit reads as 0, the
    /// width and sign are `e`'s own, and `e` is named once.
    pub(crate) fn two_state_once(&mut self, e: u32) -> u32 {
        self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::TwoState,
            args: vec![e],
        })
    }
}
