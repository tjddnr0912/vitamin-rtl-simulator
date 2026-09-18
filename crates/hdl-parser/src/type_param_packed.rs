//! §3.a ⑤: the PACKED-dimension half of the `parameter type T` desugar —
//! `parameter type T = logic [1:0][3:0]` and every other type with two or more
//! packed dimensions (inline, or through a vector typedef).
//!
//! `T$w` carries the TOTAL packed width, exactly as it does for the one-dimensional
//! spelling, so `$bits(T)`, `T'(e)` and every width consumer keep the single value
//! they already read. What a multi-dimensional type adds is the per-dimension SHAPE
//! a select on `T v` needs (`v[1]`, `$size(v,2)`, `$left(v,1)`), and that shape is
//! a LIST — so it rides the same channel the unpacked extents ride: two synthesized
//! value parameters per dimension, `T$p<i>a` / `T$p<i>b`, declared after the
//! `T$d<i>a` / `T$d<i>b` pairs and pushed in the same order by an instance
//! override. The typedef registered for `T` names them (`[T$p0a:T$p0b][T$p1a:T$p1b]`
//! instead of `[T$w-1:0]`), which is the one place every declaration binder, port
//! and tf-port formal already reads its dimensions from.
//!
//! A packed dimension is always written `[msb:lsb]`, so — unlike an unpacked one —
//! there is no `[N]` spelling to normalize and the endpoints ride verbatim.
//!
//! The dimension COUNT is not carried: a module's declarations of `T` are stamped
//! with the default's dimension LIST once, at parse, so an override that changes
//! the count cannot be followed. It stays loud in both directions — a count-losing
//! override on the shape guard's `$fatal` (`T$s` records the count), a count-adding
//! one on the `T$p<i>a` carrier the module never declared (E3002, named against `T`).
use super::*;
use crate::type_params::TypeValue;

impl Parser<'_, '_> {
    /// The per-dimension value parameters an OVERRIDABLE type parameter's PACKED
    /// dimensions ride, and the dimension list that names them.
    ///
    /// Two parameters per dimension — `T$p<i>a` / `T$p<i>b`, the dimension's
    /// declared `[msb:lsb]` endpoints — and the registered typedef gets
    /// `[T$p<i>a:T$p<i>b]` in their place. So an override's own extents reach
    /// `T v;`, `$size(v,k)`, `$left`/`$right` and the element addresses by the same
    /// route the total WIDTH reaches them through `T$w`.
    ///
    /// `None` for a dimension-free type (one packed range or an atom), which is
    /// what keeps every design that predates this carrier byte-identical: the
    /// caller then registers the `[T$w-1:0]` typedef it always registered and
    /// declares no extra parameter.
    ///
    /// ⚠️ Only for an OVERRIDABLE type parameter, for the same reason
    /// [`Parser::type_param_dim_params`] is: making a `localparam type` / package
    /// one's dimensions symbolic would move `$bits(T)` onto `sym_range_width`,
    /// which answers only when a bound NAMES an overridable parameter — a decline
    /// where the literal dimensions fold today.
    pub(crate) fn type_param_packed_params(
        &self,
        tname: &str,
        dims: &[Range],
        kind: ParamKind,
        span: Span,
    ) -> Option<(Vec<ParamDecl>, Vec<Range>)> {
        if dims.is_empty() {
            return None;
        }
        let mut decls = Vec::new();
        let mut out = Vec::new();
        for (i, d) in dims.iter().enumerate() {
            let na = format!("{tname}$p{i}a");
            let nb = format!("{tname}$p{i}b");
            for (n, v) in [(&na, d.msb.clone()), (&nb, d.lsb.clone())] {
                decls.push(ParamDecl {
                    kind,
                    signed: false,
                    ty: ParamType::Implicit,
                    range: None,
                    name: Ident {
                        name: n.clone(),
                        span,
                    },
                    value: v,
                    span,
                });
            }
            out.push(Range {
                msb: Self::ident_expr(&na, span),
                lsb: Self::ident_expr(&nb, span),
                span,
            });
        }
        Some((decls, out))
    }

