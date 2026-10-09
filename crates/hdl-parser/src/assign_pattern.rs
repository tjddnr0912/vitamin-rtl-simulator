//! SV §10.9 assignment patterns `'{…}` — parsing (positional and keyed) and the
//! parser-side desugars that resolve one against a PACKED-STRUCT target.
//!
//! Split out of `struct_sel.rs` when V34-3's keyed support pushed that file past
//! the 1000-line policy. Nothing here is a type, so no SchemaHash key moves; the
//! items are the same `impl Parser` methods, reachable through the same crate
//! root re-export.
//!
//! Struct layout is a PARSER fact (`StructLayout::fields` carries member names,
//! widths and 2-state-ness), which is why a §10.9.2 named pattern is resolved
//! here and never reaches elaborate. The only keyed shape that does reach
//! elaborate is `'{default: v}` on an unpacked ARRAY, whose dimensions the parser
//! does not know — see `elaborate::arrays::expand_array_default_pattern`.

use super::*;

impl Parser<'_, '_> {
    /// Parse an assignment pattern `'{…}` (cursor at `'`), in either the POSITIONAL
    /// form `'{e0,…,eN}` (§10.9) or the KEYED form `'{k: v, …}` (§10.9.1/§10.9.2).
    /// The two never mix inside one pattern (IEEE 1800 §10.9: a pattern is either
    /// all-positional or all-keyed), and a mixed one is loud here rather than
    /// silently taking one interpretation.
    ///
    /// Only two key spellings are accepted: `default` (§10.9.1) and a bare member
    /// NAME (§10.9.2). An integer key (`'{0: a}`) and a type key (`'{int: 0}`) stay
    /// loud — see `AssignPatternKey`. A replicated `'{N{e}}` also stays loud: `N`
    /// parses, then the trailing `{` fails the `,`/`}` expectation.
    ///
    /// ⚠️ Measured 2026-08-26, NOT inherited: iverilog 13 rejects EVERY keyed
    /// pattern (`'{mode:4'h3,…}`, `'{default:5}`) AND `'{4{9}}` with a bare
    /// "syntax error / Malformed statement", in a procedural assignment and in a
    /// declaration initializer alike; verilator 5.050 accepts all three and agrees
    /// with §10.9. So the pre-slice docstring's claim that the replication reject
    /// "matches iverilog" was true of iverilog and false of verilator and of the
    /// LRM — iverilog is simply not an oracle on this axis, which is why the shapes
    /// below are pinned against verilator plus a hand-IEEE reading.
    pub(crate) fn parse_assign_pattern(&mut self) -> Expr {
        let start = self.cur_span();
        self.bump(); // '
        self.expect(TokenKind::LBrace, "'{' to open an assignment pattern");
        let mut elems: Vec<Expr> = Vec::new();
        let mut keyed: Vec<(AssignPatternKey, Expr)> = Vec::new();
        // Set once a diagnostic has been emitted for this pattern: the node then
        // becomes `ExprKind::Error` so no consumer re-reports a DERIVED complaint
        // (a half-collected keyed list looks exactly like a missing member).
        let mut bad = false;
        if self.peek() != Some(TokenKind::RBrace) {
            loop {
                match self.assign_pattern_key() {
                    Some(k) => {
                        let v = self.expr(0);
                        keyed.push((k, v));
                    }
                    None => {
                        // An element that STARTS with a token directly followed by `:`
                        // is a key form `assign_pattern_key` declined — an integer key
                        // (`'{0: a}`) or a type key (`'{int: 0}`). Report it here, before
                        // `expr(0)` runs: at `int` that call produces a five-diagnostic
                        // cascade for one mistake (measured). A ternary element is not
                        // caught by this test — in `'{a ? b : c}` the token after `a` is
                        // `?`, so the colon is never at element start + 1.
                        if self.peek_at(1) == Some(TokenKind::Colon) {
                            self.error(
                                "an assignment-pattern key that is `default` or a struct member \
                                 name (an integer or type key is not supported)",
                            );
                            while !self.at_eof() && self.peek() != Some(TokenKind::RBrace) {
                                self.bump();
                            }
                            bad = true;
                            break;
                        }
                        elems.push(self.expr(0));
                    }
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RBrace, "'}' closing an assignment pattern");
        let span = start.to(self.prev_span());
        if !keyed.is_empty() && !elems.is_empty() {
            self.error_at(
                span,
                "an assignment pattern that is either all-positional or all-keyed \
                 (IEEE 1800 §10.9 does not allow mixing)",
            );
            bad = true;
        }
        let kind = if bad {
            ExprKind::Error
        } else if keyed.is_empty() {
            ExprKind::AssignPattern(elems)
        } else {
            ExprKind::AssignPatternKeyed(keyed)
        };
        Expr { kind, span }
    }

    /// §5.2 row 36: a fresh `StructLayout::reg` for a packed struct or union typedef.
    pub(crate) fn next_type_reg(&mut self) -> u64 {
        self.type_reg_last += 1;
        self.type_reg_last
    }

    /// §5.2 row 36: register `name`'s typedef entry as the name's newest type
    /// registration (`TypeInfo::bind`).
    pub(crate) fn stamp_type_bind(&mut self, name: &str) {
        let n = self.next_type_reg();
        if let Some(info) = self.typedefs.get_mut(name) {
            info.bind = n;
        }
    }

    /// §5.2 row 36: the number of the unpacked record `scoped` an import is about to
    /// copy (`u64::MAX` — newest — for a record with no number); `None` when there is
    /// none.
    pub(crate) fn imported_record_bind(&self, scoped: &str) -> Option<u64> {
        self.unpacked_struct_layouts
            .contains_key(scoped)
            .then(|| self.unpacked_bind.get(scoped).copied().unwrap_or(u64::MAX))
    }

    /// §5.2 row 36: an import wrote `bare`'s typedef entry (`td`, its number in the
    /// package) and/or its unpacked record (`un`): each takes a fresh number, the
    /// package's order between the two kept.
    pub(crate) fn restamp_imported_type(&mut self, bare: &str, td: Option<u64>, un: Option<u64>) {
        let record_last = match (td, un) {
            (Some(t), Some(u)) => u > t,
            _ => true,
        };
        if un.is_some() && !record_last {
            let n = self.next_type_reg();
            self.unpacked_bind.insert(bare.to_string(), n);
        }
        if td.is_some() {
            self.stamp_type_bind(bare);
        }
        if un.is_some() && record_last {
            let n = self.next_type_reg();
            self.unpacked_bind.insert(bare.to_string(), n);
        }
    }

    /// §5.2 row 36: `name`'s typedef entry is its newest type registration — no
    /// unpacked record of the name (which writes no typedef entry) was registered after
    /// it. Every other kind writes the typedef entry, so this entry is the kind the
    /// name denotes where it is read.
    pub(crate) fn typedef_is_freshest(&self, name: &str) -> bool {
        let Some(info) = self.typedefs.get(name) else {
            return false;
        };
        !self.unpacked_struct_layouts.contains_key(name)
            || self.unpacked_bind.get(name).is_some_and(|b| *b < info.bind)
    }

    /// §5.2 row 36: the `reg` each nested member type key of `fields` names right now
    /// (`StructLayout::decl_nested`), recorded as the type is declared.
    pub(crate) fn nested_type_regs(&self, fields: &[StructFieldLayout]) -> Vec<(String, u64)> {
        let mut out: Vec<(String, u64)> = Vec::new();
        for f in fields {
            if let Some(k) = &f.8 {
                if !out.iter().any(|(n, _)| n == k) {
                    if let Some(l) = self.struct_layouts.get(k) {
                        out.push((k.clone(), l.reg));
                    }
                }
            }
        }
        out
    }

    /// §10.9 (§5.2 row 36): the type key of a primary that names a TYPE — a bare
    /// `T` or a scoped `p::T` registered as a typedef (packed or unpacked) — before a
    /// `'{`; `None` for any other primary (a value before `'{` is no typed pattern).
    pub(crate) fn typed_pattern_type_key(&self, e: &Expr) -> Option<String> {
        let key = match &e.kind {
            ExprKind::Ident(path) if path.segments.len() == 1 => path.segments[0].name.clone(),
            ExprKind::PkgScoped { pkg, name } => format!("{}::{}", pkg.name, name.name),
            _ => return None,
        };
        (self.typedefs.contains_key(&key) || self.unpacked_struct_layouts.contains_key(&key))
            .then_some(key)
    }

    /// §10.9 (§5.2 row 36): a typed assignment pattern `T'{…}` (cursor at `'`, `ty`
    /// the parsed type name). IEEE 1800-2017 §10.9: the pattern is typed by `T` — its
    /// value is what assigning the untyped `'{…}` to a variable of type `T` stores.
    /// So it lowers through the untyped pattern's own packed-struct rule,
    /// `build_struct_pattern_concat` against `T`: the field-width concat, each element
    /// sized to its member (a 2-state member coerced), a keyed or `default:` form put
    /// into declaration order, a nested member recursed — exactly `T`'s width and,
    /// for an unsigned `T`, its type. The result is self-determined like any concat,
    /// so it stands wherever an expression does: a `return` (OpenTitan
    /// `keymgr_dpe_pkg::extract_metadata_from_slot`), an operand, a replication, an
    /// argument, a port actual, a parameter value.
    ///
    /// Only an UNSIGNED packed struct `T` is supported. A `struct packed signed`
    /// typed pattern splits the oracles wherever its sign is read (beside a signed
    /// `?:` arm verilator zero-extends the other arm where sv2v → iverilog
    /// sign-extends; `%0d` prints 30 in verilator and -2 in sv2v → iverilog); a union,
    /// an unpacked struct, an array or vector type, and a struct laid out per instance
    /// have no packed-struct lowering here. Each is refused by name.
    ///
    /// The layout maps are keyed by type NAME and a scope that re-declares the name
    /// leaves an outer or imported layout under it (a variable of the outer type reads
    /// it there — ROADMAP §2, the T stage's §5.2 row 67). So the pattern decides what
    /// `T` means where it stands from the typedef entry, not the layout map
    /// (`typed_pattern_layout`), and lowers only when every nested member type key
    /// still names the layout `T` was declared with.
    /// Out of line: `expr_postfix` sits on the expression recursion, whose depth cap
    /// is sized by its frame.
    #[inline(never)]
    pub(crate) fn parse_typed_assign_pattern(&mut self, ty: Expr) -> Expr {
        let key = self.typed_pattern_type_key(&ty).unwrap_or_default();
        let pat = self.parse_assign_pattern();
        let span = ty.span.to(pat.span);
        if matches!(pat.kind, ExprKind::Error) {
            return Expr {
                kind: ExprKind::Error,
                span,
            };
        }
        let Some(layout) = self.typed_pattern_layout(&key) else {
            self.error_at(
                span,
                "a typed assignment pattern `T'{…}` whose type `T` is an unsigned packed \
                 struct (a signed or per-instance struct, a union, an unpacked struct, an \
                 array and a vector type are unsupported in v1)",
            );
            return Expr {
                kind: ExprKind::Error,
                span,
            };
        };
        // §5.2 row 36: a nested member's layout is looked up by its type key HERE, so a
        // key re-declared in between (a generate block, a routine, a `begin` block, an
        // import) would lay the member out by the inner type.
        if !self.nested_layouts_as_declared(layout, 0) {
            self.error_at(
                span,
                "a typed assignment pattern whose nested member types mean here what they \
                 meant where `T` was declared (a same-named type declared in between is \
                 unsupported in v1)",
            );
            return Expr {
                kind: ExprKind::Error,
                span,
            };
        }
        let before = self.errors.len();
        let outer = std::mem::replace(&mut self.typed_pattern_lowering, true);
        let lowered = self.build_struct_pattern_concat(&key, pat);
        self.typed_pattern_lowering = outer;
        if Self::is_assign_pattern(&lowered) {
            // The untyped rule refused the pattern and said why (a count mismatch, a
            // nested refusal, a 2-state member wider than 64 bits).
            if self.errors.len() == before {
                self.error_at(
                    span,
                    "a typed assignment pattern its type's members can take",
                );
            }
            return Expr {
                kind: ExprKind::Error,
                span,
            };
        }
        Expr {
            kind: lowered.kind,
            span,
        }
    }

    /// §5.2 row 36: the packed-struct layout `key` names where a typed pattern stands —
    /// `Some` only when the name's typedef entry was registered by the declaration that
    /// laid out `struct_layouts[key]` (`TypeInfo::layout_reg` = `StructLayout::reg`), that
    /// entry is the name's newest type registration (`typedef_is_freshest`: no unpacked
    /// record of the name registered after it), and it is no union and no signed struct.
    /// A vector, enum, per-instance struct, type parameter, unpacked record or import
    /// re-declaring the name leaves the old layout behind; it is refused. A packed struct
    /// declared after a same-named record or per-instance struct is the newest and runs.
    fn typed_pattern_layout(&self, key: &str) -> Option<&StructLayout> {
        let layout = self.struct_layouts.get(key)?;
        let info = self.typedefs.get(key)?;
        (info.layout_reg == Some(layout.reg)
            && !info.signed
            && !self.union_type_names.contains(key)
            && self.typedef_is_freshest(key))
        .then_some(layout)
    }

    /// §5.2 row 36: every nested member type key of `layout`, at any depth, still names
    /// the layout it named where the enclosing type was declared (`StructLayout::
    /// decl_nested`) — what the type's members are, whatever the name means where the
    /// pattern stands. `depth` stops a pathological chain.
    fn nested_layouts_as_declared(&self, layout: &StructLayout, depth: u32) -> bool {
        if depth > 64 {
            return false;
        }
        layout.fields.iter().all(|f| {
            let Some(k) = &f.8 else {
                return true;
            };
            let Some(cur) = self.struct_layouts.get(k) else {
                return false;
            };
            layout
                .decl_nested
                .iter()
                .any(|(n, reg)| n == k && *reg == cur.reg)
                && self.nested_layouts_as_declared(cur, depth + 1)
        })
    }

    /// If the cursor sits on a resolvable assignment-pattern KEY followed by `:`,
    /// consume the key and the colon and return it; otherwise leave the cursor put
    /// (the element is positional, or a key form we reject downstream).
    ///
    /// The two-token lookahead is what keeps a ternary element working: in
    /// `'{a ? b : c}` the cursor is on `a`, whose next token is `?`, so this
    /// returns `None` and `expr(0)` swallows the whole conditional — the colon is
    /// never seen at element level.
    fn assign_pattern_key(&mut self) -> Option<AssignPatternKey> {
        if self.peek_at(1) != Some(TokenKind::Colon) {
            return None;
        }
        if self.at_kw(Kw::Default) {
            self.bump(); // default
            self.bump(); // :
            return Some(AssignPatternKey::Default);
        }
        if self.is_ident() {
            let name = self.text_at(0).to_string();
            self.bump(); // name
            self.bump(); // :
            return Some(AssignPatternKey::Member(name));
        }
        None
    }

    /// N3: desugar a record-array init `'{ '{…}, … }` — each OUTER element (an inner
    /// record `'{…}`) becomes a field-width concat via `build_struct_pattern_concat`,
    /// leaving an outer `AssignPattern` of packed element VALUES. The existing dyn-array
    /// `'{…}` decl-init flush then lowers it to `new[N]` + one whole-element write each.
    pub(crate) fn desugar_record_array_init(&mut self, tyname: &str, e: Expr) -> Expr {
        let span = e.span;
        let ExprKind::AssignPattern(elems) = e.kind else {
            return e;
        };
        let parts = elems
            .into_iter()
            .map(|el| self.build_struct_pattern_concat(tyname, el))
            .collect();
        Expr {
            kind: ExprKind::AssignPattern(parts),
            span,
        }
    }

    /// §3 ⑤ ⓑ: desugar the init/rhs of a 1-D array of a PACKED STRUCT
    /// (`st_t A[N] = '{ '{…}, … }`, a `localparam st_t P[N] = …`, or a whole-array
    /// `A = '{…}`): every OUTER element that is itself a `'{…}` becomes the same
    /// field-width concat `A[i] = '{…}` desugars to (`build_struct_pattern_concat`,
    /// keyed or positional), leaving an outer pattern of packed element VALUES the
    /// unpacked-array init/assign path already lowers. A non-pattern element (a
    /// struct-typed constant, a packed literal) is left untouched, exactly as
    /// `A[i] = expr` is. The keyed outer form is walked the same way so
    /// `'{default: '{…}}` (§10.9.1) reaches elaborate's `'{default: v}` arm with a
    /// packed `v`; an outer member key on an ARRAY is not touched (loud downstream).
    /// Anything that is not a pattern returns unchanged (byte-identical).
    pub(crate) fn desugar_struct_array_init(&mut self, tyname: &str, e: Expr) -> Expr {
        let span = e.span;
        match e.kind {
            ExprKind::AssignPattern(elems) => {
                let parts = elems
                    .into_iter()
                    .map(|el| {
                        if Self::is_assign_pattern(&el) {
                            self.build_struct_pattern_concat(tyname, el)
                        } else {
                            el
                        }
                    })
                    .collect();
                Expr {
                    kind: ExprKind::AssignPattern(parts),
                    span,
                }
            }
            ExprKind::AssignPatternKeyed(keyed) => {
                let keyed = keyed
                    .into_iter()
                    .map(|(k, v)| {
                        if Self::is_assign_pattern(&v) {
                            (k, self.build_struct_pattern_concat(tyname, v))
                        } else {
                            (k, v)
                        }
                    })
                    .collect();
                Expr {
                    kind: ExprKind::AssignPatternKeyed(keyed),
                    span,
                }
            }
            kind => Expr { kind, span },
        }
    }

    /// IEEE §10.9.1/§10.9.2 packed-struct assignment pattern. When `rhs` is
    /// `'{e0,…,eN}` (or the keyed `'{name: v, …, default: v}`, first normalized to
    /// declaration order by `keyed_struct_pattern_to_positional`)
    /// and `var_name` is a *scalar* packed-struct variable, desugar
    /// it to the field-width-cast concat `{w0'(e0), …, wN'(eN)}` — field 0 is the
    /// MSB (leftmost). Each element is sized to its FIELD width (NOT
    /// self-determined), so an unsized or fill (`'1`/`'x`/`'z`) element grows to
    /// the field: `'{5,6}` ≠ `{5,6}`. The size cast reuses the existing
    /// `CastTarget::Size` lowering (which sizes a fill operand in the cast width,
    /// §11.6), so no elaborate/IR change is needed — struct layout is parser-only.
    ///
    /// `rhs` is returned untouched when it is not a pattern or `var_name` is not a
    /// scalar struct var (an array-of-struct stays on the 1-D unpacked-array path,
    /// a non-struct var is unaffected) — so every non-struct assignment is
    /// byte-identical. A struct pattern with the wrong element count is a loud
    /// parse error (matching iverilog, which rejects a field-count mismatch).
    pub(crate) fn desugar_struct_assign_pattern(&mut self, var_name: &str, rhs: Expr) -> Expr {
        if !Self::is_assign_pattern(&rhs) || !self.struct_scalar_vars.contains(var_name) {
            return rhs;
        }
        match self.var_struct.get(var_name).cloned() {
            Some(tyname) => self.build_struct_pattern_concat(&tyname, rhs),
            None => rhs,
        }
    }

    /// Build the field-width-cast concat for a packed-struct `'{…}` pattern whose
    /// target resolves to struct type `tyname` (shared by the scalar-variable path
    /// `s = '{…}` and the 1-D-array-element path `arr[i] = '{…}`). `rhs` must be an
    /// `AssignPattern`. A count mismatch or a 2-state field wider than 64 bits is a
    /// loud parse error (returning the pattern unchanged).
    /// §7.10.2/§10.9.2: a `'{…}` ACTUAL to a container method whose element is a
    /// struct — `q.push_back('{1, 2})`, the standard way to enqueue a record in an
    /// AXI/transaction model. The element's packed value is the same field concat
    /// `q[i] = '{…}` already desugars to, so this reuses `build_struct_pattern_concat`
    /// rather than inventing a second layout rule; keying on the RECEIVER's type is
    /// what makes each element land at its declared width instead of a bare concat's
    /// self-determined one.
    ///
    /// Only `'{…}` actuals are rewritten, so `q.insert(i, '{…})`'s index is untouched
    /// and every non-pattern call is byte-identical.
    pub(crate) fn desugar_container_pattern_args(
        &mut self,
        path: &HierPath,
        args: Vec<Expr>,
    ) -> Vec<Expr> {
        if path.segments.len() != 2
            || !matches!(
                path.segments[1].name.as_str(),
                "push_back" | "push_front" | "insert"
            )
            || !args.iter().any(Self::is_assign_pattern)
        {
            return args;
        }
        let recv = &path.segments[0].name;
        let Some(tyname) = self
            .var_struct
            .get(recv)
            .cloned()
            .or_else(|| self.record_array_vars.get(recv).cloned())
        else {
            return args;
        };
        args.into_iter()
            .map(|a| {
                if Self::is_assign_pattern(&a) {
                    self.build_struct_pattern_concat(&tyname, a)
                } else {
                    a
                }
            })
            .collect()
    }

    /// Either `'{…}` spelling — the positional `AssignPattern` or the keyed
    /// `AssignPatternKeyed`. Both are desugared by the SAME struct/array machinery
    /// (a keyed one is first put into declaration order), so every gate that asks
    /// "is this rhs a pattern?" must ask about both or a keyed pattern silently
    /// falls off the desugar path and lands, unresolved, in elaborate.
    pub(crate) fn is_assign_pattern(e: &Expr) -> bool {
        matches!(
            e.kind,
            ExprKind::AssignPattern(_) | ExprKind::AssignPatternKeyed(_)
        )
    }

    /// §10.9.2 named + §10.9.1 `default:` → the POSITIONAL element list the rest of
    /// the struct desugar already consumes. Field order comes from the DECLARATION
    /// (`fields`), never from the order the keys were written — that is the whole
    /// point of the named form: inserting a member cannot shift a later value.
    ///
    /// Loud (returns `None`, error already emitted) on an unknown member name, a
    /// duplicate key, or a member left unfilled with no `default:` — §10.9.1 says
    /// every member must be covered exactly once.
    ///
    /// ⚠️ `default:`'s value expression is CLONED into every slot it fills, so a
    /// call there would be evaluated once per member instead of once. §10.9.1 does
    /// not pin that count, verilator and iverilog cannot be compared (iverilog
    /// rejects the whole form), so a call-bearing `default:` stays loud rather than
    /// silently multiplying a side effect.
    ///
    /// §3 ⑤ ⓓ: the second vector marks the slots the `default:` filled — a NESTED
    /// struct member takes a default only when the value is a fill or zero
    /// (`build_struct_pattern_concat`).
    fn keyed_struct_pattern_to_positional(
        &mut self,
        fields: &[(String, u32, bool)],
        keyed: Vec<(AssignPatternKey, Expr)>,
        span: Span,
    ) -> Option<(Vec<Expr>, Vec<bool>)> {
        let mut slots: Vec<Option<Expr>> = vec![None; fields.len()];
        let mut default: Option<Expr> = None;
        for (k, v) in keyed {
            match k {
                AssignPatternKey::Default => {
                    if default.is_some() {
                        self.error_at(span, "at most one `default:` in an assignment pattern");
                        return None;
                    }
                    if Self::expr_has_call(&v) {
                        self.error_at(
                            span,
                            "a call-free `default:` value (it is duplicated into every member \
                             it fills, which would run the call once per member)",
                        );
                        return None;
                    }
                    default = Some(v);
                }
                AssignPatternKey::Member(name) => {
                    let Some(i) = fields.iter().position(|(n, _, _)| *n == name) else {
                        self.error_at(
                            span,
                            "an assignment-pattern key naming a member of this struct",
                        );
                        return None;
                    };
                    if slots[i].is_some() {
                        self.error_at(span, "each struct member named at most once in `'{…}`");
                        return None;
                    }
                    slots[i] = Some(v);
                }
            }
        }
        let mut out = Vec::with_capacity(slots.len());
        let mut from_default = Vec::with_capacity(slots.len());
        for slot in slots {
            let defaulted = slot.is_none();
            match slot.or_else(|| default.clone()) {
                Some(e) => {
                    out.push(e);
                    from_default.push(defaulted);
                }
                None => {
                    self.error_at(
                        span,
                        "every packed-struct member given by name or by `default:` \
                         (IEEE 1800 §10.9.1)",
                    );
                    return None;
                }
            }
        }
        Some((out, from_default))
    }

    /// §3 ⑤ ⓓ: the sized literal a fill `e` denotes at width `w` (`'1` → `w'b11…1`,
    /// `'0` → `w'b0`, `'x`/`'z` → all-x/all-z, or all-zero on a 2-state field);
    /// `None` when `e` is not a fill literal.
    pub(crate) fn fill_at_width(e: &Expr, w: u32, two_state: bool) -> Option<Expr> {
        let ExprKind::IntLit { raw, .. } = &e.kind else {
            return None;
        };
        let digit = match raw.trim() {
            "'0" => '0',
            "'1" => '1',
            "'x" | "'X" if !two_state => 'x',
            "'z" | "'Z" if !two_state => 'z',
            "'x" | "'X" | "'z" | "'Z" => '0',
            _ => return None,
        };
        let body: String = std::iter::repeat_n(digit, w as usize).collect();
        Some(Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Sized,
                raw: format!("{w}'b{body}"),
            },
            span: e.span,
        })
    }

