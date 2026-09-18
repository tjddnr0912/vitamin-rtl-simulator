//! §3.b: the PACKAGE half of the `parameter type T` desugar — `package p;
//! parameter type PT = logic [7:0]; endpackage` used as `p::PT` — and the
//! literal-range registration a NON-overridable type parameter shares with it.
//!
//! IEEE §26.3 makes a package's type parameters referable scope-qualified exactly
//! like its typedefs (§6.20.1 makes them non-overridable, so a package one names a
//! single concrete type for the whole design). The desugar already emitted `PT$w` /
//! `PT$s` as ordinary package parameters and registered a BARE typedef `PT`, but
//! `restore_scope_unit` drops every bare name a unit added and only the `pkg::t`
//! twins survive — so `p::PT` resolved to nothing and every use was a parse error.
//! [`Parser::register_pkg_typedef_twin`] is the twin registration the `endpackage`
//! pass runs for a typedef, shared verbatim with a type parameter: there is no
//! struct / enum / union map work to share, because a type parameter is always a
//! vector type.
//!
//! The twin must carry DIMENSIONS a consumer outside the package can fold, which is
//! what [`Parser::type_param_literal_range`] is for: `[PT$w-1:0]` names a constant
//! the importing module never declared, and it loses the declared LSB even inside
//! the package (`logic [8:1]` read `v[1]` as bit 0). A non-overridable parameter of
//! a concrete type has no override to follow, so its typedef registers the DECLARED
//! range — byte-identical dims to the `typedef` spelling of the same type.
use super::*;
use crate::type_params::TypeValue;

impl Parser<'_, '_> {
    /// The single packed range a NON-overridable type parameter of a CONCRETE type
    /// registers its typedef with: the declared range FOLDED TO LITERAL BOUNDS at
    /// the declaration point (`[8:1]`, `[W-1:0]` over a parse-time constant ⇒
    /// `[11:0]`, `[$clog2(N)-1:0]` ⇒ `[6:0]`), else `[width-1:0]` when the type is
    /// an atom whose width folded (`int`, `byte`, a range-less kind). `None`
    /// everywhere the symbolic `[T$w-1:0]` registration must stay:
    ///
    /// * `literal` is false — an OVERRIDABLE parameter (its dims must follow the
    ///   override through `T$w` / `T$p<i>a`), or an ALIAS of another type parameter
    ///   (`shape_expr` is `Some`), which must follow the parameter it names.
    /// * the type has two or more packed dimensions — `carried_packed` is the
    ///   carrier for those and already holds the declared ranges here.
    /// * a declared bound does NOT fold here (a header parameter, a variable).
    ///
    /// ⚠️ The bounds are FOLDED, never stored as the parsed expression. vita's type
    /// dimensions are name-keyed and re-resolved in the scope of each USE, so a
    /// stored `[W-1:0]` is captured by any inner generate scope that declares its
    /// own `W` — measured: a module `localparam W = 8` with a generate-local
    /// `localparam W = 4` gave `$bits(L)` 4 and a 4-bit `L v` where both oracles
    /// read 8, and a per-iteration `localparam W = 4+i` gave the same type two
    /// different widths in two loop bodies. `[T$w-1:0]` was immune because a `$`
    /// carrier is a name no user declaration can shadow; a folded literal is immune
    /// because it names nothing. The fold is the same `const_bound` the packed
    /// struct member layout is built from, so a bound this accepts is one
    /// `member_width` already folded at the same point.
    pub(crate) fn type_param_literal_range(
        &self,
        tv: &TypeValue,
        literal: bool,
        carried_packed: &[Range],
        span: Span,
    ) -> Option<Range> {
        if !literal || !carried_packed.is_empty() {
            return None;
        }
        let Some(r) = &tv.range else {
            // An atom (`int`, `byte`, a range-less kind): no declared range, but a
            // literal width — `[w-1:0]` names nothing either.
            return Self::lit_u32(&tv.width).map(|w| Range {
                msb: Self::dec_lit(w.saturating_sub(1), span),
                lsb: Self::dec_lit(0, span),
                span,
            });
        };
        Some(Range {
            msb: Self::folded_bound(self.const_bound(&r.msb)?, r.span)?,
            lsb: Self::folded_bound(self.const_bound(&r.lsb)?, r.span)?,
            span: r.span,
        })
    }

    /// A folded bound as a literal expression — `-N` as the unary-minus form the
    /// package-dim respell uses. `None` outside the `u32` magnitude the literal
    /// builder carries, which keeps the caller on the `[T$w-1:0]` path rather than
    /// registering a truncated bound.
    fn folded_bound(v: i64, span: Span) -> Option<Expr> {
        if v < 0 {
            let m = u32::try_from(v.unsigned_abs()).ok()?;
            return Some(Expr {
                kind: ExprKind::Unary {
                    op: UnOp::Minus,
                    operand: Box::new(Self::dec_lit(m, span)),
                },
                span,
            });
        }
        Some(Self::dec_lit(u32::try_from(v).ok()?, span))
    }

    /// Register the `pkg::n` twin of the package-scope type `n` in `typedefs` and
    /// answer the scoped key. The twin's three dimension containers are re-spelled
    /// through `respell_pkg_dims` / `respell_pkg_unpacked` so a bound naming one of
    /// the package's OWN constants reads `pkg::W` — the bare name is undefined
    /// wherever the twin is used without importing it (§4.5.415).
    ///
    /// Shared by the `endpackage` typedef pass and its type-parameter twin: the
    /// aggregate sub-maps (struct / enum / union) stay with the typedef caller,
    /// which is the only one whose node can BE an aggregate.
    pub(crate) fn register_pkg_typedef_twin(&mut self, pkg: &str, n: &str) -> String {
        let scoped = format!("{pkg}::{n}");
        if let Some(mut ti) = self.typedefs.get(n).cloned() {
            if let Some(r) = ti.range.take() {
                ti.range = self.respell_pkg_dims(pkg, &[r]).pop();
            }
            if !ti.packed.is_empty() {
                ti.packed = self.respell_pkg_dims(pkg, &ti.packed);
            }
            if !ti.unpacked.is_empty() {
                ti.unpacked = self.respell_pkg_unpacked(pkg, &ti.unpacked);
            }
            self.typedefs.insert(scoped.clone(), ti);
        }
        scoped
    }

    /// The names of the type parameters THIS package body declared, in body order.
    ///
    /// Both halves are required. `pkg_type_param_names` is the POSITIVE record —
    /// only `parse_type_param_group` writes it, and only while `in_package` — so a
    /// name that merely LOOKS like a carrier cannot enter: a user `parameter int
    /// PT$w = 3` in a package with a compilation-unit `parameter type PT` in scope
    /// satisfied a `<stem>$w` + `type_params` pair and registered a twin for a type
    /// the package never declared (both oracles reject that program; vita read a
    /// 3-bit type). The body scan then fixes the ORDER and confirms the group
    /// actually emitted this body's carrier.
    pub(crate) fn pkg_type_param_stems(&self, body: &[ModuleItem]) -> Vec<String> {
        body.iter()
            .filter_map(|it| match it {
                ModuleItem::Param(p) => p
                    .name
                    .name
                    .strip_suffix("$w")
                    .filter(|stem| self.pkg_type_param_names.contains(*stem))
                    .map(str::to_string),
                _ => None,
            })
            .collect()
    }
}
