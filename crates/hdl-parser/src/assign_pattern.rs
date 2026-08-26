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
    fn keyed_struct_pattern_to_positional(
        &mut self,
        fields: &[(String, u32, bool)],
        keyed: Vec<(AssignPatternKey, Expr)>,
        span: Span,
    ) -> Option<Vec<Expr>> {
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
        for slot in slots {
            match slot.or_else(|| default.clone()) {
                Some(e) => out.push(e),
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
        Some(out)
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
        let fields: Vec<(String, u32, bool)> = match layout {
            Some(l) => l
                .fields
                .iter()
                .map(|(n, _, w, _, _, ts, _, _)| (n.clone(), *w, *ts))
                .collect(),
            None => return rhs,
        };
        let span = rhs.span;
        let elems = match rhs.kind {
            ExprKind::AssignPattern(elems) => elems,
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
        // A 2-state field is X/Z-coerced by squashing the element through
        // `longint'(e)` (the widest 2-state prim) before sizing; one wider than 64
        // bits cannot be squashed this way, so honest-loud rather than silent-wrong.
        if fields.iter().any(|&(w, ts)| ts && w > 64) {
            self.error("a 2-state packed-struct field no wider than 64 bits in `'{…}`");
            return Expr {
                kind: ExprKind::AssignPattern(elems),
                span,
            };
        }
        let parts = elems
            .into_iter()
            .zip(fields)
            .map(|(e, (w, two_state))| {
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
        // Fast path: only `'{…}` to an eligible target can desugar; every other
        // assignment returns `rhs` untouched (byte-identical) with no work.
        if !Self::is_assign_pattern(&rhs) {
            return rhs;
        }
        match lhs {
            // Whole scalar struct variable `s = '{…}`.
            Lvalue::Ident(p) if p.segments.len() == 1 => {
                let nm = p.segments[0].name.clone();
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
