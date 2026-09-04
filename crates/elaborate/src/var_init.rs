//! §6.8 variable-initializer collection and the t0 pre-sweep flush — split out of
//! `netdecl.rs` (mechanical move; module-size policy).

use super::*;

/// The rank key for an INSTANCE-slot scope: `(band, key, sub)` — see
/// [`Elaborator::with_rank_scope_keyed`].
pub(crate) type RankKey = (u32, u32, u32);

/// Does this statement tree DECLARE a block-local (or `fork` block) variable named
/// `name`, shadowing a module-scope one?
///
/// ⚠️ This is the guard that lets the diagnostic below be an ERROR rather than a
/// warning, and it was written because the check had a measured false positive.
/// `stmt_never_writes_ident` is the definite-assignment walk, built for ACCEPT
/// GATES where over-approximating a write is the safe direction — it is name-based
/// and treats an unresolved call as writing everything. Ask it about
///
/// ```text
///   int n = 7;                       // module scope, written by nobody
///   always_ff @(posedge clk) begin
///     int n;                         // a block-local SHADOW
///     n = 3;
///   end
/// ```
///
/// and it answers "written", because the two `n`s are one name to it. As a warning
/// that is a nuisance; as an error it rejects RTL that iverilog, verilator and
/// xrun all accept.
///
/// ⚠️⚠️ Suppressing the DIAGNOSTIC does not make that design correct here: vita
/// prints `n=3` where both oracles print `n=7`, because v1 flattens a procedural
/// block-local onto a module net BY BARE NAME and the two coalesce. That is a
/// pre-existing silent-wrong of its own, entirely separate from the driver
/// question — and it is the reason this guard is written as "the source declares a
/// shadow", which is a fact about the SOURCE, rather than as anything about which
/// net vita happens to use.
fn declares_local_named(stmts: &[ast::Stmt], name: &str) -> bool {
    fn decl_hit(decls: &[ast::NetVarDecl], name: &str) -> bool {
        decls
            .iter()
            .any(|d| d.names.iter().any(|n| n.name.name == name))
    }
    stmts.iter().any(|st| match st {
        ast::Stmt::Block { decls, stmts, .. } | ast::Stmt::Fork { decls, stmts, .. } => {
            decl_hit(decls, name) || declares_local_named(stmts, name)
        }
        ast::Stmt::If { then_s, else_s, .. } => {
            declares_local_named(std::slice::from_ref(then_s), name)
                || else_s
                    .as_deref()
                    .is_some_and(|e| declares_local_named(std::slice::from_ref(e), name))
        }
        ast::Stmt::For { body, .. }
        | ast::Stmt::While { body, .. }
        | ast::Stmt::Repeat { body, .. }
        | ast::Stmt::Forever { body, .. } => declares_local_named(std::slice::from_ref(body), name),
        // The timing statements carry an OPTIONAL body (`@(posedge clk) begin … end`
        // is one of them, and it is the shape an `always_ff` almost always has).
        ast::Stmt::DelayCtrl { body, .. }
        | ast::Stmt::EventCtrl { body, .. }
        | ast::Stmt::Wait { body, .. } => body
            .as_deref()
            .is_some_and(|b| declares_local_named(std::slice::from_ref(b), name)),
        ast::Stmt::Case { items, .. } => items.iter().any(|it| {
            let (ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. }) = it;
            declares_local_named(std::slice::from_ref(body), name)
        }),
        _ => false,
    })
}

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
            );
        }
    }

    /// Move this scope's decl-time PRE-SIZE writes (`pending_scoped_presize[cur_prefix]`)
    /// to the front of `pending_var_inits`, so a routed string array's `new[n]` precedes
    /// the element writes the collectors are about to push. Called once per flush point —
    /// module/interface scope (the `""` key) and each generate scope.
    /// Only the entries this scope OWNS: an unlabeled generate body shares its parent's
    /// prefix, and draining its pre-size here would run `new[n]` in a later process than
    /// the element writes its own flush already emitted — wiping them silently.
    pub(crate) fn drain_scoped_presize(&mut self) {
        let Some(v) = self.pending_scoped_presize.remove(&self.cur_prefix) else {
            return;
        };
        let (mine, theirs): (Vec<_>, Vec<_>) = v
            .into_iter()
            .partition(|(owner, ..)| *owner == self.rank_path);
        if !theirs.is_empty() {
            self.pending_scoped_presize
                .insert(self.cur_prefix.clone(), theirs);
        }
        self.pending_var_inits
            .splice(0..0, mine.into_iter().map(|(_, l, r)| (l, r)));
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

    /// Is the active prefix a block-local `$blk$<lo>` scope? That is exactly the shape
    /// `flush_block_local_inits` claims, so it is also the test for "this scope's t0 writes
    /// are replayed by that flush, not by the enclosing scope's".
    pub(crate) fn in_block_local_scope(&self) -> bool {
        self.cur_prefix
            .rsplit('.')
            .next()
            .is_some_and(|seg| seg.starts_with("$blk$"))
    }

    /// Record one block-local declaration initializer for the deferred, declaration-ordered
    /// replay in [`Self::flush_block_local_inits`]. `scope` is the `$blk$<lo>` segment when
    /// the declaration earned its own scope; `lo` is the declaring name's source offset.
    pub(crate) fn push_block_local_init(
        &mut self,
        scope: Option<&str>,
        lo: u32,
        lhs: ast::Lvalue,
        rhs: ast::Expr,
    ) {
        let key = self.scoped_init_key(scope);
        let owner = self.rank_path.clone();
        self.pending_block_local_inits
            .entry(key)
            .or_default()
            .push((lo, owner, lhs, rhs));
    }

    /// Every recorded block-local initializer must have been claimed by exactly one flush
    /// point. One left behind is an initializer the design wrote and the IR does not carry
    /// — a silent-wrong by construction, so say so instead of returning a clean IR. Only
    /// checked on the success path: an already-failed elaboration may bail before a flush.
    pub(crate) fn assert_block_local_inits_drained(&mut self) {
        // The pre-size map is checked with it: a routed string array whose `new[n]` is
        // never emitted stays length 0, and every element write is discarded — the exact
        // silent-wrong shape this guard exists for.
        let orphans = self.pending_block_local_inits.len() + self.pending_scoped_presize.len();
        if self.had_error || orphans == 0 {
            return;
        }
        let keys: Vec<String> = self
            .pending_block_local_inits
            .keys()
            .chain(self.pending_scoped_presize.keys())
            .cloned()
            .collect();
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "internal: block-local declaration initializers recorded under {} were \
                 never emitted — no flush point claimed the scope(s) `{}`; the design's \
                 initializers would be missing from the simulation",
                keys.len(),
                keys.join("`, `")
            ),
        );
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

    /// §4.5.254: flush this scope's collected module-scope initializers and then EVERY
    /// block-local one belonging to it, in DECLARATION order across blocks.
    ///
    /// Measured order (live iverilog): all module-scope statics first, then the
    /// block-locals by declaration offset — whether or not the declaring block earned a
    /// `$blk$` scope, and whether or not the declaration is a string. A run of consecutive
    /// same-prefix initializers becomes one synthesized `initial`; a t0 `initial` runs in
    /// ProcId order, so emitting the runs in order preserves the global order across them.
    /// A LEADING run in this very scope joins the main sweep's `initial` instead of adding
    /// a process — which is the whole design in every module with no scoped block-local, so
    /// their IR is unchanged.
    pub(crate) fn flush_block_local_inits(&mut self) {
        // At FLUSH time, not when the scope's walk opened. A generate body nested in
        // another shares its prefix, and the outer call's walk begins before the inner
        // one's flush — draining there handed the inner body's pre-size to the OUTER
        // process, so `new[n]` ran after the element writes the inner had already emitted
        // and silently wiped them. Flush order is innermost-first, which is ownership
        // order. Still spliced to the front of this scope's list, so the pre-size keeps
        // preceding the writes that ride it.
        self.drain_scoped_presize();
        let here = self.cur_prefix.clone();
        let dot = if here.is_empty() {
            String::new()
        } else {
            format!("{here}.")
        };
        // This scope owns its own key and every `$blk$` key BELOW it — at any depth, as
        // long as the whole remainder is `$blk$` segments. A generate or instance
        // segment in the remainder means that scope flushes at its own point, and is not
        // claimed here.
        //
        // R16 §3.4: the depth-1 form of this rule (`!rest.contains('.')`) was written
        // when procedural blocks could never nest their scopes, because the classifier
        // dropped any candidate block inside another candidate. With that restriction
        // lifted, a block-local inside a scoped block inside a scoped block records
        // under `t.$blk$<outer>.$blk$<inner>`, which no scope claimed — caught not by a
        // test but by the never-emitted guard, which named both orphaned keys.
        let is_here = |k: &String| {
            *k == here
                || k.strip_prefix(&dot)
                    .is_some_and(|rest| rest.split('.').all(|seg| seg.starts_with("$blk$")))
        };
        let keys: Vec<String> = self
            .pending_block_local_inits
            .keys()
            .filter(|k| is_here(k))
            .cloned()
            .collect();
        // …and only the entries this scope OWNS, by the same flag the pre-size drain uses.
        // A generate body that vita gives no prefix segment shares the MODULE's key, so a
        // generate walk's flush matched the module's own block-locals and emitted them
        // ahead of the module sweep — which both robbed a module block-local initializer of
        // the module variables it reads and split a routed string array from its `new[n]`
        // (left behind by the flag-partitioned pre-size drain), emptying it at exit 0.
        // Prefix answers WHICH scope; the owner's RANK PATH answers whose. §4.5.265: a
        // bool could not, because two NESTED generate scopes sharing a prefix are both
        // "in a generate" — so a prefix-less nested scope claimed its PARENT's
        // block-locals and emitted them under its own (later) rank.
        let ours = self.rank_path.clone();
        let mut all: Vec<(u32, String, ast::Lvalue, ast::Expr)> = Vec::new();
        for key in keys {
            let Some(v) = self.pending_block_local_inits.remove(&key) else {
                continue;
            };
            let (mine, theirs): (Vec<_>, Vec<_>) =
                v.into_iter().partition(|(_, owner, ..)| *owner == ours);
            if !theirs.is_empty() {
                self.pending_block_local_inits.insert(key.clone(), theirs);
            }
            all.extend(
                mine.into_iter()
                    .map(|(lo, _, l, r)| (lo, key.clone(), l, r)),
            );
        }
        // Stable: one declaration's several element writes share an offset and keep the
        // order the expansion built them in.
        all.sort_by_key(|(lo, ..)| *lo);
        // Split into consecutive same-prefix runs.
        let mut runs: Vec<(String, Vec<(ast::Lvalue, ast::Expr)>)> = Vec::new();
        for (_, key, lhs, rhs) in all {
            match runs.last_mut() {
                Some((k, v)) if *k == key => v.push((lhs, rhs)),
                _ => runs.push((key, vec![(lhs, rhs)])),
            }
        }
        // A leading run declared in THIS scope needs no process of its own.
        if runs.first().is_some_and(|(k, _)| *k == here) {
            let (_, v) = runs.remove(0);
            self.pending_var_inits.extend(v);
        }
        let own = if self.in_generate_body {
            Self::RANK_GEN_OWN
        } else {
            Self::RANK_MOD_OWN
        };
        self.flush_ranked(own);
        let bl = if self.in_generate_body {
            Self::RANK_GEN_BLOCK_LOCAL
        } else {
            Self::RANK_MOD_BLOCK_LOCAL
        };
        for (key, v) in runs {
            let saved = std::mem::replace(&mut self.cur_prefix, key);
            self.pending_var_inits = v;
            // No pre-size drain here: a `$blk$`-scope pre-size is recorded in THIS list
            // (`route_fixed_string_array`), ahead of the element writes it must precede,
            // because it is pushed at the declaration and they are pushed by the collector
            // that runs after it. `pending_scoped_presize` never holds a `$blk$` key — and
            // if one ever appeared, `assert_block_local_inits_drained` reports it loudly
            // rather than letting the array stay length 0.
            self.flush_ranked(bl);
            self.cur_prefix = saved;
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
        let proc = self.lower_synth_proc(&pb, "var_init");
        self.lowering_decl_init = saved;
        self.push_process(proc);
    }
}

