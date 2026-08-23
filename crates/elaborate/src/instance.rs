//! module instantiation — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// Delay multiplier `M = 10^(unit_exp − global_prec_exp)` for module `name`
    /// (≥ 1, since every module's `unit_exp ≥ global_prec_exp`). A module absent from
    /// the map (the no-timescale base) defaults to `unit_exp = global_prec_exp` ⇒ M=1.
    /// The exponent is capped at 18 so `10u64.pow` never overflows on an absurd ratio.
    pub(crate) fn module_mult(&self, name: &str) -> u64 {
        let u = self
            .mod_unit_exp
            .get(name)
            .copied()
            .unwrap_or(self.global_prec_exp);
        let diff = (u - self.global_prec_exp).max(0) as u32;
        10u64.pow(diff.min(18))
    }

    /// `S = 10^(prec_exp − global_prec_exp)` for module `name`: one step of the
    /// module's OWN precision in global ticks (≥ 1). A module absent from the
    /// map (legacy 4-arg entry / no-timescale base) defaults to prec == global
    /// ⇒ S = 1 ⇒ the two-stage delay conversion degenerates to the old single
    /// rounding. Same 18-exponent cap as [`Self::module_mult`].
    pub(crate) fn module_prec_mult(&self, name: &str) -> u64 {
        let p = self
            .mod_prec_exp
            .get(name)
            .copied()
            .unwrap_or(self.global_prec_exp);
        let diff = (p - self.global_prec_exp).max(0) as u32;
        10u64.pow(diff.min(18))
    }

    /// Record a fork's join MODE into the side table. The engine's lookup is
    /// total-or-fatal, so every fork MUST record. A base-process fork keys by
    /// `(cur_proc, process-local join_bb)`. Stage-1 fork-in-frame: a fork inside a
    /// TASK body (`frame_task_lowering`) instead collects `(local join_bb, mode)` into
    /// `pending_fork_modes`, rebased to `(FRAME_FORK_KEY, global join_bb)` at the body
    /// flush (the same +base rebase the nested task-call keys use), because its blocks
    /// live in the global `func_blocks` arena and its mode must not depend on which
    /// process runs the task.
    pub(crate) fn record_fork_mode(&mut self, join: ast::JoinKind, join_bb: u32) {
        let mode = match join {
            ast::JoinKind::Join => JoinMode::All,
            ast::JoinKind::JoinAny => JoinMode::Any,
            ast::JoinKind::JoinNone => JoinMode::None,
        };
        if self.frame_task_lowering {
            self.pending_fork_modes.push((join_bb, mode));
        } else {
            self.fork_modes.insert((self.cur_proc, join_bb), mode);
        }
    }

    /// Recursively elaborate ONE module instance into the flat SimIr.
    ///
    /// Bookkeeping order is the load-bearing determinism contract:
    ///  (1) cycle guard; (2) reserve the Instance slot + record `first_net`;
    ///  (3) bind params (so width exprs fold); (4) create THIS instance's nets
    ///  (ANSI ports, then body NetVarDecls — declaration order); (5) patch
    ///  `net_count`; (6) wire ports (parent expr ↔ child port net) as cont-assigns;
    ///  (7) lower THIS body's cont-assigns + processes; (8) recurse into child
    ///  instances in body declaration order.
    ///
    /// Step (8) runs strictly AFTER (4), so the parent's `[first_net,
    /// first_net+net_count)` slice is a contiguous run with no child nets spliced
    /// in — the Instance slice invariant.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn elaborate_instance(
        &mut self,
        module: &ast::ModuleDecl,
        inst_path: &str,
        parent_inst: Option<u32>,
        param_overrides: &[ResolvedOverride],
        binding: PortBinding<'_>,
        map: &ModuleMap<'_>,
        rank_key: crate::var_init::RankKey,
        inst_span: Option<ast::Span>,
    ) {
        // (1) CYCLE GUARD — recursive instantiation is illegal (LRM). Bail this
        //     subtree WITHOUT creating any net/Instance so the arena stays valid.
        if self.inst_stack.iter().any(|m| m == &module.name.name) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "recursive module instantiation of `{}` (cycle: {} -> {})",
                    module.name.name,
                    self.inst_stack.join(" -> "),
                    module.name.name
                ),
            );
            return;
        }
        self.inst_stack.push(module.name.name.clone());
        // The directive governs the module's DECLARATION site, so it is saved/restored
        // around this instance rather than inherited from the instantiator: a module
        // compiled under `default_nettype none keeps that policy wherever it is used.
        let saved_nettype = std::mem::replace(&mut self.cur_nettype_none, module.nettype_none);

        // (2) reserve Instance slot + first_net cursor.
        let inst_id = self.instances.len() as u32;
        let first_net = self.nets.len() as u32;
        self.instances.push(ir::Instance {
            parent: parent_inst,
            module: 0, // sim-engine ignores this; 0 for v3 (no module table needed)
            first_net,
            net_count: 0, // patched in (5)
        });
        // Design-structure export sidecar (parallel to `instances`): the module name and
        // full dotted path this instance elaborates, for `--hier-tree` / `--inst-paths`.
        self.instances_info.push(InstanceInfo {
            path: inst_path.to_string(),
            module: module.name.name.clone(),
            parent: parent_inst,
        });

        // Enter this instance's scope (restored before returning).
        let saved_prefix = std::mem::replace(&mut self.cur_prefix, inst_path.to_string());
        // AMBIENT source anchor for this subtree: the instantiation site. Port
        // wiring, parameter binding and every structural check happen before any
        // statement sets `cur_span`, so without this they report no location at
        // all — measured on picorv32: 71 diagnostics, 0 with a `file:line:col`.
        // A statement still overrides it and restores it, so the more specific
        // anchor always wins; this only fills the gap where nothing else can,
        // and there is no `return` between here and the restore, so it cannot
        // leak into the parent's scope. A ROOT has no instantiation site and
        // falls back to its own declaration — the only source location that
        // exists for it, and better than being the one class with no anchor.
        let saved_span = std::mem::replace(
            &mut self.cur_span,
            Some(inst_span.unwrap_or(module.name.span)),
        );
        // A module body is never "inside a generate", however it was reached. Without this
        // a child instantiated inside a generate elaborated its WHOLE body with the flag
        // stuck on, so its own module-scope block-locals were tagged as generate-owned and
        // claimed by the wrong flush — the ownership question is per-body, and an instance
        // boundary starts a new body.
        // Read BEFORE the flag is cleared: the slot depends on the PARENT's kind.
        let rank_slot = self.rank_slot_for_instance();
        let saved_in_gen = std::mem::replace(&mut self.in_generate_body, false);
        // §4.5.262: and neither is a module body "inside a bind". The band says how THIS
        // instance was reached; the children it declares itself are ordinary body items
        // however their parent got here. Leaving it set made a module reached through a
        // bind key its own children band 1 too — so if that module was itself a bind
        // target, its body children and its bound children collided on band and fell back
        // to comparing a body offset against a compilation-unit one, which is the exact
        // comparison the band exists to prevent.
        let saved_band = std::mem::replace(&mut self.rank_band, 0);
        // The DECLARING position, not a counter — see `with_rank_scope_keyed`.
        self.rank_path.push(rank_slot);
        self.rank_path.push(rank_key.0);
        self.rank_path.push(rank_key.1);
        self.rank_path.push(rank_key.2);
        let saved_rank_seq = std::mem::take(&mut self.rank_seq);
        // Record this as the instance currently being lowered, so a child created
        // inside a generate block can set its `Instance.parent` to `inst_id`.
        let saved_inst = std::mem::replace(&mut self.cur_inst, inst_id);
        // This module's delay multiplier governs every `#delay` and process lowered
        // in its body (restored on the way out, like cur_prefix/cur_inst).
        let new_mult = self.module_mult(&module.name.name);
        let saved_mult = std::mem::replace(&mut self.cur_time_mult, new_mult);
        let new_prec_mult = self.module_prec_mult(&module.name.name);
        let saved_prec_mult = std::mem::replace(&mut self.cur_prec_mult, new_prec_mult);

        // (3) bind params (defaults, then overrides) — BEFORE nets so [W-1:0] folds.
        // Merge any `defparam` overrides targeting THIS instance (FQ `inst_path`)
        // with the instantiation `#()` overrides. A defparam wins over `#()` per
        // IEEE §23.10.1, so it is appended LAST (bind_params is last-write-wins).
        // `remove` CONSUMES the entry — any defparam left in the map after the whole
        // hierarchy is elaborated matched no instance (a typo, or an array `u.N`
        // without an index) and is warned about below (iverilog parity: it warns and
        // the target keeps its default, which is exactly what an unconsumed entry
        // already produces here).
        let dp_overrides: Vec<ResolvedOverride> = self
            .defparams
            .remove(inst_path)
            .map(|dps| {
                dps.into_iter()
                    .map(|(param, v, fill)| ResolvedOverride {
                        name: Some(param),
                        value: Some(v),
                        is_named: true,
                        had_value: true,
                        // Carried verbatim so `bind_one_param` re-folds it at the
                        // TARGET's declared width — the same channel `#(.K('1))`
                        // uses. `v` above is the parent-side 32-bit fold and is the
                        // fallback for everything that is not a fill.
                        fill,
                        // A defparam carries no text at all, so the flag is never
                        // read here — `false` is the honest value for "not a literal".
                        str_is_literal: false,
                        str: None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let (mut saved_params, param_ovr) = if dp_overrides.is_empty() {
            self.bind_params(module, param_overrides)
        } else {
            let mut merged = param_overrides.to_vec();
            merged.extend(dp_overrides);
            self.bind_params(module, &merged)
        };

        // (3a.5) v7 P2-D imports — CONST symbols bind first so body params
        //        and ranges can use them; a later LOCAL declaration of the
        //        same name simply rebinds (local wins, iverilog-pinned).
        //        Function/task imports apply after (3.5) below.
        let import_list: Vec<ast::ImportDecl> = self
            .cu_imports
            .clone()
            .into_iter()
            .chain(module.body.iter().filter_map(|it| match it {
                ast::ModuleItem::Import(i) => Some(i.clone()),
                _ => None,
            }))
            .collect();
        // SVPART: wildcard-import ambiguity (IEEE §26.8). Track each wildcard-bound
        // name's origin package and the names bound by an EXPLICIT import (which
        // always wins). A name made available by two DIFFERENT wildcard imports is
        // ambiguous → unbound, so referencing it is a loud "undefined" at the use
        // site (never a silent last-wins value), while an UNUSED ambiguous name is
        // no error — exactly IEEE's "ambiguous only when referenced".
        let mut wc_origin: BTreeMap<String, String> = BTreeMap::new();
        let mut explicit_imports: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        for imp in &import_list {
            self.apply_import_consts(
                imp,
                &mut saved_params,
                &mut wc_origin,
                &mut explicit_imports,
            );
        }

        // (3b) BODY-level `parameter`/`localparam` (a `ModuleItem::Param`, NOT in
        //      the ANSI `#(...)` header that `bind_params` handles). Bind in decl
        //      order — a `localparam C = A*B+1` may reference an earlier param —
        //      BEFORE nets so `[W-1:0]` folds and runtime refs (`x = P`) resolve.
        //      Net decls in the SAME walk pre-register their widths (v7), so a
        //      decl-order `localparam X = $bits(mem[0])` folds too.
        // GAP-G shadow guard: gather every bare name this module declares
        // locally (header params, ports, top-level body nets/params) BEFORE any
        // body const-eval, so a local name that shadows a wildcard-imported array
        // is known regardless of decl order (catches forward refs and ports).
        let names = self.gather_local_decl_names(module);
        // IEEE §6.10 ordering — see the `decl_pos` field doc. Same walk as `names`.
        let dpos = self.gather_decl_positions(module);
        let mut saved_decl_pos = std::mem::replace(&mut self.decl_pos, dpos);
        let mut saved_dpos_scope =
            std::mem::replace(&mut self.decl_pos_scope, self.cur_prefix.clone());
        let mut saved_dpos_range =
            std::mem::replace(&mut self.decl_pos_range, (module.span.lo, module.span.hi));
        let dbl = self.gather_block_local_names(module);
        let mut saved_dbl = std::mem::replace(&mut self.decl_block_locals, dbl);
        // DUP (round-5): decide which colliding `automatic` block-locals get their
        // own `$blk$<span>` scope (pure AST fn; excludes any name that also collides
        // with a module net via `names`). Computed here so both the hoist below and
        // the later body lowering read the SAME decision. See the field doc.
        let scoped_blocks = Self::compute_scoped_block_locals(module, &names);
        let saved_scoped_blocks = std::mem::replace(&mut self.scoped_block_locals, scoped_blocks);
        // r18 (family D): per-entry automatic-with-init block-locals (same shared-set
        // pattern as `scoped_block_locals` — computed once, read by both phases).
        let per_entry_blocks = Self::compute_per_entry_block_locals(module, &names);
        let saved_per_entry_blocks =
            std::mem::replace(&mut self.per_entry_block_locals, per_entry_blocks);
        // R18-X1: which bare block-local names land on ONE shared flattened net. Same
        // shared-set pattern, and for the same reason: the hoist's own coalesce guard
        // is order-dependent (it fires only once the net exists), so the FIRST
        // declaring block would otherwise be analysed as the sole writer of a net it
        // shares. Depends on `scoped_blocks`, so it is computed after it.
        let coalesced =
            Self::compute_coalesced_block_locals(module, &names, &self.scoped_block_locals.clone());
        let saved_coalesced = std::mem::replace(&mut self.coalesced_block_locals, coalesced);
        let saved_local_names = std::mem::replace(&mut self.local_decl_names, names);
        let saved_prescan = std::mem::take(&mut self.bits_prescan);
        // §4.5.186: collect this module's functions into `const_func_table` BEFORE the
        // parameter fold below, so a `localparam W = f(N)` can interpret `f` at compile
        // time. (`func_table` is populated later, in pass 3.5.) Saved/restored with
        // `func_table` at the module-scope exit.
        let mut saved_const_funcs = std::mem::take(&mut self.const_func_table);
        for item in &module.body {
            if let ast::ModuleItem::Func(f) = item {
                self.const_func_table.insert(f.name.name.clone(), f.clone());
            }
        }
        for item in &module.body {
            match item {
                ast::ModuleItem::Param(p) => {
                    // IEEE 1364-2005 §12.2: in a module with no ANSI `#(...)` header
                    // these body declarations ARE the parameter port list, so an
                    // override may target one (`#(.W(8))`, `#(8)`, `defparam u.W=8`,
                    // `-G W=8` — all four resolve through `param_ports`). Binding it
                    // HERE, in the same decl-order walk, is what lets a later
                    // `parameter C = W*2` see the overridden value; and routing it
                    // through the shared `bind_one_param` is what keeps the string /
                    // real / wide / fill / localparam-reject routes to ONE spelling
                    // instead of a second copy that would drift.
                    //
                    // A module that HAS a header list already bound every overridable
                    // name above, and its body parameters are not overridable at all
                    // (iverilog: "Parameter cannot be overridden in the scope it has
                    // been declared in"), so `param_ports` returns the header there.
                    // `targets` is then false for every body name EXCEPT one that
                    // repeats a header name — which iverilog rejects outright ("`W`
                    // has already been declared in this scope") and vita has always
                    // accepted, so the branch decides an illegal design either way.
                    if param_ovr.targets(&p.name.name) {
                        self.bind_one_param(p, &param_ovr, &mut saved_params);
                        continue;
                    }
                    // N5: a string-valued parameter/localparam (`localparam string S =
                    // "abc"`, or the untyped `localparam S = "abc"`). It has no i64
                    // value, so record its raw literal (FQ-keyed, persistent — walk_scopes
                    // only matches from the declaring prefix) and skip the numeric fold.
                    if let Some(raw) = self.param_str_or_folded(p, false) {
                        let key = self.fq(&p.name.name);
                        self.str_param_raw.insert(key, raw);
                    } else if let Some((v, exact)) = self.param_real_value(&p.ty, &p.value) {
                        // r19: a REAL-valued parameter has no i64 value — same shape as
                        // the string case above, so it rides the same side map and skips
                        // the numeric fold that would otherwise loud-reject it. When the
                        // initializer DID fold to an exact i64, register both: the two
                        // representations agree, so `R/2` divides in the real domain and
                        // `logic [R-1:0]` keeps working in the integer domain.
                        let key = self.fq(&p.name.name);
                        self.real_param_val.insert(key.clone(), v);
                        if let Some(i) = exact {
                            // The LOCAL integer view only — an exact-integer real stays
                            // usable as a width and a bound inside this scope. It is
                            // deliberately NOT published to `hier_params`: doing so made
                            // `a.P` resolvable while answering in the wrong domain, and
                            // it also split the answer by whether the declaration
                            // happened to be OVERRIDDEN (only an override or an exact
                            // default ever produced an i64). Hierarchical reads of a
                            // real parameter are loud — ROADMAP §2 owns the axis.
                            saved_params.push((key.clone(), self.params.insert(key, i)));
                        }
                    } else {
                        // Unfoldable value = LOUD error (never a silent 0): a parameter
                        // bound to a wrong default poisons every downstream width with
                        // no trace (P0-5). 0 stays only as the post-error recovery value.
                        // A module-BODY parameter/localparam is not on the
                        // instantiation override channel — the default binds.
                        let meta = self.param_decl_width_unoverridden(p);
                        let folded = self.eval_param_init(&p.value, meta.map(|(w, _)| w));
                        // Wider than the i64 domain — see `wide_param_bits`. Reached
                        // only when the numeric fold DECLINED, so a wide declaration
                        // whose value fits keeps its integer identity.
                        if folded.is_none() {
                            if let Some(cv) = meta
                                .and_then(|(w, sg)| self.wide_param_const_in_scope(&p.value, w, sg))
                            {
                                let key = self.fq(&p.name.name);
                                self.wide_param_bits.insert(key, cv);
                                continue;
                            }
                        }
                        let v = folded.unwrap_or_else(|| {
                            self.param_value_unfoldable("parameter", &p.name.name, &p.value);
                            0
                        });
                        // Same as the generate-scope twin: `meta` already encodes
                        // that no override reached this body parameter.
                        let v = self.coerce_param_value_with(v, meta);
                        let key = self.fq(&p.name.name);
                        // persistent copy for a hierarchical read (`dut.LP`) of a body
                        // parameter/localparam (mirrors the header path in bind_params).
                        self.hier_params.insert(key.clone(), v);
                        if let Some(m) = meta {
                            self.param_meta.insert(key.clone(), m);
                        }
                        if let Some(r) = self.param_decl_range(p) {
                            self.param_range.insert(key.clone(), r);
                        }
                        saved_params.push((key.clone(), self.params.insert(key, v)));
                    }
                }
                ast::ModuleItem::NetVar(d) => {
                    self.prescan_net_bits(d);
                    // GAP-G: capture a const array param's element values in
                    // DECL ORDER, so a later same-walk `localparam X = ROT[2]`
                    // folds (the array param net itself is created later, in the
                    // Nets phase — too late for this bind).
                    self.capture_const_array_vals(d);
                }
                // A2b-prereq S2: record declared genvar names persistently —
                // their `params` binding is transient (unroll-only), but the
                // constant-shadow guard must see them at any lowering point.
                ast::ModuleItem::Genvar { names, .. } => {
                    for n in names {
                        let key = self.fq(&n.name);
                        self.genvar_decls.insert(key);
                    }
                }
                _ => {}
            }
        }

        // (3c) `typedef enum {…}` labels → integer constants (RED=0, GREEN=1, …),
        //      registered like localparams so a runtime `c = GREEN` folds. An
        //      explicit `LABEL = expr` resets the running counter (next = expr+1).
        for item in &module.body {
            if let ast::ModuleItem::Typedef(td) = item {
                #[allow(irrefutable_let_patterns)]
                if let ast::TypedefKind::Enum {
                    base,
                    signed,
                    labels,
                } = &td.kind
                {
                    // A label carries the enum base's DECLARED width so it reads at its
                    // real self-width in a concat (`{4'h5, STATE}`), not 32 bits — the
                    // `param_meta` twin of the localparam path above. §4.5.158: the label
                    // also carries the enum's DECLARED sign (`signed`) so a POSITIVE label
                    // of a SIGNED enum stays signed in a relational/collective context
                    // (`enum byte {B=2}; v > B` = signed) — `|| v < 0` keeps a negative
                    // label signed even on an (illegal) unsigned base (graceful, §4.5.154).
                    // §4.5.158: a base-less `enum {…}` is `int` (32-bit) — give its labels
                    // an explicit 32-bit `param_meta` so the sign fix below reaches them too
                    // (`enum_base_width` returns None for a rangeless base). An unfoldable
                    // range stays None (unknown width, value-inferred as before).
                    let base_w = self
                        .enum_base_width(base)
                        .or_else(|| base.is_none().then_some(32u32));
                    let mut next: i64 = 0;
                    for lab in labels {
                        let v = match &lab.value {
                            Some(e) => self.const_eval_in_scope(e).unwrap_or_else(|| {
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "enum label `{}` value is not a foldable constant",
                                        lab.name.name
                                    ),
                                );
                                0
                            }),
                            None => next,
                        };
                        let key = self.fq(&lab.name.name);
                        if let Some(w) = base_w {
                            self.param_meta.insert(key.clone(), (w, *signed || v < 0));
                        }
                        saved_params.push((key.clone(), self.params.insert(key, v)));
                        next = v.wrapping_add(1);
                    }
                }
            }
        }

        // (3.5) collect THIS module's functions/tasks (bare name → def) for inline
        //       expansion at call sites (pass 7). Saved/restored so a sibling/parent
        //       instance of another module does not inherit them. Functions are not
        //       hierarchical in v1, so the bare name is the key.
        let mut saved_funcs = std::mem::take(&mut self.func_table);
        let mut saved_tasks = std::mem::take(&mut self.task_table);
        let mut saved_rtn_pkg = std::mem::take(&mut self.rtn_pkg);
        // R19-X1: these two tables' contents are declared HERE, at this instance's own
        // prefix. Record it so a filled default-argument value can be checked against
        // the scope IEEE evaluates it in (see `default_binding_matches_decl_scope`).
        let saved_decl_scope = std::mem::replace(&mut self.tf_decl_scope, self.cur_prefix.clone());
        // R5-B: the inout-function set is module-local too (it is rebuilt by this
        // module's `lower_frame_funcs`), so a nested child instance elaborated in the
        // middle of this module must not clobber it — save/restore alongside func_table.
        let saved_inout_funcs = std::mem::take(&mut self.inout_func_names);
        // §4.5.179: dyn-formal-function set is module-local too (same rebuild path).
        let saved_dyn_formal_funcs = std::mem::take(&mut self.dyn_formal_func_names);
        // B1 frame-call: the frame-func name→id map is module-local (a sibling
        // module's call must never divert to this module's func). The global
        // `funcs`/`func_blocks`/`func_metas` arenas are NOT saved (they accumulate).
        let mut saved_frame_idx = std::mem::take(&mut self.frame_idx);
        let saved_task_frame_idx = std::mem::take(&mut self.task_frame_idx); // B2
                                                                             // Named SVA seq/prop decls collected in the SAME whole-body prescan, so an
                                                                             // `assert property(p)` BEFORE `property p; …` (forward reference) resolves —
                                                                             // identical mechanism to func/task. Saved/restored alongside.
        let saved_seqs = std::mem::take(&mut self.seq_table);
        let saved_props = std::mem::take(&mut self.prop_table);
        let saved_lets = std::mem::take(&mut self.let_table);
        for item in &module.body {
            match item {
                ast::ModuleItem::Func(f) => {
                    self.check_block_local_scope_leaks(&f.body);
                    if self
                        .func_table
                        .insert(f.name.name.clone(), f.clone())
                        .is_some()
                    {
                        self.warn(&format!(
                            "function `{}` redeclared; first declaration used",
                            f.name.name
                        ));
                    }
                }
                ast::ModuleItem::Task(t) => {
                    self.check_block_local_scope_leaks(&t.body);
                    if self
                        .task_table
                        .insert(t.name.name.clone(), t.clone())
                        .is_some()
                    {
                        self.warn(&format!(
                            "task `{}` redeclared; first declaration used",
                            t.name.name
                        ));
                    }
                }
                ast::ModuleItem::Proc(p) => {
                    self.check_block_local_scope_leaks(&p.body);
                }
                // FIRST declaration wins (insert-if-absent), so the redeclaration
                // warning text is accurate (review 2026-06-16: `BTreeMap::insert`
                // would keep the LAST, contradicting "first declaration used").
                ast::ModuleItem::SequenceDecl(s) => self.register_seq_decl(s),
                ast::ModuleItem::PropertyDecl(p) => self.register_prop_decl(p),
                ast::ModuleItem::LetDecl(l) => self.register_let_decl(l),
                // SVA decls inside a `generate` block are module-global too (slice
                // A4): the prescan walks generate structurally (no const-eval / no
                // genvar binding) so a reference resolves regardless of loop trip
                // count or which branch the const-cond later selects.
                ast::ModuleItem::Generate(g) => self.collect_gen_sva_decls(&g.items),
                // N5: register covergroup TYPES (an instance can precede the decl).
                ast::ModuleItem::Covergroup(cg) => {
                    self.cover_types
                        .entry(cg.name.name.clone())
                        .or_insert_with(|| cg.clone());
                }
                // N4: clocking blocks are lowered by `lower_clocking_blocks` (after
                // the net-creation passes, before the process loop) — see step (6.5).
                _ => {}
            }
        }
        // (3.6) v7 P2-D: imported functions/tasks — LOCAL definitions win
        // (skip-if-present, no spurious redeclare warning). SVPART: the same
        // wildcard-ambiguity rule as consts — a routine name from two different
        // wildcard imports is unbound (loud "unknown function" at the call site,
        // never a silent first-wins), and an explicit import always wins.
        let mut wc_rtn: BTreeMap<String, String> = BTreeMap::new();
        let mut explicit_rtn: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        for imp in &import_list {
            self.apply_import_routines(imp, &mut wc_rtn, &mut explicit_rtn);
        }

        // (4) this instance's nets: ANSI ports, then body NetVarDecls (decl order).
        //     add_net now keys through `fq`, so names become inst_path-prefixed.
        self.elaborate_ports(&module.ports);
        for item in &module.body {
            if let ast::ModuleItem::NetVar(d) = item {
                self.elaborate_netvar_decl(d, &module.ports, &module.body, true);
            }
        }
        // Non-ANSI port nets: a body `input/output [w] name;` (a `PortDecl`)
        // declares the port net. `elaborate_ports` only builds ANSI header ports,
        // so without this a non-ANSI module's ports are undeclared. Skip a name a
        // separate `reg`/`wire` NetVarDecl already created (`output y; reg y;` —
        // that path merges the port direction itself).
        for item in &module.body {
            if let ast::ModuleItem::PortDecl(pd) = item {
                let kind = pd.net_or_var.unwrap_or(ast::NetVarKind::Wire);
                let (width, msb, lsb, signed) =
                    self.range_to_dims(kind, pd.range.as_ref(), pd.signed);
                let dir = map_port_dir(pd.dir);
                let init = default_init(kind, width);
                for (ni, name) in pd.names.iter().enumerate() {
                    if self.symbols.contains_key(&self.fq(&name.name)) {
                        continue; // already created by a NetVarDecl
                    }
                    // UNPACKED dims, per name — the non-ANSI twin of the ANSI port
                    // path. `pd.unpacked` is parallel to `pd.names`, so an empty (or
                    // absent) entry is the scalar case and behaves as before.
                    let dims = pd.unpacked.get(ni).map(Vec::as_slice).unwrap_or(&[]);
                    let dim_extents = self.array_dim_extents(dims);
                    let array_len = dim_extents
                        .iter()
                        .fold(1u32, |acc, &(_, n)| acc.saturating_mul(n.max(1)));
                    if (array_len as u64) > MAX_ARRAY_LEN {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "unpacked array port `{}` has {array_len} elements \
                                 (cap {MAX_ARRAY_LEN})",
                                name.name
                            ),
                        );
                        continue;
                    }
                    self.add_net(
                        &name.name,
                        ir::NetVar {
                            kind: map_net_kind_or_wire(kind),
                            width,
                            msb,
                            lsb,
                            signed,
                            array_len: array_len.max(1),
                            dir,
                            init: init.clone(),
                        },
                    );
                    if !dims.is_empty() {
                        if let Some(&id) = self.symbols.get(&self.fq(&name.name)) {
                            self.record_dim_desc(id, kind, pd.range.as_ref(), &[], dims);
                            if dim_extents.len() >= 2 || dim_extents.iter().any(|&(lo, _)| lo != 0)
                            {
                                self.array_dims.insert(id, dim_extents.clone());
                            }
                        }
                    }
                    // WAND/WOR: non-ANSI port net also needs its resolution sidecar.
                    if matches!(kind, ast::NetVarKind::Wand | ast::NetVarKind::Wor) {
                        if let Some(&id) = self.symbols.get(&self.fq(&name.name)) {
                            if matches!(kind, ast::NetVarKind::Wand) {
                                self.wired_and_nets.insert(id);
                            } else {
                                self.wired_or_nets.insert(id);
                            }
                        }
                    }
                }
            }
        }
        // Procedural block-local declarations (`begin: blk integer x; …`). v1 has
        // no per-process automatic frame, so a block-local flattens to a module-
        // scope net — created HERE (Nets phase) so it lands in this instance's net
        // slice and references inside the block resolve instead of erroring E3010.
        for item in &module.body {
            if let ast::ModuleItem::Proc(p) = item {
                self.hoist_block_local_nets(&p.body, &module.ports, &module.body);
            }
        }
        // Generate-block nets belong in THIS instance's contiguous net slice too:
        // unroll the generate, in the Nets phase only, right after the plain
        // body nets so they precede every cont-assign/process (pass 7) that may
        // reference them, and precede child-instance recursion (pass 8).
        // §4.5.256: restart the GENERATE slot's counter for this walk. The four generate
        // walks are the same traversal, so each generate draws the same number in every
        // phase — which the ranks depend on, because a child instance's initializers are
        // ranked during the Instances walk under the path its generate got, while that
        // generate's own flush was ranked during VarInit. Only this slot is reset: the
        // instance slot is visited by ONE walk and must keep counting across it.
        // IEEE 1364-2005 §3.5 — implicit nets are a DECLARATION, so they are created in
        // their own pass BEFORE any item is lowered, exactly like explicit declarations.
        // Doing it at the use site instead makes them order-dependent in a way the
        // standard is not: this body lowers cont-assigns before instances, so
        // `sub u(.o(IMPL)); assign o = IMPL;` declared `IMPL` from the terminal list
        // AFTER the read had already failed with E3010 — one design, two verdicts,
        // decided by the phase order. ONE funnel, one pass, order-free.
        self.declare_implicit_nets(&module.body);
        self.rank_seq[Self::RANK_MOD_GENERATE as usize] = 0;
        for item in &module.body {
            if let ast::ModuleItem::Generate(g) = item {
                self.elaborate_generate_scoped(&g.items, GenPhase::Nets, 0, map, false);
            }
        }

        // (5) net_count = nets created for THIS instance only (not children).
        let net_count = self.nets.len() as u32 - first_net;
        self.instances[inst_id as usize].net_count = net_count;

        // (4c) v5 ⑥ (D): flatten DIRECT-body interface instances EARLY (nets
        // phase) — unlike module children (whose nets are per-instance
        // private), interface members ARE the parent-visible API (`i.sig` in
        // pass-7 bodies must resolve). Generate-nested ones flatten in the
        // late pass (8); the per-item registry guard makes both idempotent.
        for item in &module.body {
            if let ast::ModuleItem::Instance(mi) = item {
                if self.ifaces.contains_key(mi.module_name.name.as_str()) {
                    self.elaborate_iface_instances(mi, false);
                }
            }
        }

        // (5d) ⓑ-breadth (§25.9): resolve `virtual interface` handles NOW — the
        // bound interface instances exist, so the member alias can be wired.
        for item in &module.body {
            if let ast::ModuleItem::NetVar(d) = item {
                if d.kind == ast::NetVarKind::VirtualIface {
                    self.elaborate_virtual_iface(d, &module.body);
                }
            }
        }

        // (6) port-connection cont-assigns (parent expr ↔ child port net).
        //
        // ⚠️ `wire_ports` runs inside the CHILD's `elaborate_instance`, but a port
        // connection's actual is a PARENT expression — it is only the name scope
        // that `wire_ports` swaps (`cur_prefix`), and by here every routine table
        // holds the child's declarations. So `leaf u (.i_m(mk(2)))` looked `mk` up
        // in the child and reported "call to undeclared function" for a function
        // the parent declares — and, worse, if the CHILD happened to declare its own
        // `mk`, the parent's connection silently bound the child's. Swap the
        // parent's tables in for the call; they are still live in these locals.
        // Everything else `wire_ports` touches (`symbols`, `nets`, the `.*` block)
        // is child scope and is untouched by the swap.
        std::mem::swap(&mut self.func_table, &mut saved_funcs);
        std::mem::swap(&mut self.const_func_table, &mut saved_const_funcs);
        std::mem::swap(&mut self.task_table, &mut saved_tasks);
        std::mem::swap(&mut self.frame_idx, &mut saved_frame_idx);
        std::mem::swap(&mut self.rtn_pkg, &mut saved_rtn_pkg);
        std::mem::swap(&mut self.decl_pos, &mut saved_decl_pos);
        std::mem::swap(&mut self.decl_pos_scope, &mut saved_dpos_scope);
        std::mem::swap(&mut self.decl_pos_range, &mut saved_dpos_range);
        std::mem::swap(&mut self.decl_block_locals, &mut saved_dbl);
        self.wire_ports(module, binding, &saved_prefix);
        std::mem::swap(&mut self.func_table, &mut saved_funcs);
        std::mem::swap(&mut self.const_func_table, &mut saved_const_funcs);
        std::mem::swap(&mut self.task_table, &mut saved_tasks);
        std::mem::swap(&mut self.frame_idx, &mut saved_frame_idx);
        std::mem::swap(&mut self.rtn_pkg, &mut saved_rtn_pkg);
        std::mem::swap(&mut self.decl_pos, &mut saved_decl_pos);
        std::mem::swap(&mut self.decl_pos_scope, &mut saved_dpos_scope);
        std::mem::swap(&mut self.decl_pos_range, &mut saved_dpos_range);
        std::mem::swap(&mut self.decl_block_locals, &mut saved_dbl);

        // (6.5) B1 frame-call: RESERVE + LOWER every automatic/recursive function
        //       BEFORE the body lowering below, so a call site (step 7) can divert
        //       to a reserved FuncId. Runs AFTER the net_count freeze (5) so frame
        //       nets land OUTSIDE this Instance's [first_net, +net_count) slice;
        //       module scope is still active (module nets in `symbols`). No-op when
        //       the module declares no frame functions (frame_idx stays empty).
        self.lower_frame_funcs();

        // N5: register covergroup INSTANCES (allocate their hit-bitmap regs) BEFORE
        // lowering processes, so a `c.sample()` in any process finds the bitmaps
        // regardless of source order.
        for item in &module.body {
            if let ast::ModuleItem::CoverInstance(ci) = item {
                self.register_cover_instance(ci);
            }
        }

        // (6.9) §6.8: emit non-constant variable initializers (`int b = a;`) as ONE
        //       synthesized `initial`. It is not a t0 process any more: the engine runs
        //       the ranked initializer bodies as a PRE-ARM phase (IEEE 1800 §6.21), so it
        //       precedes every user process by construction rather than by ProcId, and
        //       produces no event. Constant inits ride it too (§4.5.257).
        // Decl-time pre-sizes recorded for THIS scope (the `""` key) go first: a routed
        // string array's `new[n]` must precede its element writes. Same position the
        // straight-into-`pending_var_inits` push had, so the module path is unchanged.
        // (6.9a) §6.8 for GENERATE scopes: the module-body sweep below walks the
        //        module body ONLY, so an array `'{…}` / non-constant / queue
        //        decl-init inside a generate block was silently dropped. Collect +
        //        flush them (VarInit phase, ONE synthesized `initial` per gen scope)
        //        BEFORE the module pre-sweep and BEFORE the Logic pass, so every
        //        pre-sweep initial precedes the user processes that read it.
        //
        // §4.5.255: BEFORE, not after — measured against live iverilog, a generate
        // scope's statics initialize ahead of the enclosing module's. `genvar i; …
        // begin : g int gm = $random; … end` gives `gm` the first draw and a module
        // `int mm = $random;` the second, in either source order. vita emitted the
        // module sweep first, so every generate-scope initializer read the module's
        // already-initialized values and the module read none of the generate's.
        // §4.5.256: restart the GENERATE slot's counter for this walk. The four generate
        // walks are the same traversal, so each generate draws the same number in every
        // phase — which the ranks depend on, because a child instance's initializers are
        // ranked during the Instances walk under the path its generate got, while that
        // generate's own flush was ranked during VarInit. Only this slot is reset: the
        // instance slot is visited by ONE walk and must keep counting across it.
        //
        // §4.5.263: ONE walk, in declaration order. A generate REGION is transparent
        // (IEEE 1800 §27.3), so a variable written directly inside `generate …
        // endgenerate` is an ordinary module item and must interleave with the module's
        // own by declaration order — `int a; generate int b; endgenerate int c;` draws
        // a, b, c. Two separate loops could not express that. A generate BLOCK still
        // flushes as it is reached, which is before this sweep's own flush at the end,
        // so generate scopes still precede the module's own variables whatever their
        // source position.
        self.rank_seq[Self::RANK_MOD_GENERATE as usize] = 0;
        for item in &module.body {
            match item {
                ast::ModuleItem::Generate(g) => {
                    self.elaborate_generate_scoped(&g.items, GenPhase::VarInit, 0, map, false);
                }
                ast::ModuleItem::NetVar(d) => self.collect_var_init_drivers(d),
                _ => {}
            }
        }
        // Every block-local initializer runs AFTER the module-scope ones above (measured:
        // iverilog initializes module-scope statics first), in declaration order among
        // themselves — see `flush_block_local_inits`, which also flushes the sweep above.
        self.flush_block_local_inits();

        // (6.5) N4 clocking: synthesize preponed-sampled holding nets + a marked
        // commit handler per clocking block, and register `@(cb)` events — AFTER the
        // net passes (source nets resolve) and BEFORE the process loop (so `cb.sig`
        // resolves to its holding net and `@(cb)` to the clocking event).
        self.lower_clocking_blocks(&module.body);

        // (7) lower THIS body: cont-assigns + processes (reuse v1/v2 helpers).
        // §4.5.256: restart the GENERATE slot's counter for this walk. The four generate
        // walks are the same traversal, so each generate draws the same number in every
        // phase — which the ranks depend on, because a child instance's initializers are
        // ranked during the Instances walk under the path its generate got, while that
        // generate's own flush was ranked during VarInit. Only this slot is reset: the
        // instance slot is visited by ONE walk and must keep counting across it.
        self.rank_seq[Self::RANK_MOD_GENERATE as usize] = 0;
        for item in &module.body {
            match item {
                ast::ModuleItem::ContAssign(ca) => self.elaborate_cont_assign(ca),
                ast::ModuleItem::Proc(p) => {
                    let proc = self.lower_proc_block(p);
                    debug_assert_eq!(
                        self.processes.len() as u32,
                        self.cur_proc,
                        "ProcId mismatch: fork_modes keyed by cur_proc would miss"
                    );
                    self.push_process(proc);
                }
                // (8) handled in the second loop below.
                ast::ModuleItem::Instance(_) => {}
                // Generate: nets already created (pass 4 net-walk); here lower its
                // cont-assigns + processes (Logic phase). Child instances inside it
                // recurse in pass (8) below.
                ast::ModuleItem::Generate(g) => {
                    self.elaborate_generate_scoped(&g.items, GenPhase::Logic, 0, map, false);
                }
                // Func/Task are DEFINITIONS, not logic: collected in step (3.5)
                // and expanded at their call sites (inline). No-op here.
                ast::ModuleItem::Func(_) | ast::ModuleItem::Task(_) => {}
                ast::ModuleItem::Defparam(dp) => {
                    // Collect `defparam inst.param = const;` overrides, keyed by the
                    // FQ child path (`cur_prefix.inst`). Consumed by the child's
                    // `bind_params` in pass 8 (recursion runs AFTER this pass, so the
                    // override is registered first). v1: a DIRECT-child `inst.param`
                    // (exactly 2 segments) with a CONST value — deeper paths and
                    // non-const values are loud (correct-or-loud, not silent-skip).
                    for (path, value) in &dp.assigns {
                        if path.segments.len() != 2 {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "defparam: only a direct-child `instance.param` target \
                                 is supported (a multi-level path is a follow-on)",
                            );
                            continue;
                        }
                        let Some(v) = self.const_eval_in_scope(value) else {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "defparam: a non-constant override value is unsupported",
                            );
                            continue;
                        };
                        let fq = format!("{}.{}", self.cur_prefix, path.segments[0].name);
                        let param = path.segments[1].name.clone();
                        // A fill literal is carried verbatim ALONGSIDE the fold above:
                        // its width is the TARGET's declared width, which is not known
                        // here, so `v` is only the 32-bit self-determined fold and is
                        // wrong for any target wider than that.
                        let fill = expr_as_fill(value).map(|(k, r)| (k, r.to_string()));
                        // Last write wins (IEEE §23.10.1) — drop a prior same-param entry.
                        let entry = self.defparams.entry(fq).or_default();
                        entry.retain(|(p, _, _)| p != &param);
                        entry.push((param, v, fill));
                    }
                }
                // A NET declaration initializer (`wire x = expr;`) is an implicit
                // continuous assign — lower it as a driver here (the net itself was
                // created in pass 4). A variable initializer is a one-time value.
                ast::ModuleItem::NetVar(d) => self.elaborate_net_init_drivers(d),
                // Param/PortDecl/Genvar/Error: no-op here.
                _ => {}
            }
        }

        // (7.5) v8 SVA: materialize every concurrent assertion collected during
        //       the process loop above as a synthesized clocked checker process.
        //       Drained here (this instance's scope) so child instances start clean.
        self.materialize_sva_checkers();
        // (7.6) SVA-REST: materialize each `cover property` as a clocked counter +
        //       end-of-sim `$display` report.
        self.materialize_cover();

        // (8) recurse into child instances, in body declaration order — including
        //     those nested inside a generate construct (Instances phase).
        // §4.5.256: restart the GENERATE slot's counter for this walk. The four generate
        // walks are the same traversal, so each generate draws the same number in every
        // phase — which the ranks depend on, because a child instance's initializers are
        // ranked during the Instances walk under the path its generate got, while that
        // generate's own flush was ranked during VarInit. Only this slot is reset: the
        // instance slot is visited by ONE walk and must keep counting across it.
        self.rank_seq[Self::RANK_MOD_GENERATE as usize] = 0;
        for item in &module.body {
            match item {
                ast::ModuleItem::Instance(mi) => {
                    self.elaborate_child_instances(mi, inst_id, map);
                }
                ast::ModuleItem::Generate(g) => {
                    self.elaborate_generate_scoped(&g.items, GenPhase::Instances, 0, map, false);
                }
                _ => {}
            }
        }

        // (8b) Round-9: attach bound checker instances declared by a top-level
        //      `bind <this-module> <checker> u(...)`. Fires ONCE per instance of
        //      this target module, AFTER its own children, while `cur_prefix` is
        //      still this instance's path — so each checker port `.clk(clk)`
        //      resolves against THIS instance's own `clk` net, reusing the
        //      ordinary child-instantiation path (no bind-specific wiring). The
        //      checker's `assert property` materializes inside its own instance's
        //      step (7.5). `binds.clone()` releases the `self.bind_targets` borrow
        //      before the `&mut self` recursion.
        if let Some(binds) = self.bind_targets.get(&module.name.name) {
            // §4.5.261: BAND 1. A `bind` directive lives in the
            // compilation unit, so its source offset is not a position inside this
            // module's body — using it made the answer depend on where the `bind` line
            // was written, and on the order the files were listed.
            let saved_band = std::mem::replace(&mut self.rank_band, 1);
            for mi in binds.clone() {
                self.elaborate_child_instances(&mi, inst_id, map);
            }
            self.rank_band = saved_band;
        }

        // restore scope/params so siblings + ancestors resolve correctly.
        self.restore_params(saved_params);
        self.bits_prescan = saved_prescan;
        self.local_decl_names = saved_local_names;
        self.scoped_block_locals = saved_scoped_blocks;
        self.per_entry_block_locals = saved_per_entry_blocks;
        self.coalesced_block_locals = saved_coalesced;
        self.func_table = saved_funcs;
        self.tf_decl_scope = saved_decl_scope;
        self.const_func_table = saved_const_funcs;
        self.task_table = saved_tasks;
        self.rtn_pkg = saved_rtn_pkg;
        self.decl_pos = saved_decl_pos;
        self.decl_pos_scope = saved_dpos_scope;
        self.decl_pos_range = saved_dpos_range;
        self.decl_block_locals = saved_dbl;
        self.inout_func_names = saved_inout_funcs; // R5-B
        self.dyn_formal_func_names = saved_dyn_formal_funcs; // §4.5.179
        self.frame_idx = saved_frame_idx;
        self.task_frame_idx = saved_task_frame_idx;
        self.seq_table = saved_seqs;
        self.prop_table = saved_props;
        self.let_table = saved_lets;
        self.cur_prefix = saved_prefix;
        self.cur_span = saved_span;
        self.in_generate_body = saved_in_gen;
        self.rank_band = saved_band;
        self.rank_seq = saved_rank_seq;
        self.rank_path.truncate(self.rank_path.len() - 4);
        self.cur_inst = saved_inst;
        self.cur_time_mult = saved_mult;
        self.cur_prec_mult = saved_prec_mult;
        self.inst_stack.pop();
        self.cur_nettype_none = saved_nettype;
    }

    /// Resolve a `ModuleInstance` statement (which may name several instances),
    /// and recurse into each child. Connection exprs are resolved later, in the
    /// PARENT scope (still active here), inside the child's `wire_ports`.
    pub(crate) fn elaborate_child_instances(
        &mut self,
        mi: &ast::ModuleInstance,
        parent_inst: u32,
        map: &ModuleMap<'_>,
    ) {
        let child = match map.get(mi.module_name.name.as_str()) {
            Some(&(decl, _)) => decl,
            None => {
                // v5 ⑥ (D): `intf i();` — an interface instance flattens to
                // plain nets under the instance prefix (no ir::Instance row,
                // no new IR — spike 2026-06-10).
                if self.ifaces.contains_key(mi.module_name.name.as_str()) {
                    self.elaborate_iface_instances(mi, true);
                    return;
                }
                self.error(
                    MsgCode::ElabUnresolvedInstance,
                    &format!("unknown module `{}` instantiated", mi.module_name.name),
                );
                return;
            }
        };

        // Fix 1: const-eval EVERY override expr NOW, in the PARENT scope, so a
        // parent-param-dependent override (`#(.W(PARENT_W))`) resolves. A failure
        // to fold is recorded as value=None (child keeps default + warns), never
        // a silent 0 that explodes the child's width.
        let mut overrides: Vec<ResolvedOverride> = Vec::with_capacity(mi.param_overrides.len());
        for ov in &mi.param_overrides {
            match ov {
                ast::ParamConn::Positional(e) => {
                    let value = self.const_eval_in_scope(e);
                    // Build the record BEFORE deciding what to say about it: the
                    // other two channels are computed from the same `e`, and the
                    // warning below is a statement about the record.
                    let ovr = ResolvedOverride {
                        name: None,
                        value,
                        is_named: false,
                        had_value: true,
                        fill: expr_as_fill(e).map(|(k, r)| (k, r.to_string())),
                        str_is_literal: Self::param_str_literal(e).is_some(),
                        str: self.const_str_in_scope(e),
                    };
                    if value.is_none() {
                        if Self::expr_is_real_literal(e) {
                            // r19: mirror the NAMED-connection rule — a real-literal
                            // override cannot be folded, and warning + keeping the
                            // child default runs with the wrong value at exit 0.
                            self.error(
                                MsgCode::ElabUnsupported,
                                "overriding a parameter with a real value is unsupported \
                                 (a real override cannot be folded); the child default \
                                 would be used silently",
                            );
                        } else if self.count_reads_real_param(e) {
                            // r19/B1: the guard next to this one tests a real LITERAL, but this
                            // slice newly made real-VALUED expressions reachable here (an ident
                            // bound to a real param, `R+1`, `R*2`). Those do not const-fold, so
                            // they fell into the warn-and-keep-default path below = the child
                            // silently ran with the WRONG parameter, changing port widths at
                            // exit 0 where this was loud before the slice.
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a parameter override that reads a real parameter is unsupported \
                         (a real has no integral constant value)",
                            );
                        } else if ovr.keeps_default() {
                            self.warn(
                                "parameter override expression is not a constant; child default kept",
                            );
                        }
                    }
                    overrides.push(ovr);
                }
                ast::ParamConn::Named { name, value, .. } => {
                    // `.W()` (value None) means "keep default" → record is_named with value None.
                    // Same shape as the positional arm: the two non-i64
                    // channels are decided from `value` alone, so compute them
                    // first and let the warning ask the record.
                    let fill = value
                        .as_ref()
                        .and_then(|e| expr_as_fill(e).map(|(k, r)| (k, r.to_string())));
                    let text_is_literal =
                        value.as_ref().and_then(Self::param_str_literal).is_some();
                    let text = value.as_ref().and_then(|e| self.const_str_in_scope(e));
                    let v = value.as_ref().and_then(|e| {
                        let r = self.const_eval_in_scope(e);
                        if r.is_none() {
                            // r19: a REAL-literal override is ERROR, not warn-and-keep.
                            // Real PARAMETERS are supported now, so silently running with
                            // the declared default would be the wrong value with exit 0 —
                            // and the override machinery is i64-only, so the right value
                            // cannot be applied. Every other non-constant override keeps
                            // the pre-existing warn-and-default behaviour.
                            if Self::expr_is_real_literal(e) {
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "overriding parameter `{}` with a real value is \
                                         unsupported (a real override cannot be folded); \
                                         the declared default would be used silently",
                                        name.name
                                    ),
                                );
                            } else if self.count_reads_real_param(e) {
                                // r19/B1: the guard next to this one tests a real LITERAL, but this
                                // slice newly made real-VALUED expressions reachable here (an ident
                                // bound to a real param, `R+1`, `R*2`). Those do not const-fold, so
                                // they fell into the warn-and-keep-default path below = the child
                                // silently ran with the WRONG parameter, changing port widths at
                                // exit 0 where this was loud before the slice.
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    "a parameter override that reads a real parameter is unsupported \
                             (a real has no integral constant value)",
                                );
                            } else if ResolvedOverride::keeps_default_of(
                                None,
                                fill.as_ref(),
                                text.as_ref(),
                            ) {
                                self.warn(&format!(
                                    "override of parameter `{}` is not a constant; default kept",
                                    name.name
                                ));
                            }
                        }
                        r
                    });
                    overrides.push(ResolvedOverride {
                        name: Some(name.name.clone()),
                        value: v,
                        is_named: true,
                        had_value: value.is_some(),
                        fill,
                        str_is_literal: text_is_literal,
                        str: text,
                    });
                }
            }
        }

        for item in &mi.instances {
            if !item.unpacked.is_empty() {
                // Instance array (`dff u[3:0](...)`, IEEE 1364 §12.1.2-3):
                // unroll into per-index children with sliced connections.
                self.elaborate_instance_array(child, item, &overrides, parent_inst, map);
                continue;
            }
            let child_path = self.child_prefix(&item.name.name);
            let binding = match &item.conns {
                ast::PortConnList::Named(v, wc) => PortBinding::Named(v, *wc),
                ast::PortConnList::Positional(v) => PortBinding::Positional(v),
            };
            self.elaborate_instance(
                child,
                &child_path,
                Some(parent_inst),
                &overrides,
                binding,
                map,
                (self.rank_band, item.name.span.lo, 0),
                Some(item.span),
            );
        }
    }
}