    /// §3 ⑤ ⓓ: is `e` a value whose meaning is the same whether a `default:`
    /// applies it to a nested struct member AS A WHOLE or to each of its leaves —
    /// a fill (`'0`/`'1`/`'x`/`'z`) or the literal zero? Anything else (`default:
    /// 1`, `default: 2'b10`) reads differently under the two rules (verilator
    /// applies it whole: `'{default: 1}` on `{cor[1:0], perms{U0,SE,q[1:0]}, valid}`
    /// is `01_0001_1`; a per-leaf reading gives `01_1101_1`; iverilog rejects the
    /// form; §10.9.2 does not settle it) — loud.
    fn default_value_shape_free(&self, e: &Expr) -> bool {
        if let ExprKind::IntLit { raw, .. } = &e.kind {
            let r = raw.trim();
            if matches!(r, "'0" | "'1" | "'x" | "'X" | "'z" | "'Z") {
                return true;
            }
        }
        self.const_bound(e) == Some(0)
    }

    pub(crate) fn build_struct_pattern_concat(&mut self, tyname: &str, rhs: Expr) -> Expr {
        // Each field's (name, width, is_two_state) in declaration order (field 0 =
        // MSB = leftmost concat part); cloned out so `self` is free for `error`
        // below. The NAME is what a §10.9.2 keyed pattern resolves against.
        // N3: a PACKABLE record (in `unpacked_struct_layouts`, not `struct_layouts`)
        // has an on-demand packed layout — an `arr[i] = '{…}` / decl-init element of a
        // record array desugars through the same field-width concat.
        let layout = self
            .struct_layouts
            .get(tyname)
            .cloned()
            .or_else(|| self.packable_record_layout(tyname));
        let nested: Vec<Option<String>> = match &layout {
            Some(l) => l.fields.iter().map(|f| f.8.clone()).collect(),
            None => return rhs,
        };
        let fields: Vec<(String, u32, bool)> = match layout {
            Some(l) => l
                .fields
                .iter()
                .map(|(n, _, w, _, _, ts, _, _, _)| (n.clone(), *w, *ts))
                .collect(),
            None => return rhs,
        };
        let span = rhs.span;
        let (elems, from_default) = match rhs.kind {
            ExprKind::AssignPattern(elems) => {
                let n = elems.len();
                (elems, vec![false; n])
            }
            // A keyed pattern is put into declaration order FIRST, then rides the
            // identical field-width-cast path below — one layout rule, not two.
            ExprKind::AssignPatternKeyed(keyed) => {
                match self.keyed_struct_pattern_to_positional(&fields, keyed.clone(), span) {
                    Some(v) => v,
                    None => {
                        return Expr {
                            kind: ExprKind::AssignPatternKeyed(keyed),
                            span,
                        }
                    }
                }
            }
            _ => unreachable!("caller guarantees an assignment-pattern rhs"),
        };
        let fields: Vec<(u32, bool)> = fields.into_iter().map(|(_, w, ts)| (w, ts)).collect();
        if elems.len() != fields.len() {
            self.error("exactly one `'{…}` element for each packed-struct field");
            return Expr {
                kind: ExprKind::AssignPattern(elems),
                span,
            };
        }
        // §3 ⑤ ⓓ: an element for a NESTED struct member. A `'{…}` value recurses
        // into the nested layout (its own field-width concat — exact width, each
        // leaf already 2-state-coerced, so the outer squash/size below is skipped);
        // a `default:`-supplied value must be shape-free (see
        // `default_value_shape_free`); a plain value is sized to the member like
        // any other (`perms: 12'h1eb`).
        let mut parts_in: Vec<(Expr, bool)> = Vec::with_capacity(elems.len());
        for ((e, defaulted), nested) in elems.into_iter().zip(from_default).zip(&nested) {
            match nested {
                // §5.2 row 36: a union shares `struct_layouts`, so recursing
                // concatenates EVERY member of the overlay (`'{4'h5, 4'h7}` for a 4-bit
                // union laid 8 bits wide). Refused in a typed pattern; the untyped
                // pattern keeps its pre-row-36 lowering (ROADMAP §2).
                Some(nty)
                    if self.typed_pattern_lowering
                        && Self::is_assign_pattern(&e)
                        && self.union_type_names.contains(nty) =>
                {
                    self.error_at(
                        e.span,
                        "a value for a packed-union member (a `'{…}` pattern for a union \
                         member is unsupported in v1)",
                    );
                    return Expr {
                        kind: ExprKind::AssignPattern(
                            parts_in.into_iter().map(|(e, _)| e).collect(),
                        ),
                        span,
                    };
                }
                Some(nty) if Self::is_assign_pattern(&e) => {
                    let inner = self.build_struct_pattern_concat(nty, e);
                    if Self::is_assign_pattern(&inner) {
                        // the nested pattern was loud (error emitted) — keep the
                        // outer pattern unresolved, exactly like the count-mismatch arm
                        return Expr {
                            kind: ExprKind::AssignPattern(
                                parts_in.into_iter().map(|(e, _)| e).collect(),
                            ),
                            span,
                        };
                    }
                    parts_in.push((inner, true));
                }
                Some(_) if defaulted && !self.default_value_shape_free(&e) => {
                    self.error_at(
                        e.span,
                        "a fill (`'0`/`'1`) or 0 as the `default:` of a pattern whose unnamed member is itself a packed struct (whether the value applies to the nested struct as a whole or to each of its members is not pinned in v1)",
                    );
                    return Expr {
                        kind: ExprKind::AssignPattern(
                            parts_in.into_iter().map(|(e, _)| e).collect(),
                        ),
                        span,
                    };
                }
                _ => parts_in.push((e, false)),
            }
        }
        // A 2-state field is X/Z-coerced by squashing the element through
        // `longint'(e)` (the widest 2-state prim) before sizing; one wider than 64
        // bits cannot be squashed this way, so honest-loud rather than silent-wrong.
        if parts_in
            .iter()
            .zip(&fields)
            .any(|((_, exact), &(w, ts))| !exact && ts && w > 64)
        {
            self.error("a 2-state packed-struct field no wider than 64 bits in `'{…}`");
            return Expr {
                kind: ExprKind::AssignPattern(parts_in.into_iter().map(|(e, _)| e).collect()),
                span,
            };
        }
        let parts = parts_in
            .into_iter()
            .zip(fields)
            .map(|((e, exact), (w, two_state))| {
                if exact {
                    return e;
                }
                // §3 ⑤ ⓓ: a FILL element (`'0`/`'1`/`'x`/`'z`) is emitted as the sized
                // literal it means at the field's width (§11.6 — a fill in a size cast
                // is the fill at that width), not as `w'('0)`: the size cast of a fill
                // has no constant-fold arm, so `parameter cap_t NULL_CAP = '{default:
                // '0}` was loud (§2 🆕 L ⓔ). A 2-state field coerces `'x`/`'z` to 0
                // (§6.11.3), exactly what the `longint'` squash below does for a value.
                if let Some(lit) = Self::fill_at_width(&e, w, two_state) {
                    return lit;
                }
                // 4-state field: keep the value (plain size cast). 2-state field:
                // coerce X/Z→0 (§6.11.3) via `w'(longint'(e))` — `longint'` squashes
                // unknowns to 0; the size cast then takes the field's low `w` bits.
                let inner = if two_state {
                    Expr {
                        kind: ExprKind::Cast {
                            target: CastTarget::Prim(CastPrim::Longint),
                            expr: Box::new(e),
                        },
                        span,
                    }
                } else {
                    e
                };
                Expr {
                    kind: ExprKind::Cast {
                        target: CastTarget::Size(Box::new(Self::dec_lit(w, span))),
                        expr: Box::new(inner),
                    },
                    span,
                }
            })
            .collect();
        Expr {
            kind: ExprKind::Concat { parts },
            span,
        }
    }

