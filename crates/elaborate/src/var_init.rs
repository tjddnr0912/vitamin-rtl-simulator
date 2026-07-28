//! §6.8 variable-initializer collection and the t0 pre-sweep flush — split out of
//! `netdecl.rs` (mechanical move; module-size policy).

use super::*;

impl Elaborator<'_> {
    /// §6.8: collect a VARIABLE declaration's NON-constant initializer
    /// (`logic [7:0] b = a;`) into `pending_var_inits` so a pre-sweep can emit it
    /// as a synthesized `initial b = a;` that runs BEFORE user initial blocks. A
    /// constant initializer is already folded into the net's `init` field at
    /// declaration, so it is skipped here (byte-identical IR for those designs).
    /// A net (wire) decl is not a variable — its initializer is a continuous
    /// driver, handled by `elaborate_net_init_drivers`.
    pub(crate) fn collect_var_init_drivers(&mut self, d: &ast::NetVarDecl) {
        // A `string s = expr;` initializer (v7): the heap-backed string has no
        // foldable `init` field, so its initializer ALWAYS rides this t0 pre-sweep
        // (like a non-constant var init), collected here in declaration order with
        // the other variable initializers. Only a SCALAR string is registered as a
        // net (`elaborate_netvar_decl` rejects packed/unpacked dims), so a dimensioned
        // string's init is skipped here too (the decl already errored loud).
        let scalar_string =
            matches!(d.kind, ast::NetVarKind::String) && d.range.is_none() && d.packed.is_empty();
        if !netvar_kind_is_var(d.kind) && !scalar_string {
            return;
        }
        for name in &d.names {
            let Some(init) = &name.init else {
                continue;
            };
            // A queue / dynamic-array `'{…}` decl-init has no whole-value init
            // surface; it rides the normal `pending_var_inits` path (in DECLARATION
            // order, alongside scalar inits) and is EXPANDED to runtime ops at flush
            // (a push_back sequence / `new[N]` + element writes) — see
            // `flush_pending_var_inits`. Keeping it in the one ordered list lets a
            // scalar init read an earlier queue's `.size()` correctly.
            if scalar_string {
                if !name.unpacked.is_empty() {
                    // N3 Phase 2: a `string s[]` DYNAMIC array with a `'{…}` init IS
                    // collected — the flush expands it via `dyn_decl_init_stmts` (like
                    // an int/real dyn array).
                    let is_dyn_str_init = crate::string_array_route::is_dyn_string_container_init(
                        &name.unpacked,
                        init,
                    );
                    if !is_dyn_str_init {
                        // r19: a FIXED string array (`string s[3] = '{…}`) expands to one
                        // `s[k] = <elem>` per declared index, pushed in declaration order
                        // alongside the scalar inits so a later init can read an earlier
                        // element.
                        //
                        // Gated on the decl having actually CREATED the element storage
                        // (`string_array_elems`), not on re-deciding the shape here: the
                        // decl also consults `allow_string_init`, which the collectors do
                        // not see, so a scope that louds the decl (interface / generate /
                        // package body) but still runs a collector would otherwise push
                        // element writes for nets that were never created — cascading
                        // E3010s. Keying off the decl's own output makes the two
                        // structurally unable to disagree.
                        //
                        // T1-4: the FULL `unpacked` is handed over, so a routed multi-dim
                        // array expands its nested `'{'{…},'{…}}` here too. Passing only
                        // the first dim was a SILENT-WRONG the moment the decl started
                        // accepting a nested pattern: the 1-D expansion matched the OUTER
                        // level (2 rows, 2 elements) and emitted `s[0] = '{"a","b"}` —
                        // an assignment-pattern into a string element — which rendered
                        // four empty strings at exit 0.
                        if self.has_fixed_string_array_storage(&name.name.name) {
                            if let Some(pairs) =
                                self.string_array_init_pairs(&name.name, &name.unpacked, init)
                            {
                                self.pending_var_inits.extend(pairs);
                            }
                        }
                        continue;
                    }
                }
            } else {
                let (w, ..) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
                if fold_init(init, w).is_some() || self.const_eval_in_scope(init).is_some() {
                    continue; // constant ⇒ already folded into net.init
                }
            }
            let path = ast::HierPath {
                segments: vec![name.name.clone()],
                span: name.name.span,
            };
            self.pending_var_inits
                .push((ast::Lvalue::Ident(path), init.clone()));
        }
    }

    // ── PASS 2: continuous assigns ─────────────────────────────────
    /// A NET-type declaration initializer (`wire [3:0] x = a & b;`) is an IMPLICIT
    /// continuous assign — a driver, equivalent to a separate `assign x = a & b;`.
    /// A variable (reg/logic/integer/real/…) initializer is instead a one-time
    /// value applied at net creation, so it is skipped here.
    pub(crate) fn elaborate_net_init_drivers(&mut self, d: &ast::NetVarDecl) {
        // A `string` is a heap-backed VARIABLE, not a continuously-driven net — its
        // initializer is a one-time t0 assignment (`collect_var_init_drivers`), NOT
        // a continuous driver. Without this guard a `string s = "x";` would gain a
        // spurious `assign s = "x"` that fights later procedural writes (silent-wrong).
        if netvar_kind_is_var(d.kind) || matches!(d.kind, ast::NetVarKind::String) {
            // A variable's initializer is handled by `collect_var_init_drivers`
            // (a pre-sweep, so the synthesized `initial` runs before user blocks);
            // here a variable decl contributes no continuous driver.
            return;
        }
        // IEEE §6.1.3: an optional net-declaration delay (`wire #3 w = a;`) applies
        // to EVERY net-decl-assignment in this decl, IDENTICAL to a delay on the
        // equivalent `assign #3 w = a;` — fold it the same way (uniform delay +
        // distinct rise/fall/turnoff `ca_delays` sidecar). `None` (no delay, the
        // common case) ⇒ `(None, None)` ⇒ byte-identical to before.
        let (delay, rft) = self.fold_ca_delay(d.delay.as_ref());
        for name in &d.names {
            let Some(init) = &name.init else {
                continue;
            };
            let path = ast::HierPath {
                segments: vec![name.name.clone()],
                span: name.name.span,
            };
            let lhs = self.lower_lvalue(&ast::Lvalue::Ident(path));
            let rhs_id = self.lower_expr(init);
            // The index of THIS cont-assign is the len BEFORE the push (matches
            // `elaborate_cont_assign`'s sidecar keying).
            let idx = self.cont_assigns.len() as u32;
            if let Some(rft) = rft {
                self.ca_delays.insert(idx, rft);
            }
            self.cont_assigns.push(ir::ContAssign {
                lhs,
                rhs: rhs_id,
                delay,
            });
        }
    }

    /// Move this scope's decl-time PRE-SIZE writes (`pending_scoped_presize[cur_prefix]`)
    /// to the front of `pending_var_inits`, so a routed string array's `new[n]` precedes
    /// the element writes the collectors are about to push. Called once per flush point —
    /// module/interface scope (the `""` key) and each generate scope.
    pub(crate) fn drain_scoped_presize(&mut self) {
        if let Some(v) = self.pending_scoped_presize.remove(&self.cur_prefix) {
            self.pending_var_inits.splice(0..0, v);
        }
    }

    /// Twin of [`Self::drain_scoped_presize`] for block-local STRING inits, which go at the
    /// END instead: a block-local string may read a module-scope string, so it must be
    /// assigned after the ordinary inits rather than before them.
    pub(crate) fn drain_scoped_bl_strings(&mut self) {
        if let Some(v) = self.pending_scoped_bl_strings.remove(&self.cur_prefix) {
            self.pending_var_inits.extend(v);
        }
    }

    /// Drain `pending_var_inits` into ONE synthesized `initial` process whose body
    /// assigns each non-constant variable initializer in declaration order (§6.8).
    /// Lowered in the current instance scope; a no-op when none were collected (so
    /// a design with no such initializer adds no process — byte-identical IR).
    /// The prefix a scoped block-local's t0 init is recorded under: the current
    /// instance prefix, plus the `$blk$<lo>` segment when the declaration has one.
    pub(crate) fn scoped_init_key(&self, scope: Option<&str>) -> String {
        match scope {
            Some(seg) if self.cur_prefix.is_empty() => seg.to_string(),
            Some(seg) => format!("{}.{seg}", self.cur_prefix),
            None => self.cur_prefix.clone(),
        }
    }

    /// §4.5.251: replay the t0 initializers of `$blk$`-scoped block-locals, each under
    /// the prefix it was declared in. Emitted AFTER the main sweep and as its own
    /// `initial`, so a scoped init may read a module-scope one — the same ordering
    /// `pending_scoped_bl_strings` already relies on.
    pub(crate) fn flush_pending_blk_inits(&mut self) {
        let here = self.cur_prefix.clone();
        let dot = if here.is_empty() {
            String::new()
        } else {
            format!("{here}.")
        };
        let is_here = |k: &String| {
            k.strip_prefix(&dot)
                .is_some_and(|rest| rest.starts_with("$blk$") && !rest.contains('.'))
        };
        // A scoped block-local's inits can be in EITHER pending list — a string one goes
        // to `pending_scoped_bl_strings` (drained last, since it may read a module-scope
        // string), everything else to `pending_blk_inits`. Both are keyed by the same
        // scoped prefix, so replay every scope that appears in either.
        let mut keys: Vec<String> = self
            .pending_blk_inits
            .keys()
            .filter(|k| is_here(k))
            .cloned()
            .collect();
        keys.extend(
            self.pending_scoped_bl_strings
                .keys()
                .filter(|k| is_here(k))
                .cloned(),
        );
        keys.sort();
        keys.dedup();
        for key in keys {
            let inits = self.pending_blk_inits.remove(&key).unwrap_or_default();
            let saved = std::mem::replace(&mut self.cur_prefix, key);
            self.pending_var_inits = inits;
            self.drain_scoped_bl_strings();
            self.flush_pending_var_inits();
            self.cur_prefix = saved;
        }
    }

    pub(crate) fn flush_pending_var_inits(&mut self) {
        if self.pending_var_inits.is_empty() {
            return;
        }
        let inits = std::mem::take(&mut self.pending_var_inits);
        let sp = inits[0].1.span;
        // Each (lvalue, rhs) becomes a `Blocking` t0 assignment — EXCEPT a queue /
        // dynamic-array `'{…}` init, which has no whole-value surface and is
        // EXPANDED here (in declaration order, where the handle net is registered) to
        // runtime ops: a queue pushes each element (`push_back`), a dyn array does
        // `new[N]` + element writes. Keeping it in the one ordered list means a later
        // scalar init reads an earlier queue's `.size()` correctly.
        let mut stmts: Vec<ast::Stmt> = Vec::with_capacity(inits.len());
        for (lhs, rhs) in inits {
            if let ast::Lvalue::Ident(p) = &lhs {
                // A queue / dyn-array `'{…}` OR `{…}` (§10.10 unpacked concat) decl-init —
                // both expand to the same push_back / `new[N]`+element-write sequence for a
                // registered handle. A STRING-element `{…}` never reaches here as a handle
                // (loud at the decl gate), so no silent-empty slips through.
                if let Some(elems) = dyn_pattern_elems(&rhs) {
                    if p.segments.len() == 1 {
                        if let Some((_, kind @ (ir::NetKind::Queue | ir::NetKind::DynArray))) =
                            self.dyn_handle(&p.segments[0].name)
                        {
                            stmts.extend(self.dyn_decl_init_stmts(&p.segments[0], kind, elems));
                            continue;
                        }
                    }
                }
            }
            let span = rhs.span;
            stmts.push(ast::Stmt::Blocking {
                lhs,
                delay: None,
                event: None,
                rhs,
                span,
            });
        }
        let pb = ast::ProceduralBlock {
            kind: ast::ProcKind::Initial,
            sensitivity: None,
            body: Box::new(ast::Stmt::Block {
                label: None,
                decls: vec![],
                stmts,
                span: sp,
            }),
            span: sp,
        };
        // A2a: this synthesized initial holds ONLY declaration initializers —
        // a desugared array parameter's own `= '{…}` is its legitimate
        // one-time init (§6.8), not a user write; the const-param deny must
        // not fire on it.
        let saved = self.lowering_decl_init;
        self.lowering_decl_init = true;
        let proc = self.lower_proc_block(&pb);
        self.lowering_decl_init = saved;
        self.push_process(proc);
    }
}
