//! §6.8 variable-initializer collection and the t0 pre-sweep flush — split out of
//! `netdecl.rs` (mechanical move; module-size policy).

use super::*;

/// The rank key for an INSTANCE-slot scope: `(band, key, sub)` — see
/// [`Elaborator::with_rank_scope_keyed`].
pub(crate) type RankKey = (u32, u32, u32);

impl Elaborator<'_> {
    /// §6.8: collect a VARIABLE declaration's NON-constant initializer
    /// (`logic [7:0] b = a;`) into `pending_var_inits` so a pre-sweep can emit it
    /// as a synthesized `initial b = a;` that runs BEFORE user initial blocks. A
    /// §4.5.257: a CONSTANT initializer is collected too. It used to fold into the net's
    /// `init` field at declaration and be skipped here, which took it out of the
    /// initialization order — see `netdecl.rs`.
    /// A net (wire) decl is not a variable — its initializer is a continuous
    /// driver, handled by `elaborate_net_init_drivers`.
    pub(crate) fn collect_var_init_drivers(&mut self, d: &ast::NetVarDecl) {
        // A `string s = expr;` initializer (v7): the heap-backed string has no
        // foldable `init` field, so its initializer ALWAYS rides this t0 pre-sweep
        // (like a non-constant var init), collected here in declaration order with
        // the other variable initializers. Only a SCALAR string is registered as a
        // net (`elaborate_netvar_decl` rejects packed/unpacked dims), so a dimensioned
        // string's init is skipped here too (the decl already errored loud).
        let scalar_string = matches!(d.dtype.net(), ast::NetVarKind::String)
            && d.dtype.range.is_none()
            && d.dtype.packed_dims().is_empty();
        if !netvar_kind_is_var(d.dtype.net()) && !scalar_string {
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
                        if self.has_fixed_string_array_storage(&name.name.name, name.name.span) {
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
                // §4.5.257: a CONSTANT initializer is collected too. It used to be folded
                // into `net.init` and skipped here, which removed it from the
                // initialization order entirely.
            }
            // §3 ⑤ ⓒ: a header array parameter's override / whole-array default
            // replaces the declared pattern (same lvalue, same flush slot).
            let init = match self.array_param_init_pattern(d, name) {
                crate::const_eval::ArrayParamInit::Keep => init.clone(),
                crate::const_eval::ArrayParamInit::Skip => continue,
                crate::const_eval::ArrayParamInit::Use(e) => e,
            };
            let path = ast::HierPath {
                segments: vec![name.name.clone()],
                span: name.name.span,
            };
            self.pending_var_inits
                .push((ast::Lvalue::Ident(path), init));
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
        if netvar_kind_is_var(d.dtype.net()) || matches!(d.dtype.net(), ast::NetVarKind::String) {
            // A variable's initializer is handled by `collect_var_init_drivers`
            // (a pre-sweep, so the synthesized `initial` runs before user blocks);
            // here a variable decl contributes no continuous driver.
            return;
        }
        // IEEE §6.1.3: an optional net-declaration delay (`wire #3 w = a;`) applies
        // to EVERY net-decl-assignment in this decl, IDENTICAL to a delay on the
        // equivalent `assign #3 w = a;` — fold it the same way (uniform delay +
        // distinct rise/fall/turnoff `ca_delays` sidecar + S1 runtime
        // `ca_delay_exprs`), through the one funnel `elaborate_cont_assign`
        // calls, so `wire #(dv) w = a;` and `assign #(dv) w = a;` cannot answer
        // differently. `None` (no delay, the common case) ⇒ `(None, None, None)`
        // ⇒ byte-identical to before.
        let (delay, rft, rt, zero_scope) = self.fold_ca_delay_rt(d.delay.as_ref());
        for name in &d.names {
            let Some(init) = &name.init else {
                continue;
            };
            let path = ast::HierPath {
                segments: vec![name.name.clone()],
                span: name.name.span,
            };
            let lhs = self.lower_lvalue(&ast::Lvalue::Ident(path));
            // `'{default: v}` on a packed target (`packed_pattern.rs`).
            let packed = self.packed_default_rhs(&lhs, init);
            let init = packed.as_ref().unwrap_or(init);
            let rhs_id = self.lower_expr(init);
            // §5.7.1: context-determined fill literal → lvalue width. THE SAME CALL
            // `elaborate_cont_assign` makes, because this IS that construct — a net
            // declaration initializer is an implicit continuous assign, and the two
            // must not disagree about how wide `'1` is. Without it the fill kept its
            // 1-bit self-determined width and was then zero-extended, so
            // `wire [7:0] a = '1;` read 00000001 against both oracles' 11111111 while
            // the spelled-out `assign a = '1;` beside it was correct.
            let rhs_id = self.resize_rhs_for_lvalue(init, rhs_id, &lhs);
            // The index of THIS cont-assign is the len BEFORE the push (matches
            // `elaborate_cont_assign`'s sidecar keying).
            let idx = self.cont_assigns.len() as u32;
            if let Some(rft) = rft {
                self.ca_delays.insert(idx, rft);
            }
            if let Some(rt) = rt {
                self.ca_delay_exprs.insert(idx, rt);
            }
            if zero_scope {
                self.ca_zero_scope.insert(idx);
            }
            // R14: a NET declaration initializer is an implicit continuous
            // assign (see the width note above) — labelled apart from a spelled
            // `assign` because that is what the reader will be looking for in
            // the source. Anchored on the declared NAME's span, not the decl's,
            // so `wire a = x, b = y;` gives two distinguishable rows.
            self.push_cont_assign(
                ir::ContAssign {
                    lhs,
                    rhs: rhs_id,
                    delay,
                },
                "net_init",
                Some(name.name.span),
                // §5.2 Q-WEXTPOS C2: a kind no write-view reader prints, so no position.
                None,
            );
        }
    }

    /// The synthesized declaration-initializer processes, in INITIALIZATION order.
    ///
    /// The engine runs these to completion BEFORE arming anything — measured: iverilog
    /// applies a declaration initializer without producing an event, so `reg clk = 0;`
    /// does not give `always @clk` an X→0 edge, and neither does a NON-constant
    /// `int nc = src + 1;`. Running them as ordinary t0 processes did both. IEEE 1800
    /// §6.21 says the assignment happens "before any initial or always block starts",
    /// which is exactly a pre-arm phase, not a low ProcId.
    pub(crate) fn init_procs(&self) -> Vec<u32> {
        let mut v: Vec<(&Vec<u32>, u32)> = self.init_ranks.iter().map(|(p, r)| (r, *p)).collect();
        v.sort();
        v.into_iter().map(|(_, p)| p).collect()
    }

    /// Slot numbers inside one scope's initialization rank. Two tables because the two
    /// scope kinds order their parts differently — measured, not assumed.
    /// A PACKAGE initializes before every root instance — packages elaborate first and
    /// module code observes their values at t0. Root instances take `RANK_MOD_INSTANCE`,
    /// so a slot below it puts packages ahead of the whole hierarchy.
    pub(crate) const RANK_PACKAGE: u32 = 0;
    pub(crate) const RANK_MOD_GENERATE: u32 = 0;
    pub(crate) const RANK_MOD_INSTANCE: u32 = 1;
    pub(crate) const RANK_MOD_OWN: u32 = 2;
    pub(crate) const RANK_MOD_BLOCK_LOCAL: u32 = 3;
    pub(crate) const RANK_GEN_INSTANCE: u32 = 0;
    pub(crate) const RANK_GEN_OWN: u32 = 1;
    pub(crate) const RANK_GEN_BLOCK_LOCAL: u32 = 2;
    pub(crate) const RANK_GEN_NESTED: u32 = 3;

    /// Run `f` with the rank path extended by this scope's `(slot, seq)`. `seq` comes from
    /// the ENCLOSING scope's counter, so siblings in one slot keep source order, and the
    /// counter resets inside so each scope numbers its own children independently.
    pub(crate) fn with_rank_scope<R>(&mut self, slot: u32, f: impl FnOnce(&mut Self) -> R) -> R {
        let seq = self.rank_seq[slot as usize];
        self.rank_seq[slot as usize] += 1;
        self.rank_path.push(slot);
        self.rank_path.push(seq);
        let saved = std::mem::take(&mut self.rank_seq);
        let r = f(self);
        self.rank_seq = saved;
        self.rank_path.truncate(self.rank_path.len() - 2);
        r
    }

    /// [`Self::with_rank_scope`] with an EXPLICIT key instead of the scope counter.
    ///
    /// §4.5.260: an interface instance and a module instance share the instance slot but
    /// are elaborated in DIFFERENT passes (interfaces in Nets, module children in
    /// Instances), so a per-scope counter cannot order them — every interface drew a lower
    /// number than every module child regardless of source order. Both pass the declaring
    /// name's source offset, which is what "declaration order" means and is the same in
    /// every pass.
    ///
    /// §4.5.261: three components, because one offset cannot carry all three questions.
    /// `band` separates instances DECLARED in this scope's body from ones a `bind`
    /// injected — a bind directive's offset lives in the compilation unit, not in the
    /// target module's body, so comparing the two as "declaration order" is meaningless
    /// and made the answer depend on where the `bind` line sat. `sub` is the
    /// instance-ARRAY element index: sharing one key does NOT let `init_procs` tie-break
    /// by ProcId, because an element's child scopes and its own variables produce
    /// DIFFERENT rank vectors, so the sort grouped by slot ACROSS elements and interleaved
    /// them. Distinct keys per element is what keeps each element's subtree together.
    pub(crate) fn with_rank_scope_keyed<R>(
        &mut self,
        slot: u32,
        key: RankKey,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.rank_path.push(slot);
        self.rank_path.push(key.0);
        self.rank_path.push(key.1);
        self.rank_path.push(key.2);
        let saved = std::mem::take(&mut self.rank_seq);
        let r = f(self);
        self.rank_seq = saved;
        self.rank_path.truncate(self.rank_path.len() - 4);
        r
    }

    /// The rank for an initializer process emitted by THIS scope in `slot`.
    pub(crate) fn init_rank(&mut self, slot: u32) -> Vec<u32> {
        let seq = self.rank_seq[slot as usize];
        self.rank_seq[slot as usize] += 1;
        let mut r = self.rank_path.clone();
        r.push(slot);
        r.push(seq);
        r
    }

    /// Which slot a scope of the given kind occupies inside its PARENT — the parent's kind
    /// decides, which is why `in_generate_body` is read at the point of entry.
    pub(crate) fn rank_slot_for_instance(&self) -> u32 {
        if self.in_generate_body {
            Self::RANK_GEN_INSTANCE
        } else {
            Self::RANK_MOD_INSTANCE
        }
    }

    pub(crate) fn rank_slot_for_generate(&self) -> u32 {
        if self.in_generate_body {
            Self::RANK_GEN_NESTED
        } else {
            Self::RANK_MOD_GENERATE
        }
    }

    /// [`Self::flush_pending_var_inits`], recording the emitted process's initialization
    /// rank in `slot`. A no-op flush consumes no `seq`, so the ranks stay dense and a
    /// design that emits nothing is unaffected.
    pub(crate) fn flush_ranked(&mut self, slot: u32) {
        if self.pending_var_inits.is_empty() {
            return;
        }
        let rank = self.init_rank(slot);
        let pid = self.processes.len() as u32;
        self.flush_pending_var_inits();
        self.init_ranks.insert(pid, rank);
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
                            self.dyn_handle(&p.segments[0].name, p.span)
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
        let proc = self.lower_synth_proc(&pb, "var_init");
        self.lowering_decl_init = saved;
        self.push_process(proc);
    }
}
