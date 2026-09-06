//! struct member selects — split out of the original `hdl-parser` lib.rs (mechanical
//! move). The `'{…}` assignment-pattern parsing and desugars that used to live here
//! moved to `assign_pattern.rs` when V34-3 pushed this file past the 1000-line policy;
//! they still resolve against the same `StructLayout` this module reads.

use super::*;

impl Parser<'_, '_> {
    /// If `path` is `var.field` where `var` is a packed-struct variable and `field`
    /// is one of its members, return `(base_path_to_var, lsb_offset, width,
    /// ascending, signed)`.
    ///
    /// §3 ⑤ ⓓ: a CHAIN `var.f.g[.h…]` through NESTED packed-struct members
    /// resolves to the leaf's geometry at the summed offset (`struct_field_chain`).
    /// Any segment that is not a member of the layout reached so far — including
    /// a member past a non-struct member — returns `None`, exactly as before.
    pub(crate) fn struct_field_select(
        &self,
        path: &HierPath,
    ) -> Option<(HierPath, FieldGeom, Option<String>)> {
        if path.segments.len() < 2 {
            return None;
        }
        let tyname = self.var_struct.get(&path.segments[0].name)?;
        // A PACKED struct (`struct_layouts`) first; then a packable UNPACKED record
        // (§4.5.192 packed-vector body-local, mirrors `struct_array_field_geom`) via
        // `packable_record_layout` — both yield the same `StructLayout::field()` shape.
        let layout = self
            .struct_layouts
            .get(tyname)
            .cloned()
            .or_else(|| self.packable_record_layout(tyname))?;
        let names: Vec<&str> = path.segments[1..].iter().map(|s| s.name.as_str()).collect();
        let (geom, nested) = self.struct_field_chain(&layout, &names)?;
        let base = HierPath {
            segments: vec![path.segments[0].clone()],
            span: path.segments[0].span,
        };
        Some((base, geom, nested))
    }

    /// §3 ⑤ ⓒ: `var.field` on a SYMBOLIC-layout struct variable — one level only
    /// (a chain into a nested member is not laid out symbolically; `None` keeps it
    /// loud downstream).
    pub(crate) fn sym_struct_field_select(&self, path: &HierPath) -> Option<(HierPath, SymGeom)> {
        if path.segments.len() != 2 {
            return None;
        }
        let tyname = self.var_struct.get(&path.segments[0].name)?;
        let geom = self
            .sym_struct_layouts
            .get(tyname)?
            .field(&path.segments[1].name)?;
        let base = HierPath {
            segments: vec![path.segments[0].clone()],
            span: path.segments[0].span,
        };
        Some((base, geom))
    }

    /// §3 ⑤ ⓒ: the symbolic geometry of `arr[i].field` for a packed / 1-D array
    /// of a symbolic-layout struct.
    pub(crate) fn sym_struct_array_field_geom(&self, arr: &str, field: &str) -> Option<SymGeom> {
        let tyname = self.var_struct.get(arr)?;
        self.sym_struct_layouts.get(tyname)?.field(field)
    }

    /// §3 ⑤ ⓒ: the `[msb:lsb]` of a symbolic member — `off + w - 1` and `off`,
    /// literals folded.
    fn sym_bounds(geom: &SymGeom, span: Span) -> (Expr, Expr) {
        let (off, w, _) = geom;
        let hi = Self::sym_sub_one(Self::sum_of(&[off.clone(), w.clone()], span), span);
        (hi, off.clone())
    }

    /// §3 ⑤ ⓒ: the READ of a symbolic member: the field part-select, sign-wrapped
    /// for a signed member like the numeric twin. A trailing sub-select (`s.f[i]`,
    /// `s.f[a:b]`) is loud — the field-relative remap needs the width.
    pub(crate) fn sym_member_expr_of(
        &mut self,
        base_expr: Expr,
        geom: SymGeom,
        span: Span,
    ) -> Expr {
        let (hi, lo) = Self::sym_bounds(&geom, span);
        let pv = Expr {
            kind: ExprKind::PartSelect {
                base: Box::new(base_expr),
                msb: Box::new(hi),
                lsb: Box::new(lo),
            },
            span,
        };
        if self.peek() == Some(TokenKind::LBracket) {
            self.error(
                "a whole-member read — a sub-select of a packed-struct member whose width \
                 names a header parameter is unsupported in v1",
            );
        }
        if geom.2 {
            return Expr {
                kind: ExprKind::Cast {
                    target: CastTarget::Signing { signed: true },
                    expr: Box::new(pv),
                },
                span,
            };
        }
        pv
    }

    /// §3 ⑤ ⓒ: `var.field` READ on a symbolic-layout struct variable, resolved and
    /// built here so `expr_primary`'s recursive frame holds only the `Option`
    /// (`#[inline(never)]`, like [`Self::struct_member_expr`] — the parser's
    /// `depth_guard` test overflowed with the select inline).
    #[inline(never)]
    pub(crate) fn sym_struct_member_expr(&mut self, path: &HierPath) -> Option<Expr> {
        let (base, geom) = self.sym_struct_field_select(path)?;
        let span = path.span;
        let base_expr = Expr {
            kind: ExprKind::Ident(base),
            span,
        };
        Some(self.sym_member_expr_of(base_expr, geom, span))
    }

