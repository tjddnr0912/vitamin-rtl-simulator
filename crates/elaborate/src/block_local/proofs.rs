//! R20 — the two PROOFS the round-20 report needed, kept out of `gate.rs` so both stay
//! under the 1000-line module cap.
//!
//! [`Elaborator::inout_copy_in_is_dead`] (§3.2) decides when an `inout` formal's copy-in
//! value cannot be observed, and [`Elaborator::loop_runs_once`] (§3.3) decides when a loop
//! provably executes its body. Both are ACCEPT-side, so every unknown answers "no"; both
//! were broken by the adversarial review in ways recorded in their own doc comments.

use super::*;

/// R20 §3.3: `e` with every bare `Ident(var)` replaced by the literal `0`, so a loop
/// condition can be folded for its FIRST iteration (`j < 3` with `j = 0` ⇒ `0 < 3`).
///
/// Zero is the only value substituted, because [`Elaborator::loop_runs_once`] admits only a
/// `for` whose init folds to exactly 0 — and that is what makes the substitution domain-free:
/// 0 is 0 in every width and signedness, so no declared type of the loop variable can change
/// it. (An earlier version substituted an arbitrary value and needed a unary-minus branch for
/// negatives; that branch became unreachable and is gone.)
///
/// FAIL-CLOSED, which is why the catch-all arm is safe: a position this does not descend into
/// keeps its `Ident`, and `fold_i32` admits no identifiers — so a missed position makes the
/// trip-count proof FAIL, never succeed wrongly.
fn subst_ident_zero(e: &ast::Expr, var: &str) -> ast::Expr {
    use ast::ExprKind as K;
    let sub = |x: &ast::Expr| Box::new(subst_ident_zero(x, var));
    let kind = match &e.kind {
        K::Ident(p) if p.segments.len() == 1 && p.segments[0].name == var => K::IntLit {
            kind: ast::IntLitKind::Decimal,
            raw: "0".to_string(),
        },
        K::Paren { inner } => K::Paren { inner: sub(inner) },
        K::Unary { op, operand } => K::Unary {
            op: *op,
            operand: sub(operand),
        },
        K::Binary { op, lhs, rhs } => K::Binary {
            op: *op,
            lhs: sub(lhs),
            rhs: sub(rhs),
        },
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => K::Ternary {
            cond: sub(cond),
            then_e: sub(then_e),
            else_e: sub(else_e),
        },
        _ => return e.clone(),
    };
    ast::Expr { kind, span: e.span }
}

/// R20 §3.2: does `st`, or anything nested in it, declare a local named `name`?
///
/// A redeclaration shadows the subroutine formal being proven, so a "write of `name`" below
/// it writes a different variable. `_`-free-exhaustive because this feeds an ACCEPT gate: a
/// future statement form that can carry declarations must be a compile error here, not a
/// silently missed shadow.
fn stmt_redeclares(st: &ast::Stmt, name: &str) -> bool {
    use ast::Stmt::*;
    let sub = |s: &ast::Stmt| stmt_redeclares(s, name);
    let opt = |s: &Option<Box<ast::Stmt>>| s.as_deref().is_some_and(|x| stmt_redeclares(x, name));
    match st {
        Block { stmts, decls, .. } | Fork { stmts, decls, .. } => {
            decls
                .iter()
                .flat_map(|d| d.names.iter())
                .any(|n| n.name.name == name)
                || stmts.iter().any(sub)
        }
        If { then_s, else_s, .. } => sub(then_s) || opt(else_s),
        Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => sub(body),
        }),
        For {
            init, step, body, ..
        } => sub(init) || sub(step) || sub(body),
        While { body, .. } | Repeat { body, .. } | Forever { body, .. } => sub(body),
        Wait { body, .. } | DelayCtrl { body, .. } | EventCtrl { body, .. } => opt(body),
        DeferredAssert { then_s, else_s, .. } => sub(then_s) || sub(else_s),
        ConcurrentAssert { pass, fail, .. } => opt(pass) || opt(fail),
        // No nested statement and no declaration position.
        Blocking { .. }
        | NonBlocking { .. }
        | Assign { .. }
        | Force { .. }
        | Deassign { .. }
        | Release { .. }
        | UserTaskCall { .. }
        | SysTaskCall { .. }
        | RandomizeWith { .. }
        | EventTrigger { .. }
        | Disable { .. }
        | WaitFork { .. }
        | CoverProperty { .. }
        | Return { .. }
        | Null(_)
        | Error(_) => false,
    }
}