    /// Shared assignment hook for the packed-struct `'{…}` pattern. Desugars two
    /// target shapes: a whole scalar struct variable (`s = '{…}`, a single-segment
    /// `Lvalue::Ident`), and a 1-D struct-array element (`arr[i] = '{…}`, a
    /// `BitSelect` of a plain Ident in `struct_1d_array_vars`). Every other lvalue
    /// (field-select, multi-dim/nested index, concat, scalar bit-select) is left
    /// untouched (loud downstream). Called at every statement/expression
    /// `lvalue = rhs` site — blocking/nonblocking, continuous /
    /// procedural-continuous / `force` assigns, and for-init/step. (A scalar
    /// decl-init `st_t s = '{…}` is desugared by a direct `desugar_struct_assign_pattern`
    /// call in `parse_typed_decl`, not through this hook.)
    pub(crate) fn maybe_struct_pattern_rhs(&mut self, lhs: &Lvalue, rhs: Expr) -> Expr {
        // §3 ⑤ ⓓ: the nested struct type of a whole-member write target
        // (`s.perms = '{…}` / `arr[i].perms = '{…}`), recorded by the lvalue parse
        // that produced `lhs`. Taken unconditionally so it never outlives its lvalue.
        let member_ty = self.member_pattern_ty.take();
        // Fast path: only `'{…}` to an eligible target can desugar; every other
        // assignment returns `rhs` untouched (byte-identical) with no work.
        if !Self::is_assign_pattern(&rhs) {
            return rhs;
        }
        if let (Some(ty), Lvalue::PartSelect { .. }) = (member_ty, lhs) {
            return self.build_struct_pattern_concat(&ty, rhs);
        }
        match lhs {
            // Whole scalar struct variable `s = '{…}`, or a whole 1-D struct
            // array `arr = '{ '{…}, … }` (§3 ⑤ ⓑ — per-element, same helper as
            // the decl-init).
            Lvalue::Ident(p) if p.segments.len() == 1 => {
                let nm = p.segments[0].name.clone();
                if self.struct_1d_array_vars.contains(&nm) {
                    if let Some(tyname) = self.var_struct.get(&nm).cloned() {
                        return self.desugar_struct_array_init(&tyname, rhs);
                    }
                }
                self.desugar_struct_assign_pattern(&nm, rhs)
            }
            // 1-D struct-array element `arr[i] = '{…}`: the base must be a plain
            // single-name Ident registered as a 1-D struct array (this excludes a
            // scalar struct's bit-select `s[i]`, a multi-dim element `arr[i][j]`
            // whose base is itself a BitSelect, and a union array). The element's
            // struct type is the array variable's struct type.
            Lvalue::BitSelect { base, .. } => {
                if let Lvalue::Ident(p) = base.as_ref() {
                    if p.segments.len() == 1 {
                        let nm = &p.segments[0].name;
                        if self.struct_1d_array_vars.contains(nm) {
                            if let Some(tyname) = self.var_struct.get(nm).cloned() {
                                return self.build_struct_pattern_concat(&tyname, rhs);
                            }
                        }
                        // N3: a record-array element `arr[i] = '{…}` — desugar the
                        // pattern to a packed field concat (via `packable_record_layout`),
                        // leaving a whole-element dyn write the engine already supports.
                        if let Some(tyname) = self.record_array_vars.get(nm).cloned() {
                            return self.build_struct_pattern_concat(&tyname, rhs);
                        }
                    }
                }
                rhs
            }
            _ => rhs,
        }
    }
}