    /// §3 ⑤ ⓒ: `var.field = …` on a symbolic-layout struct variable — the WRITE
    /// twin of [`Self::sym_struct_member_expr`], cold for the same reason.
    #[inline(never)]
    pub(crate) fn sym_struct_member_lval(&mut self, path: &HierPath) -> Option<Lvalue> {
        let (base, geom) = self.sym_struct_field_select(path)?;
        let span = path.span;
        Some(self.sym_member_lval_of(Lvalue::Ident(base), geom, span))
    }

    /// §3 ⑤ ⓒ: the WRITE twin of [`Self::sym_member_expr_of`].
    pub(crate) fn sym_member_lval_of(&mut self, base: Lvalue, geom: SymGeom, span: Span) -> Lvalue {
        let (hi, lo) = Self::sym_bounds(&geom, span);
        if self.peek() == Some(TokenKind::LBracket) {
            self.error(
                "a whole-member write — a sub-select write of a packed-struct member whose \
                 width names a header parameter is unsupported in v1",
            );
        }
        Lvalue::PartSelect {
            base: Box::new(base),
            msb: Box::new(hi),
            lsb: Box::new(lo),
            span,
        }
    }

    /// §3 ⑤ ⓓ: resolve member NAMES `f.g.h…` against `layout`, descending into a
    /// nested struct member's own layout at each step. Returns the LEAF geometry
    /// `(off, w, ascending, signed, dbase, stride)` — `off` summed along the chain,
    /// everything else the leaf's — plus the leaf's nested type key (`Some` when the
    /// leaf is itself a struct, so a caller can keep chaining). `None` when a name
    /// is not a member of the layout reached, or a further name follows a
    /// non-struct member.
    pub(crate) fn struct_field_chain(
        &self,
        layout: &StructLayout,
        names: &[&str],
    ) -> Option<(FieldGeom, Option<String>)> {
        let (first, rest) = names.split_first()?;
        let (mut off, mut w, mut asc, mut sgn, mut dbase, mut stride) = layout.field(first)?;
        let mut nested = layout.nested_of(first).map(str::to_string);
        for name in rest {
            let nl = self.struct_layouts.get(nested.as_deref()?)?;
            let (o2, w2, asc2, sgn2, db2, st2) = nl.field(name)?;
            off += o2;
            (w, asc, sgn, dbase, stride) = (w2, asc2, sgn2, db2, st2);
            nested = nl.nested_of(name).map(str::to_string);
        }
        Some(((off, w, asc, sgn, dbase, stride), nested))
    }

    /// §3 ⑤ ⓓ: after `arr[i].field` resolved to `geom` whose member is a nested
    /// struct (`nested`), consume any further `.name` tokens down the nesting
    /// (`arr[i].perms.EX`). Stops at the first `.` that is not followed by an
    /// identifier (an enum method's `.name()` on a leaf, say — left to the caller /
    /// loud downstream) or at an unknown member (loud here). Returns the leaf
    /// geometry.
    pub(crate) fn extend_member_chain(
        &mut self,
        mut geom: FieldGeom,
        mut nested: Option<String>,
    ) -> (FieldGeom, Option<String>) {
        while let Some(nty) = nested.clone() {
            if self.peek() != Some(TokenKind::Dot)
                || !matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
            {
                break;
            }
            self.bump(); // '.'
            let Some(name) = self.ident() else { break };
            match self.struct_layouts.get(&nty).and_then(|l| {
                l.field(&name.name)
                    .map(|g| (g, l.nested_of(&name.name).map(str::to_string)))
            }) {
                Some(((o2, w2, asc2, sgn2, db2, st2), n2)) => {
                    geom = (geom.0 + o2, w2, asc2, sgn2, db2, st2);
                    nested = n2;
                }
                None => {
                    self.error_at(name.span, "a member of the nested struct in a member chain");
                    nested = None;
                    break;
                }
            }
        }
        (geom, nested)
    }

    /// N3: is `e` an `arr[i]` bit-select whose base is a record-ARRAY var (either the
    /// packed single-net representation or a SoA per-field representation)?
    pub(crate) fn record_array_member_base(&self, e: &Expr) -> bool {
        if let ExprKind::BitSelect { base, .. } = &e.kind {
            if let ExprKind::Ident(p) = &base.kind {
                let nm = &p.segments[0].name;
                return p.segments.len() == 1
                    && (self.record_array_vars.contains_key(nm)
                        || self.record_soa_vars.contains_key(nm));
            }
        }
        false
    }

