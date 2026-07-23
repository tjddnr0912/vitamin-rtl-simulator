//! SVA declarations / sampled rewrites — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// Register a named SVA sequence into the module-global table (first-wins, with a
    /// redeclaration warning). Shared by the top-level prescan and the generate-scope
    /// collector (slice A4) so both keep identical first-wins semantics.
    pub(crate) fn register_seq_decl(&mut self, s: &ast::SeqDecl) {
        if self.seq_table.contains_key(&s.name.name) {
            self.warn(&format!(
                "sequence `{}` redeclared; first declaration used",
                s.name.name
            ));
        } else {
            self.seq_table.insert(s.name.name.clone(), s.clone());
        }
    }

    /// Register a named SVA property (first-wins + redeclare warning). See
    /// [`Self::register_seq_decl`].
    pub(crate) fn register_prop_decl(&mut self, p: &ast::PropDecl) {
        if self.prop_table.contains_key(&p.name.name) {
            self.warn(&format!(
                "property `{}` redeclared; first declaration used",
                p.name.name
            ));
        } else {
            self.prop_table.insert(p.name.name.clone(), p.clone());
        }
    }

    /// Lower a `let` use (SVA-REST): positional formal→actual binding, then lower the
    /// substituted body (a 0-formal `let` lowers its body verbatim). Pure IR-0: the
    /// body is an ordinary expression, so this is a macro expansion at lowering time.
    /// Arity mismatch, an unknown name, and self/mutual recursion are loud (returning
    /// a placeholder so elaboration continues).
    pub(crate) fn lower_let_use(&mut self, name: &str, args: &[ast::Expr], _sp: ast::Span) -> u32 {
        let Some(decl) = self.let_table.get(name).cloned() else {
            return self.placeholder_expr();
        };
        if decl.formals.len() != args.len() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "`let {}` expects {} argument(s), got {}",
                    name,
                    decl.formals.len(),
                    args.len()
                ),
            );
            return self.placeholder_expr();
        }
        if self.sva_inline_stack.iter().any(|n| n == name) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("recursive `let {name}` is illegal (IEEE 1800 §11.13)"),
            );
            return self.placeholder_expr();
        }
        let body = if decl.formals.is_empty() {
            decl.body.clone()
        } else {
            subst_expr(&decl.body, &sva_formal_map(&decl.formals, args))
        };
        self.sva_inline_stack.push(name.to_string());
        let eid = self.lower_expr(&body);
        self.sva_inline_stack.pop();
        eid
    }

    /// Register a `let` declaration (SVA-REST, first-wins + redeclare warning). See
    /// [`Self::register_seq_decl`].
    pub(crate) fn register_let_decl(&mut self, l: &ast::LetDecl) {
        if self.let_table.contains_key(&l.name.name) {
            self.warn(&format!(
                "let `{}` redeclared; first declaration used",
                l.name.name
            ));
        } else {
            self.let_table.insert(l.name.name.clone(), l.clone());
        }
    }

    /// True iff `e` is a single-segment identifier that names a declared property
    /// and NOT a net of the same name (a real net wins the leaf path).
    pub(crate) fn is_property_name(&self, e: &ast::Expr) -> bool {
        if let ast::ExprKind::Ident(p) = &e.kind {
            if p.segments.len() == 1 {
                let n = &p.segments[0].name;
                return self.lookup_net_scoped(n).is_none() && self.prop_table.contains_key(n);
            }
        }
        false
    }

    /// Recursively rewrite SVA sampled-value functions in `e` into reads of
    /// synthesized prev-registers, registering each prev-reg + its `prev <=
    /// signal` NBA in `regs` (one prev-reg per distinct signal, shared). The
    /// argument must be a simple signal; anything else is a loud E3009.
    ///   $past(x)   → prev_x                (value one clock ago)
    ///   $stable(x) → (prev_x === x)        (no change, full 4-state)
    ///   $rose(x)   → (~prev_x[0] & x[0])   (LSB 0→1)
    ///   $fell(x)   → ( prev_x[0] & ~x[0])  (LSB 1→0)
    pub(crate) fn rewrite_sampled(&mut self, e: &ast::Expr, regs: &mut SvaRegs) -> ast::Expr {
        let sp = e.span;
        match &e.kind {
            ast::ExprKind::SysCall { name, args } if is_sva_sampled_fn(&name.name) => {
                if args.len() != 1 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("`{}` takes one signal argument in v1", name.name),
                    );
                    return e.clone();
                }
                // $sampled(e) = e (identity): the sampled value equals the current
                // value in our region model (no Preponed region — same approximation
                // as the existing $past family). Accepts any expression and recurses
                // so a nested sampled fn still resolves to its prev-register.
                if name.name == "$sampled" {
                    return self.rewrite_sampled(&args[0], regs);
                }
                let ast::ExprKind::Ident(path) = &args[0].kind else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("`{}` argument must be a simple signal in v1", name.name),
                    );
                    return e.clone();
                };
                // A hierarchical (multi-segment) reference would be keyed only by
                // its last segment below — two distinct signals (`top.x`/`u.x`)
                // would silently ALIAS onto one prev-register. Reject it loudly,
                // matching the existing hierarchical-reference policy (E3009).
                if path.segments.len() != 1 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "`{}` of a hierarchical signal is unsupported in v1",
                            name.name
                        ),
                    );
                    return e.clone();
                }
                let sig = path
                    .segments
                    .last()
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                let prev = self.sva_prev_for(&sig, &args[0], regs);
                let prev_ref = sva_ident_expr(&prev, sp);
                match name.name.as_str() {
                    "$past" => prev_ref,
                    "$stable" => sva_binary(ast::BinOp::CaseEq, prev_ref, args[0].clone(), sp),
                    // $changed(e) = (prev !== e): the negation of $stable, 1-bit.
                    "$changed" => sva_binary(ast::BinOp::CaseNe, prev_ref, args[0].clone(), sp),
                    "$rose" => sva_binary(
                        ast::BinOp::BitAnd,
                        sva_unary(ast::UnOp::BitNot, sva_bit0(prev_ref, sp), sp),
                        sva_bit0(args[0].clone(), sp),
                        sp,
                    ),
                    "$fell" => sva_binary(
                        ast::BinOp::BitAnd,
                        sva_bit0(prev_ref, sp),
                        sva_unary(ast::UnOp::BitNot, sva_bit0(args[0].clone(), sp), sp),
                        sp,
                    ),
                    _ => unreachable!("guarded by is_sva_sampled_fn"),
                }
            }
            ast::ExprKind::Unary { op, operand } => {
                sva_unary(*op, self.rewrite_sampled(operand, regs), sp)
            }
            ast::ExprKind::Binary { op, lhs, rhs } => sva_binary(
                *op,
                self.rewrite_sampled(lhs, regs),
                self.rewrite_sampled(rhs, regs),
                sp,
            ),
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => ast::Expr {
                kind: ast::ExprKind::Ternary {
                    cond: Box::new(self.rewrite_sampled(cond, regs)),
                    then_e: Box::new(self.rewrite_sampled(then_e, regs)),
                    else_e: Box::new(self.rewrite_sampled(else_e, regs)),
                },
                span: sp,
            },
            ast::ExprKind::Paren { inner } => ast::Expr {
                kind: ast::ExprKind::Paren {
                    inner: Box::new(self.rewrite_sampled(inner, regs)),
                },
                span: sp,
            },
            // Leaf or a form that cannot host a sampled-value call in the subset:
            // clone verbatim (a sampled call nested in e.g. a concat is left as a
            // plain SysCall → the usual "unsupported system function" E3009).
            _ => e.clone(),
        }
    }

    /// Walk an SVA action-block statement (slice A2), rewriting every contained
    /// expression through `rewrite_sampled` so `$past`/`$rose`/`$fell`/`$stable`
    /// inside `$error`/`$display`/condition/assignment leaves resolve to the SAME
    /// shared prev-registers as the property body. Structural clone otherwise, so an
    /// action with NO sampled-value fn allocates no nets (byte-identical to pre-A2).
    /// rewrite_sampled keeps its own guards (hierarchical / multi-arg / non-signal
    /// sampled args → E3009; sampled fn nested in a concat/select stays unsupported),
    /// because every Expr is routed through it rather than cloned blind.
    pub(crate) fn rewrite_sampled_stmt(&mut self, s: &ast::Stmt, regs: &mut SvaRegs) -> ast::Stmt {
        use ast::Stmt as S;
        match s {
            S::SysTaskCall { name, args, span } => S::SysTaskCall {
                name: name.clone(),
                args: args.iter().map(|e| self.rewrite_sampled(e, regs)).collect(),
                span: *span,
            },
            S::UserTaskCall { name, args, span } => S::UserTaskCall {
                name: name.clone(),
                args: args.iter().map(|e| self.rewrite_sampled(e, regs)).collect(),
                span: *span,
            },
            S::If {
                cond,
                then_s,
                else_s,
                span,
            } => S::If {
                cond: self.rewrite_sampled(cond, regs),
                then_s: Box::new(self.rewrite_sampled_stmt(then_s, regs)),
                else_s: else_s
                    .as_ref()
                    .map(|e| Box::new(self.rewrite_sampled_stmt(e, regs))),
                span: *span,
            },
            S::Block {
                label,
                decls,
                stmts,
                span,
            } => S::Block {
                label: label.clone(),
                decls: decls.clone(),
                stmts: stmts
                    .iter()
                    .map(|st| self.rewrite_sampled_stmt(st, regs))
                    .collect(),
                span: *span,
            },
            S::Blocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } => S::Blocking {
                lhs: lhs.clone(),
                delay: delay.clone(),
                event: event.clone(),
                rhs: self.rewrite_sampled(rhs, regs),
                span: *span,
            },
            S::NonBlocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } => S::NonBlocking {
                lhs: lhs.clone(),
                delay: delay.clone(),
                event: event.clone(),
                rhs: self.rewrite_sampled(rhs, regs),
                span: *span,
            },
            S::Case {
                kind,
                scrutinee,
                items,
                span,
            } => S::Case {
                kind: *kind,
                scrutinee: self.rewrite_sampled(scrutinee, regs),
                items: items
                    .iter()
                    .map(|it| match it {
                        ast::CaseItem::Match { labels, body, span } => ast::CaseItem::Match {
                            labels: labels
                                .iter()
                                .map(|e| self.rewrite_sampled(e, regs))
                                .collect(),
                            body: Box::new(self.rewrite_sampled_stmt(body, regs)),
                            span: *span,
                        },
                        ast::CaseItem::Default { body, span } => ast::CaseItem::Default {
                            body: Box::new(self.rewrite_sampled_stmt(body, regs)),
                            span: *span,
                        },
                    })
                    .collect(),
                span: *span,
            },
            // Action statements with no sampled-value-hosting expressions (or forms
            // out of the action-block subset — timing controls, fork, …) clone
            // verbatim: no net allocation, so byte-identical to pre-A2.
            other => other.clone(),
        }
    }
}
