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
        // §11.6: a bare FILL literal in a self-determined position is ONE bit —
        // `'1` is the value 1, `'0` is 0 — while the generic const-eval reads a
        // 32-bit all-ones pattern. That mis-answer was latent while nothing
        // consumed the funnel's fill value (`repeat` special-cases fills first,
        // and the index funnel used to prefer the lowered 1-bit node); the
        // agreement substitution in `lower_index_expr` made it live —
        // `vec['1 +: 4]` handed the engine a 4294967295 offset (a shift-overflow
        // panic). `'x`/`'z` have no index value and decline.
        if let Some((raw, kind)) = fill_literal_ast(e) {
            let cv = literal::fill_literal_const(raw, kind, 1)?;
            if cv.bits.unk.iter().any(|&u| u != 0) {
                return None;
            }
            return Some((cv.bits.val.first().copied().unwrap_or(0) & 1) as u32);
        }
        if let Some(v) = const_eval_u32(e) {
            return Some(v);
        }
        if self.const_fold_is_width_exact(e) {
            return u32::try_from(self.const_eval_in_scope(e)?).ok();
        }
        // Width-INEXACT shapes used to decline here, and the consumers' silent
        // `unwrap_or(1)`/`unwrap_or(0)` defaults were the §2 defect: a `**` in a
        // replication count (`{(8'd2 ** 8'd3){1'b1}}` → empty) or an indexed
        // part-select width (`v[0 +: (8'd2 ** 8'd3)]` → 1 bit). A bound/count is
        // its own context (§11.6.1 — no outer width reaches it), so the
        // self-determined walk (§4.5.343) computes the SV value, wraps included:
        // `repeat (4'd15 + 4'd1)` folds the 4-bit 0 iverilog runs.
        //
        // Gated on EVERY foldable node having a known self width: the walk
        // DEGRADES to the width-unlimited domain where a width is unknown (a
        // const-array element, an unmodeled leaf), and an unlimited value in a
        // wrap-sensitive bound would trade the consumer's silent default for a
        // differently-silent unwrapped value. Substituted names decline for the
        // same reason condition 1 gives the tier above: this walk resolves a
        // bare ident through the param scope, not the inline substitution stack.
        if self.ast_mentions_substituted_name(e) || !self.ast_selfwidths_all_known(e) {
            return None;
        }
        u32::try_from(self.const_int_selfdet(e)?).ok()
    }

    /// The SIGNED self-determined value of a bound / count expression, under exactly
    /// the admission [`Self::const_bound_u32`] gives its own tier-3 — `None` when the
    /// walk would degrade to the width-unlimited domain.
    ///
    /// The u32 twin cannot answer "is this negative?": it declines on a negative and
    /// its consumers then read the two's-complement bit pattern (a replication count
    /// of `-56` became 4294967240). Answering needs the value's SIGN, and the sign
    /// only exists at the expression's own width — `4'd0 - 4'd1` is 15 (both oracles
    /// replicate 15 times) while `4'sd0 - 4'sd1` is -1 (both oracles reject), and the
    /// width-unlimited fold calls BOTH of them -1. So: same guards, signed result.
    pub(crate) fn const_bound_signed(&self, e: &ast::Expr) -> Option<i64> {
        // A fill literal is one bit in a self-determined position and never negative;
        // it also has no `const_self_width`, so the guard below would decline anyway.
        if fill_literal_ast(e).is_some() {
            return None;
        }
        if self.ast_mentions_substituted_name(e) || !self.ast_selfwidths_all_known(e) {
            return None;
        }
        self.const_int_selfdet(e)
    }

    /// Does EVERY sub-expression the const domain descends into have a known,
    /// non-zero self width? This is what makes the self-determined walk mask at
    /// every step instead of degrading to the unlimited domain somewhere inside
    /// (its documented contract) — the tier-3 bound fold requires it.
    fn ast_selfwidths_all_known(&self, e: &ast::Expr) -> bool {
        self.const_self_width(e, &ConstWidths::new())
            .is_some_and(|w| w >= 1)
            && Self::const_fold_children(e)
                .iter()
                .all(|c| self.ast_selfwidths_all_known(c))
    }

    /// How many times `lower_repeat` UNROLLS this count — `None` when it does not
    /// (a runtime, unfoldable or large count, which desugars to the shared
    /// `$repeat_cnt$` net instead).
    ///
    /// ⚠️⚠️ ONE SPELLING, and that is the whole point. Two callers ask this: the
    /// LOWERING (`lower_repeat`, which builds the code) and the CLASSIFIER
    /// (`ast_has_repeat_with_timing`, which decides whether a suspendable task may
    /// contain the `repeat` at all — a runtime counter is a module net, so it would
    /// corrupt across concurrent activations, but an unrolled count carries no
    /// counter and is safe). A second spelling makes the classifier reject a shape
    /// the lowering would have unrolled. That was live: the classifier had no
    /// fill-literal arm, so `repeat('1) @(posedge clk)` in a `task automatic` was
    /// E3009 here and ran ONE iteration in iverilog.
    ///
    /// The count goes through [`Self::const_bound_u32`], the same funnel every
    /// select bound and replication count uses — so `repeat (4*16)`, `repeat (LP)`,
    /// `repeat (LP/2)` and `repeat ($clog2(LP))` fold, while its width-exactness
    /// guard keeps `repeat (4'd15 + 4'd1)` OUT of the strong domain: SV wraps that
    /// to 0 at four bits (iverilog runs the body zero times) and the unlimited i64
    /// fold would say 16. Declining leaves it on the runtime-counter path, which is
    /// loud inside a frame — loud, not wrong.
    pub(crate) fn repeat_unroll_count(&self, count: &ast::Expr) -> Option<u32> {
        // §11.6: a fill literal count is self-determined to ONE bit, so `repeat('1)`
        // is one iteration (the generic const-eval would read a 32-bit all-ones value
        // and skip the loop); `'0`/`'x`/`'z` ⇒ zero iterations.
        if let Some((raw, kind)) = fill_literal_ast(count) {
            let once = literal::fill_literal_const(raw, kind, 1)
                .map(|cv| {
                    cv.bits.unk.iter().all(|&u| u == 0)
                        && (cv.bits.val.first().copied().unwrap_or(0) & 1) == 1
                })
                .unwrap_or(false);
            return Some(u32::from(once));
        }
        match self.const_bound_u32(count) {
            Some(n) if n <= REPEAT_UNROLL_CAP => Some(n),
            _ => None,
        }
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
    /// reducible, so this can never paper over one. (Width-blind shallow
    /// reductions — `Const 4'd15 + Const 4'd1` as 16 where SV wraps to 0 — are
    /// corrected inside `lower_index_expr` itself, the one funnel every index,
    /// bound, offset and width site shares.)
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
            // The part / indexed-part twins of the arm above, restoring this
            // function's stated COMPLETENESS invariant now that `const_eval_in_scope`
            // folds those kinds.
            //
            // ⚠️ The invariant is what makes them required, NOT a specific hazard.
            // An earlier comment here claimed they were load-bearing for
            // `ast_mentions_substituted_name`; the mutation battery could not build a
            // case where that is what the arms deliver (`const_select_base` and
            // `param_sel_range` both refuse a substituted name on their own), and the
            // only test that kills their removal is the >64-bit DIAGNOSTIC one, via
            // `wide_param_name_in`. Recorded rather than left overclaiming — the arms
            // stay because the invariant is the contract three predicates read.
            K::PartSelect { base, msb, lsb } => vec![base, msb, lsb],
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => vec![base, offset, width],
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

impl Elaborator<'_> {
    /// A short rendering of an expression NODE, for a diagnostic that has to say which
    /// sub-expression it is talking about.
    ///
    /// Deliberately shallow: one level of structure and then `…`. The point is to let
    /// the reader find the operand in a long initializer, not to reproduce it.
    pub(crate) fn expr_brief(e: &ast::Expr) -> String {
        use ast::ExprKind as K;
        let seg = |p: &ast::HierPath| {
            p.segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(".")
        };
        match &e.kind {
            K::Paren { inner } => Self::expr_brief(inner),
            K::IntLit { raw, .. } | K::RealLit { raw, .. } => raw.clone(),
            K::StrLit { .. } => "a string literal".to_string(),
            K::Ident(p) => format!("`{}`", seg(p)),
            K::PkgScoped { pkg, name } => format!("`{}::{}`", pkg.name, name.name),
            K::SysCall { name, .. } => format!("`{}(…)`", name.name),
            K::Call { name, .. } => format!("`{}(…)`", seg(name)),
            K::MethodCall { recv, method, .. } => {
                format!("the method call `{}.{}(…)`", Self::plain(recv), method.name)
            }
            K::BitSelect { base, .. } => format!("the select `{}[…]`", Self::plain(base)),
            K::PartSelect { base, .. } | K::IndexedPart { base, .. } => {
                format!("the part-select `{}[…]`", Self::plain(base))
            }
            K::Concat { .. } => "the concatenation `{…}`".to_string(),
            K::Replicate { .. } => "the replication `{n{…}}`".to_string(),
            K::Binary { op, .. } => format!("the `{}` operation", bin_op_text(*op)),
            K::Unary { op, .. } => format!("the `{}` operation", un_op_text(*op)),
            K::Ternary { .. } => "the conditional `? :`".to_string(),
            K::Cast { .. } => "the cast".to_string(),
            _ => "this sub-expression".to_string(),
        }
    }

    /// `expr_brief` without the surrounding words — for embedding inside a bigger
    /// rendering (`A[…]` rather than "the select `A[…]`").
    fn plain(e: &ast::Expr) -> String {
        use ast::ExprKind as K;
        match &e.kind {
            K::Paren { inner } => Self::plain(inner),
            K::Ident(p) => p
                .segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join("."),
            K::PkgScoped { pkg, name } => format!("{}::{}", pkg.name, name.name),
            _ => "…".to_string(),
        }
    }

    /// Does the WIDE bit domain have an answer for this sub-expression?
    ///
    /// ⚠️⚠️ [`Self::unfoldable_reason`] used to ask only the i64 domain, so every
    /// operand of a >64-bit expression looked like a cause. `wide_param_name_in_pub`
    /// then blamed the first wide NAME anywhere in the tree, which is how
    /// `localparam logic C2 = A[128];` reported *"`A` is wider than 64 bits, so it has
    /// no integral constant value here"* — true of `A[127]` as well, and that folds.
    /// A sub-expression the wide domain answers is not the reason the enclosing one
    /// failed, so the walk steps over it and keeps looking outward.
    ///
    /// ⚠️ An x/z-carrying fold is NOT an answer. `fold_self_bits` returns `Some` for
    /// `128'hx` — it carries the unknown plane faithfully — but every VALUE-reading
    /// consumer declines it, so treating it as "answered" promoted the operator above
    /// it into the message and `128'hx / 128'd3` blamed the `/` instead of the operand.
    pub(crate) fn wide_self_folds(&self, e: &ast::Expr) -> bool {
        fold_self_bits(e, &|n, _| self.wide_name_bits(n))
            .is_some_and(|(b, w, _)| !bp_any_unknown(&b, w))
    }

    /// The 2-state value of a count/index sub-expression as a `u128`, or `None` when
    /// it is not a foldable 2-state constant or needs more than 128 bits. Used only
    /// to WORD a diagnostic — never to fold.
    fn count_value_for_msg(&self, e: &ast::Expr) -> Option<u128> {
        if let Some(v) = self.const_eval_in_scope(e) {
            return u128::try_from(v).ok();
        }
        let (b, w, _) = fold_self_bits(e, &|n, _| self.wide_name_bits(n))?;
        if bp_any_unknown(&b, w) || (128..w as usize).any(|i| bp_get(&b, i).0) {
            return None;
        }
        let mut v: u128 = 0;
        for i in 0..(w as usize).min(128) {
            if bp_get(&b, i).0 {
                v |= 1u128 << i;
            }
        }
        Some(v)
    }

    /// Name the real cause when a SELECT or a REPLICATION did not fold because its
    /// index / count is out of range rather than because an arm is missing.
    ///
    /// Both halves are reachable only after [`Self::wide_self_folds`] has cleared the
    /// operands, so anything this reports is genuinely about the index or the count.
    fn select_or_count_reason(&self, e: &ast::Expr) -> Option<String> {
        use ast::ExprKind as K;
        // The base's width, from whichever domain can state it.
        let base_width = |b: &ast::Expr| -> Option<u32> {
            fold_self_bits(b, &|n, _| self.wide_name_bits(n)).map(|(_, w, _)| w)
        };
        let over_u32 = |i: &ast::Expr| -> Option<u128> {
            self.count_value_for_msg(i)
                .filter(|v| *v > u128::from(u32::MAX))
        };
        match &e.kind {
            K::BitSelect { base, index } => {
                let idx = self.count_value_for_msg(index)?;
                let w = base_width(base)?;
                if idx >= u128::from(w) {
                    return Some(format!(
                        "the select index {idx} is outside {}'s range [{}:0] — IEEE 1800 \
                         §11.5.1 makes an out-of-range select `x`, which is not a value \
                         a parameter can hold",
                        Self::expr_brief(base),
                        w - 1
                    ));
                }
                None
            }
            K::PartSelect { base, msb, lsb } => {
                let (m, l) = (
                    self.count_value_for_msg(msb)?,
                    self.count_value_for_msg(lsb)?,
                );
                let w = base_width(base)?;
                if m >= u128::from(w) || l > m {
                    return Some(format!(
                        "the part-select [{m}:{l}] is outside {}'s range [{}:0]",
                        Self::expr_brief(base),
                        w - 1
                    ));
                }
                None
            }
            K::IndexedPart { base, offset, .. } => {
                let v = over_u32(offset)?;
                Some(format!(
                    "the indexed part-select base {v} does not fit the 32-bit index \
                     channel a select travels in ({} is selected from)",
                    Self::expr_brief(base)
                ))
            }
            K::Replicate { count, .. } => {
                let v = over_u32(count)?;
                Some(format!(
                    "the replication count {v} does not fit 32 bits, so the repetition \
                     cannot be built (verilator refuses the same count)"
                ))
            }
            _ => None,
        }
    }

    /// WHY a constant expression did not fold — the first sub-expression the constant
    /// domain has no answer for, deepest first.
    ///
    /// ⭐ The message this feeds used to say only that the parameter's value "is not a
    /// foldable constant expression". Three declarations rejected for three unrelated
    /// reasons — a package `real`, a string method, a replication count — printed the
    /// same twelve words, and not one of `real`, `string` or `replication` appeared in
    /// any of them. Worse, when the cause was an UNDEFINED NAME the name was in hand
    /// and thrown away.
    ///
    /// Children first, so the answer is the innermost thing that failed rather than
    /// the whole initializer. Returns `None` when every part folds — then the caller
    /// keeps its unqualified wording, which is honest: nothing here can name a cause
    /// it did not find.
    /// Is `e` a constant ZERO in EITHER constant domain? Diagnostic-only, and
    /// deliberately conservative: an expression that does not fold at all is not a
    /// known zero, so the caller falls back to its generic wording.
    fn divisor_is_zero(&self, e: &ast::Expr) -> bool {
        self.const_eval_in_scope(e) == Some(0) || self.wide_domain_is_zero(e)
    }

    pub(crate) fn unfoldable_reason(&self, e: &ast::Expr) -> Option<String> {
        if let Some(r) = Self::const_fold_children(e)
            .into_iter()
            // ⚠️ A child the WIDE bit domain can fold is not the failure, even though
            // the integral walk below declines it. Without this skip the ONE >64-bit
            // operand in the expression answers for the whole thing: `A / 0` blamed
            // `A`'s width, which is a fact about a name this elaborator reads without
            // difficulty, and said nothing about the zero divisor that is the actual
            // reason. Same shape as any stale proxy — the membership test kept
            // standing in for a property that had moved.
            .filter(|c| !self.wide_domain_folds(c))
            .find_map(|c| self.unfoldable_reason(c))
        {
            return Some(r);
        }
        if self.const_eval_in_scope(e).is_some() || self.wide_self_folds(e) {
            return None;
        }
        // The domains that HAVE a value but not an integral one get named for what
        // they are; a caller reading "not foldable" about `pk::R` would look for a
        // typo in a name that is right there.
        if self.count_reads_real_param(e) {
            return Some(format!(
                "{} is a real, which has no integral constant value here",
                Self::expr_brief(e)
            ));
        }
        // A COUNT or an INDEX that no domain can carry — asked BEFORE the two
        // "this operand has no integral value" blames below, because both of those
        // name an OPERAND and the cause here is the index. Measured: `A[128]` over a
        // 128-bit `A` said *"`A` is wider than 64 bits"* (true of `A[127]` as well,
        // and that folds), and `B[9]` over an 8-bit `B` said *"the select `B[…]` has
        // no constant-fold arm"* (the arm exists — `B[3]` folds).
        if let Some(r) = self.select_or_count_reason(e) {
            return Some(r);
        }
        use ast::ExprKind as K;
        // ⚠️ Only when `e` IS that name. A COMPOUND expression holding a wide name
        // fails for a reason of its own — the operator has no wide arm, the divisor
        // is zero, the select runs backwards — and blaming the operand's width sent
        // the author to look at a declaration that is fine. A bare name, on the other
        // hand, really does fail for its width: the consumer wants an integral value
        // and 128 bits is not one.
        if matches!(e.kind, K::Ident(_) | K::PkgScoped { .. }) {
            if let Some(n) = self.wide_param_name_in_pub(e) {
                return Some(format!(
                    "`{n}` is wider than 64 bits, so it has no integral constant value here"
                ));
            }
        }
        // §11.4.3 makes `x / 0` and `x % 0` X, so there is no constant to fold — and
        // that is a different sentence from "no fold arm", which is what the operator
        // catch-all below would have said about an operator that has one.
        if let K::Binary { op, rhs, .. } = &e.kind {
            if matches!(op, ast::BinOp::Div | ast::BinOp::Mod) && self.divisor_is_zero(rhs) {
                return Some(format!(
                    "{} divides by zero, which is `x` and not a constant",
                    Self::expr_brief(e)
                ));
            }
        }
        Some(match &e.kind {
            K::Ident(_) | K::PkgScoped { .. } => match self.nonconst_bound_reason(e) {
                Some(r) => format!("{r} is not a constant"),
                None => format!("{} is not a constant here", Self::expr_brief(e)),
            },
            K::MethodCall { .. } => format!(
                "{} has no constant-fold arm (its runtime spelling works)",
                Self::expr_brief(e)
            ),
            K::StrLit { .. } => "a string literal has no integral constant value".to_string(),
            K::RealLit { .. } => "a real literal has no integral constant value here".to_string(),
            _ => format!("{} has no constant-fold arm", Self::expr_brief(e)),
        })
    }

    /// `"<what> is not a constant[: <why>]"` — the message a const-fold rejection
    /// prints when the rejected thing is a POSITION rather than a named declaration
    /// ([`Self::param_value_unfoldable`] is the named-declaration twin).
    ///
    /// The generate rejections used to stop at the unqualified half, so a module
    /// whose `generate if`, `generate case` and `generate for` were all rejected
    /// printed three sentences that named the construct and nothing else — while
    /// both oracles name the offending sub-expression (iverilog: "A reference to a
    /// net or variable (`q') is not allowed in a constant expression"; verilator:
    /// "Expecting expression to be constant, but variable isn't const: 'q'").
    /// [`Self::unfoldable_reason`] is the same naming, so this is reaching parity,
    /// not inventing a format. `None` keeps the unqualified wording, which stays
    /// honest: nothing here can name a cause it did not find.
    pub(crate) fn unfoldable_note(&self, what: &str, e: &ast::Expr) -> String {
        match self.unfoldable_reason(e) {
            Some(why) => format!("{what} is not a constant: {why}"),
            None => format!("{what} is not a constant"),
        }
    }
}