    /// N3: parse `arr[i].field` (cursor at `.`) → a PART-SELECT on the element value
    /// `arr[i]` at the field's packed offset (MSB-first). `arr[i]` is a dyn element
    /// read; the part-select extracts the field. An unknown field is loud.
    pub(crate) fn parse_record_array_member(&mut self, base: Expr) -> Expr {
        self.bump(); // '.'
        let field = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        // N3 SoA: `arr[i].field` → `$unp$arr$field[i]` — a native dyn element access on
        // the member's own typed dyn array (a trailing sub-select `[…]` then applies
        // natively via expr_postfix, correctly typed/signed). Checked before the packed
        // single-net path.
        if let ExprKind::BitSelect { base: b, index } = &base.kind {
            if let ExprKind::Ident(p) = &b.kind {
                if p.segments.len() == 1 && self.record_soa_vars.contains_key(&p.segments[0].name) {
                    let span = base.span.to(self.prev_span());
                    match self.soa_member_field(&p.segments[0].name, &field.name) {
                        Some(mnet) => {
                            return Expr {
                                kind: ExprKind::BitSelect {
                                    base: Box::new(Self::ident_expr(&mnet, span)),
                                    index: index.clone(),
                                },
                                span,
                            };
                        }
                        None => {
                            self.error("unknown field in a record-array element member access");
                            return base;
                        }
                    }
                }
            }
        }
        let tyname = match &base.kind {
            ExprKind::BitSelect { base: b, .. } => match &b.kind {
                ExprKind::Ident(p) if p.segments.len() == 1 => {
                    self.record_array_vars.get(&p.segments[0].name).cloned()
                }
                _ => None,
            },
            _ => None,
        };
        let span = base.span.to(self.prev_span());
        // (off, width, ascending, signed, dbase) — mirror the scalar packed-struct path:
        // a signed WHOLE-field read is `$signed`-wrapped; a sub-select stays unsigned.
        let field = tyname
            .as_ref()
            .and_then(|t| self.packable_record_layout(t))
            .and_then(|l| {
                l.fields
                    .iter()
                    .find(|f| f.0 == field.name)
                    .map(|f| (f.1, f.2, f.3, f.4, f.6))
            });
        match field {
            Some((off, w, ascending, signed, dbase)) => {
                let lit = |v: u32| Expr {
                    kind: ExprKind::IntLit {
                        kind: IntLitKind::Decimal,
                        raw: v.to_string(),
                    },
                    span,
                };
                // `arr[i][off+w-1 : off]` — the field, normalized to `[w-1:0]` (unsigned).
                let field_sel = Expr {
                    kind: ExprKind::PartSelect {
                        base: Box::new(base),
                        msb: Box::new(lit(off + w - 1)),
                        lsb: Box::new(lit(off)),
                    },
                    span,
                };
                // A trailing sub-select (`arr[i].f[hi:lo]` / `arr[i].f[k]`) follows?
                if self.peek() == Some(TokenKind::LBracket) {
                    // The `[w-1:0]`-normalized value matches the field's own coordinates
                    // ONLY for a descending, zero-declared-LSB member. A NON-zero-LSB or
                    // ASCENDING member needs a `dbase` remap this path does not perform
                    // (§4.5.113 family) → correct-or-loud: reject, never read raw/OOB
                    // bits (silent X). expr_postfix then applies the sub-select (unsigned
                    // per §5.4.1, matching the scalar packed-struct path).
                    if dbase != 0 || ascending {
                        self.error(
                            "a whole-field read of this non-zero-LSB or ascending \
                             record-array member (its sub-select is unsupported)",
                        );
                    }
                    return field_sel;
                }
                // Whole-field read: a signed member reads back sign-extended (§5.4.1).
                if signed {
                    Expr {
                        kind: ExprKind::SysCall {
                            name: Ident {
                                name: "$signed".to_string(),
                                span,
                            },
                            args: vec![field_sel],
                        },
                        span,
                    }
                } else {
                    field_sel
                }
            }
            None => {
                self.error("unknown field in a record-array element member access");
                base
            }
        }
    }

    /// §4.5.190: is `e` an `arr[i]` element-select whose base is a PACKED-STRUCT 1-D
    /// array var (`struct_1d_array_vars`)? Gates the `arr[i].field` member desugar.
    pub(crate) fn struct_1d_array_member_base(&self, e: &Expr) -> bool {
        if let ExprKind::BitSelect { base, .. } = &e.kind {
            if let ExprKind::Ident(p) = &base.kind {
                return p.segments.len() == 1
                    && (self.struct_1d_array_vars.contains(&p.segments[0].name)
                        || self.struct_packed_array_vars.contains(&p.segments[0].name));
            }
        }
        false
    }

