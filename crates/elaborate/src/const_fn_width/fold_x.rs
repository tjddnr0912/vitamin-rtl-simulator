//! The consumer side of the tri-state walk (§5.2 Q-XFALLBACK): the certificate, the two
//! opt-in refusals and the one cause clause.
//!
//! Two consumers opt in, and each can only REFUSE (turn a value into E3009), never produce
//! one: a parameter binder ([`Elaborator::eval_param_init`]) and a declaration's range bound
//! (`check_const_range_bound`). Q-AESINK's sinks call the same three pieces with their own text:
//! a `fold_*` entry, [`Elaborator::x_certified`] and [`Elaborator::x_reason`].

use super::fold::{ones, XKind};
use super::*;

impl Elaborator<'_> {
    /// May an x this walk proved in `e` be believed — is every width the walk masked at a
    /// FACT? Three certificates, each excluding a measured way the walk's width can be a guess:
    ///
    /// * c1 every node the const domain descends into has a known self width, and no name
    ///   resolves through an inline substitution stack;
    /// * c2 every name leaf has a DECLARED width (`declared_override_widths`: a value-inferred
    ///   width, an untyped alias such as `parameter PA = R;` and every call decline) and the
    ///   tree is evident under those widths (`ctx_width_names_are_evident`: no real / time
    ///   literal, cast to a type, call, `inside`, real-valued system call);
    /// * c3 no name in `e` is bound REAL — the binding, never the keyword: any `real_param_val`
    ///   / `pkg_real_val` entry (an exact integral twin included) or a bare name an import of
    ///   a package real could bind (conservative: a local `int PX` beside an imported real
    ///   `PX` is not certified).
    ///
    /// `envw` is the caller's certified local widths (a constant function's formals and body
    /// locals, Q-AESINK): a name it holds is certified by it and is never a real binding.
    /// Every caller in this row passes an empty map. ⚠️ `declared_override_widths` has no
    /// local arm, so a tree naming a local that is not also a declared parameter is not
    /// certified today.
    pub(crate) fn x_certified(&self, e: &ast::Expr, envw: &ConstWidths) -> bool {
        if !self.ast_selfwidths_all_known(e) || self.ast_mentions_substituted_name(e) {
            return false;
        }
        let Some(mut dw) = self.declared_override_widths(e) else {
            return false;
        };
        dw.extend(envw.iter().map(|(k, v)| (k.clone(), *v)));
        Self::ctx_width_names_are_evident(e, &dw) && !self.names_real_binding(e, envw)
    }

    /// c3's walk: every node (no const-fold filter — over-reporting only declines).
    fn names_real_binding(&self, e: &ast::Expr, envw: &ConstWidths) -> bool {
        let hit = match &e.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                let n = &p.segments[0].name;
                !envw.contains_key(n)
                    && (self.names_imported_real(e)
                        || self
                            .walk_scopes_key(n, |k| {
                                self.real_param_val.contains_key(k)
                                    || self.params.contains_key(k)
                                    || self.symbols.contains_key(k)
                            })
                            .is_some_and(|k| self.real_param_val.contains_key(&k)))
            }
            ast::ExprKind::PkgScoped { pkg, name } => self
                .pkg_real_val
                .get(&pkg.name)
                .is_some_and(|m| m.contains_key(&name.name)),
            _ => false,
        };
        let mut found = hit;
        e.for_each_child(|_, c| found = found || self.names_real_binding(c, envw));
        found
    }

    /// The binder's refusal: the width-aware walk's `f` (already truncated to the declared
    /// width `w`) has an x bit inside `w`, the width-UNLIMITED fallback `u` — the value the
    /// binder would otherwise bind — has a 1 under one of those x bits, and the tree is
    /// certified.
    ///
    /// ⚠️ The middle test is the 2-state rule (§6.11.2: a 2-state variable reads x as 0). The
    /// binder cannot tell `int` / `bit` / `shortint` from `logic` (the parser collapses the
    /// parameter type to `ParamType::Integer`), so a fallback with 0 under every x bit is KEPT,
    /// as before — `localparam int L = 16'd0 % ('1 + 1'b1);` is 0 in iverilog and sv2v. The
    /// test reads only the bits under x: the fallback's KNOWN bits may still be wrong
    /// (`(X & 32'hFFFF0000) | (('1 + 1'b1) >> 20)` keeps 4096 where an `int` holds 0 and a
    /// `logic [31:0]` holds `xxxx0000` — Q-XCORE), and on a 4-state target a kept value is
    /// silent (`logic [31:0]`: iverilog x) — Q-PARAM2STATE learns the target's state.
    pub(crate) fn binder_x_refusal(
        &self,
        e: &ast::Expr,
        f: CFold,
        w: u32,
        fallback: Option<i64>,
    ) -> Option<XWhy> {
        if !f.x_in(w) {
            return None;
        }
        let u = fallback?;
        if (u as u64) & f.x_bits() & ones(w) == 0 {
            return None;
        }
        if !self.x_certified(e, &ConstWidths::new()) {
            return None;
        }
        f.why()
    }

    /// The E3009 cause for a parameter the binder refused, or None when it did not refuse
    /// (the same decision, re-run at the error site).
    pub(crate) fn param_x_reason(
        &self,
        value: &ast::Expr,
        meta: Option<(u32, bool)>,
    ) -> Option<String> {
        let why = self.param_init_x(value, meta).err()?;
        Some(self.x_reason(why, value))
    }

    /// A declared range bound's router — the self-determined width-aware walk first, the
    /// width-unlimited walk where it declines or answers above `i32::MAX` (see
    /// `const_range_bound_fold_at`, which is this with the evidence dropped) — returning
    /// with the value the x evidence of the SAME half (`decl_split` keeps one pair or the
    /// other, never a value from one half and evidence from the other).
    ///
    /// The evidence is the first step's `Four` with an x bit, certified, and is computed
    /// only when `want_ev` (the value route pays nothing new).
    pub(crate) fn range_bound_fold_ev(
        &self,
        e: &ast::Expr,
        want_ev: bool,
    ) -> Option<(i64, Option<XWhy>)> {
        if self.ast_has_param_select(e) {
            return self.const_int_selfdet(e).map(|v| (v, None));
        }
        let first = self.fold_self(
            e,
            &std::collections::BTreeMap::new(),
            &ConstWidths::new(),
            0,
        );
        if let CFold::Val(v) = first {
            if v <= i64::from(i32::MAX) {
                return Some((v, None));
            }
        }
        let u = self.const_eval_in_scope(e)?;
        let why = if want_ev && first.x_bits() != 0 && self.x_certified(e, &ConstWidths::new()) {
            first.why()
        } else {
            None
        };
        Some((u, why))
    }

    /// The E3009 cause for a range bound that folded to a value while its own walk proves it
    /// `x`, or None. A bound holding no `/`, `%` or `**` outside a call cannot carry an
    /// origin, so it is not re-walked.
    pub(crate) fn bound_x_reason(&self, e: &ast::Expr) -> Option<String> {
        use ast::BinOp as B;
        let has_origin_op = crate::param_query::ast_any(e, &|n| {
            matches!(
                n.kind,
                ast::ExprKind::Binary {
                    op: B::Div | B::Mod | B::Pow,
                    ..
                }
            )
        });
        if !has_origin_op {
            return None;
        }
        let why = self.decl_split(|| self.range_bound_fold_ev(e, true))?.1?;
        Some(self.x_reason(why, e))
    }

    /// The one cause clause for an x this walk proved: which operand was 0, at which width,
    /// and the clause that makes it x. The operand is found under `root` by the origin's
    /// span; if it is not there the clause names the operator only.
    ///
    /// ⚠️ An origin is named only while it OWNS an x bit at the consumer's width
    /// (`why.own`, reduced by every operator and by the target's truncation on the way
    /// up): an absorbed, shifted-out or truncated origin did not make the x the consumer
    /// refused on, so the clause then names no origin (§5.2 Q-XFALLBACK lens C-W1).
    pub(crate) fn x_reason(&self, why: XWhy, root: &ast::Expr) -> String {
        let w = why.ctx_w;
        if why.own == 0 {
            return format!(
                "an operation in it is `x` in its {w}-bit context (a divisor that is 0, or a \
                 `**` base that is 0 under a negative exponent: IEEE 1800-2017 §11.4.3, Table \
                 11-4)"
            );
        }
        let operand = Self::origin_node(root, why.at).and_then(|n| match &n.kind {
            ast::ExprKind::Binary { lhs, rhs, .. } => Some(if why.kind == XKind::PowZeroNeg {
                lhs
            } else {
                rhs
            }),
            _ => None,
        });
        let op = match why.kind {
            XKind::DivZero => "/",
            XKind::ModZero => "%",
            XKind::PowZeroNeg => "**",
        };
        match (why.kind, operand) {
            (XKind::PowZeroNeg, Some(b)) => format!(
                "the base of the `**` ({}) is 0 in its {w}-bit context and the exponent is \
                 negative, so the `**` is `x` (IEEE 1800-2017 §11.4.3, Table 11-4)",
                Self::expr_brief(b)
            ),
            (XKind::PowZeroNeg, None) => format!(
                "a `**` base is 0 in its {w}-bit context under a negative exponent, so the `**` \
                 is `x` (IEEE 1800-2017 §11.4.3, Table 11-4)"
            ),
            (_, Some(d)) => format!(
                "the divisor of the `{op}` ({}) is 0 in its {w}-bit context, so the `{op}` is \
                 `x` (IEEE 1800-2017 §11.4.3)",
                Self::expr_brief(d)
            ),
            (_, None) => format!(
                "a divisor is 0 in its {w}-bit context, so a `{op}` is `x` (IEEE 1800-2017 \
                 §11.4.3)"
            ),
        }
    }

    /// The operator node an [`XWhy`] names, under `e`.
    fn origin_node(e: &ast::Expr, at: ast::Span) -> Option<&ast::Expr> {
        if e.span == at && matches!(e.kind, ast::ExprKind::Binary { .. }) {
            return Some(e);
        }
        let mut hit = None;
        e.for_each_child(|_, c| {
            if hit.is_none() {
                hit = Self::origin_node(c, at);
            }
        });
        hit
    }
}
