//! parameter machinery — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// ⓑ-breadth (§8.25): MONOMORPHIZE parameterized classes. For each distinct
    /// `C #(args)` used at a handle declaration (plus the all-defaults instance),
    /// generate a fully-concrete class with the parameter values substituted, and
    /// rewrite each handle's type to the specialization. The default specialization
    /// keeps the bare class name (so `C h;` resolves); overrides get a mangled name
    /// (`C__16`). v1: positional INTEGER-LITERAL spec args only (a non-literal arg
    /// is a loud reject — its value is not foldable into a stable specialization).
    pub(crate) fn monomorphize_param_classes(&mut self, items: &mut [TopItem]) {
        use std::collections::BTreeMap;
        // Pass 1: collect parameterized templates by name.
        let mut templates: BTreeMap<String, ClassDecl> = BTreeMap::new();
        let mut collect = |c: &ClassDecl| {
            if !c.params.is_empty() {
                templates.insert(c.name.name.clone(), c.clone());
            }
        };
        for it in items.iter() {
            match it {
                TopItem::Class(c) => collect(c),
                TopItem::Module(m) | TopItem::Interface(m) | TopItem::Package(m) => {
                    for bi in &m.body {
                        if let ModuleItem::Class(c) = bi {
                            collect(c);
                        }
                    }
                }
                _ => {}
            }
        }
        if templates.is_empty() {
            return;
        }
        // Build the param map (param → value-expr) for a given spec's args.
        // Missing trailing args fall back to the parameter default; an unfilled
        // parameter with no default is a loud reject.
        let mut new_specs: Vec<ClassDecl> = Vec::new();
        let mut spec_seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut errors: Vec<(Span, &'static str)> = Vec::new();
        // The map of every (orig handle) → its rewritten class name is applied by a
        // second walk; here we just register the needed specs and remember the name
        // each (template,args) maps to via `mangle`.
        let mangle = |tmpl: &str, args: &[Expr]| -> Option<String> {
            if args.is_empty() {
                return Some(tmpl.to_string());
            }
            let mut parts = Vec::new();
            for a in args {
                parts.push(arg_render(a)?);
            }
            Some(format!("{tmpl}__{}", parts.join("_")))
        };
        // Register a spec (idempotent) given its template + args.
        let mut register = |tmpl_name: &str, args: &[Expr], at: Span| -> String {
            let tmpl = &templates[tmpl_name];
            let Some(name) = mangle(tmpl_name, args) else {
                errors.push((
                    at,
                    "a parameterized class specialization argument (an integer literal, `C #(16)`)",
                ));
                return tmpl_name.to_string();
            };
            if spec_seen.insert(name.clone()) {
                // build param map
                let mut map: BTreeMap<String, Expr> = BTreeMap::new();
                let mut ok = true;
                for (i, p) in tmpl.params.iter().enumerate() {
                    let val = args.get(i).cloned().or_else(|| p.default.clone());
                    match val {
                        Some(v) => {
                            map.insert(p.name.name.clone(), v);
                        }
                        None => {
                            errors.push((
                                at,
                                "a specialization argument for a parameterized class parameter \
                                 that has no default",
                            ));
                            ok = false;
                        }
                    }
                }
                if ok {
                    new_specs.push(monomorphize_class(tmpl, &name, &map));
                }
            }
            name
        };
        // Pass 2: walk every handle decl; rewrite its type and register the spec.
        // Module-level handles (`ModuleItem::NetVar`) and class-field handles
        // (`ClassItem::Property`) are covered.
        fn rewrite_netvar(
            d: &mut NetVarDecl,
            templates: &BTreeMap<String, ClassDecl>,
            register: &mut dyn FnMut(&str, &[Expr], Span) -> String,
        ) {
            let Some(ct) = &d.class_type else { return };
            if !templates.contains_key(&ct.name) {
                return;
            }
            let new_name = register(&ct.name.clone(), &d.class_args, d.span);
            if let Some(ci) = &mut d.class_type {
                ci.name = new_name;
            }
            d.class_args = Vec::new();
        }
        for it in items.iter_mut() {
            match it {
                TopItem::Class(c) => {
                    for item in &mut c.items {
                        if let ClassItem::Property(_, d) = item {
                            rewrite_netvar(d, &templates, &mut register);
                        }
                    }
                }
                TopItem::Module(m) | TopItem::Interface(m) | TopItem::Package(m) => {
                    for bi in &mut m.body {
                        match bi {
                            ModuleItem::NetVar(d) => {
                                rewrite_netvar(d, &templates, &mut register);
                            }
                            ModuleItem::Class(c) => {
                                for item in &mut c.items {
                                    if let ClassItem::Property(_, d) = item {
                                        rewrite_netvar(d, &templates, &mut register);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        // Always register the all-defaults instance for each template (so `C h;`
        // resolves even if every concrete handle overrode the parameters).
        let tmpl_names: Vec<String> = templates.keys().cloned().collect();
        for t in &tmpl_names {
            let _ = register(t, &[], templates[t].name.span);
        }
        for (sp, msg) in errors {
            self.error_at(sp, msg);
        }
        // Pass 3: replace each template ClassDecl with its DEFAULT specialization
        // (bare name) and append the override specializations at top level.
        for it in items.iter_mut() {
            if let TopItem::Class(c) = it {
                if let Some(def) = new_specs.iter().find(|s| s.name.name == c.name.name) {
                    *c = def.clone();
                }
            }
            if let TopItem::Module(m) | TopItem::Interface(m) | TopItem::Package(m) = it {
                for bi in &mut m.body {
                    if let ModuleItem::Class(c) = bi {
                        if let Some(def) = new_specs.iter().find(|s| s.name.name == c.name.name) {
                            *c = def.clone();
                        }
                    }
                }
            }
        }
        // The OVERRIDE specs (mangled names, not matching any template) are added as
        // fresh top-level classes via the returned list (handled by the caller).
        self.pending_mono_specs = new_specs
            .into_iter()
            .filter(|s| !templates.contains_key(&s.name.name))
            .collect();
    }

    /// Parse the TYPE PREFIX of a parameter/localparam decl (the keyword is
    /// optional on `#(…)` continuations, defaulting to `Parameter`, which matches
    /// IEEE-1364 §12.2). The name+value tail is `finish_param_assignment`, split
    /// out so a comma-list applies ONE prefix to every name.
    pub(crate) fn parse_param_prefix(&mut self) -> ParamPrefix {
        let start = self.cur_span();
        let kind = if self.eat_kw(Kw::Localparam) {
            ParamKind::Localparam
        } else {
            self.eat_kw(Kw::Parameter);
            ParamKind::Parameter
        };
        // Track explicit-`signed` PRESENCE alongside the folded bool: the A2a
        // array desugar must mirror `signed_eff` (explicit wins, else the atom
        // default) — the folded bool alone conflates "absent" with `false`.
        let expl0 = self.opt_signed();
        let mut signed = expl0.unwrap_or(false);
        // SV typed parameter: a data-type KIND keyword may lead — `parameter int W`,
        // `parameter logic [3:0] X`, `byte`/`shortint`/`longint`. 2-state atoms imply
        // a fixed signed range; `int` maps to the 32-bit signed Integer path. The
        // V2005 `integer`/`real`/`realtime`/`time` types stay in the else branch.
        let mut ty = ParamType::Implicit;
        let mut forced_range = None;
        // The exact declared kind, kept for the A2a array-parameter desugar (the
        // scalar path collapses `int` into `ParamType::Integer` etc. — the desugar
        // must construct the SAME `NetVarDecl` the equivalent var decl would).
        let mut var_kind: Option<NetVarKind> = None;
        let mut expl1: Option<bool> = None;
        let kw_kind = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(
                k @ (Kw::Logic
                | Kw::Reg
                | Kw::Bit
                | Kw::Int
                | Kw::Byte
                | Kw::Shortint
                | Kw::Longint),
            ))) => Some(k),
            _ => None,
        };
        if let Some(k) = kw_kind {
            self.bump(); // the kind keyword
                         // 2-state atoms (int/byte/shortint/longint) DEFAULT to signed; logic/reg/
                         // bit default to unsigned. An explicit `signed`/`unsigned` (in EITHER
                         // position, `unsigned int` or `int unsigned`) WINS over that default — so
                         // `int unsigned` / `byte unsigned` come out unsigned, matching the
                         // equivalent var decl. (The old `signed || expl` could never flip an
                         // atom's signed default back to unsigned.)
            let atom_signed = match k {
                Kw::Int => {
                    ty = ParamType::Integer; // 32-bit 2-state
                    var_kind = Some(NetVarKind::Int);
                    true
                }
                Kw::Byte => {
                    forced_range = Some(Self::dec_range(7));
                    var_kind = Some(NetVarKind::Byte);
                    true
                }
                Kw::Shortint => {
                    forced_range = Some(Self::dec_range(15));
                    var_kind = Some(NetVarKind::Shortint);
                    true
                }
                Kw::Longint => {
                    forced_range = Some(Self::dec_range(63));
                    var_kind = Some(NetVarKind::Longint);
                    true
                }
                // logic/reg/bit: unsigned default; width from an explicit range below.
                Kw::Logic => {
                    var_kind = Some(NetVarKind::Logic);
                    false
                }
                Kw::Reg => {
                    var_kind = Some(NetVarKind::Reg);
                    false
                }
                Kw::Bit => {
                    var_kind = Some(NetVarKind::Bit);
                    false
                }
                _ => false,
            };
            expl1 = self.opt_signed();
            signed = expl0.or(expl1).unwrap_or(atom_signed);
        } else {
            ty = match self.peek() {
                Some(TokenKind::Word(WordKind::Keyword(Kw::Integer))) => {
                    self.bump();
                    var_kind = Some(NetVarKind::Integer);
                    // `integer` is 32-bit SIGNED (V2005); an explicit `signed`/
                    // `unsigned` in EITHER position (`unsigned integer` OR the
                    // trailing `integer unsigned`) wins over that default, mirroring
                    // the `int`/`byte`/… atoms above (was: only the leading position
                    // was consumed, so `integer unsigned` was a parse error).
                    expl1 = self.opt_signed();
                    signed = expl0.or(expl1).unwrap_or(true);
                    ParamType::Integer
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Real))) => {
                    self.bump();
                    var_kind = Some(NetVarKind::Real);
                    ParamType::Real
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Realtime))) => {
                    self.bump();
                    var_kind = Some(NetVarKind::Realtime);
                    ParamType::Realtime
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Time))) => {
                    self.bump();
                    var_kind = Some(NetVarKind::Time);
                    ParamType::Time
                }
                // N5: a `string` typed parameter/localparam. Consume the keyword so
                // the name parses; the value is a string literal, folded and stored
                // as a string constant by elaborate (detected by the StrLit value, so
                // no new `ParamType` variant is needed — the untyped spelling
                // `localparam S = "abc"` (N5B) rides the same value-detection path).
                Some(TokenKind::Word(WordKind::Keyword(Kw::String))) => {
                    self.bump();
                    ParamType::Implicit
                }
                _ => ParamType::Implicit,
            };
        }
        let explicit_range = if forced_range.is_some() {
            None
        } else {
            self.opt_range()
        };
        ParamPrefix {
            start,
            kind,
            signed,
            ty,
            var_kind,
            forced_range,
            explicit_range,
            expl0,
            expl1,
        }
    }

    /// Finish ONE parameter assignment — `name [array_dims] = value` — using a
    /// shared `ParamPrefix`. A body ARRAY parameter (A2a) desugars to the
    /// equivalent const variable-array decl (see `parse_array_param`); otherwise a
    /// scalar `ParamDecl`. Called once per name in a comma-list so every name
    /// inherits the SAME leading type prefix (IEEE §6.20.1).
    pub(crate) fn finish_param_assignment(
        &mut self,
        pfx: &ParamPrefix,
        body: bool,
    ) -> Option<ParamItem> {
        let ParamPrefix {
            start,
            kind,
            signed,
            ty,
            var_kind,
            forced_range,
            explicit_range,
            expl0,
            expl1,
        } = pfx.clone();
        let range = forced_range.or_else(|| explicit_range.clone());
        // §4.5.156 (§3 全 site): a typed param's kind may not carry a user packed range
        // unless it is a vector (`logic`/`reg`/`bit`). `forced_range` is the atom's OWN
        // fixed width (byte→[7:0]) not a user dim, so gate on `explicit_range` only.
        if let Some(k) = var_kind {
            self.reject_packed_dims_on_nonvector(k, explicit_range.is_some());
        }
        let name = self.ident()?;
        // A2a: `[` after the parameter name ⇒ an ARRAY parameter (IEEE §6.20.2).
        if self.peek() == Some(TokenKind::LBracket) {
            // Explicit-signing presence (either position): the desugar mirrors
            // `signed_eff` — an explicit `signed`/`unsigned` wins over the
            // atom default (`int unsigned` must come out unsigned, like the
            // equivalent var decl — the folded bool can't express that).
            let expl_signed = match (expl0, expl1) {
                (None, None) => None,
                (a, b) => Some(a.unwrap_or(false) || b.unwrap_or(false)),
            };
            return self.parse_array_param(
                body,
                kind,
                var_kind,
                expl_signed,
                explicit_range,
                name,
                start,
            );
        }
        self.expect(TokenKind::Eq, "'=' in parameter");
        let value = self.expr(0);
        Some(ParamItem::Scalar(ParamDecl {
            kind,
            signed,
            ty,
            range,
            name,
            value,
            span: start.to(self.prev_span()),
        }))
    }

    /// Does the current token begin a parameter TYPE PREFIX — `parameter`/
    /// `localparam`, a signing keyword, or a data-type keyword? The `#(…)` header
    /// loop uses this to tell a NEW type group from an unadorned continuation
    /// (`, B = 2`) that must inherit the preceding group's type.
    pub(crate) fn starts_param_prefix(&self) -> bool {
        matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Parameter
                    | Kw::Localparam
                    | Kw::Signed
                    | Kw::Unsigned
                    | Kw::Logic
                    | Kw::Reg
                    | Kw::Bit
                    | Kw::Int
                    | Kw::Byte
                    | Kw::Shortint
                    | Kw::Longint
                    | Kw::Integer
                    | Kw::Real
                    | Kw::Realtime
                    | Kw::Time
                    | Kw::String
            )))
        )
    }

    /// A2a (IEEE §6.20.2): a body `localparam <type> NAME [dims] = '{…};` —
    /// an ARRAY parameter. DESUGARED here into the EXACT `NetVarDecl` the
    /// equivalent variable-array decl (`int NAME [dims] = '{…};`) parses to,
    /// so storage, `'{…}` init, indexing, foreach and `$size`/`$bits` reuse
    /// the verified unpacked-array path verbatim, plus `const_param: true` so
    /// elaborate registers the net as an elaboration constant (any later
    /// write is a loud error). Kept loud (v1): the ANSI `#(…)` header form
    /// and an overridable body `parameter` (no override machinery for
    /// aggregates), an implicit-typed element, and non-fixed dims (`[$]`/`[]`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn parse_array_param(
        &mut self,
        body: bool,
        kind: ParamKind,
        var_kind: Option<NetVarKind>,
        expl_signed: Option<bool>,
        explicit_range: Option<Range>,
        name: Ident,
        start: Span,
    ) -> Option<ParamItem> {
        if !body {
            self.error(
                "a scalar parameter in the ANSI `#(…)` header (an array parameter is supported only as a body `localparam` in v1)",
            );
            return None;
        }
        if kind == ParamKind::Parameter {
            self.error(
                "`localparam` for an array parameter (an overridable array `parameter` is unsupported in v1)",
            );
            return None;
        }
        let Some(vk) = var_kind else {
            self.error(
                "an explicit data type on an array parameter (`localparam int …` — an implicit-typed array parameter is unsupported in v1)",
            );
            return None;
        };
        let n_start = name.span;
        let mut unpacked = Vec::new();
        while self.at_dim_start() {
            match self.parse_dim() {
                Some(d) => unpacked.push(d),
                None => break,
            }
        }
        if !unpacked
            .iter()
            .all(|d| matches!(d, Dim::Range(_) | Dim::Size(_)))
        {
            self.error("a fixed array-parameter dimension (`[msb:lsb]`/`[size]`)");
            return None;
        }
        self.expect(TokenKind::Eq, "'=' in parameter");
        let value = self.expr(0);
        // Mirror `signed_eff`: an explicit `signed`/`unsigned` wins; otherwise
        // the atom default (byte/shortint/int/longint/integer are signed),
        // exactly like the equivalent var decl.
        let signed = expl_signed.unwrap_or_else(|| atom_default_signed(Some(vk)));
        Some(ParamItem::ConstArrayVar(NetVarDecl {
            kind: vk,
            signed,
            range: explicit_range,
            packed: Vec::new(),
            delay: None,
            names: vec![DeclName {
                name,
                unpacked,
                init: Some(value),
                span: n_start.to(self.prev_span()),
            }],
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: true,
            span: start.to(self.prev_span()),
        }))
    }

    /// Convert a parsed `ParamItem` into its `ModuleItem`, recording a module-scope
    /// `localparam` whose value is a pure literal so a constant generate-hier index
    /// (`g[P].x`) can fold it. A `parameter` is overridable → never recorded.
    pub(crate) fn param_item_to_module_item(&mut self, p: ParamItem) -> ModuleItem {
        match p {
            ParamItem::Scalar(p) => {
                if p.kind == ParamKind::Localparam {
                    if let Some(v) = self.try_const_index(&p.value) {
                        self.const_locals.insert(p.name.name.clone(), v);
                    }
                }
                ModuleItem::Param(p)
            }
            // A2a: an array parameter arrives as the desugared const variable-array
            // decl — flows through every NetVar pass verbatim.
            ParamItem::ConstArrayVar(d) => ModuleItem::NetVar(d),
        }
    }

    /// Map an optional return/var type keyword to ParamType (V2005 set only).
    /// `reg`/`logic`/bit-vector returns are NOT a ParamType — they surface via
    /// signed+range with ret_type = Implicit, so those keywords are NOT consumed.
    pub(crate) fn opt_param_type(&mut self) -> ParamType {
        match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Integer))) => {
                self.bump();
                ParamType::Integer
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Real))) => {
                self.bump();
                ParamType::Real
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Realtime))) => {
                self.bump();
                ParamType::Realtime
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Time))) => {
                self.bump();
                ParamType::Time
            }
            _ => ParamType::Implicit,
        }
    }
}
