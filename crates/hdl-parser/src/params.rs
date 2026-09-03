//! parameter machinery — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

/// `(ParamType, var_kind, forced_range, explicit_range)` — see `typedef_param_shape`.
type TypedefParamShape = (ParamType, Option<NetVarKind>, Option<Range>, Option<Range>);

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
        // R28 appendix: `specparam` (IEEE 1364-2005 §3.12) is a module-local constant
        // whose only differences from `localparam` are that it may also appear inside a
        // `specify` block and that a back-annotation tool (SDF) may override it. Neither
        // matters to a functional simulation, and a vendor model that keeps its timing
        // constants there and references them from ordinary delay expressions
        // (`#(tPROBE*0.5)`) is a common pattern — the reporter had to rewrite the
        // KEYWORD to get a foundry EFUSE model through. Accepted as a localparam.
        let kind = if self.eat_kw(Kw::Localparam) || self.eat_kw(Kw::Specparam) {
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
        let mut tyname: Option<String> = None;
        // A vector typedef's own packed dimension (`typedef logic [5:0] u`) — carried
        // as the EXPLICIT range so an A2a array parameter of that type keeps its
        // element width, exactly as a spelled-out `[5:0]` would. Atom widths
        // (`typedef byte b`) stay in `forced_range` like the keyword path.
        let mut typedef_range: Option<Range> = None;
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
        } else if let Some(info) = self.peek_block_typedef_decl() {
            // §3 ⑤ (IEEE §6.20.2 / §A.2.1.1 `data_type` param_assignment): a USER type
            // leads — `localparam exc_cause_t E = '{…}`, `parameter lfsr_seed_t S = …`,
            // `localparam p::u Q = …`. Resolved here to the same (kind, sign, range)
            // `parse_typed_decl` gives the equivalent variable, so the scalar
            // `ParamDecl` below is exactly what `parameter logic [W-1:0]`/`int`/…
            // would have produced (declared-width provenance, no AST change). The
            // `<type> <name>` shape is what `peek_block_typedef_decl` asserts, so a
            // header continuation `, T = 2` (a value named like a type) is untouched.
            let key = self.type_name_key();
            self.eat_scope_qualifier();
            self.bump(); // the type-name identifier
            match self.typedef_param_shape(&info) {
                Ok((t, k, forced, explicit)) => {
                    ty = t;
                    var_kind = k;
                    forced_range = forced;
                    typedef_range = explicit;
                    signed = expl0.unwrap_or(info.signed);
                    tyname = Some(key);
                }
                Err(msg) => self.error(msg),
            }
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
        // A typedef prefix carries its OWN range (a typedef's packed dimension) as
        // `explicit_range`, so an A2a array parameter of that type (`localparam u A[2]`)
        // keeps the element width; a keyword prefix reads a user `[msb:lsb]` here.
        let explicit_range = if tyname.is_some() {
            typedef_range
        } else if forced_range.is_some() {
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
            tyname,
        }
    }

    /// The scalar-parameter shape of a resolved typedef: `(ParamType, var_kind,
    /// forced_range, explicit_range)` — the SAME fields the keyword arms of
    /// `parse_param_prefix` build for `int`/`byte`/…/`logic [r]` (an atom's fixed
    /// width is `forced`, a vector's dimension is `explicit`, and the non-vector
    /// dimension reject reads only `explicit`), so a typedef'd parameter and its
    /// spelled-out twin are one AST. `Err` = a type a scalar `ParamDecl` cannot carry without
    /// losing something (a multi-dim packed typedef would flatten `P[i]` to one bit;
    /// a class handle / net / event / container is not a parameter type) — loud.
    fn typedef_param_shape(&self, info: &TypeInfo) -> Result<TypedefParamShape, &'static str> {
        if info.class_name.is_some() {
            return Err(
                "a non-class typedef on a parameter (a class-handle parameter is unsupported)",
            );
        }
        if !info.packed.is_empty() {
            return Err(
                "a typedef with one packed dimension on a parameter (a multi-dimensional packed typedef parameter — `logic [N-1:0][M-1:0]` — is unsupported in v1)",
            );
        }
        Ok(match info.kind {
            NetVarKind::Int => (ParamType::Integer, Some(NetVarKind::Int), None, None),
            NetVarKind::Integer => (ParamType::Integer, Some(NetVarKind::Integer), None, None),
            NetVarKind::Byte => (
                ParamType::Implicit,
                Some(NetVarKind::Byte),
                Some(Self::dec_range(7)),
                None,
            ),
            NetVarKind::Shortint => (
                ParamType::Implicit,
                Some(NetVarKind::Shortint),
                Some(Self::dec_range(15)),
                None,
            ),
            NetVarKind::Longint => (
                ParamType::Implicit,
                Some(NetVarKind::Longint),
                Some(Self::dec_range(63)),
                None,
            ),
            NetVarKind::Real => (ParamType::Real, Some(NetVarKind::Real), None, None),
            NetVarKind::Realtime => (ParamType::Realtime, Some(NetVarKind::Realtime), None, None),
            NetVarKind::Time => (ParamType::Time, Some(NetVarKind::Time), None, None),
            NetVarKind::String => (ParamType::Implicit, None, None, None),
            k @ (NetVarKind::Logic | NetVarKind::Reg | NetVarKind::Bit) => {
                (ParamType::Implicit, Some(k), None, info.range.clone())
            }
            _ => {
                return Err(
                    "an integral, real or string typedef on a parameter (a net / event / container typedef is not a parameter type)",
                )
            }
        })
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
            tyname: _,
        } = pfx.clone();
        // `logic`/`reg`/`bit` with NO explicit range are ONE bit (§6.11.2). The atom
        // recorded that in `var_kind` and then dropped it: `ParamDecl` has no such
        // field, so `parameter bit P` and `parameter P` were literally
        // indistinguishable downstream, and the declared width was simply lost.
        //
        // ⚠️ It must be the LAST fallback, never `forced_range`. Setting the atom's
        // own width would override an explicit one, and `parameter logic [7:0] P`
        // would come out 1 bit.
        //
        // Found because the string constant domain's width gate (§4.5.370) reads
        // `p.range.is_none()` to mean "this declaration states no width", which was
        // false for exactly these three keywords — `localparam bit P = {"A","B"}`
        // folded to 16 bits where both oracles say 1. The same gap made
        // `localparam bit P = 8'hFF` read 255 at 8 bits where both oracles say 1.
        let range = forced_range.or_else(|| explicit_range.clone()).or_else(|| {
            matches!(
                var_kind,
                Some(NetVarKind::Logic | NetVarKind::Reg | NetVarKind::Bit)
            )
            .then(|| Self::dec_range(0))
        });
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
            // A typedef prefix already folded an explicit `signed`/`unsigned` over
            // the typedef's own signedness (`signed = expl0.unwrap_or(info.signed)`),
            // so the typedef's answer is passed as if explicit — a `typedef logic
            // signed [3:0] t; localparam t X[2]` must not fall back to `logic`'s
            // unsigned default the way a keyword prefix would.
            let expl_signed = if pfx.tyname.is_some() {
                Some(signed)
            } else {
                expl_signed
            };
            return self.parse_array_param(
                body,
                kind,
                var_kind,
                expl_signed,
                explicit_range,
                pfx.tyname.as_deref(),
                name,
                start,
            );
        }
        self.expect(TokenKind::Eq, "'=' in parameter");
        let mut value = self.expr(0);
        // §3 ⑤: a struct/enum-typed parameter binds its NAME the way `parse_typed_decl`
        // binds a variable, so `E.lower_cause` desugars to the member part-select,
        // a `'{…}` value (positional or §10.9.2 named) becomes the field-width
        // concat, and `E.name()` resolves the enum's labels. A union stays bound
        // for member reads but never pattern-desugared (same as the variable path).
        self.unbind_struct_enum_name(&name.name);
        if let Some(tn) = &pfx.tyname {
            if self.struct_layouts.contains_key(tn) {
                self.var_struct.insert(name.name.clone(), tn.clone());
                if !self.union_type_names.contains(tn) {
                    self.struct_scalar_vars.insert(name.name.clone());
                    value = self.desugar_struct_assign_pattern(&name.name, value);
                }
            }
            if self.enum_defs.contains_key(tn) {
                self.var_enum.insert(name.name.clone(), tn.clone());
            }
        }
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
        self.peek_block_typedef_decl().is_some()
            || matches!(
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
    ///
    /// §3 ⑤ ⓑ: `tyname` is the user typedef the prefix named (`None` for a
    /// keyword prefix). A 1-D array of a STRUCT typedef binds the name exactly as
    /// the equivalent variable decl does (`var_struct` + `struct_1d_array_vars`, so
    /// `P[i].member` and `P[i] = '{…}` desugar) and its `'{ '{…}, … }` init is
    /// desugared per element by `desugar_struct_array_init`; an ENUM typedef binds
    /// `var_enum`. A union array binds the type only (no member/pattern desugar,
    /// as for the variable twin); a multi-dimensional struct array stays loud —
    /// the variable twin has no member/pattern desugar for it either.
    ///
    /// IEEE §6.20.1: a `parameter` in a PACKAGE is a `localparam` (nothing can
    /// override it), so `in_package` lifts the overridable-`parameter` reject
    /// there; a module-body `parameter` array stays loud (§3 ⑤ ⓒ).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn parse_array_param(
        &mut self,
        body: bool,
        kind: ParamKind,
        var_kind: Option<NetVarKind>,
        expl_signed: Option<bool>,
        explicit_range: Option<Range>,
        tyname: Option<&str>,
        name: Ident,
        start: Span,
    ) -> Option<ParamItem> {
        if !body {
            self.error(
                "a scalar parameter in the ANSI `#(…)` header (an array parameter is supported only as a body `localparam` in v1)",
            );
            return None;
        }
        if kind == ParamKind::Parameter && !self.in_package {
            self.error(
                "`localparam` for an array parameter (an overridable array `parameter` is unsupported in v1)",
            );
            return None;
        }
        let tyname = tyname.map(str::to_owned);
        let is_struct = tyname
            .as_deref()
            .is_some_and(|t| self.struct_layouts.contains_key(t));
        let is_enum = tyname
            .as_deref()
            .is_some_and(|t| self.enum_defs.contains_key(t));
        // A union array binds `var_struct` only, like its variable twin: no member
        // desugar and no `'{…}` element desugar (a union overlay is kept loud
        // there), so its elements are the packed literals the array path already
        // lowers.
        let is_union = is_struct
            && self
                .union_type_names
                .contains(tyname.as_deref().unwrap_or(""));
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
        if is_struct && !is_union && unpacked.len() != 1 {
            self.error_at(
                n_start,
                "a one-dimensional array parameter of a struct typedef (a multi-dimensional struct array parameter is unsupported in v1)",
            );
            return None;
        }
        self.expect(TokenKind::Eq, "'=' in parameter");
        let mut value = self.expr(0);
        // Same walk as `parse_typed_decl`: a local declaration ends any wildcard
        // struct/enum binding of this name, then the declared type binds it anew.
        self.unbind_struct_enum_name(&name.name);
        if let Some(tn) = tyname.as_deref() {
            if is_struct {
                self.var_struct.insert(name.name.clone(), tn.to_owned());
                if !is_union {
                    self.struct_1d_array_vars.insert(name.name.clone());
                    value = self.desugar_struct_array_init(tn, value);
                }
            } else if is_enum {
                self.var_enum.insert(name.name.clone(), tn.to_owned());
            }
        }
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
