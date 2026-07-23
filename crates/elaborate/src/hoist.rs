//! call hoisting — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// R5-B: if `e` is a direct call `f(args)` to a framed function with an
    /// output/inout formal, return its `(FuncId, FunctionDef)`.
    pub(crate) fn inout_call_target(&self, e: &ast::Expr) -> Option<(u32, ast::FunctionDef)> {
        if let ast::ExprKind::Call { name, .. } = &e.kind {
            if name.segments.len() == 1 {
                let n = &name.segments[0].name;
                if self.inout_func_names.contains(n) {
                    let fid = *self.frame_idx.get(n)?;
                    let func = self.func_table.get(n)?.clone();
                    return Some((fid, func));
                }
            }
        }
        None
    }

    /// R5-B: does `e`'s subtree contain a call to an inout-bearing function?
    pub(crate) fn expr_has_inout_call(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        if self.inout_call_target(e).is_some() {
            return true;
        }
        match &e.kind {
            K::Unary { operand, .. } => self.expr_has_inout_call(operand),
            K::Binary { lhs, rhs, .. } => {
                self.expr_has_inout_call(lhs) || self.expr_has_inout_call(rhs)
            }
            K::Paren { inner } => self.expr_has_inout_call(inner),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.expr_has_inout_call(cond)
                    || self.expr_has_inout_call(then_e)
                    || self.expr_has_inout_call(else_e)
            }
            _ => false,
        }
    }

    /// R5-B: does `e` contain an inout-call in a position `hoist_inout_calls` does
    /// NOT hoist (a short-circuit `&&`/`||` RHS, a `?:` arm, or any node other than
    /// Binary/Unary/Paren)? If so, the statement must NOT be hoisted at all — a
    /// partial hoist would emit some calls then leave the un-hoistable one in place
    /// (and re-entering the pre-pass would loop). Returning `true` makes
    /// `hoist_stmt_top` decline, so the whole expression lowers normally and the
    /// un-hoistable call loud-rejects at `emit_frame_call` (correct-or-loud).
    pub(crate) fn has_unhoistable_inout_call(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        // A call `e` itself IS hoistable (handled at the top of `hoist_inout_calls`).
        if self.inout_call_target(e).is_some() {
            return false;
        }
        match &e.kind {
            K::Binary { op, lhs, rhs } => {
                let rhs_bad = if matches!(op, ast::BinOp::LogAnd | ast::BinOp::LogOr) {
                    // `&&`/`||` RHS is only conditionally evaluated ⇒ never hoisted.
                    self.expr_has_inout_call(rhs)
                } else {
                    self.has_unhoistable_inout_call(rhs)
                };
                self.has_unhoistable_inout_call(lhs) || rhs_bad
            }
            K::Unary { operand, .. } => self.has_unhoistable_inout_call(operand),
            K::Paren { inner } => self.has_unhoistable_inout_call(inner),
            // Any other node (Ternary, Concat, a non-inout Call's args, …) is not a
            // hoist site — an inout-call anywhere inside is un-hoistable.
            _ => self.expr_has_inout_call(e),
        }
    }

    /// R5-B: is it SAFE to hoist the inout-calls out of `e`? A hoist moves a call's
    /// copy-out (its output/inout side-effect) to BEFORE the whole expression, but
    /// IEEE evaluates the expression's operands in place, left-to-right. So if any
    /// OTHER part of `e` READS a variable a hoisted call MUTATES, that read would see
    /// the post-call value instead of the in-order (pre-call, if to the left) value —
    /// a silent eval-order wrong (`y = x + f(x)` must be `x_old + f(x)`, not
    /// `x_new + …`). Decline the hoist in that case → the call loud-rejects at
    /// `emit_frame_call`. Conservative: also declines a harmless read to the RIGHT of
    /// the call (which would in fact be correct) — acceptable (correct-or-loud).
    pub(crate) fn hoist_is_safe(&self, e: &ast::Expr) -> bool {
        let mut mutated: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        self.collect_inout_mutated(e, &mut mutated);
        !mutated.iter().any(|v| self.reads_ident_outside_inout(e, v))
    }

    /// R5-B: collect the root net names of every output/inout ACTUAL of every
    /// inout-call in `e` — the variables a hoist of those calls would mutate.
    pub(crate) fn collect_inout_mutated(
        &self,
        e: &ast::Expr,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        use ast::ExprKind as K;
        if let Some((_fid, func)) = self.inout_call_target(e) {
            if let K::Call { args, .. } = &e.kind {
                for (p, a) in func.ports.iter().zip(args.iter()) {
                    if !matches!(p.dir, ast::PortDir::Input) {
                        if let Some(root) = expr_root_ident(a) {
                            out.insert(root);
                        }
                    }
                }
            }
            return; // the call's own args are the mutated set; don't double-count
        }
        match &e.kind {
            K::Unary { operand, .. } => self.collect_inout_mutated(operand, out),
            K::Binary { lhs, rhs, .. } => {
                self.collect_inout_mutated(lhs, out);
                self.collect_inout_mutated(rhs, out);
            }
            K::Paren { inner } => self.collect_inout_mutated(inner, out),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.collect_inout_mutated(cond, out);
                self.collect_inout_mutated(then_e, out);
                self.collect_inout_mutated(else_e, out);
            }
            _ => {}
        }
    }

    /// R5-B: does `e` read `name` in a position OUTSIDE any inout-call subtree? (An
    /// inout-call's own args are its copy-in, evaluated at the hoisted call site, so
    /// they are skipped; every other read is evaluated in place and matters for the
    /// hoist-safety check.) Mirrors `expr_reads_ident` minus the inout-call subtrees.
    pub(crate) fn reads_ident_outside_inout(&self, e: &ast::Expr, name: &str) -> bool {
        use ast::ExprKind as K;
        if self.inout_call_target(e).is_some() {
            return false;
        }
        match &e.kind {
            K::Ident(p) => p.segments.len() == 1 && p.segments[0].name == name,
            K::Unary { operand, .. } => self.reads_ident_outside_inout(operand, name),
            K::Binary { lhs, rhs, .. } => {
                self.reads_ident_outside_inout(lhs, name)
                    || self.reads_ident_outside_inout(rhs, name)
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.reads_ident_outside_inout(cond, name)
                    || self.reads_ident_outside_inout(then_e, name)
                    || self.reads_ident_outside_inout(else_e, name)
            }
            K::BitSelect { base, index } => {
                self.reads_ident_outside_inout(base, name)
                    || self.reads_ident_outside_inout(index, name)
            }
            K::PartSelect { base, msb, lsb } => {
                self.reads_ident_outside_inout(base, name)
                    || self.reads_ident_outside_inout(msb, name)
                    || self.reads_ident_outside_inout(lsb, name)
            }
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                self.reads_ident_outside_inout(base, name)
                    || self.reads_ident_outside_inout(offset, name)
                    || self.reads_ident_outside_inout(width, name)
            }
            K::Concat { parts } => parts
                .iter()
                .any(|x| self.reads_ident_outside_inout(x, name)),
            K::Replicate { count, value } => {
                self.reads_ident_outside_inout(count, name)
                    || value
                        .iter()
                        .any(|x| self.reads_ident_outside_inout(x, name))
            }
            K::Call { args, .. } | K::SysCall { args, .. } => {
                args.iter().any(|x| self.reads_ident_outside_inout(x, name))
            }
            K::Paren { inner } => self.reads_ident_outside_inout(inner, name),
            K::MinTypMax { min, typ, max } => {
                self.reads_ident_outside_inout(min, name)
                    || self.reads_ident_outside_inout(typ, name)
                    || self.reads_ident_outside_inout(max, name)
            }
            K::Cast { target, expr } => {
                self.reads_ident_outside_inout(expr, name)
                    || matches!(target, ast::CastTarget::Size(s) if self.reads_ident_outside_inout(s, name))
            }
            // A method call / `new` / assignment-pattern reads its receiver + args —
            // MUST be walked (a mutated var read here is the eval-order hazard the
            // soundness review flagged: `y = obj.m(x) + f(x)` would silently read the
            // post-`f` x). `Dist` `value` likewise.
            K::MethodCall { recv, args, .. } => {
                self.reads_ident_outside_inout(recv, name)
                    || args.iter().any(|x| self.reads_ident_outside_inout(x, name))
            }
            K::New { size, src } => {
                self.reads_ident_outside_inout(size, name)
                    || src
                        .as_ref()
                        .is_some_and(|s| self.reads_ident_outside_inout(s, name))
            }
            K::ClassNew { args } => args.iter().any(|x| self.reads_ident_outside_inout(x, name)),
            K::NamedArg { value, .. } => value
                .as_ref()
                .is_some_and(|v| self.reads_ident_outside_inout(v, name)),
            K::AssignPattern(parts) => parts
                .iter()
                .any(|x| self.reads_ident_outside_inout(x, name)),
            K::Dist { value, .. } => self.reads_ident_outside_inout(value, name),
            // Leaves that read no variable → cannot read `name`.
            K::IntLit { .. }
            | K::RealLit { .. }
            | K::StrLit { .. }
            | K::TimeLit { .. }
            | K::PkgScoped { .. }
            | K::Null
            | K::Dollar
            | K::Error => false,
            // Any OTHER kind (RandomizeWith / ArrayMethodWith / a future node) may
            // read the variable in a way this walker does not model — assume it does
            // so the hoist is DECLINED (→ loud), never silently mis-ordered.
            _ => true,
        }
    }

    /// R5-B: rewrite `e`, hoisting each inout-function call in an unconditionally-
    /// evaluated position to a fresh temp — emitting its copy-out `Terminator::Call`
    /// (`emit_frame_func_out_call`) so the surrounding expression lowers as a plain
    /// read of the temp. An inout-call in a SHORT-CIRCUIT operand (`&&`/`||` RHS,
    /// `?:` arms) or any position not walked here is left in place → it reaches
    /// `emit_frame_call` and is loud (correct-or-loud: never a conditional call
    /// silently made unconditional).
    pub(crate) fn hoist_inout_calls(&mut self, b: &mut ProcessBuilder, e: &ast::Expr) -> ast::Expr {
        use ast::ExprKind as K;
        if let Some((fid, func)) = self.inout_call_target(e) {
            let args = match &e.kind {
                K::Call { args, .. } => args.clone(),
                _ => unreachable!(),
            };
            let (rw, rsig) = self
                .func_metas
                .get(fid as usize)
                .map(|m| (m.ret_width, m.ret_signed))
                .unwrap_or((32, true));
            let (tmp_net, tmp_name) = self.fresh_ret_temp(&func, rw, rsig);
            self.emit_frame_func_out_call(b, fid, &func, &args, whole_net_lvalue(tmp_net));
            return ast::Expr {
                kind: K::Ident(ast::HierPath {
                    segments: vec![ast::Ident {
                        name: tmp_name,
                        span: e.span,
                    }],
                    span: e.span,
                }),
                span: e.span,
            };
        }
        match &e.kind {
            K::Binary { op, lhs, rhs } => {
                let l = self.hoist_inout_calls(b, lhs);
                // `&&`/`||` short-circuit: the RHS is only conditionally evaluated, so
                // hoisting it (an unconditional call) would change semantics → leave it
                // in place (it will loud-reject at `emit_frame_call` if it is an
                // inout-call).
                let r = if matches!(op, ast::BinOp::LogAnd | ast::BinOp::LogOr) {
                    (**rhs).clone()
                } else {
                    self.hoist_inout_calls(b, rhs)
                };
                ast::Expr {
                    kind: K::Binary {
                        op: *op,
                        lhs: Box::new(l),
                        rhs: Box::new(r),
                    },
                    span: e.span,
                }
            }
            K::Unary { op, operand } => ast::Expr {
                kind: K::Unary {
                    op: *op,
                    operand: Box::new(self.hoist_inout_calls(b, operand)),
                },
                span: e.span,
            },
            K::Paren { inner } => ast::Expr {
                kind: K::Paren {
                    inner: Box::new(self.hoist_inout_calls(b, inner)),
                },
                span: e.span,
            },
            _ => e.clone(),
        }
    }

    /// §4.5.179: if `e` is a direct call `f(args)` to a FRAMED function with an `input`
    /// dyn-array formal (`dyn_formal_func_names` — the exact set §4.5.177 blesses on the
    /// direct-rhs path), return its `(FuncId, FunctionDef)`.
    pub(crate) fn dyn_formal_call_target(&self, e: &ast::Expr) -> Option<(u32, ast::FunctionDef)> {
        if let ast::ExprKind::Call { name, .. } = &e.kind {
            if name.segments.len() == 1 {
                let n = &name.segments[0].name;
                if self.dyn_formal_func_names.contains(n) {
                    let fid = *self.frame_idx.get(n)?;
                    let func = self.func_table.get(n)?.clone();
                    return Some((fid, func));
                }
            }
        }
        None
    }

    /// §4.5.179: does `e`'s subtree contain a framed dyn-formal call?
    pub(crate) fn expr_has_dyn_formal_call(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        if self.dyn_formal_call_target(e).is_some() {
            return true;
        }
        match &e.kind {
            K::Unary { operand, .. } => self.expr_has_dyn_formal_call(operand),
            K::Binary { lhs, rhs, .. } => {
                self.expr_has_dyn_formal_call(lhs) || self.expr_has_dyn_formal_call(rhs)
            }
            K::Paren { inner } => self.expr_has_dyn_formal_call(inner),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.expr_has_dyn_formal_call(cond)
                    || self.expr_has_dyn_formal_call(then_e)
                    || self.expr_has_dyn_formal_call(else_e)
            }
            _ => false,
        }
    }

    /// r18 (F3): is every framed dyn-formal call in `e` to a SIDE-EFFECT-FREE function (no
    /// `$display`/`$error`/… anywhere in its body)? A pure function's VALUE is identical
    /// whether evaluated conditionally or unconditionally, so its call may be hoisted out of
    /// a `?:` arm (a conditionally-evaluated position) safely. An IMPURE function (one that
    /// prints / raises severity) must NOT be hoisted out of a conditional position — the
    /// side effect would fire even when the arm is not taken (silent extra output). Any
    /// unrecognized node carrying a dyn-formal call is conservatively treated as impure.
    pub(crate) fn dyn_formal_expr_all_pure(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        if let Some((_fid, func)) = self.dyn_formal_call_target(e) {
            return !Self::stmt_has_observable_effect(&func.body);
        }
        match &e.kind {
            K::Binary { lhs, rhs, .. } => {
                self.dyn_formal_expr_all_pure(lhs) && self.dyn_formal_expr_all_pure(rhs)
            }
            K::Unary { operand, .. } => self.dyn_formal_expr_all_pure(operand),
            K::Paren { inner } => self.dyn_formal_expr_all_pure(inner),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.dyn_formal_expr_all_pure(cond)
                    && self.dyn_formal_expr_all_pure(then_e)
                    && self.dyn_formal_expr_all_pure(else_e)
            }
            _ => !self.expr_has_dyn_formal_call(e),
        }
    }

    /// §4.5.179 / r18: does `e` contain a framed dyn-formal call in a position
    /// `hoist_dyn_formal_calls` does NOT hoist? Un-hoistable = a short-circuit `&&`/`||`
    /// RHS (would change conditional evaluation for a non-pure function), OR any node other
    /// than Binary/Unary/Paren/Ternary. A `?:` (r18/F3) is hoistable IFF every dyn-formal
    /// call in it is PURE (`dyn_formal_expr_all_pure`) — else the conditional-eval side
    /// effect would leak. Returning `true` makes `hoist_stmt_top` decline → the call stays
    /// loud at `emit_frame_call` (correct-or-loud).
    pub(crate) fn has_unhoistable_dyn_formal_call(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        // A call `e` itself IS hoistable (handled at the top of `hoist_dyn_formal_calls`).
        if self.dyn_formal_call_target(e).is_some() {
            return false;
        }
        match &e.kind {
            K::Binary { op, lhs, rhs } => {
                let rhs_bad = if matches!(op, ast::BinOp::LogAnd | ast::BinOp::LogOr) {
                    // `&&`/`||` RHS is only conditionally evaluated ⇒ never hoisted.
                    self.expr_has_dyn_formal_call(rhs)
                } else {
                    self.has_unhoistable_dyn_formal_call(rhs)
                };
                self.has_unhoistable_dyn_formal_call(lhs) || rhs_bad
            }
            K::Unary { operand, .. } => self.has_unhoistable_dyn_formal_call(operand),
            K::Paren { inner } => self.has_unhoistable_dyn_formal_call(inner),
            // r18 (F3): a `?:` is hoistable when its parts are structurally hoistable AND
            // every dyn-formal call in it is pure (a conditionally-evaluated impure call
            // would leak its side effect if hoisted).
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.has_unhoistable_dyn_formal_call(cond)
                    || self.has_unhoistable_dyn_formal_call(then_e)
                    || self.has_unhoistable_dyn_formal_call(else_e)
                    || !self.dyn_formal_expr_all_pure(e)
            }
            // Any other node (Concat, a non-dyn Call's args, …) is not a hoist site — a
            // dyn-formal call anywhere inside is un-hoistable.
            _ => self.expr_has_dyn_formal_call(e),
        }
    }

    /// §4.5.179: rewrite `e`, hoisting each framed dyn-formal call in an
    /// unconditionally-evaluated position (Binary/Unary/Paren operand) to a fresh temp.
    /// Emits `__t = f(args)` as a DIRECT-rhs blocking assign via `lower_stmt` — which
    /// re-enters the marker path (`emit_frame_dyn_formal_markers`): the direct-rhs guard
    /// there fires §4.5.177's snapshot + blessed bind, so the temp receives the correct
    /// value. The surrounding expression then lowers as a plain read of the temp. A call
    /// in a SHORT-CIRCUIT operand (`&&`/`||` RHS) is left in place → it reaches
    /// `emit_frame_call` and is loud (correct-or-loud). No eval-order guard is needed: a
    /// framed function is pure (no output formals), so hoisting its evaluation earlier
    /// never changes another operand's value.
    pub(crate) fn hoist_dyn_formal_calls(
        &mut self,
        b: &mut ProcessBuilder,
        e: &ast::Expr,
    ) -> ast::Expr {
        use ast::ExprKind as K;
        if let Some((fid, func)) = self.dyn_formal_call_target(e) {
            let (rw, rsig) = self
                .func_metas
                .get(fid as usize)
                .map(|m| (m.ret_width, m.ret_signed))
                .unwrap_or((32, true));
            let (_tmp_net, tmp_name) = self.fresh_ret_temp(&func, rw, rsig);
            // `__t = f(args)` — a direct-rhs blocking assign. `lower_stmt`'s hoist gate
            // will NOT re-hoist it (its Blocking arm excludes a rhs that IS the direct
            // call), and its marker path (`emit_frame_dyn_formal_markers`) blesses it.
            let assign = ast::Stmt::Blocking {
                lhs: ast::Lvalue::Ident(ast::HierPath {
                    segments: vec![ast::Ident {
                        name: tmp_name.clone(),
                        span: e.span,
                    }],
                    span: e.span,
                }),
                delay: None,
                event: None,
                rhs: e.clone(),
                span: e.span,
            };
            self.lower_stmt(b, &assign);
            return ast::Expr {
                kind: K::Ident(ast::HierPath {
                    segments: vec![ast::Ident {
                        name: tmp_name,
                        span: e.span,
                    }],
                    span: e.span,
                }),
                span: e.span,
            };
        }
        match &e.kind {
            K::Binary { op, lhs, rhs } => {
                let l = self.hoist_dyn_formal_calls(b, lhs);
                let r = if matches!(op, ast::BinOp::LogAnd | ast::BinOp::LogOr) {
                    (**rhs).clone()
                } else {
                    self.hoist_dyn_formal_calls(b, rhs)
                };
                ast::Expr {
                    kind: K::Binary {
                        op: *op,
                        lhs: Box::new(l),
                        rhs: Box::new(r),
                    },
                    span: e.span,
                }
            }
            K::Unary { op, operand } => ast::Expr {
                kind: K::Unary {
                    op: *op,
                    operand: Box::new(self.hoist_dyn_formal_calls(b, operand)),
                },
                span: e.span,
            },
            K::Paren { inner } => ast::Expr {
                kind: K::Paren {
                    inner: Box::new(self.hoist_dyn_formal_calls(b, inner)),
                },
                span: e.span,
            },
            // r18 (F3): a `?:` — reached only when `has_unhoistable_dyn_formal_call`
            // confirmed every dyn-formal call in it is pure, so hoisting each arm's call to
            // an unconditional temp is result-equivalent (the arm's value is picked as
            // before; the extra evaluation is side-effect-free).
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => ast::Expr {
                kind: K::Ternary {
                    cond: Box::new(self.hoist_dyn_formal_calls(b, cond)),
                    then_e: Box::new(self.hoist_dyn_formal_calls(b, then_e)),
                    else_e: Box::new(self.hoist_dyn_formal_calls(b, else_e)),
                },
                span: e.span,
            },
            _ => e.clone(),
        }
    }

    /// r18 (F3): does `s` (recursively) contain an OBSERVABLE side effect — a `$display`/
    /// `$write`/`$error`/… system task, or a user task/function call statement that might
    /// print? Used to gate hoisting a dyn-formal call out of a conditional (`?:`) position:
    /// only a side-effect-free function may be evaluated unconditionally without changing
    /// behaviour. Conservative (a nested user call is treated as possibly-printing).
    pub(crate) fn stmt_has_observable_effect(s: &ast::Stmt) -> bool {
        use ast::Stmt as S;
        match s {
            S::SysTaskCall { .. } | S::UserTaskCall { .. } => true,
            S::Block { stmts, .. } | S::Fork { stmts, .. } => {
                stmts.iter().any(Self::stmt_has_observable_effect)
            }
            S::If { then_s, else_s, .. } => {
                Self::stmt_has_observable_effect(then_s)
                    || else_s
                        .as_deref()
                        .is_some_and(Self::stmt_has_observable_effect)
            }
            S::Case { items, .. } => items
                .iter()
                .any(|it| Self::stmt_has_observable_effect(case_item_body(it))),
            S::For { body, .. }
            | S::While { body, .. }
            | S::Repeat { body, .. }
            | S::Forever { body, .. } => Self::stmt_has_observable_effect(body),
            S::DelayCtrl { body, .. } | S::EventCtrl { body, .. } | S::Wait { body, .. } => body
                .as_deref()
                .is_some_and(Self::stmt_has_observable_effect),
            _ => false,
        }
    }

    /// R5-B: hoist pre-pass for `lower_stmt` (only entered when `inout_func_names`
    /// is non-empty). For the statement forms whose key expression is evaluated
    /// EXACTLY ONCE (an `if` condition, or a blocking-assign RHS with no event/delay),
    /// rewrite that expression with `hoist_inout_calls` (emitting the copy-out call
    /// before the statement) and return the rewritten statement. (A `case` scrutinee
    /// is NOT hoisted — it stays loud; only `if`/blocking are handled here.)
    /// `while`/`for` conditions are re-evaluated per iteration, so a one-shot hoist
    /// would be wrong — those are loud-rejected. Returns `None` when nothing needs
    /// hoisting (the caller then lowers `s` as-is → byte-identical).
    pub(crate) fn hoist_stmt_top(
        &mut self,
        b: &mut ProcessBuilder,
        s: &ast::Stmt,
    ) -> Option<ast::Stmt> {
        use ast::Stmt as S;
        match s {
            S::If {
                cond,
                then_s,
                else_s,
                span,
            } if self.expr_has_inout_call(cond)
                && !self.has_unhoistable_inout_call(cond)
                && self.hoist_is_safe(cond) =>
            {
                let cond2 = self.hoist_inout_calls(b, cond);
                Some(S::If {
                    cond: cond2,
                    then_s: then_s.clone(),
                    else_s: else_s.clone(),
                    span: *span,
                })
            }
            S::Blocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } if delay.is_none()
                && event.is_none()
                && self.expr_has_inout_call(rhs)
                && !self.has_unhoistable_inout_call(rhs)
                && self.hoist_is_safe(rhs) =>
            {
                let rhs2 = self.hoist_inout_calls(b, rhs);
                Some(S::Blocking {
                    lhs: lhs.clone(),
                    delay: delay.clone(),
                    event: event.clone(),
                    rhs: rhs2,
                    span: *span,
                })
            }
            // §4.5.179: the same one-shot hoist for a BURIED framed dyn-formal call. These
            // arms are reached only when the inout arms above did NOT match (their guard
            // was false), so a design with both kinds still hoists each across passes.
            S::If {
                cond,
                then_s,
                else_s,
                span,
            } if self.expr_has_dyn_formal_call(cond)
                && !self.has_unhoistable_dyn_formal_call(cond) =>
            {
                let cond2 = self.hoist_dyn_formal_calls(b, cond);
                Some(S::If {
                    cond: cond2,
                    then_s: then_s.clone(),
                    else_s: else_s.clone(),
                    span: *span,
                })
            }
            S::Blocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } if delay.is_none()
                && event.is_none()
                && self.expr_has_dyn_formal_call(rhs)
                // EXCLUDE a rhs that IS the direct call — §4.5.177 handles `x = f(arr)`
                // itself (and re-hoisting it would loop). Only a NESTED call is hoisted.
                && self.dyn_formal_call_target(rhs).is_none()
                && !self.has_unhoistable_dyn_formal_call(rhs) =>
            {
                let rhs2 = self.hoist_dyn_formal_calls(b, rhs);
                Some(S::Blocking {
                    lhs: lhs.clone(),
                    delay: delay.clone(),
                    event: event.clone(),
                    rhs: rhs2,
                    span: *span,
                })
            }
            // r18 (F3): the same one-shot hoist for a framed dyn-formal call in a
            // NON-BLOCKING assign rhs (`reg_out <= packk(b)` / `reg_out <= en ? packk(b) : 0`).
            // Unlike blocking, §4.5.177's direct-rhs marker is NOT emitted for `<=`, so a
            // DIRECT-call rhs is hoisted too (no `dyn_formal_call_target(rhs).is_none()`
            // exclusion): the hoist emits `__t = f(arr)` (a blessed blocking assign) then the
            // NBA reads `__t`. The NBA's target-net update still schedules for the region end.
            S::NonBlocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } if delay.is_none()
                && event.is_none()
                && self.expr_has_dyn_formal_call(rhs)
                && !self.has_unhoistable_dyn_formal_call(rhs) =>
            {
                let rhs2 = self.hoist_dyn_formal_calls(b, rhs);
                Some(S::NonBlocking {
                    lhs: lhs.clone(),
                    delay: delay.clone(),
                    event: event.clone(),
                    rhs: rhs2,
                    span: *span,
                })
            }
            // §4.5.179: `$display`/`$write`/`$strobe`/… args are each evaluated once,
            // unconditionally → hoist a framed dyn-formal call out of any of them.
            S::SysTaskCall { name, args, span }
                if args.iter().any(|a| self.expr_has_dyn_formal_call(a))
                    && !args.iter().any(|a| self.has_unhoistable_dyn_formal_call(a)) =>
            {
                let args2 = args
                    .iter()
                    .map(|a| self.hoist_dyn_formal_calls(b, a))
                    .collect();
                Some(S::SysTaskCall {
                    name: name.clone(),
                    args: args2,
                    span: *span,
                })
            }
            // r18 (F1): a `while`/`for` condition with an inout/output-function call is now
            // hoisted at the loop head (`lower_while`/`lower_for`) — the call's copy-out temp
            // is emitted at the condition-eval block, which is re-entered every iteration. No
            // loud here; an eval-order-UNSAFE condition still loud-rejects at `emit_frame_call`.
            _ => None,
        }
    }

    /// R2: does lvalue `lhs` write (the root of) a dyn-array formal of `f`?
    pub(crate) fn lval_is_dyn_formal(&self, lhs: &ast::Lvalue, f: &ast::FunctionDef) -> bool {
        lval_root_name(lhs)
            .map(|r| {
                f.ports
                    .iter()
                    .any(|p| self.is_input_dyn_array_formal(p) && p.name.name == r)
            })
            .unwrap_or(false)
    }

    /// Hoist a top-level `$sformatf(...)` ARGUMENT of an immediate format task
    /// (§4.5.127) to a fresh string temp: emit `tmp = $sformatf(...)` into `b`
    /// (via `sformatf_special` — the pure render runs exactly ONCE, before the
    /// task, matching the immediate task's own single evaluation) and return an
    /// `Ident(tmp)` to pass in its place. `None` ⇒ `arg` is not a bare
    /// `$sformatf` call, so it is left untouched — a NESTED `$sformatf` (inside a
    /// concat/ternary/comparison) still reaches the direct-rhs loud guard in
    /// `lower_expr` (a separate follow-on).
    pub(crate) fn hoist_sformatf_arg(
        &mut self,
        b: &mut ProcessBuilder,
        arg: &ast::Expr,
    ) -> Option<ast::Expr> {
        let ast::ExprKind::SysCall { name, .. } = &arg.kind else {
            return None;
        };
        if name.name != "$sformatf" {
            return None;
        }
        let sp = arg.span;
        let tmp = self.fresh_string_temp();
        let tmp_lv = ast::Lvalue::Ident(ast::HierPath {
            segments: vec![ast::Ident {
                name: tmp.clone(),
                span: sp,
            }],
            span: sp,
        });
        // Reuse the direct-rhs handler: it validates the string-literal format,
        // lowers the args, and emits `tmp = $sformatf(...)`. (A non-literal format
        // emits its OWN loud error there, identical to a direct-rhs assignment.)
        self.sformatf_special(b, &tmp_lv, None, arg);
        Some(ast::Expr {
            kind: ast::ExprKind::Ident(ast::HierPath {
                segments: vec![ast::Ident {
                    name: tmp,
                    span: sp,
                }],
                span: sp,
            }),
            span: sp,
        })
    }
}
