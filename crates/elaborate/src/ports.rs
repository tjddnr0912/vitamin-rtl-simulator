//! port wiring / interfaces — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// A module's ports as `(local_name, dir)` in HEADER declaration order. ANSI
/// ports read dir inline; non-ANSI merges the body `PortDecl` directions over the
/// header name list (an undeclared header name defaults to Input + is rare).
/// Port wiring walks this in order, so a named connection list in any source
/// order produces a deterministic cont-assign sequence.
/// v5 ⑥ (D): the `IfaceRef` of an ANSI interface-typed port, by name.
pub(crate) fn ansi_iface_ref<'m>(
    module: &'m ast::ModuleDecl,
    pname: &str,
) -> Option<&'m ast::IfaceRef> {
    match &module.ports {
        ast::PortList::Ansi(list) => list
            .iter()
            .find(|p| p.name.name == pname)
            .and_then(|p| p.iface.as_ref()),
        _ => None,
    }
}

pub(crate) fn port_list_dirs(module: &ast::ModuleDecl) -> Vec<(String, ir::PortDir)> {
    match &module.ports {
        ast::PortList::Ansi(list) => list
            .iter()
            .map(|p| (p.name.name.clone(), map_port_dir(p.dir)))
            .collect(),
        ast::PortList::NonAnsi(names) => {
            // find each header name's direction in a body PortDecl.
            names
                .iter()
                .map(|n| {
                    let dir = module
                        .body
                        .iter()
                        .find_map(|it| match it {
                            ast::ModuleItem::PortDecl(pd)
                                if pd.names.iter().any(|x| x.name == n.name) =>
                            {
                                Some(map_port_dir(pd.dir))
                            }
                            _ => None,
                        })
                        .unwrap_or(ir::PortDir::Input);
                    (n.name.clone(), dir)
                })
                .collect()
        }
        ast::PortList::None => Vec::new(),
    }
}

pub(crate) fn map_port_dir(d: ast::PortDir) -> ir::PortDir {
    match d {
        ast::PortDir::Input => ir::PortDir::Input,
        ast::PortDir::Output => ir::PortDir::Output,
        ast::PortDir::Inout => ir::PortDir::Inout,
    }
}

/// N1: net kind for a subroutine FORMAL, accounting for direction. A `string`
/// INPUT formal keeps the classic 1-bit Wire slot (`map_net_kind_or_wire`): the
/// body only reads it, and the call-site materializes the actual into the slot via
/// the `str_params` mask (byte-identical to the proven input path). A `string`
/// OUTPUT / INOUT formal, however, is WRITTEN in the body (`s = $sformatf(...)`) —
/// a Wire target would fail the procedural-assign check (E3018), so it becomes a
/// real `NetKind::String` slot (heap-backed, procedurally assignable); the frame
/// copy-out then moves the string Value onto the caller's `string` actual. Any
/// non-string formal is unchanged.
pub(crate) fn formal_net_kind(k: ast::NetVarKind, dir: ast::PortDir) -> ir::NetKind {
    if matches!(k, ast::NetVarKind::String)
        && matches!(dir, ast::PortDir::Output | ast::PortDir::Inout)
    {
        ir::NetKind::String
    } else {
        map_net_kind_or_wire(k)
    }
}

/// ⓑ-breadth (§25.9): collect the instance names a virtual-interface handle is
/// bound to (`vif = inst;` blocking assigns), recursing through procedural control
/// flow. Only a plain `vif = bare_ident` shape counts as a binding.
pub(crate) fn collect_vif_bindings(s: &ast::Stmt, vif: &str, out: &mut Vec<String>) {
    use ast::Stmt::*;
    match s {
        Blocking {
            lhs: ast::Lvalue::Ident(p),
            rhs,
            ..
        } if p.segments.len() == 1 && p.segments[0].name == vif => {
            if let ast::ExprKind::Ident(rp) = &rhs.kind {
                if rp.segments.len() == 1 {
                    out.push(rp.segments[0].name.clone());
                }
            }
        }
        Block { stmts, .. } | Fork { stmts, .. } => {
            for st in stmts {
                collect_vif_bindings(st, vif, out);
            }
        }
        If { then_s, else_s, .. } => {
            collect_vif_bindings(then_s, vif, out);
            if let Some(e) = else_s {
                collect_vif_bindings(e, vif, out);
            }
        }
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            collect_vif_bindings(body, vif, out)
        }
        Case { items, .. } => {
            for it in items {
                let b = match it {
                    ast::CaseItem::Match { body, .. } => body,
                    ast::CaseItem::Default { body, .. } => body,
                };
                collect_vif_bindings(b, vif, out);
            }
        }
        DelayCtrl { body: Some(b), .. } | EventCtrl { body: Some(b), .. } => {
            collect_vif_bindings(b, vif, out)
        }
        _ => {}
    }
}

