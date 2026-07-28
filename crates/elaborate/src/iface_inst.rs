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
                        } else {
                            self.warn(
                                "parameter override expression is not a constant; default kept",
                            );
                        }
                    }
                    overrides.push(ResolvedOverride {
                        name: None,
                        value,
                        is_named: false,
                        had_value: true,
                        fill: expr_as_fill(e).map(|(k, r)| (k, r.to_string())),
                    });
                }
                ast::ParamConn::Named { name, value, .. } => {
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
                            } else {
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
                        fill: value
                            .as_ref()
                            .and_then(|e| expr_as_fill(e).map(|(k, r)| (k, r.to_string()))),
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
                let mut saved_params = self.bind_params(&decl, &overrides);
                for it in &decl.body {
                    if let ast::ModuleItem::Param(pp) = it {
                        let v = self.const_eval_in_scope(&pp.value).unwrap_or_else(|| {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "parameter `{}` value is not a foldable constant expression",
                                    pp.name.name
                                ),
                            );
                            0
                        });
                        let v = self.coerce_param_value(v, pp);
                        let key = self.fq(&pp.name.name);
                        self.hier_params.insert(key.clone(), v);
                        saved_params.push((key.clone(), self.params.insert(key, v)));
                    }
                }
                // ANSI header ports → nets (the iface body + `i.<port>` see them).
                self.elaborate_ports(&decl.ports);
                // nets first (declaration order), then logic — mirroring the
                // module body passes (4)/(7).
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
                        self.elaborate_netvar_decl(d, &decl.ports, &decl.body, true);
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
                let saved_pending = std::mem::take(&mut self.pending_var_inits);
                // §4.5.259: an interface instance is a SCOPE of its own, so it takes the
                // instance slot like a module child. Without a scope of its own its flush
                // borrowed the ENCLOSING scope's own-variables slot, and — because its two
                // call sites run in different passes than the module's own flush — the
                // rank vectors collided outright: a module's own initializer ran BETWEEN
                // two interfaces, and a generate-nested interface ran after the generate's
                // own variable. Both are the enclosing scope's slot, decided by tie-break.
                let slot = self.rank_slot_for_instance();
                self.with_rank_scope(slot, |s| {
                    for it in &decl.body {
                        if let ast::ModuleItem::NetVar(d) = it {
                            s.collect_var_init_drivers(d);
                        }
                    }
                    s.flush_block_local_inits();
                });
                self.pending_var_inits = saved_pending;
                for it in &decl.body {
                    match it {
                        ast::ModuleItem::ContAssign(ca) => self.elaborate_cont_assign(ca),
                        ast::ModuleItem::Proc(pb) => {
                            let proc = self.lower_proc_block(pb);
                            self.push_process(proc);
                        }
                        ast::ModuleItem::NetVar(d) => self.elaborate_net_init_drivers(d),
                        ast::ModuleItem::Modport(_) => {} // binding enforces dirs
                        ast::ModuleItem::Error(_)
                        | ast::ModuleItem::Param(_)
                        | ast::ModuleItem::PortDecl(_)
                        | ast::ModuleItem::Genvar { .. } => {}
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
                    self.wire_ports(&decl, binding, &parent);
                    self.cur_prefix = saved_prefix;
                }
            }
        }
    }
}