    /// The packed-struct field geometry `(off, w, ascending, signed, dbase, stride)`
    /// for element member `arr[i].field` — `arr`'s struct type (`var_struct`) laid out
    /// in `struct_layouts`. `None` for an unknown field.
    /// §3 ⑤ ⓓ: also the member's nested struct type key, for a chain
    /// (`extend_member_chain`).
    pub(crate) fn struct_array_field_geom(
        &self,
        arr: &str,
        field: &str,
    ) -> Option<(FieldGeom, Option<String>)> {
        let tyname = self.var_struct.get(arr)?;
        // A PACKED struct (`struct_layouts`) first; then a packable UNPACKED record
        // (§4.5.191 fixed record array) via `packable_record_layout` — both yield the
        // same `StructLayout::field()` `(off, w, asc, sgn, dbase, stride)` shape.
        if let Some(l) = self.struct_layouts.get(tyname) {
            let g = l.field(field)?;
            return Some((g, l.nested_of(field).map(str::to_string)));
        }
        let l = self.packable_record_layout(tyname)?;
        Some((l.field(field)?, None))
    }

    /// §4.5.190 (read): parse `arr[i].field` (cursor at `.`) → a part-select on the
    /// packed element value, reusing the scalar `struct_member_expr_of` machinery
    /// (whole-field sign wrap + trailing sub-select + multi-dim member stride).
    pub(crate) fn parse_struct_array_member(&mut self, base: Expr) -> Expr {
        self.bump(); // '.'
        let field = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        let span = base.span.to(self.prev_span());
        let arr = match &base.kind {
            ExprKind::BitSelect { base: b, .. } => match &b.kind {
                ExprKind::Ident(p) if p.segments.len() == 1 => Some(p.segments[0].name.clone()),
                _ => None,
            },
            _ => None,
        };
        match arr
            .as_deref()
            .and_then(|nm| self.struct_array_field_geom(nm, &field.name))
        {
            Some((geom, nested)) => {
                let (geom, _) = self.extend_member_chain(geom, nested);
                let span = span.to(self.prev_span());
                self.struct_member_expr_of(base, geom, span)
            }
            None => match arr
                .as_deref()
                .and_then(|nm| self.sym_struct_array_field_geom(nm, &field.name))
            {
                Some(geom) => self.sym_member_expr_of(base, geom, span),
                None => {
                    self.error("unknown field in a struct-array element member access");
                    base
                }
            },
        }
    }

    /// N3 (write): is `lv` an `arr[i]` element-select whose base is a record-ARRAY var?
    /// The LVALUE twin of [`record_array_member_base`].
    pub(crate) fn record_array_lval_base(&self, lv: &Lvalue) -> bool {
        if let Lvalue::BitSelect { base, .. } = lv {
            if let Lvalue::Ident(p) = base.as_ref() {
                let nm = &p.segments[0].name;
                return p.segments.len() == 1
                    && (self.record_array_vars.contains_key(nm)
                        || self.record_soa_vars.contains_key(nm));
            }
        }
        false
    }

    /// N3 (write): parse `arr[i].field = …` (cursor at `.`) → a PART-SELECT lvalue on
    /// the dyn element at the field's packed offset — the WRITE twin of
    /// [`parse_record_array_member`]. The engine deposits the field bits with a
    /// read-modify-write on the element (`dyn_write`). A whole-field write of a
    /// non-zero-LSB member is fine (the field occupies packed bits `[off, off+w)`
    /// regardless of its declared LSB), but a member SUB-select (`arr[i].f[a:b] = …`)
    /// needs a `dbase` remap this path does not do → correct-or-LOUD.
    /// §4.5.190 (write): is `lv` an `arr[i]` element-select whose base is a
    /// PACKED-STRUCT 1-D array var? The LVALUE twin of [`struct_1d_array_member_base`].
    pub(crate) fn struct_1d_array_lval_base(&self, lv: &Lvalue) -> bool {
        if let Lvalue::BitSelect { base, .. } = lv {
            if let Lvalue::Ident(p) = base.as_ref() {
                return p.segments.len() == 1
                    && (self.struct_1d_array_vars.contains(&p.segments[0].name)
                        || self.struct_packed_array_vars.contains(&p.segments[0].name));
            }
        }
        false
    }

    /// §4.5.190 (write): parse `arr[i].field = …` (cursor at `.`) on a packed-struct
    /// 1-D array → a part-select lvalue on the packed element (whole-field), or the
    /// generalized `parse_struct_field_lval` for a trailing sub-select. The WRITE twin
    /// of [`Self::parse_struct_array_member`].
    pub(crate) fn parse_struct_array_member_lval(&mut self, base: Lvalue) -> Lvalue {
        self.bump(); // '.'
        let field = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        let span = base.span().to(self.prev_span());
        let arr = match &base {
            Lvalue::BitSelect { base: b, .. } => match b.as_ref() {
                Lvalue::Ident(p) if p.segments.len() == 1 => Some(p.segments[0].name.clone()),
                _ => None,
            },
            _ => None,
        };
        match arr
            .as_deref()
            .and_then(|nm| self.struct_array_field_geom(nm, &field.name))
        {
            Some((geom, nested)) => {
                let ((off, w, asc, _sgn, dbase, stride), leaf_nested) =
                    self.extend_member_chain(geom, nested);
                let span = span.to(self.prev_span());
                if self.peek() != Some(TokenKind::LBracket) {
                    self.member_pattern_ty = leaf_nested;
                }
                if self.peek() == Some(TokenKind::LBracket) {
                    self.parse_struct_field_lval(base, (off, w, asc, dbase, stride), span)
                } else {
                    Lvalue::PartSelect {
                        base: Box::new(base),
                        msb: Box::new(Self::dec_lit(off + w - 1, span)),
                        lsb: Box::new(Self::dec_lit(off, span)),
                        span,
                    }
                }
            }
            None => match arr
                .as_deref()
                .and_then(|nm| self.sym_struct_array_field_geom(nm, &field.name))
            {
                Some(geom) => self.sym_member_lval_of(base, geom, span),
                None => {
                    self.error("unknown field in a struct-array element member write");
                    base
                }
            },
        }
    }

