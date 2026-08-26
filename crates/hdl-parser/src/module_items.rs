//! module-item dispatch — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// v7 P2-D: `import pkg::*;` / `import pkg::sym;` — the whole statement,
    /// INCLUDING a comma list (IEEE 1800 §26.8 `package_import_declaration` is
    /// `import package_import_item { , package_import_item } ;`). Each term
    /// becomes its own `ImportDecl`; every consumer is per-decl and
    /// order-independent, so N terms behave exactly like N statements.
    pub(crate) fn parse_import_decl_list(&mut self) -> Option<Vec<ImportDecl>> {
        self.bump(); // import
        let mut out = Vec::new();
        loop {
            // A malformed term aborts the STATEMENT: the caller's recovery
            // synchronises on the next item, and continuing the comma loop from
            // an unknown cursor position would report the rest as garbage.
            out.push(self.parse_import_term()?);
            if self.peek() == Some(TokenKind::Comma) {
                self.bump();
                continue;
            }
            break;
        }
        if !self.expect(TokenKind::Semi, "';'") {
            return None;
        }
        Some(out)
    }

    /// One `pkg::sym` / `pkg::*` term, no `import` keyword and no `;`.
    fn parse_import_term(&mut self) -> Option<ImportDecl> {
        let start = self.cur_span();
        let pkg = self.ident()?;
        if !self.expect(TokenKind::ColonColon, "'::'") {
            return None;
        }
        let item = if self.peek() == Some(TokenKind::Star) {
            self.bump();
            None
        } else {
            Some(self.ident()?)
        };
        // Bring the package's TYPE names into the current bare scope at parse time
        // (type names are parse-resolved). A package body's bare typedefs are now
        // unit-scoped (`restore_scope_unit` drops them, keeping only the `pkg::t`
        // twins), so `import p::t;` / `import p::*;` must copy the scoped twin back
        // to its bare name for a later `t x;` to parse. Value/const imports are
        // still handled at elaborate from the returned `ImportDecl`.
        let prefix = format!("{}::", pkg.name);
        match &item {
            Some(name) => {
                let scoped = format!("{prefix}{}", name.name);
                let bare = name.name.clone();
                if let Some(v) = self.typedefs.get(&scoped).cloned() {
                    self.typedefs.insert(bare.clone(), v);
                }
                if let Some(v) = self.struct_layouts.get(&scoped).cloned() {
                    self.struct_layouts.insert(bare.clone(), v);
                }
                if let Some(v) = self.enum_defs.get(&scoped).cloned() {
                    self.enum_defs.insert(bare.clone(), v);
                }
                // G6: UNPACKED struct typedefs live only in `unpacked_struct_layouts`
                // (never in `typedefs`), so a bare `rec_t r;` after `import p::rec_t`
                // needs the scoped twin copied here too, exactly like the packed kinds.
                if let Some(v) = self.unpacked_struct_layouts.get(&scoped).cloned() {
                    self.unpacked_struct_layouts.insert(bare.clone(), v);
                }
                if self.union_type_names.contains(&scoped) {
                    self.union_type_names.insert(bare);
                }
            }
            None => {
                // Wildcard `import p::*` — copy every `p::X` type twin to bare `X`.
                // `or_insert`: a local/explicit-import name of the same kind wins.
                let td: Vec<(String, TypeInfo)> = self
                    .typedefs
                    .iter()
                    .filter_map(|(k, v)| {
                        k.strip_prefix(&prefix).map(|b| (b.to_string(), v.clone()))
                    })
                    .collect();
                for (b, v) in td {
                    self.typedefs.entry(b).or_insert(v);
                }
                let sl: Vec<(String, StructLayout)> = self
                    .struct_layouts
                    .iter()
                    .filter_map(|(k, v)| {
                        k.strip_prefix(&prefix).map(|b| (b.to_string(), v.clone()))
                    })
                    .collect();
                for (b, v) in sl {
                    self.struct_layouts.entry(b).or_insert(v);
                }
                let ed: Vec<(String, Vec<(String, i64)>)> = self
                    .enum_defs
                    .iter()
                    .filter_map(|(k, v)| {
                        k.strip_prefix(&prefix).map(|b| (b.to_string(), v.clone()))
                    })
                    .collect();
                for (b, v) in ed {
                    self.enum_defs.entry(b).or_insert(v);
                }
                let un: Vec<String> = self
                    .union_type_names
                    .iter()
                    .filter_map(|k| k.strip_prefix(&prefix).map(|b| b.to_string()))
                    .collect();
                self.union_type_names.extend(un);
                // G6: wildcard-copy UNPACKED struct typedefs too (see the explicit arm).
                let usl: Vec<(String, Vec<StructMember>)> = self
                    .unpacked_struct_layouts
                    .iter()
                    .filter_map(|(k, v)| {
                        k.strip_prefix(&prefix).map(|b| (b.to_string(), v.clone()))
                    })
                    .collect();
                for (b, v) in usl {
                    self.unpacked_struct_layouts.entry(b).or_insert(v);
                }
            }
        }
        Some(ImportDecl {
            pkg,
            item,
            span: start.to(self.prev_span()),
        })
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
                    Some(m) => items.push(TopItem::Module(m)),
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
                    Some(m) => items.push(TopItem::Interface(m)),
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
    pub(crate) fn parse_module_like(&mut self, _start_kw: Kw, end_kw: Kw) -> Option<ModuleDecl> {
        let start = self.cur_span();
        // Variable→struct bindings are module-scoped (type *names* are not).
        self.var_struct.clear();
        self.var_unpacked_struct.clear();
        self.record_array_vars.clear();
        self.record_soa_vars.clear();
        self.struct_scalar_vars.clear();
        self.struct_1d_array_vars.clear();
        self.var_enum.clear();
        self.const_locals.clear();
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
                // body=false: an array parameter in the header is a loud error
                // inside `finish_param_assignment` (never a `ConstArrayVar` here).
                if let Some(ParamItem::Scalar(p)) = self.finish_param_assignment(&pfx, false) {
                    params.push(p);
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')'");
        }

        // port list: ANSI ( dir type name, … ) | non-ANSI ( name, … ) | none
        let ports = self.parse_port_list();
        self.expect(TokenKind::Semi, "';' after module header");

        // For the post-package scoped-twin pass below: snapshot the aggregate type
        // sub-maps BEFORE the body so a chained-alias-of-aggregate (`typedef pk::s a;`
        // — an `Alias` node that nonetheless writes a layout) can be told apart from a
        // plain vector alias that merely LEAVES a stale same-name entry from another
        // package. Only a package needs this; a module never runs the twin pass.
        let (struct_before, enum_before) = if end_kw == Kw::Endpackage {
            (self.struct_layouts.clone(), self.enum_defs.clone())
        } else {
            (Default::default(), Default::default())
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
            for it in &body {
                if let ModuleItem::Typedef(td) = it {
                    let n = td.name.name.clone();
                    if n.contains("::") {
                        continue; // already-scoped (defensive; package typedefs are bare)
                    }
                    let scoped = format!("{pkg}::{n}");
                    if let Some(ti) = self.typedefs.get(&n).cloned() {
                        self.typedefs.insert(scoped.clone(), ti);
                    }
                    // Was `n`'s struct/enum layout (re)written by THIS package body?
                    // `Struct`/`Enum` nodes own their map unconditionally; an `Alias`
                    // is fresh only if its aggregate entry differs from the pre-body
                    // snapshot (chained-alias-of-aggregate), never when it is a stale
                    // cross-package leftover.
                    let struct_fresh = match td.kind {
                        TypedefKind::Struct { .. } => self.struct_layouts.contains_key(&n),
                        TypedefKind::Alias { .. } => {
                            self.struct_layouts.get(&n) != struct_before.get(&n)
                                && self.struct_layouts.contains_key(&n)
                        }
                        TypedefKind::Enum { .. } => false,
                    };
                    let enum_fresh = match td.kind {
                        // Unlike a struct (whose layout is ALWAYS written), an `Enum`
                        // node writes `enum_defs[n]` only when its labels are literal-
                        // foldable; a non-foldable enum (`{X = SOME_PARAM}`) leaves a
                        // same-name entry from another package intact — so, exactly like
                        // an `Alias`, it is fresh only when THIS body changed the entry
                        // (else the enum methods on a scoped var stay honest-loud rather
                        // than silently binding the wrong package's labels).
                        TypedefKind::Enum { .. } | TypedefKind::Alias { .. } => {
                            self.enum_defs.get(&n) != enum_before.get(&n)
                                && self.enum_defs.contains_key(&n)
                        }
                        TypedefKind::Struct { .. } => false,
                    };
                    if struct_fresh {
                        if let Some(sl) = self.struct_layouts.get(&n).cloned() {
                            self.struct_layouts.insert(scoped.clone(), sl);
                        }
                        // A union is a struct-layout overlay; its flag rides with the
                        // (fresh) layout twin.
                        if self.union_type_names.contains(&n) {
                            self.union_type_names.insert(scoped.clone());
                        }
                    }
                    // Round-9: an UNPACKED struct is never in `struct_layouts`, so it
                    // needs its own `pkg::T` layout twin (map membership is the gate —
                    // only an unpacked-struct typedef puts its name here).
                    if let Some(usl) = self.unpacked_struct_layouts.get(&n).cloned() {
                        self.unpacked_struct_layouts.insert(scoped.clone(), usl);
                    }
                    if enum_fresh {
                        if let Some(ed) = self.enum_defs.get(&n).cloned() {
                            self.enum_defs.insert(scoped, ed);
                        }
                    }
                }
            }
        }
        // Inject the synthetic `$enum_name$<T>` functions generated by any
        // `x.name()` desugar in this container's body, appended in deterministic
        // (BTreeMap key) order. Drained so the next container starts fresh — each
        // container gets its own copy (module-scoped functions, no collision).
        for (_, f) in std::mem::take(&mut self.pending_enum_name_fns) {
            body.push(ModuleItem::Func(f));
        }
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

    pub(crate) fn parse_module_item(&mut self) -> Option<ModuleItem> {
        // skip a stray lexer error token without re-reporting (already diagnosed)
        if self.at_lex_error() {
            let s = self.cur_span();
            self.bump();
            return Some(ModuleItem::Error(s));
        }
        // G9: an optional `label :` prefix on a labelable concurrent-assertion module
        // item (IEEE 1800 §16.2: `name : assert|assume property …` / `name : cover
        // property …`). At module scope a leading `IDENT :` is ONLY a label — an
        // instantiation is `TYPE inst (...)`, never `TYPE :` — so gate on the following
        // assert/assume/cover-property keyword; a stray `IDENT :` still falls through
        // and errors loudly. The label only names the assertion (the checker is wrapped
        // in a synthetic `initial` with no label slot), so consume + discard it and let
        // the existing assert/assume/cover arms below materialize the checker unchanged.
        if self.is_ident() && self.peek_at(1) == Some(TokenKind::Colon) {
            let after_colon_assert = matches!(
                self.peek_at(2),
                Some(TokenKind::Word(WordKind::Keyword(Kw::Assert | Kw::Assume)))
            );
            let after_colon_cover = self.text_at(2) == "cover"
                && self.peek_at(3) == Some(TokenKind::Word(WordKind::Keyword(Kw::Property)));
            if after_colon_assert || after_colon_cover {
                self.bump(); // label ident
                self.bump(); // ':'
            }
        }
        // parameter / localparam — a COMMA-LIST shares ONE type prefix across every
        // name (`localparam [T] A = 1, B = 2;` — IEEE §6.20.1). The first name emits
        // inline; the rest queue in `pending_module_items` and drain (in order, same
        // scope) at the next `parse_module_item`/`parse_gen_item`.
        if self.at_kw(Kw::Parameter) || self.at_kw(Kw::Localparam) || self.at_kw(Kw::Specparam) {
            let pfx = self.parse_param_prefix();
            let mut first: Option<ModuleItem> = None;
            loop {
                let Some(pi) = self.finish_param_assignment(&pfx, true) else {
                    break; // parse error already recorded by finish_param_assignment
                };
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
            return first;
        }
        // defparam path = expr [, path = expr]* ;  (IEEE §23.10.1)
        if self.at_kw(Kw::Defparam) {
            return self.parse_defparam().map(ModuleItem::Defparam);
        }
        // continuous assign
        if self.at_kw(Kw::Assign) {
            return self.parse_cont_assign().map(ModuleItem::ContAssign);
        }
        // GATE: gate-level primitive instantiation (and/or/nand/nor/xor/xnor/
        // buf/not/bufif0/bufif1/notif0/notif1) — desugared to continuous assigns.
        if let Some(g) = self.gate_kind() {
            return self.parse_gate_primitive(g).map(ModuleItem::ContAssign);
        }
        // non-ANSI body port direction decl
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Input | Kw::Output | Kw::Inout
            )))
        ) {
            return self.parse_port_decl().map(ModuleItem::PortDecl);
        }
        // SV `typedef enum/…/<type> name;` (Phase-2 user-defined types).
        if self.at_kw(Kw::Typedef) {
            return self.parse_typedef();
        }
        // ⓑ-breadth (§25.9): `virtual INTERFACE vif [, vif2];` handle declaration.
        // Distinguished from a `virtual function/task` method by the keyword that
        // follows: an interface/type NAME (an ident) vs `function`/`task`.
        if self.at_kw(Kw::Virtual)
            && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
        {
            return self.parse_virtual_iface_decl().map(ModuleItem::NetVar);
        }
        // N7: `class NAME …; … endclass` declared inside a module/package body.
        if self.at_kw(Kw::Class) {
            return self.parse_class_decl().map(ModuleItem::Class);
        }
        // v5 ⑥: `modport mp (input a, output b);` — interface body item.
        if self.at_kw(Kw::Modport) {
            return self.parse_modport().map(ModuleItem::Modport);
        }
        // v7 P2-D: module/package-scope `import pkg::…;`.
        if self.at_kw(Kw::Import) {
            // A comma list emits its FIRST term inline and queues the rest in
            // `pending_module_items` — the same mechanism the body-param comma
            // list uses, because this function may return only one item.
            let mut list = self.parse_import_decl_list()?.into_iter();
            let first = list.next()?;
            self.pending_module_items
                .extend(list.map(ModuleItem::Import));
            return Some(ModuleItem::Import(first));
        }
        // net/var declaration
        if self.net_var_kind().is_some() {
            // Module-item scope (also reached for generate items via
            // parse_gen_item → parse_module_item): a net-decl delay IS allowed.
            return self.parse_net_var(true).map(ModuleItem::NetVar);
        }
        // typedef-name declaration: `color_t c, d;` where `color_t` was typedef'd.
        if let Some(info) = self.peek_typedef_name() {
            return self.parse_typed_decl(info).map(ModuleItem::NetVar);
        }
        // procedural blocks → REAL parsing (PR2).
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Initial
                    | Kw::Always
                    | Kw::AlwaysFf
                    | Kw::AlwaysComb
                    | Kw::AlwaysLatch
                    | Kw::Final
            )))
        ) {
            return Some(ModuleItem::Proc(self.parse_procedural_block()));
        }
        // function/endfunction and task/endtask definitions.
        if self.at_kw(Kw::Function) {
            let (fd, is_void) = self.parse_function_def();
            if is_void {
                // `function void` in module/package scope ⇒ task-equivalent: reuse the
                // full task machinery (statement call, output formals, control flow).
                return Some(ModuleItem::Task(TaskDef {
                    automatic: fd.automatic,
                    name: fd.name,
                    ports: fd.ports,
                    body_decls: fd.body_decls,
                    body_enums: fd.body_enums,
                    body: fd.body,
                    span: fd.span,
                }));
            }
            return Some(ModuleItem::Func(fd));
        }
        if self.at_kw(Kw::Task) {
            return Some(ModuleItem::Task(self.parse_task_def()));
        }
        // genvar declaration:  genvar i, j;
        if self.at_kw(Kw::Genvar) {
            return Some(self.parse_genvar_decl());
        }
        // generate construct:  generate … endgenerate  (PR3 — real parsing).
        if self.at_kw(Kw::Generate) {
            return Some(ModuleItem::Generate(self.parse_generate_construct()));
        }
        // IEEE 1800-2017 §27.3: **the `generate` and `endgenerate` keywords are
        // optional.** A conditional or loop generate written without them is the
        // dominant spelling in modern SV and is what synthesis tools and other
        // simulators are handed, so refusing it made the wrapper mandatory in vita
        // and only in vita. The error it produced pointed at the `end`/`else` that
        // followed, never at the missing keyword.
        //
        // Only these three keywords reach here: `parse_gen_item` tests them BEFORE
        // falling through to `parse_module_item`, so a bare `if` inside an explicit
        // `generate` block never takes this path and the two spellings cannot
        // recurse into each other.
        if self.at_kw(Kw::If) || self.at_kw(Kw::For) || self.at_kw(Kw::Case) {
            let start = self.cur_span();
            let items = self.parse_gen_item().into_iter().collect::<Vec<_>>();
            return Some(ModuleItem::Generate(GenerateConstruct {
                items,
                span: start.to(self.prev_span()),
            }));
        }
        // module-level concurrent assertion: `assert property(@(clk) …);`
        // (slice S10). Only `assert property` is a module item — an immediate
        // `assert (expr)` is procedural-only and is a loud error here. The
        // concurrent form is wrapped in a synthetic `initial` so it flows
        // through the same procedural ConcurrentAssert collection
        // (`pending_sva`); the checker is materialized at module level
        // regardless, so this is a pure parser-placement change (no AST shape
        // change, no sim-ir change).
        if self.at_kw(Kw::Assert) || self.at_kw(Kw::Assume) {
            let start = self.cur_span();
            self.bump(); // `assert` / `assume`
            if !self.at_kw(Kw::Property) {
                self.error(
                    "`property` after `assert`/`assume` at module level (immediate \
                     assertions are procedural-only)",
                );
                return Some(ModuleItem::Error(start.to(self.prev_span())));
            }
            // SVA-REST: `assume property` is checked exactly like `assert property`
            // in simulation (IEEE §16.12 — the assumption is verified); the same
            // synthesized checker is materialized.
            let stmt = self.parse_concurrent_assert(start);
            let span = start.to(self.prev_span());
            return Some(ModuleItem::Proc(ProceduralBlock {
                kind: ProcKind::Initial,
                sensitivity: None,
                body: Box::new(stmt),
                span,
            }));
        }
        // SVA-REST: module-level `cover property(@(clk) seq);` — wrapped in a synthetic
        // `initial` (like module-level `assert property`) so it flows through the same
        // procedural collection; the counter/report is materialized at module level.
        if self.at_ident_kw("cover")
            && self.peek_at(1) == Some(TokenKind::Word(WordKind::Keyword(Kw::Property)))
        {
            let start = self.cur_span();
            let stmt = self.parse_cover_property();
            let span = start.to(self.prev_span());
            return Some(ModuleItem::Proc(ProceduralBlock {
                kind: ProcKind::Initial,
                sensitivity: None,
                body: Box::new(stmt),
                span,
            }));
        }
        // SVA-REST: `let NAME [(formals)] = expr;` (IEEE 1800 §11.13) — a named
        // expression macro. `let` is contextual (an SV reserved word, never a legal
        // net name), recognized only when followed by an identifier.
        if self.at_ident_kw("let")
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
        {
            return self.parse_let_decl();
        }
        // Named SVA declarations (Phase-3 named-SVA slice). `sequence` /
        // `endsequence` / `endproperty` are CONTEXTUAL keywords (`at_ident_kw`,
        // like `throughout`/`within`/`iff`); `property` is `Kw::Property`. Placed
        // before the bare-ident instantiation arm so `sequence s; …` is not
        // mis-parsed as a module instantiation. A net named `sequence`
        // (`wire sequence;`) is unaffected — `net_var_kind` matches first.
        //
        // `sequence` is NOT a Verilog-2005 reserved word, so a V2005 module TYPE
        // literally named `sequence` and its instantiation (`sequence u(.o(o))`) must
        // STILL parse. A no-formals decl `sequence NAME ;` routes here on the cheap
        // 2-token guard. A PARAMETERIZED decl `sequence NAME ( … ) ;` (slice A1)
        // collides with a positional/named module instantiation of the same shape;
        // disambiguate by a content-independent forward scan for the terminating
        // `endsequence` (a decl always has one; an instantiation never does). The
        // scan is what lets `sequence u(.o(o));` (no `endsequence`) stay an
        // instantiation while `sequence s(x,y); … endsequence` is a decl.
        // `property` IS a hard keyword (`Kw::Property`) — it cannot name a module, so
        // there is no masking there.
        if self.at_ident_kw("sequence")
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
            && (self.peek_at(2) == Some(TokenKind::Semi)
                || (self.peek_at(2) == Some(TokenKind::LParen) && self.is_sequence_decl_ahead()))
        {
            return self.parse_sequence_decl();
        }
        if self.at_kw(Kw::Property) {
            return self.parse_property_decl();
        }
        // N5: functional-coverage model `covergroup NAME; … endgroup`.
        if self.at_kw(Kw::Covergroup) {
            return self.parse_covergroup();
        }
        // §16.15 `default disable iff (expr);`. Checked BEFORE the `default clocking`
        // arm below so the two `default`-led items do not have to share a lookahead.
        if self.at_kw(Kw::Default)
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Keyword(Kw::Disable)))
            )
        {
            self.bump(); // `default`
            self.bump(); // `disable`
            if self.at_ident_kw("iff") {
                self.bump();
            } else {
                self.error("`iff` after `default disable`");
            }
            self.expect(TokenKind::LParen, "'(' after `default disable iff`");
            let e = self.expr(0);
            self.expect(
                TokenKind::RParen,
                "')' after `default disable iff` condition",
            );
            self.expect(TokenKind::Semi, "';' after `default disable iff`");
            return Some(ModuleItem::DefaultDisableIff(e));
        }
        // N4: `clocking …` / `default clocking …` block (IEEE 1800 §14).
        if self.at_kw(Kw::Clocking)
            || (self.at_kw(Kw::Default)
                && matches!(
                    self.peek_at(1),
                    Some(TokenKind::Word(WordKind::Keyword(Kw::Clocking)))
                ))
        {
            return self.parse_clocking();
        }
        // N5: a covergroup INSTANCE `CG_TYPE NAME = new;` — distinguished from a module
        // instantiation (`CG_TYPE NAME ( … )`) by the `=` at lookahead 2. Placed before
        // the bare-ident instantiation arm.
        if self.is_ident()
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
            && self.peek_at(2) == Some(TokenKind::Eq)
        {
            return self.parse_cover_instance();
        }
        // bare ident at module-item position ⇒ module instantiation.
        // (No keyword-led item matched above; in V2005 module scope a leading
        //  bare identifier can ONLY begin an instantiation — there is no
        //  bare-ident contassign/decl. The dispatch position itself is the
        //  disambiguation, so no multi-token lookahead is needed to decide.
        //  Gate PRIMITIVES (`and`/`or`/`buf`/…) are keyword-led, never reach
        //  this arm, and are not parsed in v1 — they fall through to the loud
        //  "expected module item" E2002 below.)
        if self.is_ident() {
            let module_name = self.ident().unwrap();
            return Some(ModuleItem::Instance(
                self.parse_module_instance(module_name),
            ));
        }
        self.error("module item");
        None
    }
}
