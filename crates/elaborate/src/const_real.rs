//! Real-domain constant folding (`localparam real R = 2.0 + 3.0`).
//!
//! `param_real_value` folded a real LITERAL, and for a declared-`real` parameter
//! it fell back to the INTEGER const domain (so `parameter real R = 3;` binds
//! 3.0 and keeps its i64 twin). Real ARITHMETIC reached neither and the parameter
//! went loud — `2.0+3.0`, `*`, `/`, `-`, `**` all E3009 — even though vita's own
//! runtime computes them. `localparam real` is a common idiom, so this was a
//! visible hole rather than an exotic one.
//!
//! This evaluator stays in f64 from end to end. It never converts a real leaf to
//! an integer: doing that at the leaf was tried before and destroyed the value
//! before the enclosing operator could pick its domain (a real `R` compared with
//! `R > 2` took the wrong generate branch). Conversion belongs at the CONTEXT
//! boundary — which here is the parameter binding, where an exactly-integral
//! result additionally registers its i64 twin so the integral capabilities of
//! `parameter real R = 4;` are not lost.
//!
//! The reverse promotion IS correct and is done: an INTEGER operand inside a real
//! expression widens to f64 (§11.8.1 — an operation with any real operand is
//! evaluated in the real domain), so `10 / 4.0` is 2.5, not 2.

use super::*;