    pub(crate) fn parse_record_array_member_lval(&mut self, base: Lvalue) -> Lvalue {
        self.bump(); // '.'
        let field = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        let span = base.span().to(self.prev_span());
        // N3 SoA: `arr[i].field = …` → `$unp$arr$field[i] = …` — a native, correctly-
        // typed dyn element write (the WRITE twin of the SoA read rewrite). Checked
        // before the packed part-select path.
        if let Lvalue::BitSelect { base: b, index, .. } = &base {
            if let Lvalue::Ident(p) = b.as_ref() {
                if p.segments.len() == 1 && self.record_soa_vars.contains_key(&p.segments[0].name) {
                    return match self.soa_member_field(&p.segments[0].name, &field.name) {
                        Some(mnet) => Lvalue::BitSelect {
                            base: Box::new(Self::ident_lval(&mnet, span)),
                            index: index.clone(),
                            span,
                        },
                        None => {
                            self.error("unknown field in a record-array element member write");
                            base
                        }
                    };
                }
            }
        }
        let tyname = match &base {
            Lvalue::BitSelect { base: b, .. } => match b.as_ref() {
                Lvalue::Ident(p) if p.segments.len() == 1 => {
                    self.record_array_vars.get(&p.segments[0].name).cloned()
                }
                _ => None,
            },
            _ => None,
        };
        let off_w = tyname
            .as_ref()
            .and_then(|t| self.packable_record_layout(t))
            .and_then(|l| {
                l.fields
                    .iter()
                    .find(|f| f.0 == field.name)
                    .map(|f| (f.1, f.2))
            });
        match off_w {
            Some((off, w)) => {
                // A member SUB-select write (`arr[i].f[…] = …`) is unsupported (no dbase
                // remap on the write path) → loud, never a silent wrong.
                if self.peek() == Some(TokenKind::LBracket) {
                    self.error(
                        "a whole-field write of this record-array member \
                         (a member sub-select write is unsupported)",
                    );
                }
                Lvalue::PartSelect {
                    base: Box::new(base),
                    msb: Box::new(Self::dec_lit(off + w - 1, span)),
                    lsb: Box::new(Self::dec_lit(off, span)),
                    span,
                }
            }
            None => {
                self.error("unknown field in a record-array element member write");
                base
            }
        }
    }

    /// Build the read-side `Expr` for a packed-struct member access. The base is
    /// always the field part-select `pv = s[off+w-1 : off]`; a trailing sub-select
    /// becomes an `IndexedPart` on `pv` (FIELD-bounded, direction-aware).
    /// `#[inline(never)]` keeps these locals out of the recursive `expr_primary`
    /// frame (see MAX_EXPR_DEPTH).
    ///
    /// `sgn` is the member's effective signedness. A WHOLE-field read of a signed
    /// member (`int`/`byte`/… or a `signed`-qualified vector) is wrapped in a
    /// `signed'(pv)` cast so it reads back negative — a packed-struct member ref is
    /// TYPED, not a raw part-select, so iverilog preserves member signedness here.
    /// A sub-select (`s.f[a:b]`) stays unsigned (§5.4.1), matching iverilog.
    #[inline(never)]
    /// `geom` = the resolved member geometry `(flat_off, width, ascending, signed,
    /// dbase)` from [`Self::struct_field_select`] (bundled to keep the arg count
    /// under clippy's limit).
    pub(crate) fn struct_member_expr(
        &mut self,
        base: HierPath,
        geom: (u32, u32, bool, bool, i64, u32),
        span: Span,
    ) -> Expr {
        let base_expr = Expr {
            kind: ExprKind::Ident(base),
            span,
        };
        self.struct_member_expr_of(base_expr, geom, span)
    }

