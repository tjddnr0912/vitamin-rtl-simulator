//! A parameter override's own TYPE, read off the expression at the collector — what an
//! UNTYPED target takes from its final value (IEEE 1800-2017 §6.20.2).
//!
//! Two questions, each answered only where the node itself states the answer:
//!
//! - Is the RESULT real (§11.8.1)? [`Elaborator::override_real`] — then the target is a
//!   real parameter whatever its default was, and its value is the REAL fold of the
//!   expression (the i64 in `by_name` truncates a real sub-result).
//! - For an integral top the operator channel (`override_self_meta`) cannot size — a
//!   prim cast, a comparison, a logical or reduction operator — what is its
//!   `(width, signed)`? [`Elaborator::override_top_meta`] — the cast's type and Table
//!   11-21 answer it without looking at an operand's width or at any scope.
//!
//! Every other shape answers nothing and keeps the route its default picked. An
//! integral override whose width no rule states (a name whose own width is
//! value-inferred, `localparam N2 = 2.5 > 1`) is deliberately left there: moving it
//! off a real default types the value right and its width wrong, trading one
//! silent-wrong for another. So is a CALL: its declared return range belongs to the
//! function's declaring scope (a module, a generate block, `$unit`, a package), and
//! folding it here, at the call site, bound a shadowing constant's width in three
//! review rounds running (ROADMAP §2 "Real").

use super::*;

