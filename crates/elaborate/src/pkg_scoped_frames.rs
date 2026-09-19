//! §3.b: frame the LATE-injected package callees of a scoped `pk::g()` call.
//!
//! A scoped call is lowered by `inline_pkg_function` (`inline_fn.rs`) in pass 7, long
//! after step 6.5 (`lower_frame_funcs`, `instance.rs`) built this instance's frame set.
//! The root's TRANSITIVE same-package callees only enter `func_table` there, put in by
//! `inject_pkg_callees` (`package.rs`) — so a callee the step-6.5 barrier WOULD have
//! framed (`build_frame_set`) has no `frame_idx` entry, and the root's body falls
//! through to the INLINE fold for it. That fold is a straight-line SSA substitution and
//! cannot carry a decl-init local (`int x = a*2;` → `E3010` undeclared
//! `top.$func$pk::g.x`), a read-before-write static local, control flow, a loop, or an
//! unpacked local (`E3009` "body is not reducible to an expression"). The IMPORT twin
//! (`import pk::g;`) is correct on every one of those shapes, because step (3.6)
//! injects the same callees BEFORE (6.5) and the barrier frames them.
//!
//! This module does for the late-injected callees exactly what step 6.5 does for the
//! early-injected ones, with the SAME predicate (`build_frame_set`): classify, reserve
//! every qualifying callee AND the root before any body is lowered — which is what
//! makes a mutual recursion between them resolve — then lower the callee bodies, then
//! the root's.
//!
//! This arm WIDENS a route; it never narrows one and emits no diagnostic of its own.
//! Two shapes are left exactly where they were, on the inline fold and with whatever
//! that fold already said about them: a STATIC routine with a local the per-block
//! definite-assignment walk cannot prove is written before it is read, and a body that
//! reads its own RETURN VARIABLE before assigning it. The frame lane answers both of
//! them differently from the standard — one copy of the local per SCOPE where IEEE §6.21
//! keeps one for the whole design, and a return slot that starts at 0 where the standard
//! starts at the value the call left — so moving them here would trade one wrong answer
//! for another. Excluding the ROOT excludes the whole call: its callees keep the inline
//! fold too, because framing them changes how the root's own body is lowered around
//! them. That per-scope copy is the ROADMAP §2 row "A PACKAGE task's static local is ONE
//! variable", and it is the same row on the IMPORT lane, which frames the same bodies at
//! step 6.5 and is untouched here; closing it needs a design-wide frame for a static
//! package routine, not a decision at a call site. No guard lives in this module: three
//! shapes of one were built and each turned a design that worked on a pre-slice build
//! loud.
//!
//! TASKS are deliberately NOT mirrored (`build_task_frame_set` is not called here). A
//! scoped task enable `pk::t();` is a PARSE error (`VITA-E2002`), so a package task can
//! only be reached from a package FUNCTION body, and such a call is outside the scoped
//! function subset the caller's admission gate (`pkg_func_self_contained`) accepts. No
//! task reaches this funnel, and a frame set built for one would reserve a task nothing
//! calls.

use super::*;