impl Elaborator<'_> {
    /// The TRUTH of a constant control expression (a generate-if condition, a
    /// generate-for condition). Folds in the integer domain first; if that
    /// declines and the expression mentions a real, folds in the real domain and
    /// converts to a truth value THERE.
    ///
    /// This is the context boundary for a real control expression: the comparison
    /// `R/2 > 2` is evaluated wholly in the real domain (§11.8.1) and only its
    /// 1-bit RESULT crosses into the integer world. Converting `R` to an integer
    /// first would decide the branch on the wrong value — the exact leaf-conversion
    /// mistake this domain exists to avoid.
    pub(crate) fn const_truth_in_scope(&self, e: &ast::Expr) -> Option<bool> {
        // A CONDITION is a self-determined position (§11.6.1 Table 11-21): nothing
        // around it supplies a width. `generate if (4'd15 + 4'd1)` therefore tests the
        // 4-bit 0 and takes the `else` — which is what iverilog AND verilator do, and
        // what the width-unlimited fold got wrong by elaborating the other branch,
        // silently, at exit 0.
        if let Some(v) = self.const_int_selfdet(e) {
            return Some(v != 0);
        }
        if !self.expr_mentions_real(e) {
            return None;
        }
        Some(self.const_eval_real_in_scope(e)? != 0.0)
    }

    /// Fold `e` in the REAL domain. `None` (⇒ the caller stays loud) for anything
    /// this domain cannot evaluate exactly: an unmodeled node, an unbound name, a
    /// division by zero, or a non-finite result.
    ///
    /// Callers must reach this only AFTER the integer domain has declined, so a
    /// wholly integral expression keeps its integer value and its exact binding —
    /// `parameter real R = 3/2;` stays 1.0 (integer division), matching iverilog,
    /// rather than becoming 1.5.
    pub(crate) fn const_eval_real_in_scope(&self, e: &ast::Expr) -> Option<f64> {
        use ast::ExprKind as K;
        let fin = |v: f64| v.is_finite().then_some(v);
        // §11.8.1: a real operator CONVERTS its integral operand, and the
        // conversion reads the integral subtree's SELF-DETERMINED value — the real
        // side gives it no width context. Recursing into the subtree with f64
        // arithmetic instead re-implemented integer arithmetic width-unlimited:
        // `1.0 + -4'sd8` folded 9.0 (the 4-bit negate wraps to −8 ⇒ −7.0, iverilog
        // agrees), `1.0 + 3/2` folded 2.5 (integer division ⇒ 2.0), and the `**`
        // exponent cell `2.0 ** -4'sd8` promoted −8 to +8. So a real-free subtree
        // folds in the INTEGER domain at its own width and only its RESULT crosses
        // into f64. Declining integral shapes stay loud here (falling back to the
        // f64 re-walk would revive exactly the widening this gate closes).
        // `expr_mentions_real` is the same conservative discriminator
        // `param_real_value` orders the two domains with.
        if !self.expr_mentions_real(e) {
            return self.const_int_selfdet(e).map(|v| v as f64);
        }
        match &e.kind {
            K::RealLit { raw, .. } => Some(parse_real_f64(raw)),
            // An integer literal inside a real expression promotes (§11.8.1).
            // (Reached only for a literal the gate above declined to claim — kept
            // for the day `expr_mentions_real` learns a form this arm models.)
            K::IntLit { .. } => const_eval_i64_lit(e).map(|v| v as f64),
            K::Paren { inner } => self.const_eval_real_in_scope(inner),
            K::Unary { op, operand } => {
                let v = self.const_eval_real_in_scope(operand)?;
                match op {
                    ast::UnOp::Plus => Some(v),
                    ast::UnOp::Minus => Some(-v),
                    // `!r` is defined (§11.4.7) but a real has no bit operators;
                    // keep the rest loud rather than inventing a bit pattern.
                    ast::UnOp::LogNot => Some((v == 0.0) as i64 as f64),
                    _ => None,
                }
            }
            // A single-segment name: the real parameter map first, then an integer
            // parameter promoted to f64 — the same precedence the lowering uses, so
            // the fold and the lowered value cannot pick different bindings.
            K::Ident(p) if p.segments.len() == 1 => {
                let n = &p.segments[0].name;
                self.walk_scopes(n, &self.real_param_val)
                    .or_else(|| self.lookup_scoped(n).map(|v| v as f64))
            }
            K::Binary { op, lhs, rhs } => {
                let a = self.const_eval_real_in_scope(lhs)?;
                let b = self.const_eval_real_in_scope(rhs)?;
                use ast::BinOp as B;
                match op {
                    B::Add => fin(a + b),
                    B::Sub => fin(a - b),
                    B::Mul => fin(a * b),
                    // A real division by zero is ±inf / NaN in IEEE-754, which is
                    // not a usable parameter value — stay loud.
                    B::Div if b != 0.0 => fin(a / b),
                    B::Pow => fin(a.powf(b)),
                    // Relational / equality operators yield a 1-bit INTEGER result,
                    // which the real domain carries as 0.0 / 1.0 so a ternary or a
                    // generate condition built from reals still folds.
                    B::Lt => Some((a < b) as i64 as f64),
                    B::Le => Some((a <= b) as i64 as f64),
                    B::Gt => Some((a > b) as i64 as f64),
                    B::Ge => Some((a >= b) as i64 as f64),
                    B::Eq | B::CaseEq => Some((a == b) as i64 as f64),
                    B::Ne | B::CaseNe => Some((a != b) as i64 as f64),
                    B::LogAnd => Some(((a != 0.0) && (b != 0.0)) as i64 as f64),
                    B::LogOr => Some(((a != 0.0) || (b != 0.0)) as i64 as f64),
                    // Bit-wise / shift / modulus are not defined on a real operand
                    // (§11.4) — loud, never a silently coerced bit pattern.
                    _ => None,
                }
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                // The condition is a truth value in a SELF-DETERMINED position
                // (§11.4.11) — integer first (the common `P > 2 ? …` spelling),
                // but at the condition's own width: the unlimited fold read
                // `(4'd15 + 4'd1) ? 1.5 : 2.5` as 16 ⇒ 1.5 where the 4-bit sum
                // wraps to 0 ⇒ 2.5 (iverilog agrees). A real-mentioning
                // condition declines in the integer walk and folds here.
                let c = match self.const_int_selfdet(cond) {
                    Some(v) => v != 0,
                    None => self.const_eval_real_in_scope(cond)? != 0.0,
                };
                if c {
                    self.const_eval_real_in_scope(then_e)
                } else {
                    self.const_eval_real_in_scope(else_e)
                }
            }
            _ => None,
        }
    }
}