impl Elaborator<'_> {
    /// The override's value folded in the REAL domain, when its RESULT is real by
    /// construction — see [`ResolvedOverride::real`]. `None` when the result is not
    /// provably real, when the real fold declines, or when the expression names a
    /// constant an IMPORT bound: the import lane binds a package real as its i64 twin
    /// alone (ROADMAP §3.a ⑨), so such a name reads as an integer to every walk here
    /// and the answer would be half-real.
    pub(crate) fn override_real(&self, e: &ast::Expr) -> Option<f64> {
        if !self.expr_result_is_real(e)
            || crate::param_query::ast_any(e, &|x| self.names_imported_real(x))
        {
            return None;
        }
        self.const_eval_real_in_scope(e)
    }

    /// Is `e`'s RESULT real by construction (§11.8.1)? `+ - * / **` and unary `+`/`-`
    /// over a real operand, a ternary with a real ARM, `real'(…)`, the real-returning
    /// system functions, and a name bound to a real parameter (`bare_ident_route`, the
    /// resolver the lowering reads). A call is not claimed: the real fold cannot
    /// evaluate one, so the claim could only ever name a value it does not have.
    /// `_ => false`: every arm answering `true` here is one `rhs_has_real_domain` also
    /// answers `true`, so a shape this walk cannot see is simply not claimed.
    fn expr_result_is_real(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        let r = |x: &ast::Expr| self.expr_result_is_real(x);
        match &e.kind {
            K::RealLit { .. } => true,
            K::Paren { inner } => r(inner),
            K::MinTypMax { typ, .. } => r(typ),
            K::Unary {
                op: ast::UnOp::Plus | ast::UnOp::Minus,
                operand,
            } => r(operand),
            K::Binary {
                op:
                    ast::BinOp::Add
                    | ast::BinOp::Sub
                    | ast::BinOp::Mul
                    | ast::BinOp::Div
                    | ast::BinOp::Pow,
                lhs,
                rhs,
            } => r(lhs) || r(rhs),
            K::Ternary { then_e, else_e, .. } => r(then_e) || r(else_e),
            K::Cast {
                target: ast::CastTarget::Prim(ast::CastPrim::Real),
                ..
            } => true,
            K::SysCall { name, .. } => match systask::map_sysfunc(&name.name) {
                Some(w) => {
                    ir::realness::real_math_arity(w).is_some()
                        || matches!(
                            w,
                            ir::SysFuncId::Realtime
                                | ir::SysFuncId::Itor
                                | ir::SysFuncId::BitsToReal
                        )
                }
                None => false,
            },
            K::Ident(p) if p.segments.len() == 1 => matches!(
                self.bare_ident_route(&p.segments[0].name, e.span),
                BareIdentRoute::Real(_)
            ),
            K::PkgScoped { pkg, name } => self
                .pkg_real_val
                .get(&pkg.name)
                .is_some_and(|m| m.contains_key(&name.name)),
            _ => false,
        }
    }

    /// Is `x` a bare name some import of this scope could have bound from a package
    /// REAL? Conservative on purpose (a local declaration shadowing the import answers
    /// `true` too): its only use is to decline.
    fn names_imported_real(&self, x: &ast::Expr) -> bool {
        let ast::ExprKind::Ident(p) = &x.kind else {
            return false;
        };
        let [seg] = p.segments.as_slice() else {
            return false;
        };
        let n = seg.name.as_str();
        self.scope_imports.iter().any(|imp| {
            imp.item.as_ref().is_none_or(|i| i.name == n)
                && self
                    .pkg_real_val
                    .get(&imp.pkg.name)
                    .is_some_and(|m| m.contains_key(n))
        })
    }

    /// The operator channel's pair — `(width, signed)` and the value at it — for one
    /// override expression, computed once for every collector: `override_self_meta` /
    /// `override_self_value` first, and for a top that arm declines, the top's own
    /// type ([`Self::override_top_meta`]) with the constant value folded and converted
    /// to it. The two halves come from one arm or neither does.
    pub(crate) fn override_operator_channel(
        &self,
        e: &ast::Expr,
    ) -> (Option<(u32, bool)>, Option<i64>) {
        if let Some(m) = self.override_self_meta(e) {
            return (Some(m), self.override_self_value(e, m));
        }
        let Some((w, sg)) = Self::override_top_meta(e) else {
            return (None, None);
        };
        match self.const_eval_in_scope(e) {
            Some(v) => (
                Some((w, sg)),
                Some(crate::const_eval::coerce_i64_to_width(v, w, sg)),
            ),
            None => (None, None),
        }
    }

    /// The `(width, signed)` an integral override top states by itself, for the tops
    /// the operator channel's certified walk declines (its accept set needs every leaf
    /// width): a prim cast — its type (`byte'` 8 signed, `int'` 32 signed, `bit'` 1); a
    /// comparison, a logical operator, `!` or a reduction — one unsigned bit (Table
    /// 11-21); unary `+`, `-` and `~` over any of those — the operand's type. At most 64
    /// bits; everything else is `None`.
    ///
    /// Without it the untyped target took its DEFAULT's type: `#(.P(byte'(100)))` bound
    /// 32 bits where both oracles bind 8, `#(.P(X > 1))` 32 for 1, and onto `parameter R
    /// = 2.5` each of them stayed real (`R/2` 1.5 for 1).
    pub(crate) fn override_top_meta(e: &ast::Expr) -> Option<(u32, bool)> {
        use ast::ExprKind as K;
        let m = match &e.kind {
            K::Paren { inner } => return Self::override_top_meta(inner),
            K::Cast {
                target: ast::CastTarget::Prim(p),
                ..
            } => match p {
                ast::CastPrim::Int | ast::CastPrim::Integer => (32, true),
                ast::CastPrim::Byte => (8, true),
                ast::CastPrim::Shortint => (16, true),
                ast::CastPrim::Longint => (64, true),
                ast::CastPrim::Bit | ast::CastPrim::Logic | ast::CastPrim::Reg => (1, false),
                ast::CastPrim::Time => (64, false),
                ast::CastPrim::Real => return None,
            },
            K::Binary {
                op:
                    ast::BinOp::Lt
                    | ast::BinOp::Le
                    | ast::BinOp::Gt
                    | ast::BinOp::Ge
                    | ast::BinOp::Eq
                    | ast::BinOp::Ne
                    | ast::BinOp::CaseEq
                    | ast::BinOp::CaseNe
                    | ast::BinOp::WildEq
                    | ast::BinOp::WildNe
                    | ast::BinOp::InsideEq
                    | ast::BinOp::LogAnd
                    | ast::BinOp::LogOr,
                ..
            } => (1, false),
            K::Unary { op, operand } => match op {
                ast::UnOp::LogNot
                | ast::UnOp::RedAnd
                | ast::UnOp::RedNand
                | ast::UnOp::RedOr
                | ast::UnOp::RedNor
                | ast::UnOp::RedXor
                | ast::UnOp::RedXnor => (1, false),
                ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot => {
                    Self::override_top_meta(operand)?
                }
            },
            _ => return None,
        };
        (1..=64).contains(&m.0).then_some(m)
    }
}