/// Source text of a binary operator, for [`Elaborator::expr_brief`].
fn bin_op_text(op: ast::BinOp) -> &'static str {
    use ast::BinOp as B;
    match op {
        B::Add => "+",
        B::Sub => "-",
        B::Mul => "*",
        B::Div => "/",
        B::Mod => "%",
        B::Pow => "**",
        B::Eq => "==",
        B::Ne => "!=",
        B::CaseEq => "===",
        B::CaseNe => "!==",
        B::Lt => "<",
        B::Le => "<=",
        B::Gt => ">",
        B::Ge => ">=",
        B::LogAnd => "&&",
        B::LogOr => "||",
        B::BitAnd => "&",
        B::BitOr => "|",
        B::BitXor => "^",
        B::BitXnor => "~^",
        B::Shl => "<<",
        B::Shr => ">>",
        B::AShl => "<<<",
        B::AShr => ">>>",
        _ => "?",
    }
}

/// Source text of a unary operator, for [`Elaborator::expr_brief`].
fn un_op_text(op: ast::UnOp) -> &'static str {
    use ast::UnOp as U;
    match op {
        U::Plus => "+",
        U::Minus => "-",
        U::LogNot => "!",
        U::BitNot => "~",
        U::RedAnd => "&",
        U::RedOr => "|",
        U::RedXor => "^",
        U::RedNand => "~&",
        U::RedNor => "~|",
        U::RedXnor => "~^",
    }
}
