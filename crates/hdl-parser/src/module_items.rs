//! module-item dispatch — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// A body `parameter`/`localparam`/`specparam` COMMA-LIST: the FIRST name's item
    /// is returned, the rest queue in `pending_module_items` (IEEE §6.20.1: one type
    /// prefix shared by every name).
    pub(crate) fn parse_param_list_item(&mut self) -> Option<ModuleItem> {
        // §4.5.437: a body / unit-scope TYPE parameter group.
        if self.starts_type_param() {
            let (decls, guards) = self.parse_type_param_group(false);
            self.expect(TokenKind::Semi, "';'");
            let mut items: Vec<ModuleItem> = decls.into_iter().map(ModuleItem::Param).collect();
            items.extend(guards);
            let mut it = items.into_iter();
            let first = it.next();
            self.pending_module_items.extend(it);
            return first;
        }
        let pfx = self.parse_param_prefix();
        let mut first: Option<ModuleItem> = None;
        // A `None` is a parse error already recorded by finish_param_assignment.
        while let Some(pi) = self.finish_param_assignment(&pfx, true) {
            let mi = self.param_item_to_module_item(pi);
            match first {
                None => first = Some(mi),
                Some(_) => self.pending_module_items.push(mi),
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';'");
        first
    }

    /// `function … endfunction` as a module item. `function void` in module/package
    /// scope ⇒ task-equivalent: reuse the full task machinery (statement call, output
    /// formals, control flow).
    pub(crate) fn parse_function_item(&mut self) -> ModuleItem {
        let (fd, is_void) = self.parse_function_def();
        if is_void {
            return ModuleItem::Task(TaskDef {
                automatic: fd.automatic,
                name: fd.name,
                ports: fd.ports,
                body_decls: fd.body_decls,
                body_enums: fd.body_enums,
                body: fd.body,
                span: fd.span,
                is_void_fn: true,
            });
        }
        ModuleItem::Func(fd)
    }

    /// §4.5.434: replicate the compilation-unit-scope declarations parsed so far
    /// (`cu_items`) into the front of this module's body — every one whose name the
    /// module does not declare itself (a header parameter, a body typedef/parameter/
    /// function/task/net/genvar of the same name shadows the unit's, §3.12.1). The
    /// unit-scope names were registered in the parser's scope as they were parsed, so
    /// the body already resolved them; this hands elaborate the declarations to bind.
    fn inject_cu_items(&self, m: &mut ModuleDecl) {
        self.inject_cu_items_filtered(m, false);
    }

    /// `inject_cu_items`, restricted to `Param` items when `consts_only`.
    fn inject_cu_items_filtered(&self, m: &mut ModuleDecl, consts_only: bool) {
        if self.cu_items.is_empty() {
            return;
        }
        let mut local: std::collections::BTreeSet<&str> =
            m.params.iter().map(|p| p.name.name.as_str()).collect();
        // Review B A-1: a PORT is a declaration of the module too (ANSI or non-ANSI).
        match &m.ports {
            PortList::Ansi(ps) => local.extend(ps.iter().map(|p| p.name.name.as_str())),
            PortList::NonAnsi(ns) => local.extend(ns.iter().map(|n| n.name.as_str())),
            PortList::None => {}
        }
        for it in &m.body {
            match it {
                ModuleItem::PortDecl(pd) => {
                    for n in &pd.names {
                        local.insert(n.name.as_str());
                    }
                }
                // Review B A-2: an imported name is nearer than the unit scope (§26.3) —
                // an explicit import names it, a wildcard brings the package's exports.
                ModuleItem::Import(imp) => match &imp.item {
                    Some(n) => {
                        local.insert(n.name.as_str());
                    }
                    None => {
                        if let Some(ex) = self.pkg_exports.get(&imp.pkg.name) {
                            local.extend(ex.iter().map(|s| s.as_str()));
                        }
                    }
                },
                ModuleItem::Typedef(td) => {
                    local.insert(td.name.name.as_str());
                }
                ModuleItem::Param(p) => {
                    local.insert(p.name.name.as_str());
                }
                ModuleItem::Func(f) => {
                    local.insert(f.name.name.as_str());
                }
                ModuleItem::Task(t) => {
                    local.insert(t.name.name.as_str());
                }
                ModuleItem::NetVar(d) => {
                    for n in &d.names {
                        local.insert(n.name.name.as_str());
                    }
                }
                ModuleItem::Genvar { names, .. } => {
                    for n in names {
                        local.insert(n.name.as_str());
                    }
                }
                // Review A A-3: an instance name is a declaration too (both oracles
                // refuse a read of it; a unit constant must not answer instead).
                ModuleItem::Instance(mi) => {
                    for it in &mi.instances {
                        local.insert(it.name.name.as_str());
                    }
                }
                _ => {}
            }
        }
        // Review A A-1: a unit `typedef enum`'s LABELS are declarations the module's
        // own net / variable / constant / port of that name shadows (§3.12.1); the
        // whole enum item stays out of that module (its other labels are loud there,
        // never wrong).
        let enum_shadowed = |td: &TypedefDecl| match &td.kind {
            TypedefKind::Enum { labels, .. } => {
                labels.iter().any(|l| local.contains(l.name.name.as_str()))
            }
            _ => false,
        };
        let mut pre: Vec<ModuleItem> = self
            .cu_items
            .iter()
            .filter(|it| {
                if consts_only && !matches!(it, ModuleItem::Param(_)) {
                    return false;
                }
                let n = match it {
                    ModuleItem::Typedef(td) if enum_shadowed(td) => return false,
                    ModuleItem::Typedef(td) => td.name.name.as_str(),
                    ModuleItem::Param(p) => p.name.name.as_str(),
                    ModuleItem::Func(f) => f.name.name.as_str(),
                    ModuleItem::Task(t) => t.name.name.as_str(),
                    _ => return false,
                };
                !local.contains(n)
            })
            .cloned()
            .collect();
        if pre.is_empty() {
            return;
        }
        pre.append(&mut m.body);
        m.body = pre;
    }

    pub fn parse_source_unit(&mut self) -> SourceUnit {
        // N7: pre-scan the token stream for every `class NAME` (any nesting) so a
        // class-typed declaration `Packet p;` parses through the ordinary
        // typed-decl path even when the variable precedes the class decl
        // (forward reference) — registered as a `NetVarKind::Class` type alias.
        self.prescan_class_names();
        let start = self.cur_span();
        let mut items = Vec::new();
        while !self.at_eof() {
            let before = self.pos;
            if self.at_kw(Kw::Module) || self.at_kw(Kw::Macromodule) {
                // A top-level unit's BARE type names are unit-scoped (IEEE §3.12.1):
                // snapshot before, restore after (keeping any `pkg::` twins) so a
                // module-local / package typedef does not leak into the next unit.
                let snap = self.snapshot_scope();
                match self.parse_module() {
                    Some(mut m) => {
                        self.inject_cu_items(&mut m);
                        items.push(TopItem::Module(m));
                    }
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
                self.restore_scope_unit(snap);
            } else if self.at_kw(Kw::Interface) {
                // v5 ⑥: `interface … endinterface` — same shape as a module.
                let snap = self.snapshot_scope();
                match self.parse_module_like(Kw::Interface, Kw::Endinterface) {
                    Some(mut m) => {
                        // An interface body binds unit-scope CONSTANTS only: its
                        // elaboration refuses typedef/function/task items, and a
                        // unit-scope type is already resolved by the parser scope.
                        self.inject_cu_items_filtered(&mut m, true);
                        items.push(TopItem::Interface(m));
                    }
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
                self.restore_scope_unit(snap);
            } else if self.at_kw(Kw::Program) {
                // ⓑ-breadth (§24): `program … endprogram` parses into the module
                // AST and elaborates as a top-level module container. The §24
                // Reactive-region scheduling of program processes is approximated
                // as Active (documented limitation). Pure parser addition (IR-0).
                let snap = self.snapshot_scope();
                match self.parse_module_like(Kw::Program, Kw::Endprogram) {
                    Some(m) => items.push(TopItem::Module(m)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
                self.restore_scope_unit(snap);
            } else if self.at_kw(Kw::Package) {
                // v7 P2-D: `package … endpackage` — body shape reuses modules.
                // The package body's scoped `pkg::t` twins survive the restore
                // (kept by `restore_scope_unit`); only its bare names are dropped.
                let snap = self.snapshot_scope();
                match self.parse_module_like(Kw::Package, Kw::Endpackage) {
                    Some(m) => items.push(TopItem::Package(m)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
                self.restore_scope_unit(snap);
            } else if self.at_kw(Kw::Import) {
                // v7 P2-D: compilation-unit-scope import.
                match self.parse_import_decl_list() {
                    Some(list) => items.extend(list.into_iter().map(TopItem::Import)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_kw(Kw::Class) {
                // N7: top-level `class … endclass`.
                match self.parse_class_decl() {
                    Some(c) => items.push(TopItem::Class(c)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_kw(Kw::Primitive) {
                // YELLOW #1: combinational User-Defined Primitive (IEEE 1364 §29).
                // DESUGARED in the parser (mirroring `parse_gate_primitive`) into a
                // synthetic ordinary `ModuleDecl` — so it auto-registers in the module
                // map, root-picks, and instantiates with ZERO downstream change. No new
                // AST node / no `.vu` schema-hash flip / IR-0.
                match self.parse_udp_decl() {
                    Some(m) => items.push(TopItem::Module(m)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_ident_kw("bind") {
                // Round-9: top-level `bind <target> <checker> <inst> (…);`. `bind`
                // is a CONTEXTUAL keyword (the lexer has no `Kw::Bind`, so it lexes
                // as an `Ident`); at source-unit position a bare ident can only be
                // an error today, so catching `bind` here is purely additive.
                match self.parse_bind_decl() {
                    Some(b) => items.push(TopItem::Bind(b)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_kw(Kw::Typedef) {
                // §4.5.434: a compilation-unit-scope typedef (IEEE §3.12.1). The type
                // registers under the unit scope here; the item rides into every later
                // module/interface body (`inject_cu_items`).
                if let Some(it) = self.parse_typedef() {
                    if let ModuleItem::Typedef(td) = &it {
                        self.cu_type_names.insert(td.name.name.clone());
                    }
                    self.cu_items.push(it);
                }
            } else if self.at_kw(Kw::Parameter) || self.at_kw(Kw::Localparam) {
                // §4.5.434: a unit-scope constant. `parameter` here is a localparam
                // (§6.20.1: not overridable outside a module header).
                self.type_params = self.cu_type_params.clone();
                // §6.20.1: a unit-scope `parameter` is a localparam — the arm below
                // rewrites `kind`, but that is AFTER the item is parsed, so every
                // predicate the parse itself asks (`parse_type_param_group`'s
                // `overridable`, `param_item_to_module_item`'s `non_overridable`)
                // saw an overridable `parameter`. `in_package` is the flag that
                // already carries this clause (lib.rs `has_param_header` doc), so
                // the unit scope raises it for the parse and restores it after.
                // Measured: without it a unit-scope `parameter type PT = logic [8:1]`
                // registered `[PT$w-1:0]` and read `$low` 0 / `$high` 7 where both
                // oracles read 1 / 8, while the same declaration after a package was
                // right only because `in_package` used to leak past `endpackage`.
                let was_in_package = self.in_package;
                self.in_package = true;
                let first_item = self.parse_param_list_item();
                self.in_package = was_in_package;
                if let Some(first) = first_item {
                    let mut list = vec![first];
                    list.append(&mut self.pending_module_items);
                    for mut it in list {
                        if let ModuleItem::Param(p) = &mut it {
                            p.kind = ParamKind::Localparam;
                        }
                        // §4.5.437: a unit-scope TYPE parameter is a localparam in
                        // every module, so its override shape guard has nothing to
                        // guard — it is not replicated.
                        if matches!(it, ModuleItem::Proc(_)) {
                            continue;
                        }
                        self.cu_items.push(it);
                    }
                }
                // §4.5.434 / §26.3: a unit-scope TYPE parameter declares a bare type
                // NAME exactly as a unit-scope `typedef` does, so a wildcard import
                // of a package that declares the same name replaces it — measured on
                // a unit `typedef` twin in both oracles. Without the record the
                // import's `or_insert` left the unit's type in place.
                for n in self.type_params.keys() {
                    self.cu_type_names.insert(n.clone());
                }
                self.cu_type_params = self.type_params.clone();
            } else if self.at_kw(Kw::Function) {
                // §4.5.434: a unit-scope function (`function void` = task-equivalent,
                // the same desugar as the module-body arm).
                let it = self.parse_function_item();
                self.cu_items.push(it);
            } else if self.at_kw(Kw::Task) {
                let t = self.parse_task_def();
                self.cu_items.push(ModuleItem::Task(t));
            } else {
                self.error("'module'");
                let s = self.cur_span();
                items.push(TopItem::Error(s));
                self.synchronize();
            }
            // BLOCKER B3 (top level): guarantee forward progress.
            if self.pos == before {
                self.bump();
            }
        }
        // ⓑ-breadth (§8.25): expand parameterized classes into concrete
        // specializations (no-op when no class is parameterized).
        self.monomorphize_param_classes(&mut items);
        items.extend(
            std::mem::take(&mut self.pending_mono_specs)
                .into_iter()
                .map(TopItem::Class),
        );
        // §23.11 body binds hoisted out of module/interface bodies. Order among binds
        // is preserved; position relative to the modules is irrelevant because
        // elaborate prescans the whole unit for `TopItem::Bind` before any module
        // lowers (`elaborate/driver.rs`), exactly as it does for a unit-scope bind.
        items.extend(
            std::mem::take(&mut self.pending_binds)
                .into_iter()
                .map(TopItem::Bind),
        );
        SourceUnit {
            items,
            span: start.to(self.prev_span()),
        }
    }

    pub(crate) fn parse_module(&mut self) -> Option<ModuleDecl> {
        self.parse_module_like(Kw::Module, Kw::Endmodule)
    }

    /// One body shared by `module…endmodule` and `interface…endinterface`
    /// (v5 ⑥): the header/body grammar is identical for the MVP subset.
    pub(crate) fn parse_module_like(&mut self, start_kw: Kw, end_kw: Kw) -> Option<ModuleDecl> {
        let start = self.cur_span();
        // Packages never nest, and every module-like resets this, so a body that
        // fails to parse cannot leak `true` into the next module.
        self.in_package = start_kw == Kw::Package;
        self.in_program = start_kw == Kw::Program;
        // §3.b: a container owns its own type-parameter names (the `endpackage`
        // twin pass reads this set), so a previous package's cannot reach it.
        self.pkg_type_param_names.clear();
        // Variable→struct bindings are module-scoped (type *names* are not).
        self.var_struct.clear();
        self.var_unpacked_struct.clear();
        self.record_array_vars.clear();
        self.record_soa_vars.clear();
        self.struct_scalar_vars.clear();
        self.struct_1d_array_vars.clear();
        self.struct_packed_array_vars.clear();
        self.var_enum.clear();
        self.packed_md_params.clear();
        self.wildcard_bound.clear();
        self.local_decl_names.clear();
        self.scope_type_names.clear();
        self.pattern_used.clear();
        self.pattern_rebound.clear();
        self.const_locals.clear();
        self.var_class.clear();
        self.container_seq += 1;
        self.cur_container = Some(self.container_seq);
        self.overridable_params.clear();
        self.shape_carriers.clear();
        self.shape_uncarried.clear();
        self.shape_alias.clear();
        // §4.5.437: the unit scope's type parameters are visible to every module.
        self.type_params = self.cu_type_params.clone();
        self.has_param_header = false;
        let is_macromodule = self.at_kw(Kw::Macromodule);
        self.bump(); // module / macromodule / interface
        let name = self.ident()?;

        // ANSI module-header package imports (IEEE §A.1.2 / §26.4): zero or more
        // `import pkg::item;` between the module name and the parameter/port list.
        // They exist so the imported symbols are visible to the port list that
        // follows (`module m import p::*; (input logic [W-1:0] a)`). Collected as
        // `ModuleItem::Import` at the FRONT of the body — elaborate's import pass
        // already scans body imports (and applies them before resolving port
        // widths), so a header import and a body import register identically.
        let mut header_imports = Vec::new();
        while self.at_kw(Kw::Import) {
            match self.parse_import_decl_list() {
                Some(list) => header_imports.extend(list.into_iter().map(ModuleItem::Import)),
                None => break, // diagnostic already emitted; stop the header loop
            }
        }

        // ANSI param port list: #( parameter … )
        let mut params = Vec::new();
        if self.peek() == Some(TokenKind::Hash) {
            self.bump();
            self.expect(TokenKind::LParen, "'(' after '#'");
            let mut last_pfx: Option<ParamPrefix> = None;
            loop {
                // §4.5.437: a TYPE parameter group — `[parameter] type T = <type>`
                // desugars to two value parameters (`T$w`, `T$s`) and a parser
                // typedef; the shape guard leads the body.
                if self.starts_type_param() {
                    let (decls, guards) = self.parse_type_param_group(true);
                    params.extend(decls);
                    header_imports.extend(guards);
                    last_pfx = None;
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                    continue;
                }
                // A type prefix (`parameter [T]`) is parsed once per GROUP; an
                // unadorned continuation (`, B = 2`) inherits the PRECEDING group's
                // type/width/signedness (IEEE §6.20.1) rather than silently
                // re-defaulting to a value-sized implicit param. A comma followed by
                // a fresh prefix keyword (`, parameter …`) starts a new group.
                let pfx = if last_pfx.is_none() || self.starts_param_prefix() {
                    let p = self.parse_param_prefix();
                    last_pfx = Some(p.clone());
                    p
                } else {
                    last_pfx.clone().unwrap()
                };
                match self.finish_param_assignment(&pfx, false) {
                    Some(ParamItem::Scalar(p)) => {
                        // Overridable as soon as it is declared, so a LATER header
                        // parameter's default may name it symbolically (§4.5.437:
                        // `parameter W = 8, parameter type T = logic [W-1:0]`).
                        if p.kind == ParamKind::Parameter {
                            self.overridable_params.insert(p.name.name.clone());
                        }
                        params.push(p)
                    }
                    // §3 ⑤ ⓒ: a header ARRAY parameter. The desugared const
                    // array decl leads the body (after the header imports, whose
                    // symbols its dims/default may name) and a scalar TWIN with the
                    // same name and span holds its override slot in `params`.
                    Some(ParamItem::ConstArrayVar(d)) => {
                        if start_kw != Kw::Module {
                            self.error_at(
                                d.span,
                                "a scalar parameter in this header (an array parameter is supported only in a module header in v1)",
                            );
                        } else if let Some(dn) = d.names.first() {
                            params.push(ParamDecl {
                                kind: pfx.kind,
                                signed: d.signed,
                                ty: ParamType::Implicit,
                                range: d.range.clone(),
                                name: dn.name.clone(),
                                value: dn.init.clone().unwrap_or_else(|| Expr {
                                    kind: ExprKind::AssignPattern(Vec::new()),
                                    span: d.span,
                                }),
                                span: d.span,
                            });
                            header_imports.push(ModuleItem::NetVar(d));
                        }
                    }
                    None => {}
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')'");
        }
        // §3 ⑤ ⓓ / IEEE §6.20.1: with an ANSI parameter header, a body
        // `parameter` is a localparam. The twin of a header ARRAY parameter counts
        // (it is in `params`), exactly as elaborate's `param_ports` counts it.
        self.has_param_header = !params.is_empty();
        // §3 ⑤ ⓒ: the header's `parameter`s are the overridable ones.
        for p in &params {
            if p.kind == ParamKind::Parameter {
                self.overridable_params.insert(p.name.name.clone());
            }
        }

        // port list: ANSI ( dir type name, … ) | non-ANSI ( name, … ) | none
        let mut ports = self.parse_port_list();
        // Port names are local declarations too (a body `import p::*` must not
        // bind a package struct over a port of the same name).
        match &ports {
            PortList::Ansi(ps) => {
                for pt in ps {
                    self.local_decl_names.insert(pt.name.name.clone());
                    // §3 ⑤ ⓐ: a port named like a header packed-md parameter of an
                    // OUTER scope / an imported one shadows it (review A-1).
                    self.packed_md_params.remove(&pt.name.name);
                    self.const_locals.remove(&pt.name.name);
                }
            }
            PortList::NonAnsi(ids) => {
                for id in ids {
                    self.local_decl_names.insert(id.name.clone());
                    self.packed_md_params.remove(&id.name);
                    self.const_locals.remove(&id.name);
                }
            }
            PortList::None => {}
        }
        self.expect(TokenKind::Semi, "';' after module header");

        // For the post-package scoped-twin pass below: snapshot the aggregate type
        // sub-maps BEFORE the body so a chained-alias-of-aggregate (`typedef pk::s a;`
        // — an `Alias` node that nonetheless writes a layout) can be told apart from a
        // plain vector alias that merely LEAVES a stale same-name entry from another
        // package. Only a package needs this; a module never runs the twin pass.
        let names_before = if end_kw == Kw::Endpackage {
            self.type_names.clone()
        } else {
            Default::default()
        };

        // body until the end keyword — with forward-progress guard (BLOCKER B3).
        // Header imports lead the body so elaborate registers them first.
        let mut body = header_imports;
        // The `pending` disjunct keeps the loop alive to drain a body-param
        // comma-list continuation even when it is the LAST item before `end_kw`
        // (the first name already advanced the cursor onto `end_kw`).
        while !self.at_eof() && (!self.pending_module_items.is_empty() || !self.at_kw(end_kw)) {
            // Emit queued body-param comma-list continuations (already parsed, same
            // scope) FIRST — before the forward-progress guard below, which would
            // else `bump` (a drained item advances no cursor).
            if !self.pending_module_items.is_empty() {
                body.push(self.pending_module_items.remove(0));
                continue;
            }
            let before = self.pos;
            // G6B: module/interface/program-scope scalar UNPACKED-struct decl
            // (`[pkg::]T k;`) → N member NetVars, mirroring block_body's branch. The
            // typedef branch of `parse_module_item` resolves only `typedefs`, which
            // never holds unpacked structs — so without this they fall to module-
            // instantiation parsing and choke. Cold helper keeps the loop frame small.
            if self.try_module_unpacked_struct_decl(&mut body) {
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            // §23.11 `bind <target-module> <checker> u (…);` written INSIDE this body.
            // Handled HERE rather than in `parse_module_item` because a bind produces
            // no module item at all: it is hoisted whole to the source unit (see
            // `pending_binds`), and this loop — unlike `parse_module_item`, which must
            // return one item — can simply continue. A bind is the last item before
            // `endmodule` often enough that the difference matters.
            if self.at_bind_directive() {
                match self.parse_bind_decl() {
                    Some(b) => self.pending_binds.push(b),
                    None => {
                        body.push(ModuleItem::Error(self.cur_span()));
                        self.synchronize();
                    }
                }
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            match self.parse_module_item() {
                Some(it) => body.push(it),
                None => {
                    body.push(ModuleItem::Error(self.cur_span()));
                    self.synchronize();
                }
            }
            if self.pos == before {
                self.bump();
            } // B3: never spin on a stuck token
        }
        // §3 ⑤ⓕ: every use of each type parameter is now parsed, so a `T` whose uses
        // ALL reached a container carrying `T$s` gets its shape guard narrowed to the
        // unpacked-dimension COUNT (the sign and the 2-state kind follow the override
        // per instance) and re-worded. A `T` with an uncarried use keeps the strict one.
        self.narrow_shape_guards(&mut body);
        // Every queued body-param comma-list continuation must have drained into
        // `body` above; the loop condition keeps it alive for that. The sole
        // exception is a truncated source (EOF before `end_kw`) — where the queued
        // names are dropped alongside the already-emitted missing-`end` error, and
        // never leak into a sibling container. This guard future-proofs that
        // invariant (a new un-drained `parse_module_item` caller / an early loop
        // break would trip it).
        debug_assert!(
            self.at_eof() || self.pending_module_items.is_empty(),
            "body-param continuations left un-drained at a container end"
        );
        self.expect(
            TokenKind::Word(WordKind::Keyword(end_kw)),
            if end_kw == Kw::Endinterface {
                "'endinterface'"
            } else {
                "'endmodule'"
            },
        );
        // Optional `: name` end-label (IEEE 1800 §9.3.4/§26/§27) after any
        // container end — `endmodule : m`, `endpackage : p`, `endinterface : i`,
        // `endprogram : p`. Accept-and-ignore, matching the established policy for
        // endfunction/endtask/endclass/block/generate ends (a mismatched label is
        // not silent-wrong: the container name is already fixed above).
        self.opt_block_label();
        // IEEE §26.3: a package's typedefs are referable elsewhere scope-qualified
        // (`pkg::t`). Register a `"pkg::name"` twin of each package-scope typedef so a
        // scoped type resolves to THIS package's definition — collision-safe against a
        // same-named typedef in another package (the flat bare-keyed registry keeps
        // only the last-registered, so a bare lookup would be silent-wrong). The pass
        // runs at each `endpackage`, before any later package overwrites the bare key.
        //
        // The `typedefs` twin is unconditional: every typedef writes `typedefs[n]`, so
        // it is THIS package's value. The AGGREGATE sub-maps are copied per the node's
        // KIND so a plain vector alias `pb::t` does NOT inherit a stale `struct_layouts`
        // / `enum_defs` entry a same-named struct/enum in ANOTHER package left behind
        // (that mis-bind would be silent-wrong): a `Struct`/`Enum` node owns its map;
        // an `Alias` node copies an aggregate twin ONLY when this body actually (re)wrote
        // it (a chained-alias-of-aggregate `typedef pk::s a;` — vs a plain alias that
        // left the same-name entry untouched). The bare entry is left as-is (a package
        // typedef is also visible bare in vita's flat model — pre-existing over-leniency).
        if end_kw == Kw::Endpackage {
            let pkg = name.name.clone();
            // §3.b: the type parameters THIS body declared (`parameter type PT`).
            // IEEE §26.3 makes them referable `pkg::PT` exactly like a typedef, and
            // §6.20.1 makes a package one non-overridable — so the twin below is the
            // typedef twin, unchanged.
            let tp_stems = self.pkg_type_param_stems(&body);
            // §4.5.434: what this package exports by name (`inject_cu_items` shadow set).
            let mut exported: std::collections::BTreeSet<String> = body
                .iter()
                .flat_map(|it| match it {
                    ModuleItem::Typedef(td) => {
                        let mut v = vec![td.name.name.clone()];
                        if let TypedefKind::Enum { labels, .. } = &td.kind {
                            v.extend(labels.iter().map(|l| l.name.name.clone()));
                        }
                        v
                    }
                    ModuleItem::Param(p) => vec![p.name.name.clone()],
                    ModuleItem::Func(f) => vec![f.name.name.clone()],
                    ModuleItem::Task(t) => vec![t.name.name.clone()],
                    ModuleItem::NetVar(d) => d.names.iter().map(|n| n.name.name.clone()).collect(),
                    _ => Vec::new(),
                })
                .collect();
            // The `Param` arm exports the `PT$w` / `PT$s` carriers; `PT` itself is
            // a NAME no item carries, so it joins the shadow set here — a wildcard
            // import must shadow a unit item of that name, as a typedef's does.
            exported.extend(tp_stems.iter().cloned());
            self.pkg_exports.insert(pkg.clone(), exported);
            for it in &body {
                if let ModuleItem::Typedef(td) = it {
                    let n = td.name.name.clone();
                    if n.contains("::") {
                        continue; // already-scoped (defensive; package typedefs are bare)
                    }
                    // §4.5.415 (§2 🆕 L ⓟ): the twin's dims name the package's OWN
                    // constants as `pkg::W` — the bare `W` is undefined wherever the
                    // twin is used without importing it (`p::t v;` was E3009 on its
                    // range, and a header `parameter p::t X` silently went
                    // value-inferred). `register_pkg_typedef_twin` owns that respell
                    // for all three dimension containers, shared with the §3.b
                    // type-parameter pass below.
                    let scoped = self.register_pkg_typedef_twin(&pkg, &n);
                    // Was `n`'s struct/enum layout (re)written by THIS package body?
                    // `Struct`/`Enum` nodes own their slot unconditionally; an `Alias`
                    // is fresh only if its aggregate entry differs from the pre-body
                    // snapshot (chained-alias-of-aggregate), never when it is a stale
                    // cross-package leftover. The comparison is of the layouts and
                    // labels themselves (`StructLayout` / label equality), not of their
                    // keys: two packages' equal layouts are not fresh.
                    let packed_before = names_before
                        .get(&n)
                        .and_then(|s| s.packed.as_ref())
                        .and_then(|p| self.packed_of(p));
                    let struct_fresh = match td.kind {
                        TypedefKind::Struct { .. } => self.packed_layout(&n).is_some(),
                        TypedefKind::Alias { .. } => {
                            self.packed_layout(&n) != packed_before
                                && self.packed_layout(&n).is_some()
                        }
                        TypedefKind::Enum { .. } => false,
                    };
                    let labels_before = names_before
                        .get(&n)
                        .and_then(|s| s.enm)
                        .and_then(|k| self.decl_labels(k));
                    let enum_fresh = match td.kind {
                        // Unlike a struct (whose layout is ALWAYS written), an `Enum`
                        // node writes the enum slot only when its labels are literal-
                        // foldable; a non-foldable enum (`{X = SOME_PARAM}`) leaves a
                        // same-name entry from another package intact — so, exactly like
                        // an `Alias`, it is fresh only when THIS body changed the entry
                        // (else the enum methods on a scoped var stay honest-loud rather
                        // than silently binding the wrong package's labels).
                        TypedefKind::Enum { .. } | TypedefKind::Alias { .. } => {
                            self.enum_def(&n).map(|e| e.labels) != labels_before
                                && self.enum_def(&n).is_some()
                        }
                        TypedefKind::Struct { .. } => false,
                    };
                    if struct_fresh {
                        // §3 ⑤ ⓓ: a nested member's bare key is this package's
                        // own type (declared earlier in the body, so its twin is
                        // already registered) — re-spell it `pkg::t` so the twin
                        // an importer copies still chains (`stable_type_key`).
                        // §5.2 row 67b: only the stored NAME is re-spelled; the
                        // member keeps its declaration key, which is what a chain
                        // or a nested `'{…}` reads.
                        if let Some(type_rec::Slot::Packed(mut p)) =
                            self.tn_slot(&n, type_rec::Kind::Packed)
                        {
                            if let Some(mut sl) = self.packed_of(&p).cloned() {
                                let mut respelled = false;
                                for f in &mut sl.fields {
                                    if let Some(k) = &mut f.8 {
                                        if !k.name.contains("::") {
                                            let sk = format!("{pkg}::{}", k.name);
                                            if self.packed_layout(&sk).is_some() {
                                                k.name = sk;
                                                respelled = true;
                                            }
                                        }
                                    }
                                }
                                if respelled {
                                    p.respelled = Some(Box::new(sl));
                                }
                            }
                            self.tn_set(&scoped, type_rec::Slot::Packed(p));
                        }
                        // A union is a struct-layout overlay; its flag rides with the
                        // (fresh) layout twin.
                        if let Some(u) = self.tn_slot(&n, type_rec::Kind::Union) {
                            self.tn_set(&scoped, u);
                        }
                    }
                    // Round-9: an UNPACKED struct has no packed slot, so it needs its
                    // own `pkg::T` record twin (slot presence is the gate — only an
                    // unpacked-struct typedef sets it). §5.2 row 36: the twin keeps the
                    // record's number (the package's own order of its registrations).
                    for k in [type_rec::Kind::Record, type_rec::Kind::RecordBind] {
                        if let Some(v) = self.tn_slot(&n, k) {
                            self.tn_set(&scoped, v);
                        }
                    }
                    if enum_fresh {
                        if let Some(e) = self.tn_slot(&n, type_rec::Kind::Enm) {
                            self.tn_set(&scoped, e);
                        }
                    }
                }
            }
            // §3.b: the same twin per type parameter, so `p::PT` names the type
            // everywhere a `p::t` typedef does (declaration, port, tf formal, cast,
            // `$bits`, struct member, queue / array element, `typedef p::PT`). No
            // aggregate sub-map work: a type parameter is always a vector type
            // (`parse_type_param_value` refuses a struct / enum / union / class
            // default), and it is registered with the DECLARED dims
            // (`type_param_literal_range`), so the twin folds outside the package.
            for n in &tp_stems {
                self.register_pkg_typedef_twin(&pkg, n);
            }
            // §3 ⑤: capture this package's struct/enum NAME bindings for `import`.
            // The type is re-spelled as the `pkg::t` twin registered just above when
            // one exists (collision-safe against a same-named type elsewhere). A type
            // this package itself IMPORTED has no twin yet: the bare `t` in scope
            // right now is the one the package used, so a `pkg::t` twin of THAT
            // layout is registered for the binding's sake (layout/label maps only —
            // not `typedefs`, so `pkg::t` is still not a usable type name: a package
            // does not re-export what it imports). A bare key would re-resolve in
            // the importer's scope against whatever same-named type it has
            // (measured: `V.a` cut as `V[15:4]` out of an 8-bit net, exit 0).
            let mut pb = PkgBindings::default();
            // §5.2 row 67b: the binding keeps its declaration key; only the stored
            // spelling becomes `pkg::t`.
            let mut vs: Vec<(String, TyRef)> = self
                .var_struct
                .iter()
                .map(|(n, t)| (n.clone(), t.clone()))
                .collect();
            vs.sort_by(|a, b| a.0.cmp(&b.0));
            for (n, ty) in vs {
                let name = self.pkg_binding_type_key(&pkg, &ty.name);
                pb.var_struct.push((n, TyRef { name, key: ty.key }));
            }
            let mut ss: Vec<&String> = self.struct_scalar_vars.iter().collect();
            ss.sort();
            pb.struct_scalar = ss.into_iter().cloned().collect();
            let mut sa: Vec<&String> = self.struct_1d_array_vars.iter().collect();
            sa.sort();
            pb.struct_1d_array = sa.into_iter().cloned().collect();
            let mut ve: Vec<(String, TyRef)> = self
                .var_enum
                .iter()
                .map(|(n, t)| (n.clone(), t.clone()))
                .collect();
            ve.sort_by(|a, b| a.0.cmp(&b.0));
            for (n, ty) in ve {
                let name = self.pkg_binding_type_key(&pkg, &ty.name);
                pb.var_enum.push((n, TyRef { name, key: ty.key }));
            }
            // §3 ⑤ ⓐ: the multi-dimensional packed parameters, for `import` (bare
            // name) and for a scoped `pkg::P[i]` read (unit-scoped twin). Sorted:
            // the bindings are replayed into a HashMap only, never iterated into
            // the AST, but a deterministic order costs nothing.
            let mut pm: Vec<(String, Vec<Range>)> = self
                .packed_md_params
                .iter()
                .map(|(n, d)| (n.clone(), self.respell_pkg_dims(&pkg, d)))
                .collect();
            pm.sort_by(|a, b| a.0.cmp(&b.0));
            for (n, d) in &pm {
                self.packed_md_scoped
                    .insert(format!("{pkg}::{n}"), d.clone());
            }
            pb.packed_md = pm;
            // §3 ⑤ ⓓ: the package's literal-valued constants (`parameter` and
            // `localparam` alike — §6.20.1), for `import` and for a scoped `pkg::W`
            // read. Only the package's OWN declarations (`local_decl_names`): a
            // constant it imported is not re-exported (IEEE §26.3). Sorted for a
            // deterministic replay order.
            let mut cs: Vec<(String, ConstVal)> = self
                .const_locals
                .iter()
                .filter(|(n, _)| self.local_decl_names.contains(*n))
                .map(|(n, v)| (n.clone(), *v))
                .collect();
            cs.sort_by(|a, b| a.0.cmp(&b.0));
            for (n, v) in &cs {
                self.pkg_const_scoped.insert(format!("{pkg}::{n}"), *v);
            }
            pb.consts = cs;
            self.pkg_bindings.insert(pkg, pb);
        }
        // Inject the synthetic `$enum_name$<T>` functions generated by any
        // `x.name()` desugar in this container's body, appended in deterministic
        // (BTreeMap key) order. Drained so the next container starts fresh — each
        // container gets its own copy (module-scoped functions, no collision).
        for (_, (_, f)) in std::mem::take(&mut self.pending_enum_name_fns) {
            body.push(ModuleItem::Func(f));
        }
        // §6.20.1: no container leaks `in_package` to what follows it. It used to
        // stay true from `endpackage` to the next container's start, which made a
        // unit-scope `parameter type` after a package accidentally right and the
        // same declaration before one wrong — the predicate was position-dependent.
        self.in_package = false;
        self.in_program = false;
        self.drop_rebound_pattern_members(&mut ports, &mut body);
        Some(ModuleDecl {
            is_macromodule,
            name,
            params,
            ports,
            body,
            span: start.to(self.prev_span()),
            // Overwritten by the driver from `resolve_module_nettype`; the parser cannot
            // see the stripped directive, so it writes the IEEE default (`wire`).
            nettype_none: false,
        })
    }

    /// v5 ⑥: `modport name (input a, b, output c);` — the direction is sticky
    /// across commas. Parsed + ACCEPTED (per-member direction checks are a
    /// follow-on); task/function modport members are outside the MVP.
    pub(crate) fn parse_modport(&mut self) -> Option<ModportDecl> {
        let start = self.cur_span();
        self.bump(); // modport
        let name = self.ident()?;
        self.expect(TokenKind::LParen, "'('");
        let mut ports = Vec::new();
        let mut dir: Option<PortDir> = None;
        loop {
            match self.peek() {
                Some(TokenKind::Word(WordKind::Keyword(Kw::Input))) => {
                    self.bump();
                    dir = Some(PortDir::Input);
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                    self.bump();
                    dir = Some(PortDir::Output);
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Inout))) => {
                    self.bump();
                    dir = Some(PortDir::Inout);
                }
                _ => {}
            }
            let Some(d) = dir else {
                self.error("a direction (input/output/inout) before the first modport member");
                break;
            };
            let Some(member) = self.ident() else {
                self.error("modport member name");
                break;
            };
            ports.push((d, member));
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "')'");
        self.expect(TokenKind::Semi, "';'");
        Some(ModportDecl {
            name,
            ports,
            span: start.to(self.prev_span()),
        })
    }
}