    /// §4.5.190: the same packed-struct member desugar as [`Self::struct_member_expr`]
    /// but over an ARBITRARY base expression (a `BitSelect` array element `arr[i]`),
    /// not just a whole-variable `Ident`. `arr[i].field` becomes a part-select on the
    /// element value `arr[i][off+w-1 : off]`, reusing the entire scalar machinery
    /// (whole-field sign wrap, trailing sub-select, multi-dim member stride).
    pub(crate) fn struct_member_expr_of(
        &mut self,
        base_expr: Expr,
        geom: (u32, u32, bool, bool, i64, u32),
        span: Span,
    ) -> Expr {
        let (off, w, asc, sgn, dbase, stride) = geom;
        let pv = Expr {
            kind: ExprKind::PartSelect {
                base: Box::new(base_expr),
                msb: Box::new(Self::dec_lit(off + w - 1, span)),
                lsb: Box::new(Self::dec_lit(off, span)),
            },
            span,
        };
        match self.parse_struct_field_sel(w, asc, dbase, stride) {
            FieldSel::Whole if sgn => Expr {
                kind: ExprKind::Cast {
                    target: CastTarget::Signing { signed: true },
                    expr: Box::new(pv),
                },
                span,
            },
            FieldSel::Whole => pv,
            FieldSel::Indexed { offset, width, dir } => Expr {
                kind: ExprKind::IndexedPart {
                    base: Box::new(pv),
                    offset: Box::new(offset),
                    width: Box::new(width),
                    dir,
                },
                span: span.to(self.prev_span()),
            },
        }
    }

    /// Parse the trailing `[...]` of a packed-struct member READ sub-select and
    /// normalize it to one indexed part-select on the field part-select `pv`. No
    /// `[` ⇒ `Whole`. Every form is FIELD-bounded by `pv` (OOB reads X). `dbase` is
    /// the member's declared base index, removed from each source index so a
    /// NON-zero-LSB member (`logic [15:8] a; s.a[11:8]`) selects field-relative bits
    /// (see [`Self::remap_pv_idx`]). For an ascending member the `+:`/`-:` direction
    /// flips and the offset mirrors, matching an ascending NET part-select; a
    /// reversed regular range (`s.f[3:0]` on `logic [0:N]`, or `s.f[0:3]` on
    /// `logic [N:0]`) is a loud parse error.
    pub(crate) fn parse_struct_field_sel(
        &mut self,
        w: u32,
        ascending: bool,
        dbase: i64,
        elem_stride: u32,
    ) -> FieldSel {
        if self.peek() != Some(TokenKind::LBracket) {
            return FieldSel::Whole;
        }
        // A NEGATIVE-LSB member (`logic [7:-4]`) sub-select needs signed field-
        // relative offsets across every form (deep); loud-reject it (the whole-field
        // read returned above is unaffected). The error fails the compile, so the
        // node produced by the (clamped) fall-through below is never simulated.
        if dbase < 0 {
            self.error(
                "a whole-member read — a sub-select of a packed-struct member with a \
                 NEGATIVE declared LSB (`logic [7:-4]`) is unsupported in v1",
            );
        }
        let dbase = dbase.max(0) as u32;
        self.bump(); // '['
        let first = self.expr(0);
        // Multi-dim packed member (`logic [1:0][3:0] m`): a bare `m[i]` reads the i-th
        // `elem_stride`-bit ELEMENT (`pv[i*stride +: stride]`), not a single bit. Only
        // the common DESCENDING, ZERO-BASED outer dim and the bare `[i]` form are
        // supported; an ascending / non-zero-base outer dim, a range/indexed/`m[i][j]`
        // sub-select, stays loud (correct-or-loud — a follow-on). `i` may be runtime.
        if elem_stride > 1 {
            let sp = first.span;
            let sel = if self.peek() == Some(TokenKind::RBracket) && !ascending && dbase == 0 {
                FieldSel::Indexed {
                    offset: mk_bin(BinOp::Mul, first, Self::dec_lit(elem_stride, sp)),
                    width: Self::dec_lit(elem_stride, sp),
                    dir: PartDir::PlusColon,
                }
            } else {
                self.error(
                    "a multi-dimensional packed struct/union member supports only a \
                     whole-member read or a constant/runtime first-level element select \
                     `m[i]` on a descending zero-based outer dim in v1 (a range / indexed \
                     `[i±:w]` / `m[i][j]` / ascending or non-zero-base outer dim is loud)",
                );
                FieldSel::Indexed {
                    offset: Self::dec_lit(0, sp),
                    width: Self::dec_lit(elem_stride, sp),
                    dir: PartDir::PlusColon,
                }
            };
            self.expect(TokenKind::RBracket, "']'");
            return sel;
        }
        let sel = match self.peek() {
            // regular `[a:b]` — bounds must be constant and run in the member's
            // declared direction (a≥b descending, a≤b ascending). Normalize to the
            // equivalent indexed part `[min(a,b) +: |a-b|+1]` and reuse the indexed
            // remap below, so an out-of-field range X-extends on the correct end
            // (the indexed path is differentially validated against the NET oracle).
            Some(TokenKind::Colon) => {
                self.bump();
                let last = self.expr(0);
                match (Self::const_lit(&first), Self::const_lit(&last)) {
                    (Some(a), Some(b)) if (ascending && a <= b) || (!ascending && a >= b) => {
                        let lo = a.min(b).max(0) as u32;
                        let width = (a - b).unsigned_abs() as u32 + 1;
                        let dir = if ascending {
                            PartDir::MinusColon
                        } else {
                            PartDir::PlusColon
                        };
                        // Out-of-field-LOW (source below the member's declared base)
                        // reads X, exactly like an out-of-field-HIGH select (which
                        // pv's own bounds X-extend). Address a far-OOB bit so the
                        // whole select reads X — matching iverilog — rather than
                        // underflowing the field-relative index. Only reachable when
                        // `dbase > 0` (a non-zero-LSB member), so the zero-base path
                        // stays byte-identical.
                        let offset = if lo < dbase {
                            Self::dec_lit(OOB_DROP_BIT, first.span)
                        } else {
                            Self::remap_pv_idx(w, ascending, dbase, Self::dec_lit(lo, first.span))
                        };
                        FieldSel::Indexed {
                            offset,
                            width: Self::dec_lit(width, first.span),
                            dir,
                        }
                    }
                    _ => {
                        self.error_at(
                            first.span,
                            "packed-struct member part-select must be a constant range in the \
                             member's declared direction",
                        );
                        FieldSel::Indexed {
                            offset: Self::dec_lit(0, first.span),
                            width: Self::dec_lit(1, first.span),
                            dir: PartDir::PlusColon,
                        }
                    }
                }
            }
            Some(TokenKind::PlusColon) => {
                self.bump();
                let width = self.expr(0);
                let dir = if ascending {
                    PartDir::MinusColon
                } else {
                    PartDir::PlusColon
                };
                FieldSel::Indexed {
                    offset: Self::remap_pv_idx(w, ascending, dbase, first),
                    width,
                    dir,
                }
            }
            Some(TokenKind::MinusColon) => {
                self.bump();
                let width = self.expr(0);
                let dir = if ascending {
                    PartDir::PlusColon
                } else {
                    PartDir::MinusColon
                };
                FieldSel::Indexed {
                    offset: Self::remap_pv_idx(w, ascending, dbase, first),
                    width,
                    dir,
                }
            }
            // bit-select `[i]` — a width-1 indexed part-select on `pv`.
            _ => {
                let span = first.span;
                FieldSel::Indexed {
                    offset: Self::remap_pv_idx(w, ascending, dbase, first),
                    width: Self::dec_lit(1, span),
                    dir: PartDir::PlusColon,
                }
            }
        };
        self.expect(TokenKind::RBracket, "']'");
        sel
    }

