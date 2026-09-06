//! cast expressions — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Finish a size/typedef cast whose casting type is the already-parsed primary
    /// `ty` and whose cursor is at `'(`. A plain `Ident` casting type → `Named`
    /// (resolved at elaborate; class casts loud-reject there); any other primary
    /// (number, `(W+1)`, arithmetic) → `Size`. (SV §6.24.)
    pub(crate) fn parse_size_or_named_cast(&mut self, ty: Expr) -> Expr {
        let start = ty.span;
        self.bump(); // '
        self.expect(TokenKind::LParen, "'(' to open a cast");
        let operand = self.expr(0);
        self.expect(TokenKind::RParen, "')' closing a cast");
        let end = self.prev_span();
        // B2 (§6.24.1): `T'(e)` where T is a simple vector/enum typedef → a numeric
        // cast to T's width and sign. Desugar to the composition of the existing
        // built-in casts `(signed'|unsigned')(W'(e))` — the size cast sets the
        // width (extending with the OPERAND's sign), then the signing cast stamps
        // T's signedness. A struct/union/class/multi-dim/atom/2-state typedef has
        // no simple (width, signed) form and stays loud (the `Named` arm).
        //
        // Round-9 PKG2: the casting type may be package-scoped (`pkg::T'(e)`),
        // which parses as a `PkgScoped` primary. The parser registers a
        // `"pkg::T"` twin of each package typedef in `self.typedefs` /
        // `struct_layouts` / `union_type_names`, so `simple_typedef_cast` resolves
        // the scoped key exactly like a bare name. A scoped SIZE cast
        // `pkg::WIDTH'(e)` (WIDTH a localparam, not a typedef) returns `None` here
        // and falls through to the `Size` arm unchanged — a genuine size cast.
        let td_key: Option<String> = match &ty.kind {
            ExprKind::Ident(path) if path.segments.len() == 1 => {
                Some(path.segments[0].name.clone())
            }
            ExprKind::PkgScoped { pkg, name } => Some(format!("{}::{}", pkg.name, name.name)),
            _ => None,
        };
        if let Some(key) = &td_key {
            // §3 ⑤ ⓒ: a symbolic-layout struct casts to its width EXPRESSION.
            let sized = match self.simple_typedef_cast(key) {
                Some((w, signed)) => Some((
                    Expr {
                        kind: ExprKind::IntLit {
                            kind: IntLitKind::Decimal,
                            raw: w.to_string(),
                        },
                        span: start,
                    },
                    signed,
                )),
                None => self.sym_typedef_cast(key),
            };
            if let Some((width_expr, signed)) = sized {
                let inner = Expr {
                    kind: ExprKind::Cast {
                        target: CastTarget::Size(Box::new(width_expr)),
                        expr: Box::new(operand),
                    },
                    span: start,
                };
                return Expr {
                    kind: ExprKind::Cast {
                        target: CastTarget::Signing { signed },
                        expr: Box::new(inner),
                    },
                    span: start.to(end),
                };
            }
        }
        let target = match ty.kind {
            ExprKind::Ident(path) => CastTarget::Named(path),
            _ => CastTarget::Size(Box::new(ty)),
        };
        Expr {
            kind: ExprKind::Cast {
                target,
                expr: Box::new(operand),
            },
            span: start.to(end),
        }
    }

    /// Map a casting-type keyword (`int`/`byte`/…/`signed`/`unsigned`) to its
    /// `CastTarget`, or `None` if the keyword is not a casting type. `realtime'`
    /// folds to `real`. (SV §6.24.)
    pub(crate) fn cast_type_kw(kw: Kw) -> Option<CastTarget> {
        use CastPrim as P;
        use CastTarget as C;
        Some(match kw {
            Kw::Int => C::Prim(P::Int),
            Kw::Integer => C::Prim(P::Integer),
            Kw::Byte => C::Prim(P::Byte),
            Kw::Shortint => C::Prim(P::Shortint),
            Kw::Longint => C::Prim(P::Longint),
            Kw::Bit => C::Prim(P::Bit),
            Kw::Logic => C::Prim(P::Logic),
            Kw::Reg => C::Prim(P::Reg),
            Kw::Time => C::Prim(P::Time),
            Kw::Real | Kw::Realtime => C::Prim(P::Real),
            Kw::Signed => C::Signing { signed: true },
            Kw::Unsigned => C::Signing { signed: false },
            _ => return None,
        })
    }

    /// Finish a keyword cast `int'(e)` / `signed'(e)` whose cursor is at the type
    /// keyword and where `'(` is already confirmed to follow. (SV §6.24.)
    pub(crate) fn parse_keyword_cast(&mut self, target: CastTarget) -> Expr {
        let start = self.cur_span();
        self.bump(); // type keyword
        self.bump(); // '
        self.expect(TokenKind::LParen, "'(' to open a cast");
        let operand = self.expr(0);
        self.expect(TokenKind::RParen, "')' closing a cast");
        Expr {
            kind: ExprKind::Cast {
                target,
                expr: Box::new(operand),
            },
            span: start.to(self.prev_span()),
        }
    }

    /// The compile-time bit width of a struct/union or simple-typedef TYPE NAME (used
    /// by `$bits(T)`). A packed struct is the SUM of its field widths; a packed union
    /// is the MAX; a simple typedef (`typedef logic [7:0] t` / `typedef int i_t`) is
    /// its base kind/range width times any packed dims. `None` (→ the `$bits` site
    /// stays a normal expression, loud in elaborate) for a class-handle alias or an
    /// unknown name.
    pub(crate) fn bits_of_type_name(&self, name: &str) -> Option<u32> {
        if let Some(layout) = self.struct_layouts.get(name) {
            return Some(if self.union_type_names.contains(name) {
                layout.fields.iter().map(|f| f.2).max().unwrap_or(0)
            } else {
                layout.fields.iter().map(|f| f.2).sum()
            });
        }
        let info = self.typedefs.get(name)?;
        // Only an INTEGRAL alias has a fixed `$bits` here. A `real`/`realtime` (64-bit,
        // but `member_width_kind` gives a bogus 1) / `string` (dynamic) / class-handle
        // alias returns None → the site stays a normal expr (loud in elaborate) rather
        // than folding a wrong width. (`$bits(real_var)` uses the elaborate path, which
        // sizes a real variable correctly — this only gates a bare TYPE name.)
        if info.class_name.is_some() || !Self::member_kind_is_integral(info.kind) {
            return None;
        }
        let mut w = self.member_width_kind(info.kind, &info.range)?;
        for d in &info.packed {
            w = w.checked_mul(self.member_width(&Some(d.clone()))?)?;
        }
        Some(w)
    }

    /// Parse a `$bits(<type>)` TYPE argument (cursor just AFTER the `(`): a data-type
    /// keyword with optional signedness + packed dims (`logic [15:0]`, `int`), or a
    /// struct/union/simple-typedef NAME (`s_t`). Returns the folded bit width and
    /// consumes through the closing `)`. `None` (the caller then restores the cursor)
    /// when the argument is NOT a bare type — an ordinary `$bits(expr)` (a variable,
    /// `arr[i]`, a `x.field`) then parses normally and the elaborator sizes it.
    pub(crate) fn parse_bits_type_arg(&mut self) -> Option<u32> {
        // A data-type keyword (`logic`/`bit`/`int`/…) is never a valid expression, so
        // this is unambiguously a type. Optional signedness + packed dims fold in.
        if let Some(kind) = self.net_var_kind() {
            // Only an INTEGRAL keyword has a fixed integral `$bits`; `real`/`realtime`/
            // `string`/`event`/… return None so the site stays loud (correct-or-loud),
            // never a silently-wrong 1-bit fold. Checked BEFORE consuming any token.
            if !Self::member_kind_is_integral(kind) {
                return None;
            }
            self.bump(); // kind keyword
            let _ = self.opt_signed(); // signedness does not change the bit width
            let range = self.opt_range();
            let mut w = self.member_width_kind(kind, &range)?;
            while self.peek() == Some(TokenKind::LBracket) {
                let d = self.opt_range()?;
                w = w.checked_mul(self.member_width(&Some(d))?)?;
            }
            if self.peek() == Some(TokenKind::RParen) {
                self.bump(); // )
                return Some(w);
            }
            return None;
        }
        // A bare type NAME (`$bits(s_t)`) — only when it is a KNOWN type AND the very
        // next token is `)`, so `$bits(var)` / `$bits(var.field)` / `$bits(arr[i])`
        // fall through to the expression path.
        if self.is_ident() && self.peek_at(1) == Some(TokenKind::RParen) {
            let w = self.bits_of_type_name(self.cur_text())?;
            self.bump(); // type name
            self.bump(); // )
            return Some(w);
        }
        None
    }
}
