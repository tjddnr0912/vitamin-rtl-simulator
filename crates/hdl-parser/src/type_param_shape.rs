//! §3 ⑤ⓕ: the SHAPE half of the `parameter type T` desugar — the per-instance
//! signedness and 2-state kind.
//!
//! `T$w` (the width) always rode an EXPRESSION, so elaborate folded it per instance
//! and an override's width reached every declaration of `T`. `T$s` (the shape) did
//! not: `signed` and the 2-state half of the kind are parse-time scalars on the AST
//! declaration containers, stamped once per MODULE from the DEFAULT type, so `T$s`
//! had exactly one reader — a synthesized `initial if (T$s != <default>) $fatal`
//! that refused the override outright.
//!
//! This module holds the pieces that changed that: the guard's two wordings, the
//! module-END narrowing of the guard to the ARITY bits, the record of a use of `T`
//! that lands where no per-instance shape can follow, and the predicate that keeps a
//! user from naming a synthesized carrier directly. The carrier itself is
//! `NetVarDecl`/`AnsiPort`/`PortDecl`/`TfPort`.`shape_param`, which elaborate folds
//! (`Elaborator::shape_kind` / `shape_signed`).
use super::*;

impl Parser<'_, '_> {
    /// `T$w` / `T$s` / `T$d<i>a` / `T$d<i>b` — the value parameters a `parameter
    /// type T` desugars to. A user cannot legitimately name one: the desugar builds
    /// those `ParamConn`s directly, and both oracles reject the spelling because no
    /// such parameter exists in their elaborated module.
    pub(crate) fn names_a_type_param_carrier(name: &str) -> bool {
        let Some((head, tail)) = name.rsplit_once('$') else {
            return false;
        };
        if head.is_empty() {
            return false;
        }
        if tail == "w" || tail == "s" {
            return true;
        }
        let Some(rest) = tail.strip_prefix('d') else {
            return false;
        };
        let Some(digits) = rest.strip_suffix(['a', 'b']) else {
            return false;
        };
        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
    }

    /// The F4004 text. `arity_only` is the compare the module ended up with:
    /// every use of `T` reached a container that carries `T$s`, so the SIGN and the
    /// 2-STATE kind now follow an override and only the unpacked dimension COUNT
    /// cannot (the declarators are stamped with the default's dim LIST at parse).
    /// The strict wording is what a module with an UNCARRIED use keeps.
    pub(crate) fn shape_guard_msg(tname: &str, arity_only: bool) -> String {
        if arity_only {
            format!(
                "\"type parameter `{tname}`: the override changes the type's unpacked dimension COUNT, which the module's declarations of `{tname}` cannot follow (an override may change the width, the unpacked extents, the signedness and the 2-state kind; only the dimension count is fixed — v1)\""
            )
        } else {
            format!(
                "\"type parameter `{tname}`: the override changes the type's signedness, 2-state kind or unpacked dimensions, which the module's declarations of `{tname}` cannot follow, because `{tname}` is used here in a position that carries no shape (an enum base, a packed struct/union member, a class property, a function RETURN type or a `{tname}'(e)` cast) (an override must keep the default type's shape; only its width and unpacked extents may differ — v1)\""
            )
        }
    }

    /// The single `Ident` name of a shape default EXPRESSION (`parameter type U = T`
    /// gives `U$s` the expression `T$s`), or `None` for a literal / anything else.
    pub(crate) fn shape_ident_name(e: &Expr) -> Option<String> {
        let ExprKind::Ident(hp) = &e.kind else {
            return None;
        };
        match hp.segments.as_slice() {
            [seg] => Some(seg.name.clone()),
            _ => None,
        }
    }