impl Elaborator<'_> {
    /// v5 ⑥ (D), extended v6 ②: flatten interface instances (`intf i();`)
    /// into plain nets under `cur_prefix.i` — interface signals ARE nets
    /// (spike: no new IR). v6 adds `#(parameter)` overrides and ANSI header
    /// ports (`interface bus(input logic c)`). Header-port CONNECTIONS wire
    /// in the LATE pass only (`wire_phase` — pass 8, when every parent net
    /// exists); the early 4c pass flattens for parent-body visibility.
    /// Non-ANSI / interface-typed header ports stay loud.
    /// ⓑ-breadth (§25.9): resolve a `virtual INTERFACE vif;` handle as a STATIC
    /// ALIAS. Scan the module body for the single binding `vif = inst;`, validate
    /// the interface type, and alias every `vif.member` symbol to the bound
    /// instance's flattened member net. 0 bindings / dynamic re-binding / a
    /// non-instance rhs are loud (never silent).
    pub(crate) fn elaborate_virtual_iface(
        &mut self,
        d: &ast::NetVarDecl,
        body: &[ast::ModuleItem],
    ) {
        let Some(iface_ty) = d.class_type.as_ref().map(|i| i.name.clone()) else {
            return;
        };
        for n in &d.names {
            let vif = &n.name.name;
            self.vif_handles.insert(self.fq(vif));
            // collect the distinct instances `vif` is bound to.
            let mut bound: Vec<String> = Vec::new();
            for it in body {
                if let ast::ModuleItem::Proc(p) = it {
                    collect_vif_bindings(&p.body, vif, &mut bound);
                }
            }
            bound.dedup();
            let inst = match bound.as_slice() {
                [one] => one.clone(),
                [] => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("virtual interface `{vif}` is never bound (`{vif} = instance;`)"),
                    );
                    continue;
                }
                _ => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "virtual interface `{vif}` is bound to more than one instance \
                             (dynamic re-binding is outside v1)"
                        ),
                    );
                    continue;
                }
            };
            // resolve the bound instance + verify its interface type.
            let inst_path = self.child_prefix(&inst);
            let Some(actual_iface) = self.iface_insts.get(&inst_path).cloned() else {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("virtual interface `{vif}` is bound to `{inst}`, which is not an interface instance"),
                );
                continue;
            };
            if actual_iface != iface_ty {
                self.error(
                    MsgCode::ElabPortMismatch,
                    &format!("virtual interface `{vif}` is typed `{iface_ty}` but `{inst}` is `{actual_iface}`"),
                );
                continue;
            }
            // alias each interface member: `<mod>.vif.member` → the bound net.
            let members = self.iface_member_names(&actual_iface);
            let vif_fq = self.fq(vif);
            for m in members {
                let src = format!("{inst_path}.{m}");
                let dst = format!("{vif_fq}.{m}");
                if let Some(&net) = self.symbols.get(&src) {
                    self.symbols.insert(dst, net);
                    if let Some(c) = self.net_class.get(&net).cloned() {
                        self.net_class.insert(net, c);
                    }
                }
            }
        }
    }

    /// The signal member names of an interface declaration (for vif aliasing).
    pub(crate) fn iface_member_names(&self, iface: &str) -> Vec<String> {
        let Some(decl) = self.ifaces.get(iface) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for it in &decl.body {
            if let ast::ModuleItem::NetVar(nv) = it {
                for n in &nv.names {
                    out.push(n.name.name.clone());
                }
            }
        }
        out
    }

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
                // Same two drains as the module and generate flushes: this scope's
                // decl-time pre-sizes first, its block-local string inits last.
                self.drain_scoped_presize();
                for it in &decl.body {
                    if let ast::ModuleItem::NetVar(d) = it {
                        self.collect_var_init_drivers(d);
                    }
                }
                self.drain_scoped_bl_strings();
                self.flush_pending_var_inits();
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

    /// v5 ⑥ (D): bind an interface-typed module port by SYMBOL ALIASING —
    /// every net under the connected interface instance becomes visible as
    /// `<child>.<port>.<sig>` (net creation 0; canonical VCD naming is the
    /// lexicographically-smallest FQ, the established multi-FQ rule).
    pub(crate) fn bind_iface_port(
        &mut self,
        iref: &ast::IfaceRef,
        pname: &str,
        conn_expr: &ast::Expr,
        parent_prefix: &str,
    ) {
        let inst_name = match &conn_expr.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => p.segments[0].name.clone(),
            _ => {
                self.error(
                    MsgCode::ElabPortMismatch,
                    &format!(
                        "interface port `{pname}` must be connected to an interface instance name"
                    ),
                );
                return;
            }
        };
        // v6 ② (D): resolve the instance name with the SAME outward scope
        // walk nets use, so a child inside a generate block binds an iface
        // declared in the enclosing module body. The walk runs in the PARENT
        // scope (cur_prefix is the child's during port binding).
        let saved_for_lookup = std::mem::replace(&mut self.cur_prefix, parent_prefix.to_string());
        let found = self.walk_scopes_key(&inst_name, |k| self.iface_insts.contains_key(k));
        self.cur_prefix = saved_for_lookup;
        let Some(parent_fq) = found else {
            self.error(
                MsgCode::ElabPortMismatch,
                &format!(
                    "interface port `{pname}`: `{inst_name}` is not an interface instance in the parent scope"
                ),
            );
            return;
        };
        let actual = self.iface_insts[&parent_fq].clone();
        if actual != iref.iface.name {
            self.error(
                MsgCode::ElabPortMismatch,
                &format!(
                    "interface port `{pname}` is typed `{}` but `{inst_name}` is an instance of `{actual}`",
                    iref.iface.name
                ),
            );
            return;
        }
        // v6 ②: with a modport, only the LISTED members are visible through
        // the port (§25.5) and `input` members are read-only. Without one,
        // every member aliases with full access.
        let mp_dirs: Option<BTreeMap<String, ast::PortDir>> = match &iref.modport {
            Some(mp) => {
                let decl = self.ifaces.get(&actual).and_then(|d| {
                    d.body.iter().find_map(|it| match it {
                        ast::ModuleItem::Modport(m) if m.name.name == mp.name => Some(m.clone()),
                        _ => None,
                    })
                });
                let Some(m) = decl else {
                    self.error(
                        MsgCode::ElabPortMismatch,
                        &format!("interface `{actual}` has no modport `{}`", mp.name),
                    );
                    return;
                };
                Some(
                    m.ports
                        .iter()
                        .map(|(d, id)| (id.name.clone(), *d))
                        .collect(),
                )
            }
            None => None,
        };
        // Alias the visible symbols under the instance into the child port scope.
        let src_prefix = format!("{parent_fq}.");
        let dst_prefix = format!("{}.", self.fq(pname));
        let aliases: Vec<(String, u32, Option<ast::PortDir>)> = self
            .symbols
            .range(src_prefix.clone()..)
            .take_while(|(k, _)| k.starts_with(&src_prefix))
            .filter_map(|(k, &id)| {
                let suffix = &k[src_prefix.len()..];
                // The modport lists direct MEMBERS (single segment) — match on
                // the first path segment so any future nested suffix follows
                // its member's visibility.
                let member = suffix.split('.').next().unwrap_or(suffix);
                let dir = match &mp_dirs {
                    Some(dirs) => Some(*dirs.get(member)?), // unlisted → invisible
                    None => None,
                };
                Some((format!("{dst_prefix}{suffix}"), id, dir))
            })
            .collect();
        for (k, id, dir) in aliases {
            if matches!(dir, Some(ast::PortDir::Input)) {
                self.modport_readonly.insert(k.clone());
            }
            self.symbols.insert(k, id);
        }
    }

    /// v6 ②: error on a WRITE that resolves through a modport `input` alias.
    /// Called once per lvalue root; mirrors the symbol lookup exactly via
    /// [`Self::walk_scopes_key`], so name-level granularity is preserved (the
    /// same net stays writable through the parent or an `output` modport).
    pub(crate) fn check_modport_write(&mut self, path: &ast::HierPath) {
        if self.modport_readonly.is_empty() {
            return;
        }
        let joined = path
            .segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        let key = self.walk_scopes_key(&joined, |k| self.symbols.contains_key(k));
        if let Some(k) = key {
            if self.modport_readonly.contains(&k) {
                self.error(
                    MsgCode::ElabPortMismatch,
                    &format!("cannot write `{joined}` through a modport `input` (read-only)"),
                );
            }
        }
    }

    // ── port wiring (parent expr ↔ child port net) ─────────────────
    /// Emit one cont-assign per CONNECTED port. Called from inside the child
    /// instance, where `self.cur_prefix == child_path`; the connection expr must
    /// be lowered in the PARENT scope, so we temporarily swap the prefix back to
    /// `parent_prefix` around each connection lowering.
    ///
    /// Direction wiring (doc-04):
    ///  - INPUT  : child port net DRIVEN by the parent expr  → `child_port = parent_expr`
    ///  - OUTPUT : child net DRIVES the parent lvalue         → `parent_lval = child_port`
    ///  - INOUT  : approximated child→parent (one-directional) + warn
    ///
    /// Unconnected ports: an INPUT floats (z, the net's time-0 default, no
    /// assign); an OUTPUT/INOUT is allowed + warns. Ports are walked in HEADER
    /// declaration order, so the cont-assign sequence is deterministic regardless
    /// of connection source order.
    pub(crate) fn wire_ports(
        &mut self,
        module: &ast::ModuleDecl,
        binding: PortBinding<'_>,
        parent_prefix: &str,
    ) {
        let ports = port_list_dirs(module);
        // `.*` wildcard (IEEE §23.3.2.5): connect every port the explicit list
        // does not name to a same-named NET or VARIABLE in the instantiating
        // scope. A same-named NET is connected; a same-named constant
        // (parameter/enum-label/function) or no match at all is a LOUD error
        // (iverilog: "did not find a matching identifier") — never a silent
        // connect-to-constant or float. Built once before the loop so the loop
        // can borrow the synthesized expressions.
        let wildcard_conns: Vec<(String, ast::Expr)> = match &binding {
            PortBinding::Named(v, true) => {
                // Decide each port in the PARENT scope (where a connection actual
                // resolves). `self.symbols` holds nets/variables only, so a hit
                // means a real signal — a parameter/enum-label is absent here.
                let saved = std::mem::replace(&mut self.cur_prefix, parent_prefix.to_string());
                let mut synth = Vec::new();
                let mut missing = Vec::new();
                for (pname, _) in ports.iter() {
                    if v.iter().any(|c| &c.name.name == pname) {
                        continue; // explicitly named (incl. an open `.p()`)
                    }
                    if self
                        .walk_scopes_key(pname, |k| self.symbols.contains_key(k))
                        .is_some()
                    {
                        synth.push(pname.clone());
                    } else {
                        missing.push(pname.clone());
                    }
                }
                self.cur_prefix = saved;
                for pname in missing {
                    self.error(
                        MsgCode::ElabPortMismatch,
                        &format!("`.*` wildcard found no net or variable matching port `{pname}`"),
                    );
                }
                synth
                    .into_iter()
                    .map(|pname| {
                        let id = ast::Ident {
                            name: pname.clone(),
                            span: module.name.span,
                        };
                        let e = ast::Expr {
                            span: module.name.span,
                            kind: ast::ExprKind::Ident(ast::HierPath {
                                segments: vec![id],
                                span: module.name.span,
                            }),
                        };
                        (pname, e)
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        for (i, (pname, dir)) in ports.iter().enumerate() {
            // find the connection expr for this port (None ⇒ unconnected).
            let conn: Option<&ast::Expr> = match &binding {
                PortBinding::None => None,
                PortBinding::Positional(v) => v.get(i).and_then(|o| o.as_ref()),
                PortBinding::Named(v, _) => v
                    .iter()
                    .find(|c| &c.name.name == pname)
                    .and_then(|c| c.value.as_ref())
                    // not explicitly named → a `.*` wildcard same-name reference,
                    // if one was synthesized for this port.
                    .or_else(|| {
                        wildcard_conns
                            .iter()
                            .find(|(n, _)| n == pname)
                            .map(|(_, e)| e)
                    }),
            };
            // v5 ⑥ (D): interface-typed port → symbol aliasing, not wiring.
            if let Some(iref) = ansi_iface_ref(module, pname) {
                match conn {
                    Some(c) => self.bind_iface_port(iref, pname, c, parent_prefix),
                    None => self.error(
                        MsgCode::ElabPortMismatch,
                        &format!("interface port `{pname}` left unconnected"),
                    ),
                }
                continue;
            }
            let Some(conn_expr) = conn else {
                // unconnected port.
                match dir {
                    ir::PortDir::Output => {
                        self.warn(&format!("output port `{pname}` left unconnected"));
                    }
                    ir::PortDir::Inout => {
                        self.warn(&format!("inout port `{pname}` left unconnected"));
                    }
                    _ => {} // input floats silently (z = time-0 default)
                }
                continue;
            };

            // child port net id (current scope is the child).
            let child_id = {
                let key = self.fq(pname);
                *self.symbols.get(&key).unwrap_or(&POISON_NET)
            };
            let child_prefix = self.cur_prefix.clone();

            match dir {
                // INPUT: child_port = parent_expr  (rhs lowered in PARENT scope).
                ir::PortDir::Input | ir::PortDir::Inout => {
                    if matches!(dir, ir::PortDir::Inout) {
                        self.warn(&format!(
                            "inout port `{pname}` approximated as one-directional (parent→child)"
                        ));
                    }
                    self.cur_prefix = parent_prefix.to_string();
                    // §11.6: the actual is in the child port's width context, so a
                    // fill grows to it (non-fill ⇒ byte-identical via lower_expr).
                    let pw = self
                        .nets
                        .get(child_id as usize)
                        .map(|n| n.width)
                        .unwrap_or(32);
                    let rhs = self.lower_ctx_or_plain(conn_expr, pw);
                    self.cur_prefix = child_prefix;
                    let lhs = whole_net_lvalue(child_id);
                    self.cont_assigns.push(ir::ContAssign {
                        lhs,
                        rhs,
                        delay: None,
                    });
                }
                // OUTPUT: parent_lval = child_port  (lval lowered in PARENT scope).
                ir::PortDir::Output => {
                    self.cur_prefix = parent_prefix.to_string();
                    let lhs = match expr_to_lvalue(conn_expr) {
                        Some(lv) => self.lower_lvalue(&lv),
                        None => {
                            self.error(
                                MsgCode::ElabPortMismatch,
                                &format!(
                                    "output port `{pname}` connected to a non-lvalue expression"
                                ),
                            );
                            ir::Lvalue {
                                chunks: vec![whole_net_chunk(POISON_NET)],
                            }
                        }
                    };
                    self.cur_prefix = child_prefix;
                    let rhs = self.push_expr(ir::Expr::Signal {
                        net: child_id,
                        word: None,
                    });
                    self.cont_assigns.push(ir::ContAssign {
                        lhs,
                        rhs,
                        delay: None,
                    });
                }
                ir::PortDir::Internal => {
                    // a non-port net in the header list — module-decl bug.
                    self.error(MsgCode::ElabPortMismatch, "connection to a non-port net");
                }
            }
        }

        // Fix 2 (Finding M2): detect connections that match NO declared port.
        // Symmetric with bind_params' surplus-positional / unknown-named checks.
        match &binding {
            PortBinding::None => {}
            PortBinding::Positional(v) => {
                if v.len() > ports.len() {
                    self.error(
                        MsgCode::ElabPortMismatch,
                        &format!(
                            "instance of `{}` has {} positional connection(s) but the module declares {} port(s)",
                            module.name.name,
                            v.len(),
                            ports.len()
                        ),
                    );
                }
            }
            PortBinding::Named(v, _) => {
                for c in v.iter() {
                    if !ports.iter().any(|(pname, _)| pname == &c.name.name) {
                        self.error(
                            MsgCode::ElabPortMismatch,
                            &format!(
                                "connection `.{}(...)` names no port of module `{}`",
                                c.name.name, module.name.name
                            ),
                        );
                    }
                }
            }
        }
    }

    // ── PASS 1a: ANSI ports → nets ─────────────────────────────────
    pub(crate) fn elaborate_ports(&mut self, ports: &ast::PortList) {
        if let ast::PortList::Ansi(list) = ports {
            for p in list {
                if p.iface.is_some() {
                    // v5 ⑥ (D): an interface-typed port creates NO net — its
                    // members alias the connected instance's nets at binding.
                    continue;
                }
                let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Wire); // default net type
                let (mut width, mut msb, lsb, signed) =
                    self.range_to_dims(kind, p.range.as_ref(), p.signed);
                // A packed multi-dim port (`input [1:0][7:0] m`) is a flat vector.
                let packed_ext = self.packed_extents(p.range.as_ref(), &p.packed);
                if !p.packed.is_empty() {
                    width = packed_ext
                        .iter()
                        .fold(1u32, |a, &(_, w, _)| a.saturating_mul(w.max(1)));
                    msb = width.saturating_sub(1);
                }
                let dir = map_port_dir(p.dir);
                let init = default_init(kind, width);
                self.add_net(
                    &p.name.name,
                    ir::NetVar {
                        kind: map_net_kind_or_wire(kind),
                        width,
                        msb,
                        lsb,
                        signed,
                        array_len: 1,
                        dir,
                        init,
                    },
                );
                if !p.packed.is_empty() {
                    if let Some(&id) = self.symbols.get(&self.fq(&p.name.name)) {
                        self.packed_dims.insert(id, packed_ext);
                    }
                }
                // SYS-INTRO descriptor for the port (packed dims; ANSI ports carry
                // no unpacked dims). Without this a multi-dim packed port would fall
                // back to a single derived dim (silent-wrong $size/$dimensions).
                if let Some(&id) = self.symbols.get(&self.fq(&p.name.name)) {
                    self.record_dim_desc(id, kind, p.range.as_ref(), &p.packed, &[]);
                }
                // WAND/WOR: a port net declared `output wand`/`wor` (etc.) also
                // needs its resolution-kind sidecar — multi-driven INSIDE the
                // module (≥2 internal `assign`s) resolves wired-AND/OR, not wire.
                if matches!(kind, ast::NetVarKind::Wand | ast::NetVarKind::Wor) {
                    if let Some(&id) = self.symbols.get(&self.fq(&p.name.name)) {
                        if matches!(kind, ast::NetVarKind::Wand) {
                            self.wired_and_nets.insert(id);
                        } else {
                            self.wired_or_nets.insert(id);
                        }
                    }
                }
                if !net_kind_supported(kind) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "unsupported net/var kind on port (v1)",
                    );
                }
            }
        }
        // NonAnsi/None: dir comes from body PortDecls; v1 leaves ports Internal
        // unless ANSI. (Body PortDecl dir-merge is a small follow-up.)
    }

    /// Direction of a body-declared net: Input/Output/Inout if it appears in the
    /// port list, else Internal.
    pub(crate) fn dir_for_name(
        &mut self,
        name: &str,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
    ) -> ir::PortDir {
        match ports {
            ast::PortList::Ansi(list) => list
                .iter()
                .find(|p| p.name.name == name)
                .map(|p| map_port_dir(p.dir))
                .unwrap_or(ir::PortDir::Internal),
            ast::PortList::NonAnsi(names) => {
                if names.iter().any(|i| i.name == name) {
                    // Fix 4: merge the body PortDecl direction (`output reg y;`)
                    // just like `port_list_dirs` does — no more silent Input
                    // default for a non-ANSI `output`/`inout` port.
                    body.iter()
                        .find_map(|it| match it {
                            ast::ModuleItem::PortDecl(pd)
                                if pd.names.iter().any(|x| x.name == name) =>
                            {
                                Some(map_port_dir(pd.dir))
                            }
                            _ => None,
                        })
                        .unwrap_or(ir::PortDir::Input)
                } else {
                    ir::PortDir::Internal
                }
            }
            ast::PortList::None => ir::PortDir::Internal,
        }
    }

    /// If `base` is a direct single-segment net `Ident`, normalize the offset by its
    /// declared range; otherwise (a computed/concat base, range `[?:0]`) leave it raw.
    /// If `path` is a multi-segment KNOWN dotted symbol — an interface-member alias
    /// (`bi.data`, inserted at port-binding) — return its net. `None` for a
    /// hierarchical cross-instance ref (NOT in `lookup_net_scoped`; it defers via
    /// `hier_chain`) or a non-net dotted access (a class field). Confines the
    /// offset-normalization multi-seg arms below to interface members.
    pub(crate) fn iface_member_net(&self, path: &ast::HierPath) -> Option<u32> {
        if path.segments.len() < 2 {
            return None;
        }
        let joined = path
            .segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        self.lookup_net_scoped(&joined)
    }

    /// Peel a TOP-LEVEL named-sequence reference (a bare-ident `Boolean` or an
    /// `Instance`) to its declared body, recursing through a chain (`s1`→`s2`→body),
    /// so the materialize dispatch sees the real top-level shape (notably a top-level
    /// `within`). Returns an owned clone; non-name shapes (Delay/Repeat/literal
    /// Within/…) and unknown names are returned as-is (their nested named references
    /// are still resolved later by `expand_sequence`). Cycle-guarded: a recursive top
    /// sequence is loud and collapses to `1'b0`.
    pub(crate) fn resolve_named_top(&mut self, seq: &ast::Sequence) -> ast::Sequence {
        let name = match seq {
            ast::Sequence::Boolean(e) => match &e.kind {
                ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                    Some(p.segments[0].name.clone())
                }
                _ => None,
            },
            ast::Sequence::Instance { name, args, .. } if args.is_empty() => {
                Some(name.name.clone())
            }
            _ => None,
        };
        let Some(name) = name else {
            return seq.clone();
        };
        let Some(decl) = self.seq_table.get(&name).cloned() else {
            return seq.clone(); // a real net / unknown name — leave it for expand_sequence
        };
        // A bare top-level reference (`s |-> …`) passes ZERO actuals; a parameterized
        // sequence needs its formals bound. Mirror `inline_named_sequence`'s arity
        // error (review 2026-06-16: this path was peeling `decl.body` with the formals
        // left as net references — silent-wrong when a formal name shadowed a real
        // net). Every sibling path (expand_sequence Boolean/Call/Instance, property
        // collect) already arity-checks; close the hole here too.
        if !decl.formals.is_empty() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "named sequence `{}` expects {} formal argument(s), got 0",
                    name,
                    decl.formals.len()
                ),
            );
            return ast::Sequence::Boolean(sva_zero(decl.span));
        }
        if self.sva_inline_stack.iter().any(|n| n == &name) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("recursive sequence `{}` is illegal (IEEE 1800 §16.8)", name),
            );
            return ast::Sequence::Boolean(sva_zero(decl.span));
        }
        self.sva_inline_stack.push(name);
        let r = self.resolve_named_top(&decl.body);
        self.sva_inline_stack.pop();
        r
    }
}