    /// Parse the trailing `[...]` of a packed-struct member WRITE sub-select and
    /// fold it to a FLAT, field-bounded lvalue on the struct net `base[total-1:0]`.
    /// The cursor is on the `[`. `off`/`w`/`asc` are the member's flat offset, width
    /// and declared direction (ascending = `logic [0:N]`, source index 0 = field
    /// MSB); `dbase` is the member's declared base index (`min(msb,lsb)`), removed
    /// from each source index so a NON-zero-LSB member (`logic [15:8] a`) writes the
    /// right bits. This is the WRITE twin of [`Self::parse_struct_field_sel`]: every
    /// form maps a SOURCE index `k` (field-relative `r = k - dbase`) onto flat bit
    /// `off + r` (descending) or `off + (w-1-r)` (ascending), so the write stays
    /// inside the member region — never leaking into an adjacent member.
    ///
    /// SCOPE (correct-or-loud): only a CONSTANT range `[a:b]` running in the
    /// member's declared direction and a CONSTANT bit-select `[i]` are folded —
    /// these are exactly the forms iverilog 13.0 supports for a struct-member
    /// write. An indexed `[i±:w]`, a runtime/non-constant index, or a reversed
    /// range is a loud parse error (iverilog refuses the indexed/runtime forms
    /// outright; the reversed range stays loud to match the READ side). An OOB
    /// bit-select drops (no-op), matching iverilog; an OOB range is loud (iverilog
    /// itself asserts on it — no oracle).
    pub(crate) fn parse_struct_field_lval(
        &mut self,
        // §4.5.190: the base LVALUE the field lives in — a whole-variable `Ident`
        // (scalar `s.field`) OR an array element `arr[i]` BitSelect (`arr[i].field`).
        base_lv: Lvalue,
        // (off, width, ascending, dbase, elem_stride) — bundled to keep the arg count
        // down (mirrors the READ twin `struct_member_expr`'s `geom`).
        geom: (u32, u32, bool, i64, u32),
        span: Span,
    ) -> Lvalue {
        let (off, w, asc, dbase, stride) = geom;
        // Negative-LSB member WRITE sub-select: loud (signed field-relative offsets,
        // like the READ twin `parse_struct_field_sel`). Emitted BEFORE consuming `[`
        // so the diagnostic's `found` token is the sub-select `[` (matching the READ
        // twin), not the post-`[` token. The whole-field write does not reach here;
        // the clamped fall-through node is never simulated.
        if dbase < 0 {
            self.error_at(
                span,
                "a whole-member write — a sub-select WRITE of a packed-struct member \
                 with a NEGATIVE declared LSB (`logic [7:-4]`) is unsupported in v1",
            );
        }
        // A multi-dim packed member ELEMENT write (`s.m[i] = …`) is a follow-on: the
        // READ side supports `s.m[i]` (element select), but the WRITE twin's flat
        // field-relative fold is bit-oriented, so an element write stays loud
        // (correct-or-loud). The whole-member write `s.m = …` does NOT reach here.
        if stride > 1 {
            self.error_at(
                span,
                "a multi-dimensional packed struct/union member ELEMENT write \
                 `s.m[i] = …` is unsupported in v1 (whole-member `s.m = …` is supported; \
                 element READ `s.m[i]` is supported)",
            );
        }
        let dbase = dbase.max(0) as u32;
        self.bump(); // '['
        let first = self.expr(0);
        // The member's declared source range is `[dbase, dbase+w)`; a field-relative
        // index `r = k - dbase` (0 for a zero-base member, so byte-identical).
        let hi = dbase as i64 + w as i64;
        match self.peek() {
            // Regular `[a:b]` — must be constant and run in the member's direction.
            Some(TokenKind::Colon) => {
                self.bump();
                let last = self.expr(0);
                let end = self.cur_span();
                self.expect(TokenKind::RBracket, "']'");
                match (Self::const_lit(&first), Self::const_lit(&last)) {
                    // In-direction range, both bounds inside the field [dbase, dbase+w).
                    (Some(a), Some(b))
                        if ((asc && a <= b) || (!asc && a >= b))
                            && a >= dbase as i64
                            && b >= dbase as i64
                            && a < hi
                            && b < hi =>
                    {
                        let (ka, kb) = (a as u32 - dbase, b as u32 - dbase);
                        // Map field-relative MSB/LSB index onto the flat vector.
                        // Ascending: index r → flat `off + (w-1-r)`; descending: `off + r`.
                        let (fmsb, flsb) = if asc {
                            (off + (w - 1 - ka), off + (w - 1 - kb))
                        } else {
                            (off + ka, off + kb)
                        };
                        Lvalue::PartSelect {
                            base: Box::new(base_lv),
                            msb: Box::new(Self::dec_lit(fmsb, span)),
                            lsb: Box::new(Self::dec_lit(flsb, span)),
                            span: span.to(self.prev_span()),
                        }
                    }
                    _ => {
                        self.error_at(
                            span.to(end),
                            "a constant in-bounds packed-struct member range WRITE in the \
                             member's declared direction",
                        );
                        Lvalue::Error(span.to(self.prev_span()))
                    }
                }
            }
            // Bit-select `[i]` — constant index; OOB drops (no-op), matching iverilog.
            Some(TokenKind::RBracket) => {
                self.bump(); // ']'
                match Self::const_lit(&first) {
                    Some(i) if i >= dbase as i64 && i < hi => {
                        let ri = i as u32 - dbase;
                        let fbit = if asc { off + (w - 1 - ri) } else { off + ri };
                        Lvalue::BitSelect {
                            base: Box::new(base_lv),
                            index: Box::new(Self::dec_lit(fbit, span)),
                            span: span.to(self.prev_span()),
                        }
                    }
                    Some(_) => {
                        // OOB bit-select: iverilog drops it (no-op). Address a flat
                        // bit guaranteed past the struct net so the engine drops the
                        // write too — never a leak into a neighbour member.
                        Lvalue::BitSelect {
                            base: Box::new(base_lv),
                            index: Box::new(Self::dec_lit(OOB_DROP_BIT, span)),
                            span: span.to(self.prev_span()),
                        }
                    }
                    None => {
                        self.error_at(
                            span.to(self.prev_span()),
                            "a constant packed-struct member bit-select WRITE index",
                        );
                        Lvalue::Error(span.to(self.prev_span()))
                    }
                }
            }
            // Indexed `[i+:w]` / `[i-:w]`: iverilog refuses these for a struct-member
            // write ("sorry: not yet supported"), so there is no oracle — loud.
            Some(TokenKind::PlusColon) | Some(TokenKind::MinusColon) => {
                self.bump();
                let _ = self.expr(0);
                let _ = self.expect(TokenKind::RBracket, "']'");
                self.error_at(
                    span.to(self.prev_span()),
                    "a constant `[a:b]` range or bit-select on a packed-struct member \
                     WRITE (an indexed `[i+:w]`/`[i-:w]` is unsupported — iverilog \
                     refuses it too)",
                );
                Lvalue::Error(span.to(self.prev_span()))
            }
            _ => {
                self.error_at(span, "']' or ':' after a struct-member sub-select index");
                Lvalue::Error(span.to(self.prev_span()))
            }
        }
    }
}
