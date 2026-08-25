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
    /// Connect one UNPACKED ARRAY port, one element per cont-assign.
    ///
    /// The actual must be a plain name bound to an array of the SAME length. A
    /// length mismatch, a non-array actual, or an expression that is not a bare name
    /// is loud: connecting the elements that happen to line up and leaving the rest
    /// floating would be a partial wiring with no diagnostic, and reading word 0 for
    /// the whole array (what a single whole-net cont-assign does) is worse still.
    fn wire_array_port(
        &mut self,
        child_id: u32,
        child_len: u32,
        dir: ir::PortDir,
        pname: &str,
        conn_expr: &ast::Expr,
        parent_prefix: &str,
    ) {
        let saved = std::mem::replace(&mut self.cur_prefix, parent_prefix.to_string());
        let actual = match &conn_expr.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.lookup_net_scoped(&p.segments[0].name)
            }
            _ => None,
        };
        let Some(actual_id) = actual else {
            self.cur_prefix = saved;
            self.error(
                MsgCode::ElabPortMismatch,
                &format!(
                    "unpacked array port `{pname}` must be connected to a declared \
                     array of the same length (an expression has no array value)"
                ),
            );
            return;
        };
        let actual_len = self
            .nets
            .get(actual_id as usize)
            .map(|n| n.array_len)
            .unwrap_or(1);
        let actual_w = self.nets.get(actual_id as usize).map(|n| n.width);
        let child_w = self.nets.get(child_id as usize).map(|n| n.width);
        let actual_dims = self.array_dims.get(&actual_id).cloned();
        let child_dims = self.array_dims.get(&child_id).cloned();
        let actual_desc = self.array_dim_desc.get(&actual_id).cloned();
        let child_desc = self.array_dim_desc.get(&child_id).cloned();
        self.cur_prefix = saved;
        // IEEE 1800 §7.6 makes an unpacked array connection correspond by POSITION,
        // not by index — for `[3:0]` the FIRST element is index 3. This wiring is
        // flat-index to flat-index, which is the same thing only when both sides run
        // the same way: measured, child `[0:3]` into parent `[3:0]` gave 4 where
        // iverilog gives 1, silently. Refuse rather than reverse, because reversing
        // is a second correspondence rule and this one has no test corpus yet.
        if actual_desc != child_desc {
            self.error(
                MsgCode::ElabPortMismatch,
                &format!(
                    "unpacked array port `{pname}` and the connected signal have \
                     opposite dimension directions (IEEE 1800 §7.6 pairs elements by \
                     POSITION, so `[0:3]` into `[3:0]` reverses them — declare both \
                     the same way)"
                ),
            );
            return;
        }
        // The GEOMETRY must match, not just the flattened element count: `o [4]`
        // connected to `b [2][2]` has eight elements on both sides and wired silently,
        // where iverilog says "Unpacked dimensions are not compatible".
        if actual_dims != child_dims {
            self.error(
                MsgCode::ElabPortMismatch,
                &format!(
                    "unpacked array port `{pname}` and the connected signal have \
                     different unpacked dimensions (same element count is not enough — \
                     the shapes must match)"
                ),
            );
            return;
        }
        // …and so must the ELEMENT width. The scalar port path truncates silently
        // here, but an array is a new shape and iverilog rejects it outright
        // ("Element types are not compatible in array assignment"), so this one is
        // loud rather than bug-compatible with the scalar path.
        if actual_w != child_w {
            self.error(
                MsgCode::ElabPortMismatch,
                &format!(
                    "unpacked array port `{pname}` has {}-bit elements but the connected \
                     signal has {}-bit elements",
                    child_w.unwrap_or(0),
                    actual_w.unwrap_or(0)
                ),
            );
            return;
        }
        if actual_len != child_len {
            self.error(
                MsgCode::ElabPortMismatch,
                &format!(
                    "unpacked array port `{pname}` has {child_len} elements but the \
                     connected signal has {actual_len}"
                ),
            );
            return;
        }
        // An `inout` ARRAY port takes the input direction, exactly as the scalar path
        // does — but the scalar path SAYS so (W3056 "approximated as one-directional")
        // and this one took its `continue` before that arm, so a known approximation
        // became silent on the new shape. iverilog refuses `inout` unpacked ports
        // outright, so the warning is already more than the oracle offers.
        if matches!(dir, ir::PortDir::Inout) {
            self.warn(&format!(
                "inout array port `{pname}` approximated as one-directional (parent→child)"
            ));
        }
        for w in 0..child_len {
            let (dst, src) = match dir {
                ir::PortDir::Output => (actual_id, child_id),
                _ => (child_id, actual_id),
            };
            let widx_r = self.const_u32_expr(w, 32);
            let widx_l = self.const_u32_expr(w, 32);
            let rhs = self.push_expr(ir::Expr::Signal {
                net: src,
                word: Some(widx_r),
            });
            self.cont_assigns.push(ir::ContAssign {
                lhs: ir::Lvalue {
                    chunks: vec![ir::LvalChunk {
                        net: dst,
                        word: Some(widx_l),
                        offset: None,
                        width: None,
                        kind: ir::SelKind::Bit,
                    }],
                },
                rhs,
                delay: None,
            });
        }
    }

    /// `is_root` — this instance is a TOP, nothing instantiated it. A top's ports are
    /// unconnected BY DEFINITION (that is what being a top means), so the dangling-output
    /// warning below is noise there, not a finding: neither iverilog (even `-Wall`) nor
    /// verilator says anything. ⚠️ The binding is NOT a usable proxy for this — a CHILD
    /// written `dut u();` reaches here with an empty binding too, and for that one the
    /// warning is real information. The caller knows which it is (`parent_inst.is_none()`).
    pub(crate) fn wire_ports(
        &mut self,
        module: &ast::ModuleDecl,
        binding: PortBinding<'_>,
        parent_prefix: &str,
        is_root: bool,
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
                // §3 ⑥: silent for a ROOT. Measured on serv: auto-top elaborates the
                // library modules `serv_rf_top` and `servile_rf_mem_if` as independent
                // roots — which is what IEEE 1364 and iverilog both do, so the ROOT
                // SELECTION is not the defect the queue line took it for — and every one
                // of their output ports then reported as "left unconnected", 19 warnings
                // that say nothing an author can act on. Pinning `--top tb` hid it only
                // because that testbench happens to have no ports; a single explicitly
                // pinned top WITH one warns just the same.
                if !is_root {
                    match dir {
                        ir::PortDir::Output => {
                            self.warn(&format!("output port `{pname}` left unconnected"));
                        }
                        ir::PortDir::Inout => {
                            self.warn(&format!("inout port `{pname}` left unconnected"));
                        }
                        // ⭐ The asymmetry ran OPPOSITE to the consequence. A dangling
                        // OUTPUT discards a value; a dangling INPUT *manufactures* one
                        // — `z` at time 0 — and propagates it through everything the
                        // child computes. The first was warned about and the second was
                        // silent, so the only unconnected port that can produce a wrong
                        // answer was the one nothing said anything about, and the
                        // author learned about it as a mismatching digest much later.
                        _ => {
                            self.warn(&format!(
                                "input port `{pname}` left unconnected — it floats at \
                                 `z`, and every value the instance derives from it is \
                                 unknown (tie it off explicitly)"
                            ));
                        }
                    }
                }
                continue;
            };

            // child port net id (current scope is the child).
            let child_id = {
                let key = self.fq(pname);
                *self.symbols.get(&key).unwrap_or(&POISON_NET)
            };
            let child_prefix = self.cur_prefix.clone();

            // An UNPACKED ARRAY port connects ELEMENT BY ELEMENT. There is no
            // whole-array value in this IR — a single `ContAssign` over the whole net
            // would read (or write) word 0 only — so one cont-assign per element is
            // what "connect the array" means here. Both sides must be arrays of the
            // same length; anything else is loud, never a partial wiring.
            let child_len = self
                .nets
                .get(child_id as usize)
                .map(|n| n.array_len)
                .unwrap_or(1);
            if child_len > 1 {
                self.wire_array_port(child_id, child_len, *dir, pname, conn_expr, parent_prefix);
                self.cur_prefix = child_prefix;
                continue;
            }
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
                                                                          // §7.4.2 / §4.5.359: a port declared with a negative bound
                                                                          // (`input logic [-3:0] p`) is sized `|msb-lsb|+1` like any other net.
                                                                          // The opt-in and `record_declared_bounds` below are one unit.
                let odd_bound = self.declared_odd_bound(p.range.as_ref()).is_some();
                let (mut width, mut msb, lsb, signed) =
                    self.range_to_dims_opt(kind, p.range.as_ref(), p.signed, odd_bound);
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
                // UNPACKED dims (`output logic [7:0] o [4]`, IEEE 1800 §23.2.2.3) —
                // same computation and same cap as `elaborate_netvar_decl`, because
                // a port array IS a net array; only the declaration site differs.
                let dim_extents = self.array_dim_extents(&p.unpacked);
                let array_len = dim_extents
                    .iter()
                    .fold(1u32, |acc, &(_, n)| acc.saturating_mul(n.max(1)));
                if (array_len as u64) > MAX_ARRAY_LEN {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "unpacked array port `{}` has {} elements (cap {MAX_ARRAY_LEN})",
                            p.name.name, array_len
                        ),
                    );
                    continue;
                }
                self.add_net(
                    &p.name.name,
                    ir::NetVar {
                        kind: map_net_kind_or_wire(kind),
                        width,
                        msb,
                        lsb,
                        signed,
                        array_len: array_len.max(1),
                        dir,
                        init,
                    },
                );
                self.record_declared_bounds(&p.name.name, p.range.as_ref());
                if !p.packed.is_empty() {
                    if let Some(&id) = self.symbols.get(&self.fq(&p.name.name)) {
                        self.packed_dims.insert(id, packed_ext);
                    }
                }
                // MULTI-DIM (or non-zero-based) unpacked geometry, exactly as
                // `elaborate_netvar_decl` registers it. `array_len` alone is the
                // flattened count; without the per-dim extents a two-index write
                // `o[i][0]` cannot compute its flat element and the port read all-X
                // while the same array declared as an ordinary net worked.
                if dim_extents.len() >= 2 || dim_extents.iter().any(|&(lo, _)| lo != 0) {
                    if let Some(&id) = self.symbols.get(&self.fq(&p.name.name)) {
                        self.array_dims.insert(id, dim_extents.clone());
                    }
                }
                // Declared per-dim DIRECTION, exactly as `elaborate_netvar_decl`
                // records it. A port array needs it for the same reason an ordinary
                // array does — and `wire_array_port` needs it to refuse a connection
                // whose two sides run in opposite directions.
                let pdesc: Vec<bool> = p
                    .unpacked
                    .iter()
                    .map(|d| match d {
                        ast::Dim::Range(r) => {
                            let msb = self.const_eval_in_scope(&r.msb);
                            let lsb = self.const_eval_in_scope(&r.lsb);
                            matches!((msb, lsb), (Some(m), Some(l)) if m > l)
                        }
                        _ => false,
                    })
                    .collect();
                if pdesc.iter().any(|&d| d) {
                    if let Some(&id) = self.symbols.get(&self.fq(&p.name.name)) {
                        self.array_dim_desc.insert(id, pdesc);
                    }
                }
                // SYS-INTRO descriptor for the port. Without this a multi-dim packed
                // port would fall back to a single derived dim (silent-wrong
                // $size/$dimensions) — and the UNPACKED half was `&[]` because ports
                // could not carry unpacked dims at all until they parsed.
                if let Some(&id) = self.symbols.get(&self.fq(&p.name.name)) {
                    self.record_dim_desc(id, kind, p.range.as_ref(), &p.packed, &p.unpacked);
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
