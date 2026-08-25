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

/// Convert a real constant to the integer domain AT A CONTEXT BOUNDARY (§6.24.1).
///
/// The conversion ROUNDS, and rounds a `.5` AWAY FROM ZERO: `2.5`→3, `3.5`→4,
/// `−2.5`→−3, `−0.5`→−1. Rust's `f64::round` is exactly that rule, which is why
/// this is a cast and not the `e + (e >= 0 ? 0.5 : −0.5)` construction the LOWERED
/// form has to use — that spelling is tie-to-EVEN for odd values in [2^52, 2^53)
/// and cost §4.5.365 a silent-wrong.
///
/// `$rtoi` TRUNCATES instead and has its own arm; the two are not interchangeable
/// (`$rtoi(2.9)` is 2 where `int'(2.9)` is 3 — measured on both oracles).
///
/// Declines a non-finite value and anything outside the i64 walk, so a caller that
/// cannot represent the result stays loud rather than wrapping.
pub(crate) fn real_round_to_i64(x: f64) -> Option<i64> {
    let r = x.round();
    // 2^63 exactly: `i64::MAX as f64` rounds UP to it, so comparing against the
    // cast bound would admit a value that overflows on the way back.
    const LIM: f64 = 9_223_372_036_854_775_808.0;
    (r.is_finite() && (-LIM..LIM).contains(&r)).then_some(r as i64)
}

impl Elaborator<'_> {
    /// Fold `e` in the REAL domain and hand back its INTEGER reading (§6.24.1).
    ///
    /// This is the context-boundary converter the real domain's header calls for.
    /// It is deliberately NOT reachable from a leaf: callers must be a position the
    /// language itself defines as integral — an `int'()`/`byte'()` cast, a `$clog2`
    /// argument, a replication count, or a parameter whose DECLARED type is
    /// integral. Registering an i64 twin at the leaf instead was tried in §4.5.232
    /// and opened five silent-wrongs, because it let the INTEGER domain answer an
    /// expression that mentions a real, and only the real domain applies §11.8.1's
    /// "any real operand ⇒ evaluate in the real domain" ordering: `generate if
    /// (R/2 > 2)` with R = 5.0 then took the ELSE branch. Here the whole expression
    /// is folded in the real domain first and only its RESULT crosses over.
    ///
    /// Guarded on `expr_mentions_real` so a wholly integral expression can never be
    /// re-folded through f64 — the integer domain owns that case, at its own width.
    pub(crate) fn const_int_via_real(&self, e: &ast::Expr) -> Option<i64> {
        if !self.expr_mentions_real(e) {
            return None;
        }
        real_round_to_i64(self.const_eval_real_in_scope(e)?)
    }

    /// `$rtoi(e)` in a const domain: §20.10 TRUNCATES toward zero, unlike the
    /// rounding a cast performs. Kept beside its rounding sibling so the pair
    /// cannot drift apart.
    pub(crate) fn const_rtoi_via_real(&self, e: &ast::Expr) -> Option<i64> {
        // ⚠️ Integer FIRST, and the order is load-bearing rather than stylistic. A
        // wholly integral argument is already its own truncation, and asking the real
        // domain for it routes an exact i64 through f64: `const_eval_real_in_scope`
        // promotes a real-free subtree with `as f64`, which above 2^53 is lossy.
        // Measured — `$rtoi(64'd9007199254740993)` came back 9007199254740992, and
        // since PRE had no `$rtoi` const arm at all that was a loud → silently
        // off-by-one. The real domain can only ADD answers here, never correct one.
        if !self.expr_mentions_real(e) {
            return self.const_int_selfdet(e);
        }
        real_round_to_i64(self.const_eval_real_in_scope(e)?.trunc())
    }
}
