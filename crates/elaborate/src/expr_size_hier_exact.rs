//! Is a declared width that the hierarchical declaration walk folds EXACT by
//! construction? A structural walk over the same AST `env_fold` folds, asked
//! only by the placeholder-shape record (`hier_leaf_shape.rs`); the size-cast
//! lane never reads it.
//!
//! `env_fold` answers in i64 and a few of its arms can land on a value the
//! binder does not bind (`-4'd1` binds `4'd15`, s15 review p22_lp; a typed
//! `integer` binder evaluates its initializer in a 32-bit context, s15 review
//! r03_minus). A width is recorded only when every node its fold passed
//! through is one the i64 fold cannot get wrong: integer literals (unsized, or
//! sized at 32 bits or more, unsigned), names bound from such values in an
//! untyped slot, a name of the live scope or a package constant (the binder's
//! own value), `+ - *`, parentheses, and `$clog2`. Anything else — any unary
//! operator, `/ %`, shifts, relational, ternary, a narrow or `signed` sized
//! literal, a typed slot — is not exact, and the placeholder stays unrecorded.

use super::*;

/// Where a name in an exactness question resolves.
enum ExactScope<'a> {
    /// The module being lowered: its names are the binder's own values.
    Live,
    /// A built environment: the names whose binding is exact.
    Set(&'a BTreeSet<String>),
}

fn expr_exact(e: &ast::Expr, scope: &ExactScope<'_>) -> bool {
    match &e.kind {
        ast::ExprKind::IntLit { kind, raw } => match kind {
            ast::IntLitKind::Decimal => true,
            _ => parse_int_literal(raw, *kind).is_some_and(|cv| !cv.signed && cv.width >= 32),
        },
        ast::ExprKind::Paren { inner } => expr_exact(inner, scope),
        ast::ExprKind::Ident(p) if p.segments.len() == 1 => match scope {
            ExactScope::Live => true,
            ExactScope::Set(s) => s.contains(&p.segments[0].name),
        },
        ast::ExprKind::PkgScoped { .. } => true,
        ast::ExprKind::Binary {
            op: ast::BinOp::Add | ast::BinOp::Sub | ast::BinOp::Mul,
            lhs,
            rhs,
        } => expr_exact(lhs, scope) && expr_exact(rhs, scope),
        ast::ExprKind::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
            expr_exact(&args[0], scope)
        }
        _ => false,
    }
}

impl Elaborator<'_> {
    /// The exactly-bound parameter names of one instance of `child` — the twin of
    /// `child_env`, with the same override mapping. `None` where `child_env`'s
    /// mapping fails too.
    fn env_exact(
        child: &ModuleFacts,
        overrides: &[ast::ParamConn],
        parent: Option<&BTreeSet<String>>,
    ) -> Option<BTreeSet<String>> {
        let slots = child.params.as_ref()?;
        let pscope = match parent {
            None => ExactScope::Live,
            Some(s) => ExactScope::Set(s),
        };
        let positional: Vec<&ParamSlot> = slots.iter().filter(|s| s.overridable).collect();
        let mut bound: BTreeMap<&str, bool> = BTreeMap::new();
        let mut pos_i = 0usize;
        for ov in overrides {
            let (name, value) = match ov {
                ast::ParamConn::Positional(e) => {
                    let s = positional.get(pos_i)?;
                    pos_i += 1;
                    (s.name.as_str(), e)
                }
                ast::ParamConn::Named { name, value, .. } => {
                    let s = slots
                        .iter()
                        .find(|s| s.name == name.name)
                        .filter(|s| s.overridable)?;
                    (s.name.as_str(), value.as_ref()?)
                }
            };
            if bound.insert(name, expr_exact(value, &pscope)).is_some() {
                return None;
            }
        }
        let mut exact = BTreeSet::new();
        for s in slots {
            let e = match bound.get(s.name.as_str()) {
                Some(&e) => e,
                None => expr_exact(&s.default, &ExactScope::Set(&exact)),
            };
            if e && s.ty == Some(ast::ParamType::Implicit) {
                exact.insert(s.name.clone());
            }
        }
        Some(exact)
    }

    /// Is the declared width `hier_leaf_net` (`func == false`) or
    /// `hier_leaf_func` (`func == true`) answers for `path` exact by
    /// construction? Walks the same instance chain as `hier_leaf_scope`.
    pub(crate) fn hier_leaf_width_exact(&self, path: &[&str], func: bool) -> bool {
        let Some((leaf, insts)) = path.split_last() else {
            return false;
        };
        let Some(mut f) = self.module_facts.get(&self.cur_module) else {
            return false;
        };
        let mut exact: Option<BTreeSet<String>> = None;
        let mut live = true;
        for seg in insts {
            let Some((module, overrides)) = f.insts.get(*seg) else {
                return false;
            };
            let Some(child) = self.module_facts.get(module) else {
                return false;
            };
            exact = if live || exact.is_some() {
                Self::env_exact(child, overrides, exact.as_ref())
            } else {
                None
            };
            live = false;
            f = child;
        }
        let width = if func {
            f.funcs.get(*leaf).map(|(w, _)| w)
        } else {
            f.nets.get(*leaf).map(|n| &n.width)
        };
        match width {
            Some(WidthFact::Lit(_)) => true,
            Some(WidthFact::Range(r)) => exact.is_some_and(|s| {
                let sc = ExactScope::Set(&s);
                expr_exact(&r.msb, &sc) && expr_exact(&r.lsb, &sc)
            }),
            None => false,
        }
    }
}