    /// Follow `shape_alias` to the ROOT carrier of `name` (itself when it is one).
    /// The map is built as each group is parsed, so a chain is already flattened;
    /// the bounded loop is the cycle guard a hand-written `V = U = V` cannot survive.
    pub(crate) fn shape_alias_root(&self, name: &str) -> String {
        let mut cur = name;
        for _ in 0..16 {
            match self.shape_alias.get(cur) {
                Some(next) if next != cur => cur = next,
                _ => break,
            }
        }
        cur.to_string()
    }

    /// §3 ⑤ⓕ: a use of `T` that lands in a container with NO `shape_param` slot —
    /// an enum base, a class property, a packed struct/union member. Such a
    /// declaration is stamped with the DEFAULT's signedness and 2-state kind and
    /// nothing can re-fold it per instance, so `T`'s guard keeps the STRICT compare
    /// and the design stays loud rather than silently binding the default's shape.
    pub(crate) fn note_uncarried_shape_use(&mut self, info: &TypeInfo) {
        if let Some(sp) = &info.shape_param {
            // TRANSITIVE: an uncarried use of `U` where `parameter type U = T` is an
            // uncarried use of `T` — `U` has no guard of its own, and `T`'s is the one
            // that must stay strict. Both names are recorded so a direct use of either
            // spelling is caught.
            let root = self.shape_alias_root(sp);
            self.shape_uncarried.insert(sp.clone());
            self.shape_uncarried.insert(root);
        }
    }

    /// The `shape_param` an AST declaration container copies out of `info`.
    pub(crate) fn carried_shape_param(&self, info: &TypeInfo, span: Span) -> Option<Ident> {
        info.shape_param.as_ref().map(|n| Ident {
            name: n.clone(),
            span,
        })
    }

    /// Module END: narrow every shape guard whose type parameter had ONLY carried
    /// uses to the ARITY bits, and re-word it. Walks `body` for the synthesized
    /// `initial if (T$s != <default>) $fatal(…)` — identified by its `T$s` left
    /// operand, a name the lexer cannot produce for user source.
    pub(crate) fn narrow_shape_guards(&mut self, body: &mut [ModuleItem]) {
        let carriers = std::mem::take(&mut self.shape_carriers);
        let uncarried = std::mem::take(&mut self.shape_uncarried);
        if carriers.is_empty() {
            return;
        }
        for it in body.iter_mut() {
            let ModuleItem::Proc(pb) = it else { continue };
            if pb.kind != ProcKind::Initial {
                continue;
            }
            let Stmt::If { cond, then_s, .. } = &mut *pb.body else {
                continue;
            };
            let ExprKind::Binary {
                op: BinOp::Ne, lhs, ..
            } = &mut cond.kind
            else {
                continue;
            };
            let ExprKind::Ident(hp) = &lhs.kind else {
                continue;
            };
            let Some(seg) = hp.segments.first() else {
                continue;
            };
            let sname = seg.name.clone();
            if hp.segments.len() != 1
                || !carriers.contains(&sname)
                || uncarried.contains(&sname)
                || !sname.ends_with("$s")
            {
                continue;
            }
            // `(T$s >> 2) != (<default> >> 2)` — the dim COUNT alone. The sign bit
            // and the 2-state bit are now carried per instance.
            let span = cond.span;
            if let ExprKind::Binary { lhs, rhs, .. } = &mut cond.kind {
                let shr = |e: &mut Box<Expr>| {
                    let inner = (**e).clone();
                    **e = Expr {
                        kind: ExprKind::Binary {
                            op: BinOp::Shr,
                            lhs: Box::new(inner),
                            rhs: Box::new(Self::dec_lit(2, span)),
                        },
                        span,
                    };
                };
                shr(lhs);
                shr(rhs);
            }
            // Re-word the `$fatal` message.
            let tname = sname.trim_end_matches("$s").to_string();
            if let Stmt::SysTaskCall { args, .. } = &mut **then_s {
                if let Some(a) = args.get_mut(1) {
                    a.kind = ExprKind::StrLit {
                        raw: Self::shape_guard_msg(&tname, true),
                    };
                }
            }
        }
    }
}
