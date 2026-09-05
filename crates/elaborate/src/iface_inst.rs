//! Interface INSTANCE elaboration (`ifc u();` — params, member nets, the scope's own
//! §6.8 pre-sweep, then its logic) — split out of `ports.rs` (mechanical move;
//! module-size policy).

use super::*;

impl Elaborator<'_> {
    pub(crate) fn elaborate_iface_instances(&mut self, mi: &ast::ModuleInstance, wire_phase: bool) {
        let iface_name = mi.module_name.name.clone();
        let Some(decl) = self.ifaces.get(&iface_name).cloned() else {
            return;
        };
        // v6 ②: per-instance parameter overrides — resolved NOW, in the
        // PARENT scope, exactly like the module-instance path (Fix 1).
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
                        bits: self.override_bits(e),
                        signed: Some(self.const_signed_env(e, &ConstWidths::new())),
                        array: None,
                        elem_select: false,
                    };
                    if value.is_none() {
                        if Self::expr_is_real_literal(e) {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "overriding a parameter with a real value is unsupported \
                                 (a real override cannot be folded); the default would \
                                 be used silently",
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
                                "parameter override expression is not a constant; default kept",
                            );
                        }
                    }
                    overrides.push(ovr);
                }
                ast::ParamConn::Named { name, value, .. } => {
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
                            } else if ResolvedOverride::keeps_default_of(
                                None,
                                fill.as_ref(),
                                text.as_ref(),
                                self.override_bits(e).as_ref(),
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
                        bits: value.as_ref().and_then(|e| self.override_bits(e)),
                        signed: value
                            .as_ref()
                            .map(|e| self.const_signed_env(e, &ConstWidths::new())),
                        array: None,
                        elem_select: false,
                    });
                }
            }
        }
        let ports_ok = match &decl.ports {
            ast::PortList::None => true,
            ast::PortList::Ansi(v) => v.iter().all(|p| p.iface.is_none()),
            ast::PortList::NonAnsi(v) => v.is_empty(),
        };
        if !ports_ok {
            self.error(
                MsgCode::ElabUnsupported,
                "non-ANSI or interface-typed header ports on an interface are outside the MVP",
            );
            return;
        }
        for item in &mi.instances {
            if !item.unpacked.is_empty() {
                self.error(
                    MsgCode::ElabUnsupported,
                    "interface instance arrays are outside the MVP",
                );
                continue;
            }
            let path = self.child_prefix(&item.name.name);
            if !self.iface_insts.contains_key(&path) {
                let saved_prefix = std::mem::replace(&mut self.cur_prefix, path.clone());
                // params (header `#(...)` then body localparams) BEFORE nets
                // so `[W-1:0]` folds — mirroring module passes (3)/(3b).
                // §3 ⑤ ⓕ: the interface's imports, CONST symbols only (an interface
                // body has no functions/tasks, so a routine brought in by an import
                // has no caller to resolve — a call stays loud). Two passes around
                // `bind_params`, exactly as the module scope does: a compilation-unit
                // or HEADER import (`interface i import p::*; #(parameter N = W)`) is
                // visible to the header's own defaults, a body import only after.
                let iface_imports: Vec<ast::ImportDecl> = self
                    .cu_imports
                    .clone()
                    .into_iter()
                    .chain(decl.body.iter().filter_map(|it| match it {
                        ast::ModuleItem::Import(i) => Some(i.clone()),
                        _ => None,
                    }))
                    .collect();
                let n_cu = self.cu_imports.len();
                let local_names = self.gather_local_decl_names(&decl);
                let mut wc_origin: BTreeMap<String, String> = BTreeMap::new();
                let mut explicit_imports: std::collections::BTreeSet<String> =
                    std::collections::BTreeSet::new();
                let mut saved_params: Vec<(String, Option<i64>)> = Vec::new();
                for (i, imp) in iface_imports.iter().enumerate() {
                    if Self::import_precedes_header(&decl, n_cu, i, imp) {
                        self.apply_import_consts(
                            imp,
                            &mut saved_params,
                            &mut wc_origin,
                            &mut explicit_imports,
                            &local_names,
                            i >= n_cu,
                        );
                    }
                }
                let param_ovr = {
                    let (sp, ovr) = self.bind_params(&decl, &overrides);
                    saved_params.extend(sp);
                    ovr
                };
                for (i, imp) in iface_imports.iter().enumerate() {
                    if !Self::import_precedes_header(&decl, n_cu, i, imp) {
                        self.apply_import_consts(
                            imp,
                            &mut saved_params,
                            &mut wc_origin,
                            &mut explicit_imports,
                            &local_names,
                            i >= n_cu,
                        );
                    }
                }
                for it in &decl.body {
                    if let ast::ModuleItem::Param(pp) = it {
                        // Same binder as the module body loop and the generate fold.
                        // This loop used to be its own reduced copy — `const_eval` →
                        // `coerce` → `hier_params`/`params`, and NOTHING else — so an
                        // interface body parameter never recorded `param_meta` (its
                        // declared width and sign) or `param_range`, and a `string` /
                        // `real` / >64-bit one was not routed at all (`parameter S =
                        // "abc"` in an interface was loud E3009 on its own default).
                        // `generate.rs` carries a ⚠️ note about repairing exactly this
                        // in the generate fold; the interface was the last REDUCED copy.
                        //
                        // THREE full copies remain — `instance.rs`'s module-body fold,
                        // `generate.rs`'s, and `package.rs`'s — and they disagree with
                        // this binder in more than one place, measured: `generate.rs`
                        // and `package.rs` fold with `const_eval_in_scope` instead of
                        // `eval_param_init`, so a fill DEFAULT is sized to 32 bits and
                        // not to the declared width (`parameter [63:0] Q = '1` reads
                        // `0000_0000_ffff_ffff` in both), and `package.rs` records no
                        // `param_range` (a package `parameter [15:8] P` part-selects to
                        // `x`) and routes neither `string` nor `real`. All of that is
                        // pre-existing and identical in PRE — a separate slice, one
                        // line in ROADMAP §3. Do NOT fix an instance of it here: this
                        // is a class, and the funnel is this function.
                        //
                        // With no ANSI header these declarations ARE the overridable
                        // parameters (`param_ports`), so binding them here also applies
                        // an override that targets one. Binding ONLY those through the
                        // shared path and leaving the rest on the reduced copy is what
                        // the first cut did, and it made a parameter's registered WIDTH
                        // depend on whether it happened to be overridden: two instances
                        // of one interface then disagreed inside a single run at exit 0
                        // (`ifc #(.P(8'hA5)) a(); ifc b();` — same value, `a.P[15:12]=a`
                        // and `b.P[15:12]=0`). One spelling for every declaration.
                        self.bind_one_param(pp, &param_ovr, &mut saved_params);
                        // The i64 twin, republished so `i0.P` stays readable from
                        // outside. This is PRE's behaviour and the measured reason to
                        // keep it is arithmetic, not sentiment: over 13 consumers × 6
                        // exact-integer values, the i64 view is CORRECT in 72 cells and
                        // wrong in 6 — every wrong cell is `/` with a fractional
                        // quotient (`P=5` → `i0.P/2` gives 2.0, iverilog 2.5). Dropping
                        // it took `int'`, `$rtoi`, `$sqrt`, `*1.5`, `+0.5`, `>`, a real
                        // assignment and the bare read from correct to loud — 72
                        // correct→loud regressions to remove 6 silent-wrong ones. (An
                        // earlier revision of this comment claimed the reverse and cited
                        // `P = 4`; the discriminator is not the VALUE but the OPERATOR —
                        // only division with a fractional quotient separates the two
                        // domains, so `P = 8` is correct at `/2` as well.)
                        //
                        // ⚠️ It does leave one declaration answering two ways in a
                        // single run: the BARE read now reaches `real_param_val` through
                        // the binder above and is 2.5, while this hierarchical twin is
                        // 2.0. That split is the honest state of the hierarchical-real
                        // axis, not a property of this line — patching the deferred
                        // placeholder with a real constant instead breaks strictly more
                        // cells (every integral consumer reads the IEEE-754 bits).
                        // ROADMAP §2 owns it.
                        if let Some((_, Some(i))) = self.param_real_value(&pp.ty, &pp.value) {
                            let key = self.fq(&pp.name.name);
                            self.hier_params.insert(key, i);
                        }
                    }
                }
                // ANSI header ports → nets (the iface body + `i.<port>` see them).
                self.elaborate_ports(&decl.ports);
                // nets first (declaration order), then logic — mirroring the
                // module body passes (4)/(7).
                // §4.5.265: the net-decl loop runs INSIDE the instance's rank scope too,
                // because a declaration records its pre-size and its block-local
                // initializers under the rank path in effect at the DECLARATION — and the
                // flush below claims by that path. Creating the nets outside the scope and
                // flushing inside it meant no flush ever claimed them, which the
                // never-emitted guard reported (loudly, which is the point of it).
                let slot = self.rank_slot_for_instance();
                let rkey = (self.rank_band, item.name.span.lo, 0);
                self.with_rank_scope_keyed(slot, rkey, |sc| {
                    for it in &decl.body {
                        if let ast::ModuleItem::NetVar(d) = it {
                            // A desugared array parameter is created like any var; its
                            // `'{…}` decl-init rides the interface §6.8 pre-sweep below
                            // (collect + flush, `lowering_decl_init`-exempt), so it is a
                            // supported form now (the A2a scope-gate is lifted). User
                            // writes still hit the net-id-keyed const-param deny.
                            // `allow_string_init` is TRUE here now, for the same reason as the
                            // generate walk: the flag was standing in for a string
                            // declaration's decl-time writes landing in the MODULE-scope
                            // pending list, where the bare-name lvalue resolved outside this
                            // instance's prefix. Those are keyed by the declaring scope now.
                            sc.elaborate_netvar_decl(d, &decl.ports, &decl.body, true);
                        }
                    }
                    // §6.8 pre-sweep for the interface body (mirrors the module-body
                    // sweep): an array `'{…}` / non-constant decl-init has no foldable
                    // `net.init`, so without this collect+flush it was silently dropped.
                    // Runs in the interface INSTANCE scope (bare-name lvalues resolve to
                    // `path.name`) and BEFORE the logic pass below, so the synthesized
                    // `initial` precedes the interface's own procs.
                    //
                    // SAVE/RESTORE the shared `pending_var_inits` around it: this pass
                    // runs during the PARENT's Nets phase, and `hoist_block_local_nets`
                    // may already have queued a module block-local non-const init there
                    // (it runs earlier, in pass 4a). Without the isolation this flush
                    // would STEAL that init and re-lower it in the interface scope —
                    // both a loud misresolve and (with same-named members) a silent
                    // module-side drop. The generate VarInit walk isolates the same way.
                    // §4.5.259: an interface instance is a SCOPE of its own, so it takes the
                    // instance slot like a module child. Without a scope of its own its flush
                    // borrowed the ENCLOSING scope's own-variables slot, and — because its two
                    // call sites run in different passes than the module's own flush — the
                    // rank vectors collided outright: a module's own initializer ran BETWEEN
                    // two interfaces, and a generate-nested interface ran after the generate's
                    // own variable. Both are the enclosing scope's slot, decided by tie-break.
                    let saved_pending = std::mem::take(&mut sc.pending_var_inits);
                    for it in &decl.body {
                        if let ast::ModuleItem::NetVar(d) = it {
                            sc.collect_var_init_drivers(d);
                        }
                    }
                    sc.flush_block_local_inits();
                    sc.pending_var_inits = saved_pending;
                });
                for it in &decl.body {
                    match it {
                        ast::ModuleItem::ContAssign(ca) => self.elaborate_cont_assign(ca),
                        ast::ModuleItem::Proc(pb) => {
                            if self.try_elab_task(pb) {
                                continue;
                            }
                            let proc = self.lower_user_proc(pb);
                            self.push_process(proc);
                        }
                        ast::ModuleItem::NetVar(d) => self.elaborate_net_init_drivers(d),
                        ast::ModuleItem::Modport(_) => {} // binding enforces dirs
                        ast::ModuleItem::Error(_)
                        | ast::ModuleItem::Param(_)
                        | ast::ModuleItem::PortDecl(_)
                        | ast::ModuleItem::Genvar { .. } => {}
                        // Applied above, around the parameter bind.
                        ast::ModuleItem::Import(_) => {}
                        other => {
                            let what = match other {
                                ast::ModuleItem::Instance(_) => "nested instances",
                                ast::ModuleItem::Generate(_) => "generate blocks",
                                ast::ModuleItem::Func(_) | ast::ModuleItem::Task(_) => {
                                    "functions/tasks"
                                }
                                ast::ModuleItem::Typedef(_) => "typedefs",
                                ast::ModuleItem::Defparam(_) => "defparam",
                                _ => "this construct",
                            };
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!("{what} inside an interface are outside the MVP"),
                            );
                        }
                    }
                }
                self.iface_insts.insert(path.clone(), iface_name.clone());
                self.restore_params(saved_params);
                self.cur_prefix = saved_prefix;
            }
            // v6 ②: header-port connections wire LATE (all parent nets exist
            // by pass 8); the early 4c call leaves them for this pass.
            if wire_phase {
                let has_conns = match &item.conns {
                    // `.*` alone matches zero ports on a port-less module → not
                    // "connections given", so the wildcard does not count here.
                    ast::PortConnList::Named(v, _) => !v.is_empty(),
                    ast::PortConnList::Positional(v) => !v.is_empty(),
                };
                let has_ports = !matches!(&decl.ports, ast::PortList::None)
                    && !matches!(&decl.ports, ast::PortList::Ansi(v) if v.is_empty());
                if has_conns && !has_ports {
                    self.error(
                        MsgCode::ElabPortMismatch,
                        "connections on a portless interface instance",
                    );
                    continue;
                }
                if has_ports {
                    let binding = match &item.conns {
                        ast::PortConnList::Named(v, wc) => PortBinding::Named(v, *wc),
                        ast::PortConnList::Positional(v) => PortBinding::Positional(v),
                    };
                    let saved_prefix = std::mem::replace(&mut self.cur_prefix, path.clone());
                    let parent = saved_prefix.clone();
                    self.wire_ports(&decl, binding, &parent, false);
                    self.cur_prefix = saved_prefix;
                }
            }
        }
    }
}