impl Elaborator<'_> {
    /// IEEE §9.2.2.2: a variable written by `always_comb` may have NO other driver,
    /// and a declaration INITIALIZER is another driver.
    ///
    /// ⭐ vita ran `logic rdy = 1'b1; always_comb rdy = …;` at exit 0 with nothing to
    /// say, while xrun stops elaboration (`*E,MULAXX`) and verilator errors
    /// (MULTIDRIVEN). That combination is the expensive kind of quiet: the design is
    /// green in the development loop and dies at sign-off, and the loop is where the
    /// author still has the context to fix it. Nothing about the simulated VALUE
    /// changes here — this is the diagnostic that was missing, not a semantics change.
    ///
    /// ⚠️⚠️ **`always_comb` ONLY, and the two neighbours are deliberately out.**
    /// An external report asked for `always_ff` and `always_latch` as well, on the
    /// ground that the clause "does not vary by block kind". It does. Measured, one
    /// kind per file, verilator 5.050 `--lint-only -Wall`:
    ///
    /// ```text
    ///   always_comb  + initializer -> MULTIDRIVEN (cites IEEE 1800-2023 9.2.2.2)
    ///   always_ff    + initializer -> PROCASSINIT only  (a style note)
    ///   always_latch + initializer -> PROCASSINIT only
    /// ```
    ///
    /// iverilog says nothing about any of the three. The distinction is not an
    /// oversight in verilator either — it is what the rule is FOR. `always_comb`
    /// models combinational logic, whose output must be a function of its inputs at
    /// all times, so any other write destroys the property the procedure asserts.
    /// `always_ff` models a REGISTER, and a declaration initializer is that
    /// register's power-on value — `logic [7:0] c = 0; always_ff @(posedge clk) c <=
    /// c + 1;` is the ordinary FPGA initialization idiom, which synthesis tools
    /// implement and which this repository's own `obs_procs` fixture is written in.
    ///
    /// The widened version was built and reverted: it rejected that fixture, and a
    /// test design breaking is evidence AGAINST a new rejection, not for it.
    ///
    /// A pure AST pass over the module body: the initializer set comes from the
    /// declarations, the driver set from `stmt_never_writes_ident` — the same
    /// conservative write walk the definite-assignment analysis uses, so a write
    /// through a task call or a nested block counts exactly as it does there.
    pub(crate) fn warn_always_comb_initializers(&mut self, body: &[ast::ModuleItem]) {
        // (name, span) of every module-scope variable with a declaration initializer.
        let mut inited: Vec<(String, ast::Span)> = Vec::new();
        for item in body {
            if let ast::ModuleItem::NetVar(d) = item {
                if !netvar_kind_is_var(d.kind) {
                    continue; // a `wire` initializer is a continuous assign, not this
                }
                for n in &d.names {
                    if n.init.is_some() {
                        inited.push((n.name.name.clone(), d.span));
                    }
                }
            }
        }
        if inited.is_empty() {
            return;
        }
        // `always_comb` alone — the match is `_`-free over `ProcKind` so that adding
        // a procedure kind is a forced decision here rather than a silent default,
        // and so that the three deliberate exclusions are visible as code.
        let inferred: Vec<&ast::Stmt> = body
            .iter()
            .filter_map(|it| match it {
                ast::ModuleItem::Proc(p) => match p.kind {
                    ast::ProcKind::AlwaysComb => Some(&*p.body),
                    // Registers and latches take a power-on value from a
                    // declaration initializer; measured, verilator calls neither
                    // of these a multiple driver. See the doc above.
                    ast::ProcKind::AlwaysFf | ast::ProcKind::AlwaysLatch => None,
                    // `always #5 clk = ~clk;` beside `logic clk = 0;` is the clock
                    // generator every testbench has.
                    ast::ProcKind::Always | ast::ProcKind::Initial | ast::ProcKind::Final => None,
                },
                _ => None,
            })
            .collect();
        if inferred.is_empty() {
            return;
        }
        for (name, span) in inited {
            // FIRST writer only: two `always_ff` blocks both writing an initialized
            // variable is one diagnostic about the initializer, not one per block —
            // the span reported is the DECLARATION's either way, so a second copy
            // would repeat itself at the same caret.
            //
            // ⚠️ The SHADOW guard runs first, and it is what makes an error
            // defensible: a procedure that declares its own `name` is writing THAT
            // one, and the write walk cannot tell them apart (see
            // `declares_local_named`). Skipping the whole procedure — rather than
            // just its declaring block — is the conservative direction for a
            // diagnostic that now stops the run.
            let driven = inferred.iter().any(|s| {
                let one = std::slice::from_ref(*s);
                !declares_local_named(one, &name) && !stmt_never_writes_ident(one, &name, None)
            });
            if !driven {
                continue;
            }
            self.error_at(
                MsgCode::ElabMultidriver,
                span,
                &format!(
                    "variable `{name}` has a declaration initializer AND is written by \
                     `always_comb`, which is two drivers on one variable (IEEE §9.2.2.2) \
                     — drop the initializer or the `always_comb` write"
                ),
            );
        }
    }
}