impl Elaborator<'_> {
    /// R20 §3.3: does loop `st` provably execute its body AT LEAST ONCE?
    ///
    /// The trip-count half of [`crate::da::LoopRunsOnce`] — see that type for why the
    /// question is asked from here (folding needs this Elaborator's parameter scope) and
    /// for the ACCEPT-side polarity: every form this cannot fold answers `false`, which is
    /// mechanically the pre-R20 walk.
    ///
    /// All four loop forms are answered, not just the reported `for`. Fixing one and
    /// leaving its siblings is the divergence this codebase treats as worse than a uniform
    /// restriction: `repeat (3)` and `for (int j = 0; j < 3; j++)` are the same statement
    /// about the same trip count, and a user who rewrites one as the other should not find
    /// that it changes whether the block elaborates.
    ///
    /// SOUNDNESS — this predicate must agree with what the ENGINE executes, and the
    /// adversarial lenses found FOUR independent ways for an AST constant fold to disagree
    /// with it. All four are closed by [`Self::fold_i32`], which admits only plain unsized
    /// decimal literals and evaluates them in checked **i32**:
    ///
    ///   * **domain (width/signedness).** The old fold ran in i64, ignoring the loop
    ///     variable's declared width and IEEE §11.8.1's unsigned-comparison rule. Measured
    ///     0-trip loops it called ">= 1": `for (byte j = 200; j > 100; j--)` (200 is -56 in a
    ///     signed byte) and `for (int j = -1; j < 4'd3; j++)` (one unsigned operand makes the
    ///     comparison unsigned). Requiring the init to be exactly **0** kills the truncation
    ///     axis (0 is 0 in every width and signedness); banning sized/based literals kills the
    ///     signedness axis.
    ///   * **result overflow.** Bounding only the LEAVES is not enough — `65536 * 65536` and
    ///     `2147483647 + 1` are built from admissible leaves but leave the 32-bit domain,
    ///     where the engine wraps them to values that make the loop run ZERO times (measured:
    ///     `repeat (65536*65536)` = 0 trips, and the block then read the previous activation's
    ///     leftover at exit 0). `fold_i32` overflows to `None`.
    ///   * **resolver.** `const_eval_in_scope`'s `Ident` arm is `lookup_scoped`, a params-only
    ///     outward walk that is NOT net-aware, so a generate-scope net `K` shadowing a
    ///     localparam `K` folded to the localparam while the lowering read the net — the
    ///     classifier-vs-lowering-resolver trap. It CANNOT be fixed by making the fold
    ///     shadow-aware: `walk_scopes_key_shadowed`'s contract forbids opting in any consumer
    ///     reachable from `const_eval_in_scope`, because `symbols` is populated DURING
    ///     elaboration and an order-dependent answer there previously deleted a whole generate
    ///     body at exit 0. Admitting **no identifiers at all** is the order-independent answer,
    ///     and it removes the parameter scope from this question entirely.
    ///   * **scope.** [`Self::inout_copy_in_is_dead`] walks a CALLEE's body; passing this
    ///     resolver there folded the callee's own formals and locals against the CALLER's
    ///     parameter scope (a formal named `LIM` took a module `localparam LIM`'s value —
    ///     ordinary code, no shadowing trick). That call site passes `&|_| false`.
    ///
    /// The cost is precision, not correctness: a `localparam` bound
    /// (`for (int j = 0; j < NN; j++)`) is no longer proven and stays loud. Lifting that
    /// needs an order-independent, AST-gathered name set — the same prerequisite ROADMAP §2
    /// already records for the inner-net-shadow item.
    pub(crate) fn loop_runs_once(&self, st: &ast::Stmt) -> bool {
        match st {
            // A `forever` body does always run, but claiming it buys nothing and rests on
            // "the statement after a break-less `forever` is unreachable" — which an EXTERNAL
            // `disable` of an enclosing named block, issued by another process, defeats.
            // `stmt_cannot_escape` cannot see such a disable (it is not in this statement), so
            // the claim is not checkable here. Round 3 raised it as a suspected hole; answering
            // `false` closes it at no cost, since a `forever` whose body could reach the write
            // needs a `break` to get past the loop and `stmt_cannot_escape` rejects that anyway.
            ast::Stmt::Forever { .. } => false,
            ast::Stmt::Repeat { count, .. } => Self::fold_i32(count).is_some_and(|n| n >= 1),
            ast::Stmt::While { cond, .. } => Self::fold_i32(cond).is_some_and(|v| v != 0),
            // The FIRST-iteration condition, with the loop variable bound to its init value.
            // `for (int j = 0; j < 3; j++)` folds `0 < 3` ⇒ true.
            ast::Stmt::For { init, cond, .. } => {
                let ast::Stmt::Blocking {
                    lhs: ast::Lvalue::Ident(p),
                    delay: None,
                    event: None,
                    rhs,
                    ..
                } = init.as_ref()
                else {
                    return false;
                };
                let [seg] = &p.segments[..] else {
                    return false;
                };
                // Exactly 0 — see the domain note above. A descending or non-zero-based loop
                // is therefore not proven; that is deliberate, not an oversight.
                if Self::fold_i32(rhs) != Some(0) {
                    return false;
                }
                Self::fold_i32(&subst_ident_zero(cond, &seg.name)).is_some_and(|v| v != 0)
            }
            _ => false,
        }
    }

    /// R18 §3.2: the flattened bit WIDTH of a block-local declarator, for the
    /// select-coverage rule — `Some(w)` only for a plain packed scalar/vector with no
    /// unpacked dimensions, where "bit `i` of `name`" is unambiguous. A struct is one
    /// of these after the parser's desugar (its members are constant part-selects into
    /// the flat vector), which is exactly the shape the rule exists for.
    ///
    /// `None` disables the rule, so an unpacked array (whose select indices are
    /// ELEMENTS, covered by `elem_bounds` instead) and anything absurdly wide keep the
    /// previous behaviour.
    pub(crate) fn decl_bit_width(&mut self, d: &ast::NetVarDecl, n: &ast::DeclName) -> Option<u32> {
        /// A local wider than this cannot plausibly be covered by literal selects, and
        /// the bit set would be the only unbounded thing in the walk.
        const MAX_COVERED_BITS: u32 = 4096;
        if !n.unpacked.is_empty() {
            return None;
        }
        let (w, ..) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
        (w > 0 && w <= MAX_COVERED_BITS).then_some(w)
    }

    /// R20 §3.3: fold `e` to a value in the SAME 32-bit domain the engine executes in, or
    /// `None` if it cannot be done faithfully.
    ///
    /// An ALLOW-LIST, and it must stay one — [`Self::loop_runs_once`] records four measured
    /// ways an admitted node breaks agreement with the engine. Only plain unsized decimal
    /// literals (§5.7.1 fixes their type at 32-bit SIGNED) and the operators below are
    /// admitted; an identifier, a sized/based literal, `$bits`, a concat, a select or a call
    /// all answer `None`, so the loop is simply not proven.
    ///
    /// Arithmetic is `checked_*` in **i32**, not i64. Bounding only the LEAVES was the round-2
    /// review's finding: `const_binop` folds with i64 `checked_*`, so `65536 * 65536` became
    /// 4294967296 and `2147483647 + 1` became 2147483648, while the engine wraps both to
    /// values that make the loop run ZERO times (measured: `repeat (65536*65536)` = 0 trips).
    /// Overflow here answers `None` instead. `**`, `<<` and `>>` are deliberately absent —
    /// `const_binop`'s own comment records that it folds `1 << 32` wide to match iverilog
    /// while vita's engine gives 0, so they are exactly the operators whose domains differ.
    /// Division and modulo are absent too (divide-by-zero and rounding are their own
    /// question, and no trip count needs them).
    fn fold_i32(e: &ast::Expr) -> Option<i32> {
        use ast::ExprKind as K;
        let b = |v: bool| Some(v as i32);
        match &e.kind {
            K::IntLit {
                kind: ast::IntLitKind::Decimal,
                raw,
            } => raw.trim().replace('_', "").parse::<i32>().ok(),
            K::Paren { inner } => Self::fold_i32(inner),
            K::Unary { op, operand } => {
                let v = Self::fold_i32(operand)?;
                match op {
                    ast::UnOp::Plus => Some(v),
                    ast::UnOp::Minus => v.checked_neg(),
                    ast::UnOp::LogNot => b(v == 0),
                    _ => None,
                }
            }
            K::Binary { op, lhs, rhs } => {
                let (l, r) = (Self::fold_i32(lhs)?, Self::fold_i32(rhs)?);
                match op {
                    ast::BinOp::Add => l.checked_add(r),
                    ast::BinOp::Sub => l.checked_sub(r),
                    ast::BinOp::Mul => l.checked_mul(r),
                    ast::BinOp::Lt => b(l < r),
                    ast::BinOp::Le => b(l <= r),
                    ast::BinOp::Gt => b(l > r),
                    ast::BinOp::Ge => b(l >= r),
                    ast::BinOp::Eq | ast::BinOp::CaseEq => b(l == r),
                    ast::BinOp::Ne | ast::BinOp::CaseNe => b(l != r),
                    ast::BinOp::LogAnd => b(l != 0 && r != 0),
                    ast::BinOp::LogOr => b(l != 0 || r != 0),
                    _ => None,
                }
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                if Self::fold_i32(cond)? != 0 {
                    Self::fold_i32(then_e)
                } else {
                    Self::fold_i32(else_e)
                }
            }
            _ => None,
        }
    }

    /// R20 §3.2: the CALLEE-side name for the storage the caller calls `name`, when `name`
    /// is bound to the formal `formal` by the actual `arg`.
    ///
    /// Two spellings reach here, and both denote the same storage. Either the actual IS
    /// `name` (`nxt(n, r)` with `name` = `r`, or the already-fanned-out member net), in
    /// which case the callee calls it by the formal's own name; or the actual names a
    /// record whose MEMBER is `name` (`nxt(n, r)` with `name` = `$unp$r$count`), in which
    /// case the callee's member carries the formal's variable component instead —
    /// `$unp$<formal>$count`. `None` for anything else, which keeps the caller conservative.
    pub(crate) fn callee_side_member_name(
        &self,
        arg: &ast::Expr,
        formal: &str,
        name: &str,
    ) -> Option<String> {
        let ast::ExprKind::Ident(p) = &arg.kind else {
            return None;
        };
        let [seg] = &p.segments[..] else {
            return None;
        };
        if seg.name == name {
            return Some(formal.to_string());
        }
        // `$unp$<actual>$<field>` → `$unp$<formal>$<field>`. The desugar forbids `$` in the
        // VARIABLE component, so the first `$` after the prefix delimits it unambiguously
        // and the split is injective (see `unpacked_member_net`).
        let field = name
            .strip_prefix("$unp$")?
            .strip_prefix(seg.name.as_str())?
            .strip_prefix('$')?;
        Some(format!("$unp${formal}${field}"))
    }

    /// R20 §3.2: does the resolved single-segment `callee` make any call — in its body OR in a
    /// top-level declaration initializer? An unresolvable callee answers `true` (conservative).
    ///
    /// BOTH lists, for the same reason [`Self::inout_copy_in_is_dead`] needs both: a tf body's
    /// top-level declarations live in `body_decls`, and `tf_body` builds the wrapper `Block`
    /// with `decls: Vec::new()`. Reading only `body` walked a decls-empty block, so a nested
    /// call spelled `int tt = rd();` was invisible and the proof called the copy-in dead —
    /// re-opening, one level in, exactly the silent-wrong the OUTER check had just closed
    /// (measured: activation 2 returned the planted 999 where a fresh `automatic` gives 0).
    /// The nested-BLOCK spelling stayed loud throughout, because `da_stmt`'s `Block` arm does
    /// read a nested block's decls — the same two-spellings-disagree signature as before.
    fn callee_makes_any_call(&self, callee: &ast::HierPath) -> bool {
        let [seg] = &callee.segments[..] else {
            return true;
        };
        let (body, body_decls) = if let Some(f) = self.func_table.get(seg.name.as_str()) {
            (&f.body, &f.body_decls)
        } else if let Some(t) = self.task_table.get(seg.name.as_str()) {
            (&t.body, &t.body_decls)
        } else {
            return true;
        };
        crate::da::stmt_makes_any_call(body)
            || body_decls
                .iter()
                .flat_map(|d| d.names.iter())
                .any(|n| n.init.as_ref().is_some_and(crate::da::expr_makes_any_call))
    }

    /// R20 §3.2: is the copy-IN of `callee`'s INOUT formal `formal` UNOBSERVABLE?
    ///
    /// An `inout` formal's copy-in reads the actual at the call (IEEE 1800 §13.5.2), which
    /// is why the block-local gate counts it as a read: on the v1 flatten the actual's
    /// value at that moment is the leftover from a previous activation, not a fresh
    /// `automatic`'s default. That reasoning is only sound while the copy-in value can be
    /// SEEN. When the callee overwrites the whole formal before ever looking at it, the
    /// copied-in leftover is dead — nothing reads it, and the copy-out writes back a value
    /// that came entirely from the callee. So the two storage models agree, and the call is
    /// a definite WRITE of the actual rather than a read of it.
    ///
    /// The whole of the round-20 §3.2 report is this shape: the table-driven walker
    /// `while (n < 5 && nxt(n, r) == 1)` where `nxt` fills `r` and returns whether to
    /// continue. The reporter's own boundary confirms the mechanism — pre-writing every
    /// member made it pass, i.e. the only thing at stake was that first copy-in.
    ///
    /// Three obligations, every unknown answering `false`:
    ///   1. the callee RESOLVES to a single-segment func/task in this module;
    ///   2. it cannot SUSPEND, so no other activation can interleave on the formal's
    ///      storage between the copy-in and the write;
    ///   3. a straight-line PREFIX of its body definitely writes `formal` whole, with
    ///      nothing before that write able to read it or to leave the body.
    ///
    /// Obligation 3 is deliberately NOT
    /// [`crate::da::automatic_local_definitely_assigned`], whose contract is the weaker
    /// "no read happens before the first write". That is the right question for a
    /// block-local and the WRONG one here: `if (c) r = 1;` satisfies it while leaving `r`
    /// unwritten on a live path, and the copy-out would then hand the leftover straight
    /// back to the caller — a silent-wrong. What is needed is "definitely written", which
    /// is what the prefix walk below establishes.
    pub(crate) fn inout_copy_in_is_dead(&self, callee: &ast::HierPath, formal: &str) -> bool {
        if callee.segments.len() != 1 {
            return false;
        }
        if self.call_may_suspend(callee, Self::CALL_INERT_DEPTH) {
            return false;
        }
        let nm = callee.segments[0].name.as_str();
        // BOTH declaration lists, and `body_decls` is the one that matters. A tf-body's
        // top-level declarations do NOT live in the body `Block` — `tf_body` collects them
        // into `FunctionDef`/`TaskDef::body_decls` and synthesizes the wrapper with
        // `decls: Vec::new()`. So a first version of this check that read only the Block's
        // `decls` inspected an ALWAYS-EMPTY list and was dead code for the canonical
        // spelling: `int save = r; r = fd;` at tf-body top level was accepted and returned
        // the caller's planted leftover (measured `q=1009` where a per-activation local
        // gives 10, exit 0), while the same read one block deeper, and the same read written
        // as a statement, were both correctly refused — three spellings of one hazard, two
        // right and one wrong. `callee_body_cannot_touch` in `gate.rs` already read the
        // right list; this one now matches it.
        let (body, body_decls): (&ast::Stmt, &[ast::NetVarDecl]) =
            if let Some(f) = self.func_table.get(nm) {
                (&f.body, &f.body_decls)
            } else if let Some(t) = self.task_table.get(nm) {
                (&t.body, &t.body_decls)
            } else {
                return false;
            };
        // Declarations run before the statements, and a declarator INITIALIZER can read the
        // formal (`int save = r;`) — observing exactly the copy-in value this proof calls
        // dead. `da_stmt`'s `Block` arm checks nested blocks' decl-inits for precisely that
        // reason; the top level has no such arm, so it is checked here.
        let block_decls: &[ast::NetVarDecl] = match body {
            ast::Stmt::Block { decls, .. } => decls,
            _ => &[],
        };
        let stmts: &[ast::Stmt] = match body {
            ast::Stmt::Block { stmts, .. } => stmts,
            other => std::slice::from_ref(other),
        };
        // The call resolver for the walk below. It never re-enters `call_out_actual_writes`, so
        // a pair of mutually recursive functions cannot loop here.
        //
        // Three obligations, and the round-3 review found the third one missing. An earlier
        // version was purely SYNTACTIC and justified itself with "a formal is not a module net,
        // so no other subroutine's body can name it". That is FALSE for exactly the storage this
        // proof is about: v1 flattens the caller's block-local to a module net by BARE NAME, and
        // the `inout` formal is bound to it — so any subroutine's bare `r` reads precisely the
        // value the proof declared dead. Measured: `int save = rd();` inside the callee, with
        // `function rd(); return r; endfunction`, returned activation 1's leftover 999 where a
        // fresh `automatic` gives 0, at exit 0 — while the DIRECT `int save = r;` was correctly
        // refused. One read, two spellings, one silently wrong.
        //
        //   1. the call's own path must not name the formal;
        //   2. no BINDING may reference it — taken from `callee_arg_binds`, not the written-out
        //      `args`, because an OMITTED formal binds its DEFAULT and that expression can name
        //      the formal too (`void'(g())` where `g`'s default is `r`);
        //   3. the callee's BODY must be unable to reach it, which is what closes the
        //      indirection above. `callee_body_cannot_touch` is depth-budgeted and answers
        //      `false` for anything it cannot resolve, so every unknown ends the proof.
        let inert_or_unknown = |cn: &ast::HierPath, args: &[ast::Expr], nm: &str| {
            let ok = cn.segments.iter().all(|s| s.name != nm)
                && self
                    .callee_arg_binds(cn, args)
                    .is_some_and(|b| b.iter().all(|(_, _, a)| crate::da::expr_no_ref_deep(a, nm)))
                && self.callee_body_cannot_touch(cn, nm, Self::CALL_INERT_DEPTH)
                // ...and its body must not DELEGATE. `callee_body_cannot_touch` walks the body
                // with `expr_no_ref_deep`, whose `Call` arm does not enter the called function,
                // so `rd2() -> rd() -> return r` slipped one level past the vet — loud at
                // `8cf4165`, silently the leftover after. Refusing a nested call closes it here
                // without changing that shared walker (its deep fix is ROADMAP §2).
                && !self.callee_makes_any_call(cn);
            if ok {
                crate::da::CallEffect::Inert
            } else {
                crate::da::CallEffect::Unknown
            }
        };
        for d in body_decls.iter().chain(block_decls) {
            for n in &d.names {
                // A top-level local that REDECLARES the formal shadows it, so a later
                // "write of `formal`" in this body is a write of something else entirely.
                if n.name.name == formal {
                    return false;
                }
                // Syntactic reference OR a read smuggled through a call's body.
                if n.init.as_ref().is_some_and(|e| {
                    !crate::da::expr_no_ref_deep(e, formal)
                        || crate::da::expr_may_observe_via_call(e, formal, &inert_or_unknown)
                }) {
                    return false;
                }
            }
        }
        // The same shadowing hazard one or more levels down. `da_stmt`'s `Block` arm counts a
        // nested `begin int r; r = fd; end` as a whole-var write of `r` without noticing the
        // redeclaration. Not observably wrong today — v1's flatten aliases the inner name onto
        // the formal's storage — but the proof must not depend on that accident.
        if stmts.iter().any(|s| stmt_redeclares(s, formal)) {
            return false;
        }
        let mut assigned = false;
        for st in stmts {
            // Written before anything that could exit the body or observe the copy-in —
            // everything after this point reads a value the callee itself put there.
            if assigned {
                return true;
            }
            if !crate::da::stmt_cannot_escape(st) {
                return false;
            }
            // Can this statement OBSERVE the formal through a CALL, before the write? Neither
            // walker below can answer that: `da_stmt` steps over any statement `stmt_no_ref`
            // says does not MENTION the formal, and the decl-init check is syntactic — so
            // `save = rd();` (or `int save = rd();`) with `function rd(); return r; endfunction`
            // read the copy-in this proof had called dead, and returned the caller's leftover at
            // exit 0. The DIRECT spelling `save = r;` was refused all along: one read, two
            // spellings, one silently wrong. Measured loud→silent from THIS gate, so it is mine.
            if crate::da::stmt_may_observe_via_call(st, formal, &inert_or_unknown) {
                return false;
            }
            match crate::da::da_stmt(
                st,
                false,
                formal,
                &crate::da::DaCtx {
                    out_writes: &inert_or_unknown,
                    // Never consulted: `sole` is true, and the suspend rule is the only
                    // reader. The callee's own inability to suspend was checked above.
                    suspends: &|_| true,
                    // NOT `self.loop_runs_once`. These are the CALLEE's loops, and that
                    // resolver folds identifiers in the CALLER's parameter scope — so a
                    // formal or local named like a module `localparam` (`LIM`, `N`, `WIDTH`:
                    // ordinary code, no shadowing trick needed) took the parameter's value
                    // and a 0-trip loop was "proven" to write the formal. Measured: the
                    // caller read its own leftover 999 at exit 0.
                    loop_once: &|_| false,
                    sole: true,
                },
            ) {
                Ok(crate::da::DaOut::Falls(a)) => {
                    // A statement BEFORE the write must not reference the formal under the
                    // ALL-SEGMENTS rule either. `da_stmt` resolves names with `expr_no_ref`,
                    // whose head-segment-only rule is documented as the CALLER-side one; the
                    // sibling checks in this function use the deep rule, so the same hazard
                    // split by spelling — `save = t.r;` was accepted while `int save = t.r;`
                    // and `save = r;` were refused, and the accepted one read the copy-in this
                    // proof had declared dead. The write itself is exempt (it is the statement
                    // that sets `a`), so this only screens the prefix.
                    if !a && !crate::da::stmt_no_ref_deep(st, formal, &|_, _, _| false) {
                        return false;
                    }
                    assigned = a;
                }
                _ => return false,
            }
        }
        assigned
    }
}
