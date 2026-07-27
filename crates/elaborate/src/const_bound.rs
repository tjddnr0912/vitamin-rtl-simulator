//! constant BOUND / COUNT folding — the single funnel every select-bound,
//! part-select-width and replication-count site shares, plus the guards that
//! decide when the width-unlimited i64 const domain may be trusted.
//!
//! Split out of `const_fn.rs` (module-size policy); the const domain itself
//! (`const_eval_in_scope`, the constant-function interpreter) stays there.

use super::*;

impl Elaborator<'_> {
    /// The constant value of a select BOUND or a replication COUNT in the u32
    /// domain — the single funnel every such site shares, so the rule cannot drift
    /// between the read, write, hierarchical and multi-dim-packed paths.
    ///
    /// The v1 literal-only [`const_eval_u32`] is tried FIRST, so every shape it
    /// already folded keeps its exact result — including the deliberate unsigned
    /// `wrapping_neg` of a negative literal that the part-select direction check
    /// reads as "out of order". Only when that yields None does the full elaborate
    /// const domain run ([`Self::const_eval_in_scope`]: parameters, package
    /// constants, `$clog2`/`$bits`, integral casts, constant-function calls,
    /// ternaries, and the whole `const_binop` operator set). A negative or >u32
    /// value folds to None — fail-closed, so the caller keeps its previous
    /// behavior rather than acting on a wrapped bound.
    ///
    /// The strong domain is width-UNLIMITED while SV constant arithmetic wraps at
    /// its self-determined width, so it is only consulted for an expression
    /// [`Self::const_fold_is_width_exact`] proves cannot wrap. Without that guard
    /// `$clog2(4'd15 + 4'd15)` folds 30 → 5 where SV truncates to 14 → 4: a WRONG
    /// NON-ZERO count, which is worse than the empty one it replaced. Declining
    /// leaves the caller exactly where it was.
    pub(crate) fn const_bound_u32(&self, e: &ast::Expr) -> Option<u32> {
        if let Some(v) = const_eval_u32(e) {
            return Some(v);
        }
        if !self.const_fold_is_width_exact(e) {
            return None;
        }
        u32::try_from(self.const_eval_in_scope(e)?).ok()
    }

    /// Lower a constant WIDTH / COUNT expression (an indexed part-select's `w` in
    /// `[c +: w]` / `[c -: w]`, a replication count) so the downstream consumer
    /// actually receives a constant.
    ///
    /// Both consumers reduce the lowered tree with a SHALLOW fold (`Const`, the
    /// `Add`/`Sub` of a width tree, `$clog2` of a `Const`) and treat "did not
    /// reduce" as `unwrap_or(1)` / `unwrap_or(0)` — a silent 1-bit select or an
    /// empty replication. Everything else the language calls a constant expression
    /// (a cast, a constant-function call, `*`, a ternary, `$bits(x)/k`) landed
    /// there. So: lower as before, and only if the result is NOT already reducible
    /// hand over a `Const` from the full const domain.
    ///
    /// ADDITIVE by construction — an expression whose lowered form already reduces
    /// keeps that exact node (byte-identical IR for every design that worked), and
    /// one the const domain cannot fold either keeps it too. The real/loud rejects
    /// inside [`Self::lower_index_expr`] run first and yield `Const 0`, which IS
    /// reducible, so this can never paper over one.
    pub(crate) fn lower_const_width_expr(&mut self, e: &ast::Expr) -> u32 {
        let id = self.lower_index_expr(e);
        if self.const_of_expr_u32(id).is_some() {
            return id;
        }
        match self.const_bound_u32(e) {
            Some(n) => self.const_u32_expr(n, 32),
            None => id,
        }
    }

    /// True when evaluating `e` in the width-unlimited i64 const domain gives the
    /// same value SV's width-limited constant arithmetic does.
    ///
    /// Three conditions, all necessary — the first two were learned from adversarial
    /// differential probes that each broke a two-condition version:
    ///
    /// 1. **No name the LOWERING would resolve differently.** `const_eval_in_scope`
    ///    reads a bare ident from `params`, but `lower_expr` consults the inline
    ///    substitution stacks first, so a function formal shadowing a module param
    ///    would fold to the param's value while the lowered twin used the formal.
    ///    Same decline `param_sel_range` already makes, for the same reason.
    /// 2. **Every intermediate stays inside the non-negative 32-bit range.** Leaf
    ///    widths alone are not enough: `(32'd1 << 32'd33) >> 32'd30` has 32-bit
    ///    leaves, yet SV drops the bits above 32 and yields 0 while i64 yields 8 —
    ///    a part-select that was CORRECT before (the engine could not fold a shift,
    ///    so it fell back to a 1-bit width, which is the right answer). Requiring
    ///    `0 ..= i32::MAX` at every foldable sub-expression makes the signed and
    ///    unsigned 32-bit readings agree with i64 exactly, and it subsumes the
    ///    "overflows 32 bits and shrinks back" case that has no leaf-level tell.
    /// 3. **Every width-growing operator runs on ≥32-bit leaves**, so the operation
    ///    itself is at least 32 bits wide — `4'd15 + 4'd15` is 14 in SV and 30 here.
    ///
    /// Conservative in both directions: an unmodeled shape declines, and declining
    /// only ever leaves the caller with the behavior it had before the funnel.
    pub(crate) fn const_fold_is_width_exact(&self, e: &ast::Expr) -> bool {
        !self.ast_mentions_substituted_name(e)
            && self.const_fold_subvalues_fit_i32(e)
            && (!Self::ast_has_width_growing_op(e) || self.ast_const_leaves_min32(e))
    }

    /// Condition 2: no foldable sub-expression of `e` leaves `0 ..= i32::MAX`.
    /// A sub-expression the const domain cannot fold contributes no intermediate of
    /// its own, so only its children are checked.
    fn const_fold_subvalues_fit_i32(&self, e: &ast::Expr) -> bool {
        if let Some(v) = self.const_eval_in_scope(e) {
            if !(0..=i32::MAX as i64).contains(&v) {
                return false;
            }
        }
        Self::const_fold_children(e)
            .iter()
            .all(|c| self.const_fold_subvalues_fit_i32(c))
    }

    /// Condition 1: does any bare name in `e` resolve through an inline
    /// substitution stack rather than the param scope the const domain reads?
    fn ast_mentions_substituted_name(&self, e: &ast::Expr) -> bool {
        if let ast::ExprKind::Ident(p) = &e.kind {
            if p.segments.len() == 1 {
                let n = &p.segments[0].name;
                if self.subst_lookup(n).is_some() || self.out_subst_lookup(n).is_some() {
                    return true;
                }
            }
        }
        Self::const_fold_children(e)
            .iter()
            .any(|c| self.ast_mentions_substituted_name(c))
    }

    /// The sub-expressions of `e` that the const domain can descend into — one
    /// traversal shared by all three conditions so they cannot cover different sets.
    ///
    /// COMPLETENESS: the arms below are exactly the `ExprKind`s that
    /// [`Self::const_eval_in_scope`] has a fold arm for. Any other kind returns None
    /// there, so the whole fold declines before these predicates could matter — which
    /// is why an empty child list for an unlisted kind is safe rather than a hole.
    pub(crate) fn const_fold_children(e: &ast::Expr) -> Vec<&ast::Expr> {
        use ast::ExprKind as K;
        match &e.kind {
            K::Paren { inner } => vec![inner],
            K::TimeLit { num, .. } => vec![num],
            K::Unary { operand, .. } => vec![operand],
            K::Binary { lhs, rhs, .. } => vec![lhs, rhs],
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => vec![cond, then_e, else_e],
            K::BitSelect { base, index } => vec![base, index],
            K::SysCall { args, .. } | K::Call { args, .. } => args.iter().collect(),
            K::Cast { target, expr } => match target {
                // `N'(e)` folds its WIDTH expression too, so it is a child.
                ast::CastTarget::Size(n) => vec![n, expr],
                _ => vec![expr],
            },
            _ => vec![],
        }
    }

    /// Condition 3's trigger: does `e` contain an operator whose SV result can
    /// exceed its operands' width? `+ - * / % ** <<` and unary `-`/`~` can; bit-wise
    /// `& | ^`, comparisons, logical ops and `>>` cannot grow past what they were
    /// given, so they are transparent here (their operands are still walked).
    ///
    /// A CALL always counts. Its body is arbitrary arithmetic at widths this does not
    /// model, so it must not be able to short-circuit the leaf check — that is how a
    /// `function byte g8(); g8 = (8'd200 + 8'd100) >> 2;` folded 75 where SV says 11.
    fn ast_has_width_growing_op(e: &ast::Expr) -> bool {
        use ast::BinOp as B;
        use ast::ExprKind as K;
        let growing = match &e.kind {
            K::Binary { op, .. } => matches!(
                op,
                B::Add | B::Sub | B::Mul | B::Div | B::Mod | B::Pow | B::Shl | B::AShl
            ),
            K::Unary { op, .. } => matches!(op, ast::UnOp::Minus | ast::UnOp::BitNot),
            K::Call { .. } => true,
            _ => false,
        };
        growing
            || Self::const_fold_children(e)
                .iter()
                .any(|c| Self::ast_has_width_growing_op(c))
    }

    /// Every leaf of `e` is ≥ 32 bits in its self-determined width. Fail-closed:
    /// a shape this does not model is NOT wide.
    fn ast_const_leaves_min32(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        let all = |es: &[&ast::Expr]| es.iter().all(|x| self.ast_const_leaves_min32(x));
        match &e.kind {
            K::IntLit { kind, raw } => parse_int_literal(raw, *kind).is_some_and(|c| c.width >= 32),
            K::TimeLit { num, .. } => all(&[num]),
            K::Paren { inner } => all(&[inner]),
            K::Unary { operand, .. } => all(&[operand]),
            K::Binary { lhs, rhs, .. } => all(&[lhs, rhs]),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => all(&[cond, then_e, else_e]),
            // A parameter's DECLARED width when it has a determinate one; absent ⇒
            // value-inferred, which `param_decl_width` never puts below 32.
            K::Ident(p) if p.segments.len() == 1 => self
                .walk_scopes(&p.segments[0].name, &self.param_meta)
                .is_none_or(|(w, _)| w >= 32),
            // `pkg::X` — the package twin of the arm above, reading the same
            // declared `(width, signed)` the package fold recorded. Keyed by the
            // table `const_eval_in_scope`'s own `PkgScoped` arm folds from, so the
            // two cannot disagree about which constant is being measured.
            K::PkgScoped { pkg, name } => self
                .pkg_const_meta
                .get(&pkg.name)
                .and_then(|m| m.get(&name.name))
                .is_none_or(|(w, _)| *w >= 32),
            // `$clog2`/`$bits` yield a 32-bit integer, but their ARGUMENTS still
            // have to be exact — that is the `$clog2(4'd15 + 4'd15)` case.
            K::SysCall { args, .. } => args.iter().all(|a| self.ast_const_leaves_min32(a)),
            K::Cast { target, expr } => {
                all(&[expr])
                    && match target {
                        ast::CastTarget::Prim(p) => {
                            cast_prim_wsign(*p).is_some_and(|(w, _, _)| w >= 32)
                        }
                        ast::CastTarget::Size(n) => {
                            self.const_eval_in_scope(n).is_some_and(|n| n >= 32)
                        }
                        // These do not fold in this domain at all (see
                        // `const_eval_cast`), so their width never matters.
                        ast::CastTarget::Signing { .. } | ast::CastTarget::Named(_) => false,
                    }
            }
            // A constant function is only as exact as its BODY — see
            // [`Self::const_fn_call_width_safe`].
            K::Call { name, args } if name.segments.len() == 1 => {
                args.iter().all(|a| self.ast_const_leaves_min32(a))
                    && self.const_fn_call_width_safe(&name.segments[0].name, 0)
            }
            _ => false,
        }
    }

    /// May a call to constant function `name` be folded in the width-unlimited
    /// domain? `eval_const_call` interprets the body there and never coerces an
    /// assignment to its target's declared width, so a single narrow ASSIGNMENT
    /// TARGET is enough to diverge — `int f(); bit [3:0] t; t = 4'd15 + 4'd15;
    /// return t;` is 14 in SV and 30 here, and a ≥32-bit RETURN does not save it.
    ///
    /// Narrow OPERANDS are fine (measured): SV evaluates a const-function
    /// assignment's RHS at the LHS's width, so `int f(); f = 4'd15 + 4'd15;` is 30
    /// in iverilog too. The property is therefore about targets, not operands.
    ///
    /// Every declaration the interpreter can bind has to be checked, not just
    /// `body_decls`: `exec_const_stmt` also binds decls in NESTED blocks and in a
    /// `for` init, and a wide wrapper (`int f(); f = g8();`) would otherwise
    /// launder a narrow callee — both were live wrong-non-zero folds. So the walk
    /// recurses through the body's statements and through every callee, with a
    /// depth cap that declines rather than recursing forever on a cycle.
    fn const_fn_call_width_safe(&self, name: &str, depth: u32) -> bool {
        const MAX_DEPTH: u32 = 8;
        if depth >= MAX_DEPTH {
            return false;
        }
        let Some(f) = self.const_func_table.get(name) else {
            return false;
        };
        self.const_fn_ret_wsign(f).is_some_and(|(w, _)| w >= 32)
            && f.ports
                .iter()
                .all(|p| Self::decl_is_wide(p.net_or_var, p.range.as_ref()))
            && f.body_decls
                .iter()
                .all(|d| Self::decl_is_wide(Some(d.kind), d.range.as_ref()))
            && self.const_stmt_width_safe(&f.body, depth)
    }

    /// A declared item is at least 32 bits wide. An unmodeled kind (real, string,
    /// a non-literal range) is NOT wide — `eval_const_call` already refuses those,
    /// so declining here only ever costs a fold it would have refused anyway.
    fn decl_is_wide(k: Option<ast::NetVarKind>, r: Option<&ast::Range>) -> bool {
        k.and_then(|k| ast_kind_range_width(k, r))
            .is_some_and(|w| w >= 32)
    }

    /// Statement half of [`Self::const_fn_call_width_safe`]: every declaration the
    /// interpreter binds is ≥32 bits and every callee is itself width-safe.
    fn const_stmt_width_safe(&self, s: &ast::Stmt, depth: u32) -> bool {
        use ast::Stmt as S;
        let decls_ok = |ds: &[ast::NetVarDecl]| {
            ds.iter()
                .all(|d| Self::decl_is_wide(Some(d.kind), d.range.as_ref()))
        };
        match s {
            S::Block { decls, stmts, .. } => {
                decls_ok(decls) && stmts.iter().all(|st| self.const_stmt_width_safe(st, depth))
            }
            S::Blocking { rhs, .. } => self.expr_calls_width_safe(rhs, depth),
            S::Return { value, .. } => value
                .as_ref()
                .is_none_or(|e| self.expr_calls_width_safe(e, depth)),
            S::If {
                cond,
                then_s,
                else_s,
                ..
            } => {
                self.expr_calls_width_safe(cond, depth)
                    && self.const_stmt_width_safe(then_s, depth)
                    && else_s
                        .as_ref()
                        .is_none_or(|e| self.const_stmt_width_safe(e, depth))
            }
            S::While { cond, body, .. } => {
                self.expr_calls_width_safe(cond, depth) && self.const_stmt_width_safe(body, depth)
            }
            S::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                self.const_stmt_width_safe(init, depth)
                    && self.expr_calls_width_safe(cond, depth)
                    && self.const_stmt_width_safe(step, depth)
                    && self.const_stmt_width_safe(body, depth)
            }
            S::Repeat { count, body, .. } => {
                self.expr_calls_width_safe(count, depth) && self.const_stmt_width_safe(body, depth)
            }
            S::Null(_) => true,
            // Anything else is outside `exec_const_stmt`'s modeled subset, so the
            // call cannot fold at all — declining here changes nothing.
            _ => false,
        }
    }

    /// Every constant-function CALL reachable from `e` is width-safe. Only calls
    /// matter here: narrow operands are handled by the assignment's own width.
    fn expr_calls_width_safe(&self, e: &ast::Expr, depth: u32) -> bool {
        let here = match &e.kind {
            ast::ExprKind::Call { name, .. } if name.segments.len() == 1 => {
                self.const_fn_call_width_safe(&name.segments[0].name, depth + 1)
            }
            _ => true,
        };
        here && Self::const_fold_children(e)
            .iter()
            .all(|c| self.expr_calls_width_safe(c, depth))
    }
}