    /// The TOTAL packed width of a dimension list: the product of every dimension's
    /// extent, each a literal where the parse-time table folds it and the §7.4.1
    /// expression where a bound names an overridable parameter. `None` when any
    /// dimension folds neither way — the same acceptance rule the one-dimensional
    /// spelling uses, so a multi-dimensional type declines exactly where its
    /// single-dimension twin does.
    pub(crate) fn packed_dims_width(&self, dims: &[Range], span: Span) -> Option<Expr> {
        let mut acc: Option<Expr> = None;
        for r in dims {
            let f = match self.member_width(&Some(r.clone())) {
                Some(w) => Self::dec_lit(w, span),
                None => self.sym_range_width(r)?,
            };
            acc = Some(match acc {
                None => f,
                Some(a) => Self::mul(a, f, span),
            });
        }
        acc
    }

    /// The packed dimension LIST a resolved type reports: the whole list IN SOURCE
    /// ORDER when there are two or more, EMPTY otherwise.
    ///
    /// The empty answer for a one-dimensional type is load-bearing, not a shortcut:
    /// it is what keeps the typedef registration, the `T$s` value, the parameter
    /// declaration list and the override connection list byte-identical for every
    /// design that parsed before this carrier existed.
    pub(crate) fn packed_dim_list(first: Option<&Range>, extra: &[Range]) -> Vec<Range> {
        if extra.is_empty() {
            return Vec::new();
        }
        let Some(f) = first else {
            return Vec::new();
        };
        let mut v = Vec::with_capacity(extra.len() + 1);
        v.push(f.clone());
        v.extend(extra.iter().cloned());
        v
    }

    /// Declare the `T$p<i>a/b` carriers for `tv`'s packed dimensions into `decls`
    /// (an OVERRIDABLE parameter only) and answer the dimension list the typedef and
    /// the `TypeParam` record name: the synthesized names for an overridable
    /// parameter, the literal ranges for a `localparam type` / package one.
    pub(crate) fn declare_packed_carriers(
        &mut self,
        tname: &str,
        tv: &TypeValue,
        kind: ParamKind,
        overridable: bool,
        span: Span,
        decls: &mut Vec<ParamDecl>,
    ) -> Vec<Range> {
        match self.type_param_packed_params(tname, &tv.packed, kind, span) {
            Some((packed_decls, dims)) if overridable => {
                for d in &packed_decls {
                    self.overridable_params.insert(d.name.name.clone());
                }
                decls.extend(packed_decls);
                dims
            }
            _ => tv.packed.clone(),
        }
    }

    /// The `(range, packed)` a type parameter's registered typedef gets: the carried
    /// dimension list split the way `TypeInfo` splits a declaration's (outermost in
    /// `range`, the rest in `packed`), or the `[T$w-1:0]` single range every
    /// one-dimensional type parameter has always been registered with.
    pub(crate) fn type_param_typedef_dims(
        carried: &[Range],
        width_name: &str,
        span: Span,
    ) -> (Option<Range>, Vec<Range>) {
        match carried.split_first() {
            Some((first, rest)) => (Some(first.clone()), rest.to_vec()),
            None => (
                Some(Range {
                    msb: Self::sub(
                        Self::ident_expr(width_name, span),
                        Self::dec_lit(1, span),
                        span,
                    ),
                    lsb: Self::dec_lit(0, span),
                    span,
                }),
                Vec::new(),
            ),
        }
    }

    /// An instance override's `T$p<i>a` / `T$p<i>b` connections, named when the
    /// override was (`name`), positional otherwise — pushed in the order the group
    /// declares them, after the `T$d…` pair of every unpacked dimension.
    pub(crate) fn push_packed_override_conns(
        out: &mut Vec<ParamConn>,
        name: Option<&Ident>,
        dims: &[Range],
        span: Span,
    ) {
        for (i, r) in dims.iter().enumerate() {
            for (suf, v) in [("a", r.msb.clone()), ("b", r.lsb.clone())] {
                match name {
                    Some(n) => out.push(ParamConn::Named {
                        name: Ident {
                            name: format!("{}$p{i}{suf}", n.name),
                            span: n.span,
                        },
                        value: Some(v),
                        span,
                    }),
                    None => out.push(ParamConn::Positional(v)),
                }
            }
        }
    }
}
