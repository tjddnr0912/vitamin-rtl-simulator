//! Elaborator construction, run/finish, diagnostics — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl<'s> Elaborator<'s> {
    pub(crate) fn new(sink: &'s dyn LogSink) -> Self {
        Self {
            sink,
            span_resolver: None,
            cur_span: None,
            had_error: false,
            error_count: 0,
            net_budget_blown: false,
            nets: Vec::new(),
            exprs: Vec::new(),
            consts: Vec::new(),
            cont_assigns: Vec::new(),
            wired_and_nets: BTreeSet::new(),
            wired_or_nets: BTreeSet::new(),
            instances: Vec::new(),
            instances_info: Vec::new(),
            processes: Vec::new(),
            stmts: Vec::new(),
            symbols: BTreeMap::new(),
            bind_targets: BTreeMap::new(),
            const_dedup: BTreeMap::new(),
            array_dims: BTreeMap::new(),
            net_decl_neg_lsb: BTreeMap::new(),
            net_decl_range: BTreeMap::new(),
            file_directed_stmts: std::collections::BTreeSet::new(),
            bits_prescan: BTreeMap::new(),
            local_decl_names: std::collections::BTreeSet::new(),
            scoped_block_locals: BTreeMap::new(),
            per_entry_block_locals: BTreeMap::new(),
            coalesced_block_locals: std::collections::BTreeSet::new(),
            pkg_consts: BTreeMap::new(),
            pkg_types: BTreeMap::new(),
            pkg_const_meta: BTreeMap::new(),
            pkg_funcs: BTreeMap::new(),
            pkg_tasks: BTreeMap::new(),
            cu_imports: Vec::new(),
            final_procs: std::collections::BTreeSet::new(),
            comb_inferred_procs: Vec::new(),
            clocking_inputs: std::collections::BTreeSet::new(),
            clocking_commit: std::collections::BTreeMap::new(),
            clocking_outputs: std::collections::BTreeMap::new(),
            ca_delays: std::collections::BTreeMap::new(),
            clocking_events: std::collections::BTreeMap::new(),
            default_clocking: None,
            clocking_hold_nets: std::collections::BTreeSet::new(),
            automatic_local_nets: std::collections::BTreeSet::new(),
            const_param_nets: std::collections::BTreeMap::new(),
            lowering_decl_init: false,
            pkg_vars: std::collections::BTreeMap::new(),
            pkg_var_aliases: std::collections::BTreeMap::new(),
            genvar_decls: std::collections::BTreeSet::new(),
            all_clocking_names: std::collections::BTreeSet::new(),
            anon_clocking_count: 0,
            func_metas: Vec::new(),
            frame_func_names: Vec::new(),
            funcs: Vec::new(),
            func_blocks: Vec::new(),
            frame_idx: BTreeMap::new(),
            task_frame_idx: BTreeMap::new(),
            frame_task_pending: Vec::new(),
            task_calls_proc: BTreeMap::new(),
            task_calls_func: BTreeMap::new(),
            frame_task_lowering: false,
            pending_task_calls: Vec::new(),
            pending_fork_modes: Vec::new(),
            pending_hier_task_calls: Vec::new(),
            array_dim_desc: BTreeMap::new(),
            dim_desc: BTreeMap::new(),
            intro_kind: BTreeMap::new(),
            task_arg_locals: std::collections::HashMap::new(),
            two_state_heap_handles: BTreeSet::new(),
            real_elem_dyn_nets: BTreeSet::new(),
            string_elem_dyn_nets: BTreeSet::new(),
            size_cast_real_reported: false,
            unpacked_array_nets: BTreeSet::new(),
            packed_dims: BTreeMap::new(),
            dollar_subst: None,
            array_iter: None,
            array_iter_elem: None,
            ifaces: BTreeMap::new(),
            iface_insts: BTreeMap::new(),
            vif_handles: std::collections::BTreeSet::new(),
            modport_readonly: BTreeSet::new(),
            cur_prefix: String::new(),
            params: BTreeMap::new(),
            array_const_vals: BTreeMap::new(),
            pkg_array_const_vals: BTreeMap::new(),
            param_meta: BTreeMap::new(),
            param_range: BTreeMap::new(),
            str_param_raw: BTreeMap::new(),
            real_param_val: BTreeMap::new(),
            hoisted_block_local: BTreeMap::new(),
            per_entry_in_scope: BTreeSet::new(),
            frame_fn_lowering: false,
            sformatf_expr_ok: false,
            string_array_elems: BTreeMap::new(),
            fixed_string_dyn: BTreeMap::new(),
            fixed_string_dyn_key: BTreeMap::new(),
            hier_params: BTreeMap::new(),
            hier_funcs: BTreeMap::new(),
            hier_tasks: BTreeMap::new(),
            hier_called_task_names: std::collections::BTreeSet::new(),
            hier_task_port_dirs: BTreeMap::new(),
            defparams: BTreeMap::new(),
            inst_stack: Vec::new(),
            cur_inst: 0,
            func_table: BTreeMap::new(),
            const_func_table: BTreeMap::new(),
            task_table: BTreeMap::new(),
            rtn_pkg: BTreeMap::new(),
            cur_rtn_pkg: Vec::new(),
            decl_pos: BTreeMap::new(),
            decl_pos_scope: String::new(),
            decl_pos_range: (0, 0),
            decl_block_locals: std::collections::BTreeSet::new(),
            wide_param_bits: BTreeMap::new(),
            tf_decl_scope: String::new(),
            inout_func_names: std::collections::BTreeSet::new(),
            dyn_formal_func_names: std::collections::BTreeSet::new(),
            seq_table: BTreeMap::new(),
            prop_table: BTreeMap::new(),
            let_table: BTreeMap::new(),
            sva_inline_stack: Vec::new(),
            sva_seq_depth: 0,
            class_table: BTreeMap::new(),
            class_order: Vec::new(),
            net_class: BTreeMap::new(),
            cur_this: None,
            cur_return: None,
            cur_discard: None,
            in_frame_body: false,
            dyn_formal_call_ok: false,
            frame_array_local: std::collections::BTreeSet::new(),
            frame_arr_formal_meta: std::collections::BTreeMap::new(),
            class_handle_nets: std::collections::BTreeSet::new(),
            class_new_sites: std::collections::BTreeMap::new(),
            class_vtable: Vec::new(),
            class_calls: std::collections::BTreeMap::new(),
            class_field_widths: std::collections::BTreeMap::new(),
            selfw_scan: 0,
            selfw_scratch: Vec::new(),
            selfw_seen: Vec::new(),
            selfw_seen_gen: 0,
            selfw_cache: Vec::new(),
            randomize_with: Vec::new(),
            assert_fire: std::collections::BTreeSet::new(),
            assert_ctl: std::collections::BTreeMap::new(),
            in_assert_synth: false,
            subst: Vec::new(),
            out_subst: Vec::new(),
            dyn_subst: Vec::new(),
            formal_str: Vec::new(),
            inline_stack: Vec::new(),
            fork_modes: ForkModeTable::new(),
            severities: SeverityTable::new(),
            timeformat_stmts: std::collections::BTreeSet::new(),
            stage_stmts: std::collections::BTreeSet::new(),
            handle_copy_stmts: std::collections::BTreeMap::new(),
            dyn_formal_marker_stmts: std::collections::BTreeSet::new(),
            frame_print_stmts: std::collections::BTreeSet::new(),
            queue_slice_stmts: std::collections::BTreeSet::new(),
            radixes: RadixTable::new(),
            assign_ranks: AssignRankTable::new(),
            queue_bounds: QueueBoundTable::new(),
            event_nets: std::collections::BTreeSet::new(),
            proc_scopes: Vec::new(),
            pending_var_inits: Vec::new(),
            pending_scoped_presize: BTreeMap::new(),
            pending_block_local_inits: BTreeMap::new(),
            in_generate_body: false,
            rank_path: Vec::new(),
            rank_band: 0,
            rank_seq: [0; 4],
            init_ranks: BTreeMap::new(),
            pending_sva: Vec::new(),
            pending_cover: Vec::new(),
            deferred_hier: Vec::new(),
            deferred_hier_calls: Vec::new(),
            deferred_hier_task_calls: Vec::new(),
            deferred_hier_sel: Vec::new(),
            deferred_hier_write: Vec::new(),
            deferred_hier_sel_write: Vec::new(),
            cover_types: std::collections::BTreeMap::new(),
            cover_insts: std::collections::BTreeMap::new(),
            cross_insts: std::collections::BTreeMap::new(),
            coverage_manifest: Vec::new(),
            defer_marks: DeferMarkTable::new(),
            defer_acts: DeferActTable::new(),
            cur_defer: None,
            defer_inline_warned: false,
            cur_proc: 0,
            in_fork: false,
            disable_stack: Vec::new(),
            disable_fork_floor: 0,
            mod_unit_exp: BTreeMap::new(),
            mod_prec_exp: BTreeMap::new(),
            root_override: None,
            top_param_overrides: Vec::new(),
            escape_warned: BTreeSet::new(),
            cur_nettype_none: false,
            implicit_nets: Default::default(),
            global_prec_exp: -9, // 1ns base precision (no-timescale lock)
            cur_time_mult: 1,
            cur_prec_mult: 1,
            proc_multipliers: Vec::new(),
            proc_prec_mults: Vec::new(),
        }
    }

    pub(crate) fn finish(self) -> ir::SimIr {
        ir::SimIr {
            instances: self.instances,
            nets: self.nets,
            processes: self.processes, // ← v2: procedural lowering
            cont_assigns: self.cont_assigns,
            // B1 frame-call: automatic/recursive functions lowered to the func
            // arena. EMPTY for every design with no frame functions (the inline
            // path is unchanged) → `ir.funcs`/`ir.blocks` stay empty, golden-neutral.
            funcs: self.funcs,
            exprs: self.exprs,
            stmts: self.stmts,        // ← v2: per-BB straight-line stmt arena
            blocks: self.func_blocks, // B1 frame-call: func-body CFGs (global, rebased)
            consts: self.consts,
        }
    }

    // ── diagnostics ────────────────────────────────────────────────
    /// Emit an error-severity diagnostic and flag failure. v1 has no line table
    /// → `location: None`; the byte span (when relevant) goes into `message`.
    /// HOOK: when elaborate grows a span side-table, fill `SourceLoc` here.
    pub(crate) fn error(&mut self, code: MsgCode, msg: &str) {
        self.had_error = true;
        self.error_count += 1;
        // ELAB-ERR-CAP: past the cap, suppress the flood (one final notice at the
        // boundary). The IR is already doomed (`had_error`), so dropping the tail
        // of an identical-error storm loses no diagnostic value.
        if self.error_count > MAX_ELAB_ERRORS {
            if self.error_count == MAX_ELAB_ERRORS + 1 {
                self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Error,
                    code: MsgCode::ElabUnsupported,
                    message: format!(
                        "too many elaborate errors; further diagnostics suppressed (cap {MAX_ELAB_ERRORS})"
                    ),
                    location: None,
                    context: Vec::new(),
                    sim_time: None,
                }));
            }
            return;
        }
        let location = self.cur_location();
        let context = self.cur_context();
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Error,
            code,
            message: msg.to_string(),
            location,
            context,
            sim_time: None,
        }));
    }

    /// R17 §3.3 / §4.2: emit a NOTE anchored at `span` — a follow-on line that
    /// explains the error just emitted by pointing at a DIFFERENT location.
    ///
    /// Every other elaborate diagnostic is anchored at `cur_span`, which for a
    /// block-local gate is the declaration. That is the right place for "this
    /// declaration is unsupported", and the wrong place for "…because of the
    /// construct over there" — which is what the definite-assignment rejections
    /// actually mean, and why a report could locate 21 diagnostics precisely and
    /// still not reduce a single one of them to a test case. Carries the same
    /// `MsgCode` as its error so `-Wno-`/`-Werror=` routing treats the pair as one
    /// diagnostic. Notes never set `had_error` and never count as errors.
    pub(crate) fn note_at(&mut self, code: MsgCode, span: ast::Span, msg: &str) {
        // Past the error cap the parent error was suppressed; a note explaining a
        // diagnostic that was never printed is noise.
        if self.error_count > MAX_ELAB_ERRORS {
            return;
        }
        let location = self.span_resolver.map(|r| r.resolve(span.lo, span.hi));
        let context = self.cur_context();
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Note,
            code,
            message: msg.to_string(),
            location,
            context,
            sim_time: None,
        }));
    }

    /// The INSTANCE path this diagnostic is being elaborated under, as a
    /// single `Frame`. Empty at the top level (and during any pre-pass that runs
    /// before instantiation), which renders as nothing at all.
    ///
    /// ⚠️ `file:line:col` is NOT a key on its own here. A module instantiated N
    /// times elaborates N times, so one unconnected port in a leaf becomes N
    /// identical warnings; an external report received four of them from three
    /// different instances and could only count them. `cur_prefix` is the
    /// scope-resolution prefix — the same string `fq()` builds symbol keys from
    /// — so it is the instance path by construction rather than a second
    /// spelling of one.
    pub(crate) fn cur_context(&self) -> Vec<diag::Frame> {
        if self.cur_prefix.is_empty() {
            return Vec::new();
        }
        vec![diag::Frame {
            label: self.cur_prefix.clone(),
            location: None,
        }]
    }

    /// The `SourceLoc` for `cur_span`, when both a span and a resolver are present.
    pub(crate) fn cur_location(&self) -> Option<diag::SourceLoc> {
        let sp = self.cur_span?;
        Some(self.span_resolver?.resolve(sp.lo, sp.hi))
    }

    /// Emit a WARNING-severity diagnostic and KEEP GOING — does NOT set
    /// `had_error`, so the SimIr survives and is returned. This is the lever that
    /// makes unsupported *procedural* constructs and unknown `$task`s degrade
    /// (skip / no-op) instead of discarding the whole module (COVERAGE M-A/M-B/M-D).
    /// Stamps `ElabFeatureLimit` (W-ELAB-FEATURE-LIMIT / VITA-W3056) — the
    /// "legal construct accepted but simplified" channel. The message carries
    /// the specifics.
    pub(crate) fn warn(&mut self, msg: &str) {
        // P2-10: the generic warn class is "legal construct accepted but
        // simplified" (W-ELAB-FEATURE-LIMIT) — it used to stamp EVERY warning
        // `W-ELAB-WIDTH-TRUNC`, breaking the doc-15 bijection/suppress routing.
        self.warn_code(MsgCode::ElabFeatureLimit, msg);
    }

    /// Intern a SOURCE string literal, reporting any escape IEEE 1800-2017
    /// Table 5-1 does not define (`W-ELAB-STR-ESCAPE`).
    ///
    /// The diagnostic exists because this class is invisible from inside one
    /// tool: `"\r"` compiles everywhere, evaluates to 0x0D here and in
    /// Verilator and to the letter `r` in iverilog and Xcelium, and nothing in
    /// a passing run says so. An external report spent two sign-off round trips
    /// on exactly that, with the symptom looking like a DUT bug.
    ///
    /// Only literals written in the SOURCE go through here. A synthesized
    /// string (`$typename`) must use `str_const_from_bytes`, or it would be
    /// re-escaped and could report a warning about text no one wrote.
    ///
    /// Anchored at the LITERAL's own span, not `cur_span`: `$display("\r", "\q")`
    /// is two facts on one statement, and two lines that print the same
    /// `file:line:col` are the problem this whole diagnostic class is about.
    pub(crate) fn intern_str_literal(&mut self, raw: &str, span: ast::Span) -> u32 {
        let (bytes, odd) = crate::literal::unescape_str_literal_reporting(raw);
        // ONE line per (literal, escape) FOR THE WHOLE RUN — which answers two
        // questions with one latch:
        //
        //   - `"\r\r\r"` is one fact, not three; and
        //   - `lower_expr` runs again for every instance, so a leaf
        //     instantiated four times reported the same source `"\r"` four
        //     times (measured). The unconnected-port warning genuinely IS per
        //     instance — a dangling output exists once per instantiation — but
        //     an escape is a property of the TEXT, and repeating it scales a
        //     portability note with the design's instance count.
        //
        // ⚠️ A separate intra-literal `said` vector was written first and a
        // mutation SURVIVED deleting it: the span key already covers repeats
        // inside one literal, because they share a span. Two mechanisms for one
        // question is the thing that drifts, so the vector is gone.
        for e in &odd {
            let w = e.written();
            if !self.escape_warned.insert((span.lo, span.hi, w.clone())) {
                continue;
            }
            let detail = match e {
                crate::literal::NonStdEscape::CExtension { byte, .. } => format!(
                    "vita and Verilator read it as 0x{byte:02X}, iverilog and Xcelium read it \
                     as the character `{}`",
                    w.trim_start_matches('\\')
                ),
                crate::literal::NonStdEscape::Unknown { ch } => format!(
                    "vita keeps both characters (`\\` then `{ch}`, so the string is one byte \
                     WIDER), iverilog and Xcelium drop the backslash"
                ),
            };
            self.warn_code_at(
                MsgCode::ElabStrEscape,
                span,
                &format!(
                    "`{w}` is not a string escape in IEEE 1800-2017 Table 5-1 — {detail}. \
                     Write the byte explicitly (`\\015` octal or `\\x0D` hex) to mean the \
                     same thing everywhere"
                ),
            );
        }
        self.intern_const(crate::literal::str_const_from_bytes(&bytes))
    }

    /// [`Self::warn_code`] anchored at an EXPLICIT span rather than `cur_span`
    /// — the warning twin of [`Self::note_at`]. `cur_span` is the enclosing
    /// STATEMENT, which is the right anchor for "this construct is simplified"
    /// and the wrong one for a warning about one sub-expression: two of them on
    /// one statement print the same `file:line:col` and cannot be told apart.
    pub(crate) fn warn_code_at(&mut self, code: MsgCode, span: ast::Span, msg: &str) {
        let location = self
            .span_resolver
            .map(|r| r.resolve(span.lo, span.hi))
            .or_else(|| self.cur_location());
        let context = self.cur_context();
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code,
            message: msg.to_string(),
            location,
            context,
            sim_time: None,
        }));
    }

    /// Emit a Warning with a SPECIFIC code (the generic [`Self::warn`] uses
    /// `W-ELAB-FEATURE-LIMIT`).
    pub(crate) fn warn_code(&mut self, code: MsgCode, msg: &str) {
        let location = self.cur_location();
        let context = self.cur_context();
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code,
            message: msg.to_string(),
            location,
            context,
            sim_time: None,
        }));
    }

    /// Emit a hard "construct not supported in this subset" error, reusing the
    /// EXISTING `ElabUnsupported` code (no new MsgCode minted → doc-15 bijection
    /// untouched). The `_span` is accepted for a future side-table; v1 has no line
    /// table so it carries no location (consistent with `error`).
    pub(crate) fn error_unsupported(&mut self, _span: ast::Span, msg: &str) {
        self.error(MsgCode::ElabUnsupported, msg);
    }

    // ── v3 multi-module driver ─────────────────────────────────────
    /// Build the module-name map, pick the top, then recursively flatten the
    /// hierarchy into ONE SimIr. The v1 single-module path is now the special
    /// case `top instantiating nothing` (one Instance, parent None).
    pub(crate) fn run(&mut self, unit: &ast::SourceUnit) {
        let (map, order) = build_module_map(unit);
        // §4.5.200: pre-scan EVERY module's procedural blocks for hierarchical TASK enables
        // (`u1.tk(...)`) and record the target task name, so `build_task_frame_set` can
        // FORCE-FRAME a hier-called STATIC task (otherwise it inlines and has no per-instance
        // FuncId for the §4.5.197 hier defer/resolve to bind → the hier call stays loud).
        // Runs ONCE before any framing. A hier enable nested in a task body is loud anyway
        // (§4.5.197), and one in a generate block stays loud — both correct-or-loud, so
        // scanning top-level procedural blocks is sufficient (never silent-wrong).
        for m in &order {
            for it in &m.body {
                if let ast::ModuleItem::Proc(p) = it {
                    collect_hier_task_stmt(&p.body, &mut self.hier_called_task_names);
                }
            }
        }
        // v5 ⑥ (D): interfaces live in their OWN map (they are never roots and
        // never modules); a name colliding with a module is a duplicate design
        // unit (single design-unit namespace, doc-15 E-DUP-UNIT).
        for it in &unit.items {
            if let ast::TopItem::Interface(i) = it {
                if map.contains_key(i.name.name.as_str())
                    || self.ifaces.insert(i.name.name.clone(), i.clone()).is_some()
                {
                    self.error(
                        MsgCode::DupUnit,
                        &format!("design unit `{}` declared more than once", i.name.name),
                    );
                }
            }
            // v7 P2-D: packages register into their own maps (never roots);
            // a name colliding with a module/interface/package is E-DUP-UNIT.
            if let ast::TopItem::Package(pm) = it {
                if map.contains_key(pm.name.name.as_str())
                    || self.ifaces.contains_key(&pm.name.name)
                    || self.pkg_consts.contains_key(&pm.name.name)
                {
                    self.error(
                        MsgCode::DupUnit,
                        &format!("design unit `{}` declared more than once", pm.name.name),
                    );
                } else {
                    self.elaborate_package(pm);
                }
            }
            if let ast::TopItem::Import(i) = it {
                self.cu_imports.push(i.clone());
            }
        }
        // Round-9: index every top-level `bind <target> <checker> u(...)` by its
        // TARGET module. Both target and checker must resolve to known MODULES
        // (loud otherwise — a mis-named bind must never silently no-op). v1
        // attaches OBSERVER checkers only: a checker OUTPUT/INOUT port would drive
        // a target-internal net (multi-driver), so reject one loudly. Each bound
        // checker fires once per target instance at step (8) of
        // `elaborate_instance`, wired in that instance's own scope.
        for it in &unit.items {
            let ast::TopItem::Bind(b) = it else {
                continue;
            };
            let target = b.target.name.clone();
            let checker = &b.inst.module_name.name;
            let Some(&(cdecl, _)) = map.get(checker.as_str()) else {
                self.error(
                    MsgCode::ElabUnresolvedInstance,
                    &format!("bind checker module `{checker}` not found in the design"),
                );
                continue;
            };
            if !map.contains_key(target.as_str()) {
                self.error(
                    MsgCode::ElabUnresolvedInstance,
                    &format!("bind target module `{target}` not found in the design"),
                );
                continue;
            }
            if port_list_dirs(cdecl)
                .iter()
                .any(|(_, d)| matches!(d, ir::PortDir::Output | ir::PortDir::Inout))
            {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "bind checker `{checker}` has an output/inout port; v1 supports \
                         observer (input-only) checkers so a bound instance never drives \
                         a target net"
                    ),
                );
                continue;
            }
            self.bind_targets
                .entry(target)
                .or_default()
                .push(b.inst.clone());
        }
        // N7: register every class (whole-design prescan, forward-reference safe)
        // before any module lowers, so a class-handle decl / `new` / method call
        // resolves regardless of declaration order, then lower every method into
        // the global func arena (fids resolve at module-body call sites).
        self.register_classes(unit);
        self.lower_class_methods();
        self.prescan_clocking_names(unit);
        if order.is_empty() {
            // "no module at all" is a missing-construct condition, not a failed
            // *instance* resolution → ElabUnsupported reads truer.
            self.error(MsgCode::ElabUnsupported, "no top module to elaborate");
            return;
        }
        // Warn on duplicate module names (first-decl wins in the map).
        let mut seen: BTreeMap<&str, u32> = BTreeMap::new();
        for m in &order {
            *seen.entry(m.name.name.as_str()).or_insert(0) += 1;
        }
        for (name, n) in seen {
            if n > 1 {
                // P2-11: duplicate design-unit definition is an ERROR (doc-15
                // E-DUP-UNIT; iverilog parity) — was a warn + first-decl-wins.
                self.error(
                    MsgCode::DupUnit,
                    &format!("module `{name}` declared {n} times"),
                );
            }
        }

        // Round-9: bind-checker module names — excluded from the default root set
        // (they attach via `bind`, never as free-standing tops).
        let bind_checkers: std::collections::BTreeSet<String> = self
            .bind_targets
            .values()
            .flatten()
            .map(|mi| mi.module_name.name.clone())
            .collect();
        let roots = match self.root_override.clone() {
            // `--top` override: the named units, in the given order. Unknown
            // names are loud — silently elaborating the default set instead
            // would be a silent-wrong root selection.
            Some(tops) => {
                let mut sel = Vec::new();
                for t in &tops {
                    match map.get(t.as_str()) {
                        Some((m, _)) => sel.push(*m),
                        None => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!("top module `{t}` not found in the design"),
                            );
                            return;
                        }
                    }
                }
                sel
            }
            None => {
                // r17 correct-or-loud: auto-top (no `--top`) picking among 2+
                // uninstantiated roots used to be SILENT. IEEE 1364/iverilog
                // elaborate every uninstantiated module as an independent top, so
                // this stays a WARNING (not an error) — the behavior is preserved,
                // but the ambiguity is surfaced so a masked/unintended top cannot
                // pass unnoticed. `--top <module>` pins a deterministic single top.
                let r = pick_roots(&map, &order, &bind_checkers);
                if r.len() >= 2 {
                    let names: Vec<&str> = r.iter().map(|m| m.name.name.as_str()).collect();
                    self.warn_code(
                        MsgCode::ElabAutoTopAmbiguous,
                        &format!(
                            "auto-top selected {} uninstantiated roots ({}); all are \
                             elaborated as independent tops — pin one with `--top <module>` \
                             for a deterministic single top",
                            names.len(),
                            names.join(", ")
                        ),
                    );
                }
                r
            }
        };
        if roots.is_empty() {
            self.error(MsgCode::ElabUnsupported, "no top module to elaborate");
            return;
        }

        // Each root is its OWN top instance: parent None, path = its module name
        // (root VCD scope), no incoming port/param bindings. `elaborate_instance`
        // saves/restores all scope state (cur_prefix/cur_inst/cur_time_mult/params/
        // func_table/task_table/inst_stack), so roots are independent and the flat
        // arenas stay contiguous per instance. The common single-top design has one
        // root → byte-identical to the old single-pick path.
        // `-G NAME=VALUE` applies to EVERY root. A name that no root declares is loud
        // once, after the loop, so `-G` on a two-root design does not report twice.
        let mut cli_used: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for (idx, top) in roots.iter().enumerate() {
            let top_path = top.name.name.clone();
            let cli_ovr = self.cli_overrides_for(top, &mut cli_used);
            // §4.5.261: a root's key is its POSITION IN THE ROOT LIST, not its source
            // offset. `--top zz --top aa` elaborates in the order given, and `-L` library
            // mode compiles each unit separately so offsets from different units are not
            // even comparable — either way the offset is not the order.
            self.elaborate_instance(
                top,
                &top_path,
                None,
                &cli_ovr,
                PortBinding::None,
                &map,
                (0, idx as u32, 0),
                // A ROOT has no instantiation site — nothing instantiated it.
                None,
            );
        }
        for (n, _) in self.top_param_overrides.clone() {
            if !cli_used.contains(&n) {
                self.error(
                    MsgCode::ElabPortMismatch,
                    &format!(
                        "`-G {n}=…` names no parameter of any top module — check the \
                         spelling, or that it is a `parameter` and not a `localparam`"
                    ),
                );
            }
        }
        // Any defparam still in the map targeted an instance that was never
        // elaborated — a typo'd or out-of-scope path, or an array `u.N` with no
        // index. iverilog warns ("Scope of <path> not found") and the target keeps
        // its default; mirror that with a warning rather than a silent no-op (the
        // default value is already what an unconsumed override leaves in place).
        let unmatched: Vec<String> = self.defparams.keys().cloned().collect();
        for fq in unmatched {
            self.warn(&format!(
                "defparam target `{fq}` matched no instance — override ignored \
                 (the parameter keeps its default)"
            ));
        }
        self.defparams.clear();

        // §4.5.166 HIER twin: a hierarchical read/write in an implicit
        // `@(*)`/`always_comb`/`always_latch` — whole-net (`y = dut.q`) OR indexed
        // (`y = dut.mem[idx]` / `dut.mem[idx] = v`) — lowers behind a placeholder
        // expr / sentinel chunk, so the referenced net (and any index) was
        // invisible to `comb_read_set` at process-lowering time and dropped from
        // the sensitivity list (silent stale; the LOCAL-index twin is fixed in
        // `collect_lval_reads`). ALL FOUR deferral lanes must arm the recompute —
        // the whole-net lanes too, else `always_comb y = dut.q` stays stale and
        // its correctness would hinge on an unrelated indexed ref elsewhere. The
        // resolvers below patch the real net+index into the stmt/expr arenas;
        // recompute the affected comb read-sets after.
        let had_hier_defer = !self.deferred_hier.is_empty()
            || !self.deferred_hier_sel.is_empty()
            || !self.deferred_hier_write.is_empty()
            || !self.deferred_hier_sel_write.is_empty();
        // N3.1: resolve hierarchical INDEXED reads FIRST (their index lowering may
        // itself defer a whole-net hierarchical read into `deferred_hier`)…
        self.resolve_deferred_hier_sel();
        // N3: …then resolve the whole-net hierarchical READ references, now that EVERY
        // instance's nets are in `symbols` (deferred during pass-7 lowering because
        // child nets are created in pass 8). Patches each placeholder to the real NetId.
        self.resolve_deferred_hier();
        // Family D (r17): patch deferred hierarchical FUNCTION calls (`u1.f(x)`) to the
        // callee's per-instance FuncId — now that every instance's frame funcs are
        // registered in `hier_funcs`. Loud on any unresolved / non-hier-callable target.
        self.resolve_deferred_hier_call();
        // Family D (r18): …and the deferred hierarchical TASK enables (`u1.tk(x);`) —
        // build each callee's per-instance `TaskCallInfo` into `task_calls_proc` now that
        // every instance's frame tasks are in `hier_tasks`.
        self.resolve_deferred_hier_task_call();
        // N3 follow-on (HIER-REST): patch deferred hierarchical WRITE targets
        // (`tb.dut.x = …`) — BEFORE the multidriver scan so it sees real net ids.
        self.resolve_deferred_hier_write();
        // HIER-REST①: …and the deferred hierarchical ELEMENT/bit-select writes
        // (`dut.mem[i] = …`), also before the multidriver scan (the rebuilt chunks
        // carry real net ids).
        self.resolve_deferred_hier_sel_write();
        // §4.5.166: now that hier read/write chunks + exprs carry real nets and
        // index eids, recompute the comb/latch read-sets so the referenced net
        // (and any index) enters the sensitivity list. Only runs when a hier ref
        // was deferred (any of the four lanes).
        if had_hier_defer {
            self.recompute_comb_sensitivity_after_hier();
        }

        // whole-net multidriver check over the WHOLE flat IR (instance-agnostic).
        self.check_whole_net_multidriver();
    }
}