impl Elaborator<'_> {
    /// Reserve + lower the frames a scoped `pkg::name(...)` call needs: every
    /// same-package callee `inject_pkg_callees` has just put into `func_table` that
    /// `build_frame_set` qualifies, and the root itself under `key` (`pkg::name`).
    /// Returns the root's FuncId. This arm emits no diagnostic of its own: a routine it
    /// cannot widen safely is left on the route it already had.
    ///
    /// Ordering is the step-6.5 ordering, for the step-6.5 reason: RESERVE every frame
    /// (callees in sorted key order, then the root) so that every `frame_idx` entry
    /// exists before the first body is lowered, then LOWER the callee bodies, then the
    /// root's. A callee lowered before its own caller is reserved would emit an inline
    /// fold of a name that is about to become a frame.
    pub(crate) fn frame_scoped_pkg_routines(
        &mut self,
        pkg: &str,
        key: &str,
        func: &ast::FunctionDef,
    ) -> u32 {
        // The ROOT under its own scoped key first. `inject_pkg_callees` walks OUTWARD
        // from the root and deliberately never injects the root itself, while a callee
        // body's bare sibling call resolves through `resolve_rtn_key` → `func_table`.
        // Without this entry a mutual recursion `g -> h -> g` reported `call to
        // undeclared function g` from inside `h` (measured: census x1, where both
        // oracles print 10). `or_insert` because a sibling scoped call may already have
        // injected it, and the injected definition is the same one.
        self.func_table
            .entry(key.to_string())
            .or_insert_with(|| func.clone());
        // The SAME predicate step 6.5 uses, run now that the callees are in the table.
        // A callee it does not qualify (a static, straight-line, non-2-state-return
        // sibling) keeps the inline fold it has always taken — this widens no route.
        let frame_set = self.build_frame_set();
        // ⚠️ `late` is the callees REACHABLE FROM THIS ROOT, not every `pkg::` key in
        // the table. The table accumulates: `inject_pkg_callees` never removes anything,
        // and neither does a refusal, so a key a PREVIOUS scoped call in this module put
        // there and did not frame is still present. Keying on the prefix alone let an
        // unrelated later root be re-run through the guard and refused with the other
        // root's message, naming an innocent call site (`q5f.sv` line 8, which has no
        // callee at all).
        let reachable = self.scoped_reachable_callees(pkg, key, func);
        // ⚠️ A routine this arm must not widen is EXCLUDED from `late`, never refused.
        // See the module header: a static routine carrying a value across calls, and a
        // body reading its own return variable, are both shapes the frame lane answers
        // differently from the standard, and both are the pre-existing ROADMAP §2 row on
        // the import lane too. Leaving them out means the callee keeps the INLINE fold it
        // took before this slice — with whatever that fold said, loud or not — so this
        // arm can only ever move a cell from that pre-slice line to the oracles' value.
        //
        // The ROOT decides first and decides for the whole call: if the root is excluded,
        // NOTHING is framed here (`late` empty), so the root is reserved and lowered
        // exactly as it was before. A root that carries a value is right today for the
        // scope that calls it — `pk::g` whose body is `int x; x = x + m; return x;`
        // prints `V=21 W=22` on a pre-slice build and on both oracles — and framing its
        // callees changes how its own body is lowered around it.
        let root_excluded = self.frame_widening_excluded(func);
        let late: Vec<(String, ast::FunctionDef)> = if root_excluded {
            Vec::new()
        } else {
            let cands: Vec<(String, ast::FunctionDef)> = frame_set
                .iter()
                .filter(|n| reachable.contains(n.as_str()))
                .filter(|n| !self.frame_idx.contains_key(n.as_str()))
                .filter_map(|n| {
                    self.func_table
                        .get(n.as_str())
                        .map(|f| (n.clone(), f.clone()))
                })
                .collect();
            cands
                .into_iter()
                .filter(|(_, f)| !self.frame_widening_excluded(f))
                .collect()
        };
        // RESERVE the callees (sorted key order = deterministic net/FuncId allocation).
        for (n, f) in &late {
            self.feed_scoped_block_locals(&f.body);
            self.note_frame_func_flags(n, f);
            self.reserve_frame_func(n, f);
        }
        // RESERVE the root. Its own bookkeeping stays the narrower one it has always
        // had: `body_write_func_names` alone, never `inout_func_names`. The scoped call
        // itself can never be hoisted (`inout_call_target` is single-segment only), so
        // joining the root to the out-call set would change the call shape of the one
        // call that cannot take it.
        self.feed_scoped_block_locals(&func.body);
        // §3.b: mark the body-write route BEFORE lowering, so the body is
        // classified like any other statement-executor function and the ONE
        // diagnostic the user gets is `emit_frame_call`'s — which names the
        // scoped call site as the constraint and points at `import pkg::f;`,
        // the spelling that is MEASURED to work (`pk::cnt` 7 → 8, both
        // oracles). Without it the subset sentence fires instead and lists
        // `cnt = cnt + 1;` among the forms it calls supported.
        if self.func_body_writes_outside_name(func) {
            self.body_write_func_names.insert(key.to_string());
        }
        self.reserve_frame_func(key, func);
        // LOWER each callee body — every `frame_idx` above is reserved, so a call
        // between two of them, or back to the root, resolves to a FuncId.
        for (n, f) in &late {
            let fid = self.frame_idx[n.as_str()];
            self.lower_frame_func_body(n, f, fid);
        }
        // R22 §3.1 (function half), mirrored for the LATE callees only: a framed
        // FUNCTION whose body carries an effect the `&self` executor cannot perform MUST
        // reach the engine as a `Terminator::Call`, the only call shape `run_process`
        // can route. An input-only one does not get that from its formals, so it joins
        // `inout_func_names` here. After the callee bodies (the predicate walks their
        // lowered blocks) and before the root's (the root's body calls them), which is
        // the same position step 6.5 puts it in. Keyed on
        // `func_body_needs_stmt_executor`, not on the suspendable set — the two differ
        // and over-marking is not free for anything that changes a call's SHAPE.
        for (n, _) in &late {
            let Some(&fid) = self.frame_idx.get(n.as_str()) else {
                continue;
            };
            if !self.funcs[fid as usize].is_task
                && ir::func_body_needs_stmt_executor(
                    &self.funcs,
                    &self.func_blocks,
                    &self.stmts,
                    &self.exprs,
                    fid,
                )
            {
                self.inout_func_names.insert(n.clone());
            }
        }
        let fid = self.frame_idx[key];
        self.lower_frame_func_body(key, func, fid);
        fid
    }

    /// The `pkg::name` keys of every same-package routine REACHABLE from this root,
    /// transitively, using the full declaration walkers (formal defaults, declaration
    /// initializers and statements alike). Only keys `inject_pkg_callees` actually put
    /// into `func_table` are returned; the root's own key never is.
    ///
    /// This is what `late` is selected from, rather than "every `pkg::` key not yet
    /// framed": `func_table` accumulates across the scoped calls of one module and a
    /// refusal removes nothing, so the prefix test attributed one root's callees to the
    /// next root that happened to run.
    fn scoped_reachable_callees(
        &self,
        pkg: &str,
        key: &str,
        func: &ast::FunctionDef,
    ) -> std::collections::BTreeSet<String> {
        let mut want: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        collect_callee_func(func, &mut want);
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        while let Some(n) = want.iter().next().cloned() {
            want.remove(&n);
            if !seen.insert(n.clone()) {
                continue;
            }
            let k = format!("{pkg}::{n}");
            if k == key {
                continue; // the root frames itself, and its callees are already walked
            }
            if let Some(f) = self.func_table.get(&k) {
                collect_callee_func(f, &mut want);
                keys.insert(k.clone());
            }
            // A task is walked for its EDGES only: it is never framed here (see the
            // module header), but a routine it calls may be.
            if let Some(t) = self.task_table.get(&k) {
                collect_callee_task(t, &mut want);
            }
        }
        keys
    }

    /// Is `f` a routine this arm must NOT move onto the frame path?
    ///
    /// Two shapes, both measured against the oracles and both answered the same way — by
    /// leaving the routine where it was, with no diagnostic:
    ///
    /// - a STATIC routine with a local the per-block definite-assignment walk cannot
    ///   prove is written before it is read. The frame lane gives such a local one copy
    ///   per scope where IEEE §6.21 keeps one for the whole design.
    /// - a body that reads its own RETURN VARIABLE before assigning it, whatever the
    ///   lifetime. The frame return slot answers 0 there where the standard answers the
    ///   value the call started from.
    fn frame_widening_excluded(&self, f: &ast::FunctionDef) -> bool {
        if self.return_var_read_before_write(f).is_some() {
            return true;
        }
        !f.automatic && self.static_local_read_before_write(f).is_some()
    }

    /// The name of the first body-local of `f` whose value can CARRY from one call to
    /// the next, or `None` when none can.
    ///
    /// Asked PER DECLARING SCOPE, not once over the whole body. Every walker used below
    /// is NAME-keyed, and the v1 block-local flatten lets one routine declare the same
    /// name at two sites (`begin : b1 int x; … end begin : b2 int x; x = x + a; … end`)
    /// as two distinct `$blk$` nets. Over the whole body, b1's write hides b2's
    /// read-before-write and the real hazard reads as clean; over each block's own
    /// statements the two are separate questions and both are answered. Declarators are
    /// deduped by SPAN, because a routine's top-level locals are reachable through both
    /// `body_decls` and the body block's own `decls`.
    ///
    /// Every site is asked, with no clearing rule of any kind: a declarator with an
    /// INITIALIZER is asked exactly like one without, because the initializer runs once
    /// at time 0 for a static routine and the walk over that scope's statements is the
    /// only thing that can say whether a later call reads what an earlier one left. Two
    /// attempts to clear an initialised local instead — "no initializer", then "a
    /// constant initializer" — each admitted a body the frame lane answers differently
    /// (`int s = $clog2(16);` and `int s = pv;` over a package variable).
    ///
    /// The walk uses `sole_writer = false`, the same question the block-local coalesce
    /// gate asks, and is asked with NO element bounds and NO declared width. Folding
    /// those two is what `fixed_elem_bounds` / `decl_bit_width` do, and folding a range
    /// EMITS — a `[3:-2]` local printed its `W3056` twice, once from its declaration and
    /// once from this decision. A decision must add no diagnostics, and dropping the two
    /// only removes PROOFS: a site they would have cleared now reads as persistent,
    /// which excludes the routine, which leaves it exactly where it already was.
    ///
    /// A declaration with an explicit `automatic` lifetime is skipped whatever the
    /// function's own default is. The RETURN VARIABLE is asked by
    /// [`Self::return_var_read_before_write`] over the whole body, which is its scope.
    fn static_local_read_before_write(&self, f: &ast::FunctionDef) -> Option<String> {
        let body = &*f.body;
        let top: &[ast::Stmt] = match body {
            ast::Stmt::Block { stmts, .. } => stmts,
            other => std::slice::from_ref(other),
        };
        let mut scopes: Vec<(&[ast::NetVarDecl], &[ast::Stmt])> =
            vec![(f.body_decls.as_slice(), top)];
        decl_scopes(body, &mut scopes);
        let mut asked: std::collections::BTreeSet<(u32, u32)> = std::collections::BTreeSet::new();
        for (decls, stmts) in scopes {
            for d in decls {
                if d.lifetime == Some(true) {
                    continue;
                }
                for nd in &d.names {
                    if !asked.insert((nd.span.lo, nd.span.hi)) {
                        continue; // the same declarator, reached through both lists
                    }
                    if self
                        .block_local_definitely_assigned(stmts, &nd.name.name, None, false, None)
                        .is_err()
                    {
                        return Some(nd.name.name.clone());
                    }
                }
            }
        }
        None
    }

    /// The function's RETURN VARIABLE, when the definite-assignment walk cannot prove
    /// the body writes it before reading it. `None` — the common case — for every body
    /// that returns by `return <expr>`, that assigns the name before reading it, or that
    /// never reads it at all.
    ///
    /// Asked separately from the body-locals above because the ANSWER is different: this
    /// one is wrong with one call in one scope, so it is refused immediately rather than
    /// counted. A function reaching here always has a return variable — `function void f`
    /// is parsed into a `TaskDef` in module and package scope, so no `FunctionDef` here
    /// is return-less.
    fn return_var_read_before_write(&self, f: &ast::FunctionDef) -> Option<String> {
        let body = &*f.body;
        let stmts: &[ast::Stmt] = match body {
            ast::Stmt::Block { stmts, .. } => stmts,
            other => std::slice::from_ref(other),
        };
        let rn = f.name.name.clone();
        // ⚠️ The presence conjunct first, and it is load-bearing. A RECURSIVE function
        // spells its own name in a CALL (`return x + h(x);`), and the definite-assignment
        // walk's reference detector is deliberately conservative: it cannot be told that
        // this occurrence is a callee and not a read, so it reported every recursive
        // package function as reading its return variable before assigning it
        // (`b3.sv`: `V=10` on both oracles, refused). `stmt_reads_ident` is the precise
        // walker for the question — it counts a path head as a read only from two
        // segments up, so a one-segment `h(x)` is a call and `h + 1` is a read — and it
        // already covers a nested block's declaration initializers.
        if !stmt_reads_ident(&f.body, &rn)
            && !f
                .body_decls
                .iter()
                .flat_map(|d| d.names.iter())
                .any(|n| n.init.as_ref().is_some_and(|e| expr_reads_ident(e, &rn)))
        {
            return None;
        }
        let rw = ast_func_return_width(f);
        self.block_local_definitely_assigned(stmts, &rn, None, false, rw)
            .is_err()
            .then_some(rn)
    }
}

/// Every `(decls, stmts)` pair inside one routine body: a declaring block's own
/// declarations beside the statement list they are in scope over.
///
/// The same statement forms [`collect_block_local_decls_spanned`] recurses through,
/// so the two agree on which declarations exist; this one keeps the association with
/// the scope, which the name-keyed definite-assignment question needs.
fn decl_scopes<'a>(s: &'a ast::Stmt, out: &mut Vec<(&'a [ast::NetVarDecl], &'a [ast::Stmt])>) {
    use ast::Stmt::*;
    match s {
        Block { decls, stmts, .. } | Fork { decls, stmts, .. } => {
            out.push((decls.as_slice(), stmts.as_slice()));
            for st in stmts {
                decl_scopes(st, out);
            }
        }
        If { then_s, else_s, .. } => {
            decl_scopes(then_s, out);
            if let Some(e) = else_s {
                decl_scopes(e, out);
            }
        }
        Case { items, .. } => {
            for it in items {
                match it {
                    ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                        decl_scopes(body, out)
                    }
                }
            }
        }
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            decl_scopes(body, out)
        }
        _ => {}
    }
}
