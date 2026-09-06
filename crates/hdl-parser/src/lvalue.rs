//! lvalues / selects / dims — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// True when `e` is `name[idx]` / `path[idx]` — a bit-select rooted at a plain
    /// Ident, the shape a following `.` turns into a generate/instance-array
    /// hierarchical reference. (HIER-REST②.)
    pub(crate) fn is_indexed_hier_base(e: &Expr) -> bool {
        matches!(&e.kind, ExprKind::BitSelect { base, .. }
            if matches!(base.kind, ExprKind::Ident(_)))
    }

    /// Parse `path[idx].member(.member)*` into a hierarchical `Ident` whose indexed
    /// segment folds the CONSTANT index into the scope-segment name. Reuses the normal
    /// hierarchical resolver (no new AST/IR). A non-plain-decimal index is a loud parse
    /// error (documented sub-limitation). (HIER-REST②.)
    pub(crate) fn parse_indexed_hier(&mut self, base: Expr) -> Expr {
        let start = base.span;
        let mut segs: Vec<Ident> = Vec::new();
        if let ExprKind::BitSelect { base: b, index } = base.kind {
            if let ExprKind::Ident(p) = b.kind {
                let n = p.segments.len();
                for (i, seg) in p.segments.into_iter().enumerate() {
                    if i + 1 == n {
                        let idx_str = self.const_index_string(&index);
                        segs.push(Ident {
                            name: format!("{}[{idx_str}]", seg.name),
                            span: seg.span,
                        });
                    } else {
                        segs.push(seg);
                    }
                }
            }
        }
        // Consume `.member` segments (plain names; a following `[k].` re-enters the
        // outer postfix loop, a leaf `[k]` is a normal bit-select on the whole path).
        while self.eat(TokenKind::Dot) {
            match self.ident() {
                Some(id) => segs.push(id),
                None => break,
            }
        }
        let hi = segs.last().map(|s| s.span).unwrap_or(start);
        Expr {
            kind: ExprKind::Ident(HierPath {
                segments: segs,
                span: start.to(hi),
            }),
            span: start.to(hi),
        }
    }

    pub(crate) fn parse_select(&mut self, base: Expr) -> Expr {
        let start = base.span;
        self.bump(); // '['
        let first = self.expr(0);
        let kind = match self.peek() {
            Some(TokenKind::Colon) => {
                self.bump();
                let lsb = self.expr(0);
                ExprKind::PartSelect {
                    base: Box::new(base),
                    msb: Box::new(first),
                    lsb: Box::new(lsb),
                }
            }
            Some(TokenKind::PlusColon) => {
                self.bump();
                let w = self.expr(0);
                ExprKind::IndexedPart {
                    base: Box::new(base),
                    offset: Box::new(first),
                    width: Box::new(w),
                    dir: PartDir::PlusColon,
                }
            }
            Some(TokenKind::MinusColon) => {
                self.bump();
                let w = self.expr(0);
                ExprKind::IndexedPart {
                    base: Box::new(base),
                    offset: Box::new(first),
                    width: Box::new(w),
                    dir: PartDir::MinusColon,
                }
            }
            _ => ExprKind::BitSelect {
                base: Box::new(base),
                index: Box::new(first),
            },
        };
        self.expect(TokenKind::RBracket, "']'");
        Expr {
            kind,
            span: start.to(self.prev_span()),
        }
    }

    /// `[msb:lsb]` packed range (requires `:`).
    pub(crate) fn opt_range(&mut self) -> Option<Range> {
        if self.peek() != Some(TokenKind::LBracket) {
            return None;
        }
        let start = self.cur_span();
        self.bump();
        let msb = self.expr(0);
        self.expect(TokenKind::Colon, "':' in range");
        let lsb = self.expr(0);
        self.expect(TokenKind::RBracket, "']'");
        Some(Range {
            msb,
            lsb,
            span: start.to(self.prev_span()),
        })
    }

    /// Additional packed dims after the first `[msb:lsb]` — `logic [3:0][7:0]` ⇒
    /// `[[7:0]]`. Each is a `[msb:lsb]` range; collected greedily before the name.
    pub(crate) fn opt_packed_dims(&mut self) -> Vec<Range> {
        let mut dims = Vec::new();
        while let Some(r) = self.opt_range() {
            dims.push(r);
        }
        dims
    }

    /// Unpacked dimension `[hi:lo]` (Range) or `[N]` (Size) — verdict M3.
    /// v5 ⑥ adds the dynamic-storage forms: `[]` (dyn array), `[$]`/`[$:N]`
    /// (queue / bounded queue — the bound parses, elaborate loud-rejects it),
    /// `[integer]`/`[time]` (assoc, integer key types only). `[*]` (wildcard
    /// assoc) is a parse error — outside the MVP.
    /// A dimension can start with `[` or — since slice S4 fused `[*` into one
    /// token — `[*` (the wildcard assoc `[*]` spelling). `parse_dim` handles
    /// both; the array-dim loops gate on this so the no-space `[*]` still reaches
    /// the precise wildcard diagnostic instead of a generic token cascade.
    pub(crate) fn at_dim_start(&self) -> bool {
        matches!(
            self.peek(),
            Some(TokenKind::LBracket | TokenKind::LBracketStar)
        )
    }
    pub(crate) fn parse_dim(&mut self) -> Option<Dim> {
        // `[*]` wildcard assoc index. Since slice S4 the lexer fuses `[*` into a
        // single `LBracketStar` token (for SVA `[*n]`), so the canonical no-space
        // spelling never reaches the `Star` arm below — handle it here. Outside
        // the MVP: reject loudly with the precise message, recover as a dyn dim.
        // (The spaced `[ *]` spelling still lexes as `[`+`*` and hits the `Star`
        // arm.)
        if self.peek() == Some(TokenKind::LBracketStar) {
            self.bump(); // `[*`
            self.error(
                "a concrete assoc key type (`[integer]`/`[int]`/`[longint]`/`[time]`/`[string]`/…) — wildcard `[*]` is unsupported",
            );
            self.expect(TokenKind::RBracket, "']'");
            return Some(Dim::Dyn);
        }
        if self.peek() != Some(TokenKind::LBracket) {
            return None;
        }
        self.bump(); // '['
        match self.peek() {
            // `[]` — dynamic array.
            Some(TokenKind::RBracket) => {
                self.bump();
                return Some(Dim::Dyn);
            }
            // `[$]` / `[$:N]` — queue.
            Some(TokenKind::Dollar) => {
                self.bump();
                let bound = if self.peek() == Some(TokenKind::Colon) {
                    self.bump();
                    Some(self.expr(0))
                } else {
                    None
                };
                self.expect(TokenKind::RBracket, "']'");
                return Some(Dim::Queue(bound));
            }
            // `[integer]` / `[time]` / `[int]` / `[longint]` / `[shortint]` /
            // `[byte]` — assoc key type (keyword-led, so it can never shadow a
            // same-named size parameter). Every integral spelling shares the
            // documented signed-i64 key domain (the ⑥ design pin: keys are NOT
            // truncated to the declared width — `[integer]`/`[time]` already
            // behave this way), so the 2-state atoms map onto the same
            // `AssocKey::Integer` lowering with zero AST/schema change.
            Some(TokenKind::Word(WordKind::Keyword(
                k @ (Kw::Integer | Kw::Time | Kw::Int | Kw::Longint | Kw::Shortint | Kw::Byte),
            ))) => {
                self.bump();
                self.expect(TokenKind::RBracket, "']'");
                return Some(Dim::Assoc(if k == Kw::Time {
                    AssocKey::Time
                } else {
                    AssocKey::Integer
                }));
            }
            // `[string]` (v6) — since the v7 AST flip `string` is a real
            // KEYWORD (the P2-C type), so the assoc key form is keyword-led
            // like `[integer]`/`[time]`.
            Some(TokenKind::Word(WordKind::Keyword(Kw::String))) => {
                self.bump();
                self.expect(TokenKind::RBracket, "']'");
                return Some(Dim::Assoc(AssocKey::Str));
            }
            // `[*]` — wildcard assoc index: outside the MVP, reject loudly at
            // parse (recover as a plain dyn dim so the decl still resolves).
            Some(TokenKind::Star) => {
                self.bump();
                self.error(
                    "a concrete assoc key type (`[integer]`/`[int]`/`[longint]`/`[time]`/`[string]`/…) — wildcard `[*]` is unsupported",
                );
                self.expect(TokenKind::RBracket, "']'");
                return Some(Dim::Dyn);
            }
            _ => {}
        }
        let first = self.expr(0);
        let dim = if self.peek() == Some(TokenKind::Colon) {
            let r_start = first.span;
            self.bump();
            let lsb = self.expr(0);
            Dim::Range(Range {
                msb: first,
                lsb,
                span: r_start.to(self.prev_span()),
            })
        } else {
            Dim::Size(first)
        };
        self.expect(TokenKind::RBracket, "']'");
        Some(dim)
    }
    /// Convert a gate OUTPUT terminal expression into an `Lvalue` (an output is a
    /// net reference / select / concat). Non-lvalue shapes recover as `Error`.
    pub(crate) fn expr_to_lvalue(&mut self, e: Expr) -> Lvalue {
        match e.kind {
            ExprKind::Paren { inner } => self.expr_to_lvalue(*inner),
            ExprKind::Ident(p) => Lvalue::Ident(p),
            ExprKind::BitSelect { base, index } => Lvalue::BitSelect {
                base: Box::new(self.expr_to_lvalue(*base)),
                index,
                span: e.span,
            },
            ExprKind::PartSelect { base, msb, lsb } => Lvalue::PartSelect {
                base: Box::new(self.expr_to_lvalue(*base)),
                msb,
                lsb,
                span: e.span,
            },
            ExprKind::IndexedPart {
                base,
                offset,
                width,
                dir,
            } => Lvalue::IndexedPart {
                base: Box::new(self.expr_to_lvalue(*base)),
                offset,
                width,
                dir,
                span: e.span,
            },
            ExprKind::Concat { parts } => Lvalue::Concat {
                parts: parts.into_iter().map(|p| self.expr_to_lvalue(p)).collect(),
                span: e.span,
            },
            _ => {
                self.error("gate output must be a net or net select");
                Lvalue::Error(e.span)
            }
        }
    }

    /// Inverse of `expr_to_lvalue`: rebuild the read-side `Expr` for an already-
    /// parsed lvalue. Used to desugar a compound assignment / increment
    /// (`lvalue += e` → `lvalue = lvalue + e`; `lvalue++` → `lvalue = lvalue + 1`):
    /// the lvalue appears on BOTH sides, so the rhs needs its expression form.
    /// The lvalue↔expr select shapes are 1:1, so this is a structural clone.
    /// A free associated fn (no `self`): it reads nothing from the parser state.
    pub(crate) fn lvalue_to_expr(lv: &Lvalue) -> Expr {
        match lv {
            Lvalue::Ident(p) => Expr {
                span: p.span,
                kind: ExprKind::Ident(p.clone()),
            },
            Lvalue::BitSelect { base, index, span } => Expr {
                span: *span,
                kind: ExprKind::BitSelect {
                    base: Box::new(Self::lvalue_to_expr(base)),
                    index: index.clone(),
                },
            },
            Lvalue::PartSelect {
                base,
                msb,
                lsb,
                span,
            } => Expr {
                span: *span,
                kind: ExprKind::PartSelect {
                    base: Box::new(Self::lvalue_to_expr(base)),
                    msb: msb.clone(),
                    lsb: lsb.clone(),
                },
            },
            Lvalue::IndexedPart {
                base,
                offset,
                width,
                dir,
                span,
            } => Expr {
                span: *span,
                kind: ExprKind::IndexedPart {
                    base: Box::new(Self::lvalue_to_expr(base)),
                    offset: offset.clone(),
                    width: width.clone(),
                    dir: *dir,
                },
            },
            Lvalue::Concat { parts, span } => Expr {
                span: *span,
                kind: ExprKind::Concat {
                    parts: parts.iter().map(Self::lvalue_to_expr).collect(),
                },
            },
            Lvalue::Error(span) => Expr {
                span: *span,
                kind: ExprKind::Error,
            },
        }
    }

    /// A range endpoint: `$` (type extreme) or a constant expression.
    pub(crate) fn parse_range_end(&mut self) -> RangeEnd {
        if self.peek() == Some(TokenKind::Dollar) {
            self.bump();
            RangeEnd::TypeExtreme
        } else {
            RangeEnd::Val(self.expr(0))
        }
    }

    /// N3 SoA: a single-segment `Ident` lvalue from a (mangled) net name.
    pub(crate) fn ident_lval(name: &str, span: Span) -> Lvalue {
        Lvalue::Ident(HierPath {
            segments: vec![Ident {
                name: name.to_string(),
                span,
            }],
            span,
        })
    }

    /// LHS = concat of selects/idents only. Parse directly to `Lvalue`.
    pub(crate) fn parse_lvalue(&mut self) -> Lvalue {
        if self.peek() == Some(TokenKind::LBrace) {
            let start = self.cur_span();
            self.bump();
            // `{>>{a,b}} = e;` — the streaming UNPACK target (§11.4.14). Same exact
            // predicate as the rhs dispatch in `brace_expr`, and the same
            // balanced skip, so the enclosing `= rhs ;` still parses and the three
            // cascading errors this used to print ("expected identifier" / "expected
            // '}'" / "expected '=' or '<=' after lvalue", all at one column) collapse
            // to the one named line.
            if self.reject_streaming_lvalue() {
                return Lvalue::Error(start.to(self.prev_span()));
            }
            let mut parts = Vec::new();
            loop {
                parts.push(self.parse_lvalue());
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "'}'");
            return Lvalue::Concat {
                parts,
                span: start.to(self.prev_span()),
            };
        }
        let Some(path) = self.hier_path() else {
            let s = self.cur_span();
            return Lvalue::Error(s);
        };
        // packed-struct member target `s.field = …` → constant part-select lvalue.
        // A trailing WRITE sub-select (`s.f[a:b] = …` / `s.f[i] = …`) folds to a
        // FLAT field-bounded part-select on the struct net, mirroring the READ-side
        // normalization (`parse_struct_field_sel`): the member's declared direction
        // and offset map the source index onto the flat vector, so the write never
        // leaks past the member region. An indexed `[i±:w]` / runtime / reverse
        // sub-select stays loud (iverilog 13.0 itself refuses those struct-member
        // writes — "sorry: not yet supported" — so there is no oracle to match).
        // §3 ⑤ ⓓ: reset per lvalue; set below by a whole-member write of a nested
        // struct member (consumed by `maybe_struct_pattern_rhs`).
        self.member_pattern_ty = None;
        let mut lv = if let Some(mangled) = self.unpacked_field_ident(&path) {
            // Round-9: UNPACKED-struct member write `k.field = …` → the member net
            // `k$field` (a plain Ident). A trailing sub-select (`k.field[i] = …`)
            // flows through the `loop` below on the member net, like any net.
            Lvalue::Ident(mangled)
        } else if let Some((base, (off, w, asc, _sgn, dbase, stride), nested)) =
            self.struct_field_select(&path)
        {
            let span = path.span;
            if self.peek() == Some(TokenKind::LBracket) {
                self.parse_struct_field_lval(
                    Lvalue::Ident(base),
                    (off, w, asc, dbase, stride),
                    span,
                )
            } else {
                self.member_pattern_ty = nested;
                Lvalue::PartSelect {
                    base: Box::new(Lvalue::Ident(base)),
                    msb: Box::new(Self::dec_lit(off + w - 1, span)),
                    lsb: Box::new(Self::dec_lit(off, span)),
                    span,
                }
            }
        } else if let Some(lv) = self.sym_struct_member_lval(&path) {
            // §3 ⑤ ⓒ: a member write on a symbolic-layout struct (a cold helper —
            // see `sym_struct_member_expr`).
            lv
        } else {
            Lvalue::Ident(path)
        };
        loop {
            if self.peek() == Some(TokenKind::LBracket) {
                let start = lv.span();
                self.bump();
                let first = self.expr(0);
                lv = match self.peek() {
                    Some(TokenKind::Colon) => {
                        self.bump();
                        let lsb = self.expr(0);
                        self.expect(TokenKind::RBracket, "']'");
                        Lvalue::PartSelect {
                            base: Box::new(lv),
                            msb: Box::new(first),
                            lsb: Box::new(lsb),
                            span: start.to(self.prev_span()),
                        }
                    }
                    Some(TokenKind::PlusColon) => {
                        self.bump();
                        let w = self.expr(0);
                        self.expect(TokenKind::RBracket, "']'");
                        Lvalue::IndexedPart {
                            base: Box::new(lv),
                            offset: Box::new(first),
                            width: Box::new(w),
                            dir: PartDir::PlusColon,
                            span: start.to(self.prev_span()),
                        }
                    }
                    Some(TokenKind::MinusColon) => {
                        self.bump();
                        let w = self.expr(0);
                        self.expect(TokenKind::RBracket, "']'");
                        Lvalue::IndexedPart {
                            base: Box::new(lv),
                            offset: Box::new(first),
                            width: Box::new(w),
                            dir: PartDir::MinusColon,
                            span: start.to(self.prev_span()),
                        }
                    }
                    _ => {
                        self.expect(TokenKind::RBracket, "']'");
                        Lvalue::BitSelect {
                            base: Box::new(lv),
                            index: Box::new(first),
                            span: start.to(self.prev_span()),
                        }
                    }
                };
            } else if self.peek() == Some(TokenKind::Dot) && self.record_array_lval_base(&lv) {
                // N3 (write): `arr[i].field = …` — a record-array element member write.
                // Fold to a part-select lvalue on the dyn element (mirrors the READ-side
                // `parse_record_array_member`); the engine does a read-modify-write. This
                // is checked BEFORE the generate-array hier path below, which would else
                // fold the known record-array element into a bogus hier scope name.
                lv = self.parse_record_array_member_lval(lv);
            } else if self.peek() == Some(TokenKind::Dot) && self.struct_1d_array_lval_base(&lv) {
                // §4.5.190 (write): `arr[i].field = …` on a PACKED-STRUCT 1-D array —
                // a part-select lvalue on the packed element (mirrors the scalar
                // `s.field` write). Checked before the generate-array hier path.
                lv = self.parse_struct_array_member_lval(lv);
            } else if self.peek() == Some(TokenKind::Dot) && Self::is_indexed_hier_lval(&lv) {
                // HIER-REST②: `g[0].x = …` — fold the constant index into the
                // scope-segment name, mirroring the expression side.
                lv = self.parse_indexed_hier_lval(lv);
            } else {
                break;
            }
        }
        // §4.5.418: a select chain WRITE on a multi-dim packed formal (`o[i] = …`)
        // takes the same flat rewrite as the read side (`expr.rs`); a no-op for
        // every other lvalue (the rewrite keys on the chain's root name).
        self.rewrite_packed_md_lvalue(lv)
    }

    /// The lvalue twin of `rewrite_packed_md_select`: a pure select chain rooted at a
    /// bare name the packed-md table binds is round-tripped through the expression
    /// rewrite. Any other lvalue (hier path, concat, struct-folded part-select whose
    /// root is not bound, error) is returned untouched.
    fn rewrite_packed_md_lvalue(&mut self, lv: Lvalue) -> Lvalue {
        if self.packed_md_params.is_empty() {
            return lv;
        }
        let mut cur = &lv;
        let mut depth = 0usize;
        while let Lvalue::BitSelect { base, .. }
        | Lvalue::PartSelect { base, .. }
        | Lvalue::IndexedPart { base, .. } = cur
        {
            depth += 1;
            cur = base;
        }
        let bound = depth > 0
            && matches!(cur, Lvalue::Ident(p)
                if p.segments.len() == 1 && self.packed_md_params.contains_key(&p.segments[0].name));
        if !bound {
            return lv;
        }
        let e = Self::lvalue_to_expr(&lv);
        let e = self.rewrite_packed_md_select(e);
        self.expr_to_lvalue(e)
    }

    /// True when `lv` is `name[idx]` — a bit-select lvalue rooted at a plain Ident.
    pub(crate) fn is_indexed_hier_lval(lv: &Lvalue) -> bool {
        matches!(lv, Lvalue::BitSelect { base, .. } if matches!(**base, Lvalue::Ident(_)))
    }

    /// LVALUE twin of [`Self::parse_indexed_hier`]: `g[0].x = …`.
    pub(crate) fn parse_indexed_hier_lval(&mut self, base: Lvalue) -> Lvalue {
        let start = base.span();
        let mut segs: Vec<Ident> = Vec::new();
        if let Lvalue::BitSelect { base: b, index, .. } = base {
            if let Lvalue::Ident(p) = *b {
                let n = p.segments.len();
                for (i, seg) in p.segments.into_iter().enumerate() {
                    if i + 1 == n {
                        let idx_str = self.const_index_string(&index);
                        segs.push(Ident {
                            name: format!("{}[{idx_str}]", seg.name),
                            span: seg.span,
                        });
                    } else {
                        segs.push(seg);
                    }
                }
            }
        }
        while self.eat(TokenKind::Dot) {
            match self.ident() {
                Some(id) => segs.push(id),
                None => break,
            }
        }
        let hi = segs.last().map(|s| s.span).unwrap_or(start);
        Lvalue::Ident(HierPath {
            segments: segs,
            span: start.to(hi),
        })
    }
}
