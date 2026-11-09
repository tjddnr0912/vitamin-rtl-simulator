//! primary expressions — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Format a CONSTANT generate-array index as a decimal scope-segment string. A
    /// decimal literal (`g[0]`), literal arithmetic, or a literal-valued
    /// `localparam` (`g[P]`) folds; anything else (a `parameter`, a runtime value)
    /// is a loud parse error.
    pub(crate) fn const_index_string(&mut self, idx: &Expr) -> String {
        // §2 🆕 L (aa): a generate index is self-determined — `g[C+D]` over two
        // 4-bit constants wraps to `g[0]` in both oracles (the i64 fold said 16).
        if let Some(v) = self.const_bound(idx) {
            if v >= 0 {
                // Normalize the value (strip leading zeros: `g[00]` ⇒ scope `g[0]`).
                return v.to_string();
            }
        }
        self.error_at(
            idx.span,
            "a constant generate-array index (decimal literal or literal-valued \
             localparam) in a hierarchical reference",
        );
        "0".to_string()
    }

    pub(crate) fn expr_primary(&mut self) -> Expr {
        use TokenKind as T;
        let start = self.cur_span();
        match self.peek() {
            // lexer error sentinel: skip it (already diagnosed), yield Error node
            Some(T::Error(_)) => {
                self.bump();
                Expr {
                    kind: ExprKind::Error,
                    span: start,
                }
            }
            // SV §10.9 positional assignment pattern `'{e0, e1, …}`. The lexer
            // keeps `'{` as Apostrophe + LBrace (a cast is `'(`), so a `'` followed
            // by `{` opens an assignment pattern. Named/`default:`/replicated forms
            // are loud (only positional is supported — elaborate binds it to a 1-D
            // unpacked array).
            Some(T::Apostrophe) if self.peek_at(1) == Some(T::LBrace) => {
                self.parse_assign_pattern()
            }
            // SV type/signing cast `int'(e)` / `signed'(e)` (§6.24): a casting-type
            // keyword followed by `'(`. The guard requires `'(` so a bare type kw
            // (not a primary today) still falls to the `_ => error` arm unchanged.
            Some(T::Word(WordKind::Keyword(kw)))
                if self.peek_at(1) == Some(T::Apostrophe)
                    && self.peek_at(2) == Some(T::LParen)
                    && Self::cast_type_kw(kw).is_some() =>
            {
                self.parse_keyword_cast(Self::cast_type_kw(kw).unwrap())
            }
            // SV §6.16/§6.24.1 `string'(e)`. Its own arm because `string` is a
            // KEYWORD: `cast_type_kw` is span-free and the reserved `Named` segment
            // this builds needs the keyword's span. Same `'(`-lookahead guard as the
            // arm above, so a bare `string` type keyword still falls through.
            Some(T::Word(WordKind::Keyword(Kw::String)))
                if self.peek_at(1) == Some(T::Apostrophe) && self.peek_at(2) == Some(T::LParen) =>
            {
                let target = self.string_cast_target();
                self.parse_keyword_cast(target)
            }
            // numeric / string literals (G11: a decimal/real literal may be a time
            // literal `1ns` when a time-unit ident touches it — `maybe_time_literal`).
            Some(T::IntDecimal) => {
                let n = self.lit_int(IntLitKind::Decimal);
                self.maybe_time_literal(n)
            }
            Some(T::IntSized) => self.lit_int(IntLitKind::Sized),
            Some(T::IntUnsizedBased) => self.lit_int(IntLitKind::UnsizedBased),
            Some(T::RealFixed) => {
                let n = self.lit_real(RealLitKind::Fixed);
                self.maybe_time_literal(n)
            }
            Some(T::RealExp) => {
                let n = self.lit_real(RealLitKind::Exp);
                self.maybe_time_literal(n)
            }
            Some(T::Str) => {
                let raw = self.cur_text().to_string();
                self.bump();
                Expr {
                    kind: ExprKind::StrLit { raw },
                    span: start,
                }
            }
            // system function call: $time, $signed(x). name retains the `$`.
            Some(T::SystemTask) => {
                // Reserved namespace — same rule and same reason as the statement
                // form in `parse_systask_call`; both entries must refuse it or the
                // channel is only half private.
                if self.cur_text().starts_with("$__vita_") {
                    self.error(
                        "a system function name outside vita's reserved `$__vita_` \
                         namespace (those names are synthesized by the compiler and \
                         cannot be written)",
                    );
                }
                let t = self.bump().unwrap();
                let name = Ident {
                    name: self.src[t.span.clone()].to_string(),
                    span: Self::sp(&t.span),
                };
                // §20.6.1 `$bits(TYPE)`: a compile-time type-size whose argument is a
                // data TYPE (a typedef name or a `logic`/`int`/… keyword with optional
                // packed dims), which is NOT a valid expression. Fold it to an integer
                // literal here, where the type widths (struct_layouts / typedefs / atom
                // kinds) are known — so it works everywhere an expression does, including
                // a decl range `logic [$bits(T)-1:0]`. A `$bits(expr)` (variable /
                // `arr[i]` / `x.field`) is not a bare type → falls through to the normal
                // SysCall path (the elaborator sizes it via `bits_prescan`).
                if name.name == "$bits" && self.peek() == Some(T::LParen) {
                    let save = self.pos;
                    self.bump(); // (
                    if let Some(w) = self.parse_bits_type_arg() {
                        return Self::dec_lit(w, start.to(self.prev_span()));
                    }
                    // §4.5.437: a type parameter (`T$w`) or a symbolic-width typedef
                    if let Some(e) = self.parse_bits_sym_type_arg() {
                        return e;
                    }
                    self.pos = save; // not a bare type → restore for normal expr-arg parse
                }
                let args = if self.peek() == Some(T::LParen) {
                    self.call_args()
                } else {
                    Vec::new()
                };
                let call = Expr {
                    kind: ExprKind::SysCall { name, args },
                    span: start.to(self.prev_span()),
                };
                // A §20.7 array query on a multi-dimensional packed PARAMETER is
                // answered here, from the dimensions this parser flattened away
                // (`packed_md.rs`); every other call is returned untouched.
                self.rewrite_packed_md_dim_query(call)
            }
            // v5 ⑥: bare `$` — queue last-index (`q[$]`, `q[$-1]`). A primary
            // so Pratt arithmetic folds over it; elaborate substitutes
            // `size()-1` inside a queue select and loud-rejects it elsewhere.
            Some(T::Dollar) => {
                self.bump();
                Expr {
                    kind: ExprKind::Dollar,
                    span: start,
                }
            }
            // N7: `null` — the null class-handle literal.
            Some(T::Word(WordKind::Keyword(Kw::Null))) => {
                self.bump();
                Expr {
                    kind: ExprKind::Null,
                    span: start,
                }
            }
            // identifier / hierarchical name / function call
            _ if self.is_ident() => {
                let path = self.hier_path().unwrap();
                // v7 P2-D: `pkg::name` package-scoped value reference.
                if path.segments.len() == 1 && self.peek() == Some(T::ColonColon) {
                    // The ENTIRE `pkg::name` / `pkg::name(args)` handling lives in a
                    // cold, non-inlined helper so its locals never enlarge
                    // `expr_primary`'s hot recursive frame — the MAX_EXPR_DEPTH stack
                    // budget is frame-sized, and even this small block tipped the
                    // paren-depth guard (`depth_guard.rs`), like `struct_member_expr`.
                    return self.pkg_scoped_expr(path, start);
                }
                // v5 ⑥: contextual `new[n]` / `new[n](src)` — the ident `new`
                // immediately followed by `[`. Elaborate falls back to an
                // array read when a net named `new` is actually in scope
                // (V2005 keeps `new` as an ordinary identifier).
                if path.segments.len() == 1
                    && path.segments[0].name == "new"
                    && self.peek() == Some(T::LBracket)
                {
                    self.bump(); // '['
                    let size = self.expr(0);
                    self.expect(T::RBracket, "']'");
                    let src = if self.peek() == Some(T::LParen) {
                        self.bump();
                        let s = self.expr(0);
                        self.expect(T::RParen, "')'");
                        Some(Box::new(s))
                    } else {
                        None
                    };
                    return Expr {
                        kind: ExprKind::New {
                            size: Box::new(size),
                            src,
                        },
                        span: start.to(self.prev_span()),
                    };
                }
                // N7: contextual class `new` / `new(args)` — the ident `new` NOT
                // followed by `[` (the dyn-array form is handled just above). The
                // class is inferred from the assignment LHS handle at elaborate;
                // a V2005 program using `new` as a plain net is unaffected because
                // elaborate falls back when no class-handle LHS is in play.
                if path.segments.len() == 1 && path.segments[0].name == "new" {
                    let args = if self.peek() == Some(T::LParen) {
                        self.call_args()
                    } else {
                        Vec::new()
                    };
                    return Expr {
                        kind: ExprKind::ClassNew { args },
                        span: start.to(self.prev_span()),
                    };
                }
                // Round-9: UNPACKED-struct member read `k.field` → the member net
                // `k$field` (a plain Ident — the member has its own storage). Tried
                // before the packed path; `None` for every non-unpacked var, so
                // packed structs and all else are byte-identical.
                if let Some(mangled) = self.unpacked_field_ident(&path) {
                    return Expr {
                        span: mangled.span,
                        kind: ExprKind::Ident(mangled),
                    };
                }
                // packed-struct member access `s.field` → constant part-select.
                // Extracted to a non-inlined helper so the (rare) struct-field
                // locals never inflate `expr_primary`'s frame on the hot paren-
                // recursion path (the MAX_EXPR_DEPTH stack budget is frame-sized).
                if self.peek() == Some(T::LBracket) && self.composed_member_ahead(&path) {
                    return self.composed_member_read(&path);
                }
                if let Some((base, geom, _nested)) = self.struct_field_select(&path) {
                    return self.struct_member_expr(base, geom, path.span);
                }
                // §3 ⑤ ⓒ: a member of a symbolic-layout struct. A cold helper for
                // the same reason as `struct_member_expr` — its locals must not sit
                // in this recursive frame (`depth_guard` measured the overflow).
                if let Some(e) = self.sym_struct_member_expr(&path) {
                    return e;
                }
                // SV §6.19.5 enum method `x.first/last/num/next/prev/name [()]` —
                // the arg-less form. Desugars to literals / ternary chains over the
                // enum's labels; non-enum `x.foo` returns None → normal path.
                {
                    let empty_call =
                        self.peek() == Some(T::LParen) && self.peek_at(1) == Some(T::RParen);
                    if (self.peek() != Some(T::LParen) || empty_call) && !path.segments.is_empty() {
                        if let Some(e) = self.enum_method_expr(&path) {
                            if empty_call {
                                self.bump(); // (
                                self.bump(); // )
                            }
                            return e;
                        }
                    }
                }
                // SV §6.19.5 `x.next(N)` / `x.prev(N)` with a CONSTANT step. A
                // non-constant step or a non-enum receiver falls through to the generic
                // Call below (loud in elaborate — correct-or-loud). Handled here, not in
                // the arg-less `enum_method_expr`, so `.next()` stays byte-identical.
                if self.peek() == Some(T::LParen)
                    && path.segments.len() == 2
                    && matches!(path.segments[1].name.as_str(), "next" | "prev")
                    && self.var_enum.contains_key(&path.segments[0].name)
                {
                    let is_next = path.segments[1].name == "next";
                    let args = self.call_args();
                    if args.len() == 1 {
                        if let Some(n) = Self::const_lit(&args[0]) {
                            if let Some(e) = self.enum_step_n_expr(&path, is_next, n) {
                                return e;
                            }
                        }
                    }
                    return Expr {
                        kind: ExprKind::Call { name: path, args },
                        span: start.to(self.prev_span()),
                    };
                }
                if self.peek() == Some(T::LParen) {
                    // r18 (E2): a method on a struct MEMBER receiver
                    // (`r.name.substr(a,b)`) — rewrite `r.name` to its member net
                    // `$unp$r$name` so this becomes the 2-segment `$unp$r$name.substr(a,b)`
                    // method-on-a-net form elaborate dispatches. A non-member receiver
                    // leaves `path` unchanged (→ the generic Call below).
                    let path = self.unpacked_member_method_recv(&path).unwrap_or(path);
                    // N3 SoA: `arr.size()`/`arr.num()` on a SoA record array → field 0's
                    // dyn array (`$unp$arr$field0.size()`); all fields share the length.
                    let path = self.soa_rewrite_method_recv(path);
                    let args = self.call_args();
                    let args = self.expand_struct_call_args(args); // R5: struct actual → members
                    let args = self.desugar_container_pattern_args(&path, args);
                    Expr {
                        kind: ExprKind::Call { name: path, args },
                        span: start.to(self.prev_span()),
                    }
                } else {
                    let sp = path.span;
                    Expr {
                        kind: ExprKind::Ident(path),
                        span: sp,
                    }
                }
            }
            // parenthesized / min:typ:max
            Some(T::LParen) => {
                self.bump();
                let inner = self.expr(0);
                if self.peek() == Some(T::Colon) {
                    self.bump();
                    let typ = self.expr(0);
                    self.expect(T::Colon, "':' in min:typ:max");
                    let max = self.expr(0);
                    self.expect(T::RParen, "')'");
                    Expr {
                        kind: ExprKind::MinTypMax {
                            min: Box::new(inner),
                            typ: Box::new(typ),
                            max: Box::new(max),
                        },
                        span: start.to(self.prev_span()),
                    }
                } else {
                    self.expect(T::RParen, "')'");
                    Expr {
                        kind: ExprKind::Paren {
                            inner: Box::new(inner),
                        },
                        span: start.to(self.prev_span()),
                    }
                }
            }
            // concat / replication
            Some(T::LBrace) => self.brace_expr(start),
            _ => {
                self.error("expression");
                Expr {
                    kind: ExprKind::Error,
                    span: start,
                }
            }
        }
    }

    pub(crate) fn lit_int(&mut self, kind: IntLitKind) -> Expr {
        let start = self.cur_span();
        let raw = self.cur_text().to_string();
        self.bump();
        Expr {
            kind: ExprKind::IntLit { kind, raw },
            span: start,
        }
    }
    pub(crate) fn lit_real(&mut self, kind: RealLitKind) -> Expr {
        let start = self.cur_span();
        let raw = self.cur_text().to_string();
        self.bump();
        Expr {
            kind: ExprKind::RealLit { kind, raw },
            span: start,
        }
    }

    /// The `pkg::name` / `pkg::name(args)` case of `expr_primary` (cursor just
    /// past the package `Ident`, at `::`). A plain scoped VALUE reference lowers
    /// to `ExprKind::PkgScoped`; a scoped SUBROUTINE CALL `pkg::name(args)` lowers
    /// to a 2-segment `ExprKind::Call` `[pkg, name]` (round-7 §4.5.111). Elaborate
    /// resolves the callee in the named package's scope — a self-contained,
    /// straight-line function inlines; a stateful one is loud (workaround: `import`
    /// the package and call by the bare name). Split whole out of `expr_primary` and
    /// marked `#[inline(never)]` so NONE of its locals (this branch's plus the
    /// scoped-value construction's) enlarge that hot recursive frame — the
    /// MAX_EXPR_DEPTH stack budget is frame-sized (mirrors `struct_member_expr`;
    /// `depth_guard.rs::deep_paren_nesting_errors_cleanly`).
    #[inline(never)]
    pub(crate) fn pkg_scoped_expr(&mut self, path: HierPath, start: Span) -> Expr {
        self.bump(); // '::'
        let Some(name) = self.ident() else {
            return Expr {
                kind: ExprKind::Error,
                span: start.to(self.prev_span()),
            };
        };
        if self.peek() == Some(TokenKind::LParen) {
            // IEEE §26.3 (round-7): `pkg::name(args)` package-scoped subroutine call.
            // Emit a 2-segment `Call` [pkg, name]; elaborate resolves the callee in the
            // named package's scope (a self-contained function inlines; a stateful one
            // is loud — `import` the package and call by the bare name). A method call
            // `obj.m()` is also a 2-segment `Call`, but the elaborate call resolver
            // disambiguates by whether the head segment is a known package.
            let args = self.call_args();
            let args = self.expand_struct_call_args(args); // R5: struct actual → members
            let pkg = path.segments.into_iter().next().unwrap();
            return Expr {
                kind: ExprKind::Call {
                    name: HierPath {
                        segments: vec![pkg, name],
                        span: start.to(self.prev_span()),
                    },
                    args,
                },
                span: start.to(self.prev_span()),
            };
        }
        let base = Expr {
            kind: ExprKind::PkgScoped {
                pkg: path.segments.into_iter().next().unwrap(),
                name,
            },
            span: start.to(self.prev_span()),
        };
        // A METHOD on the qualified value (`pkg::S.len()`, `pkg::E.name()`). The `.`
        // used to end the expression here, so the caller reported `expected ')', found
        // '.'` — three times, at the same column, with none of the words "method",
        // "package" or "::" in any of them. Everything else about the spelling already
        // worked: `s.len()` on a bare name, and `pkg::e_t x; x.name()` on a variable of
        // a qualified TYPE. Only the `.` after a qualified EXPRESSION was missing.
        if self.peek() == Some(TokenKind::Dot)
            && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
            && self.peek_at(2) == Some(TokenKind::LParen)
        {
            self.bump(); // '.'
            let method = self.ident().expect("peeked an identifier");
            let args = self.call_args();
            return Expr {
                kind: ExprKind::MethodCall {
                    recv: Box::new(base),
                    method,
                    args,
                },
                span: start.to(self.prev_span()),
            };
        }
        base
    }

    /// Token length of a `pkg::` scope qualifier that precedes a registered scoped
    /// type at the cursor — 2 (`pkg` + `::`) or 0 (none). Used by type-name
    /// consumers to know how many leading tokens to skip.
    pub(crate) fn scope_qualifier_len(&self) -> usize {
        if self.scoped_type_key().is_some() {
            2
        } else {
            0
        }
    }

    /// Skip a `pkg::` scope qualifier (2 tokens) if one precedes a scoped type at
    /// the cursor, leaving the cursor on the final type-name identifier. Every
    /// type-name consumer calls this before `bump`-ing the name token.
    pub(crate) fn eat_scope_qualifier(&mut self) {
        if self.scope_qualifier_len() == 2 {
            self.bump(); // pkg
            self.bump(); // ::
        }
    }

    /// A `[hi:0]` range made of decimal literals, for the synthesized struct vector.
    pub(crate) fn dec_range(hi: u32) -> Range {
        Range {
            msb: Self::dec_lit(hi, Span::new(0, 0)),
            lsb: Self::dec_lit(0, Span::new(0, 0)),
            span: Span::new(0, 0),
        }
    }

    /// A decimal integer-literal expression with the given value.
    pub(crate) fn dec_lit(v: u32, span: Span) -> Expr {
        hdl_ast::range_rules::dec_lit(v, span)
    }

    /// N3 SoA: a single-segment `Ident` expression from a (mangled) net name.
    pub(crate) fn ident_expr(name: &str, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Ident(HierPath {
                segments: vec![Ident {
                    name: name.to_string(),
                    span,
                }],
                span,
            }),
            span,
        }
    }

    // ───────────────────────── SV §6.19.5 enum methods ─────────────────────
    /// A decimal integer literal for a possibly-negative `i64` (negatives become
    /// `-<magnitude>`). Used to build the enum-method desugar's constants.
    pub(crate) fn i64_lit(v: i64, span: Span) -> Expr {
        let mag = Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: v.unsigned_abs().to_string(),
            },
            span,
        };
        if v < 0 {
            Expr {
                kind: ExprKind::Unary {
                    op: UnOp::Minus,
                    operand: Box::new(mag),
                },
                span,
            }
        } else {
            mag
        }
    }

    /// Read a small unsigned decimal constant from the current `IntDecimal`
    /// token (digit separators stripped). Non-literal / oversized → loud, 1.
    pub(crate) fn parse_small_const(&mut self, what: &'static str) -> u32 {
        if self.peek() == Some(TokenKind::IntDecimal) {
            let v = hdl_ast::value_rules::decimal_i64(self.cur_text())
                .and_then(|v| u32::try_from(v).ok());
            self.bump();
            if let Some(v) = v {
                return v;
            }
        }
        self.error(what);
        1
    }
}
