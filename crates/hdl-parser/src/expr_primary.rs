//! primary expressions — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Const-fold a generate-array hier index to its non-negative value: a decimal
    /// literal, literal arithmetic (`P-1`, `1+2`), or a module-scope literal-valued
    /// `localparam` (recorded in `const_locals`). A `parameter` / param-derived value
    /// is NOT in `const_locals`, so it does not fold (its index stays loud — folding
    /// it could disagree with an instance override). Returns `None` if not foldable.
    pub(crate) fn try_const_index(&self, e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw,
            } => raw
                .chars()
                .filter(|c| *c != '_')
                .collect::<String>()
                .parse::<i64>()
                .ok(),
            ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.const_locals.get(&p.segments[0].name).copied()
            }
            ExprKind::Paren { inner } => self.try_const_index(inner),
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => Some(-self.try_const_index(operand)?),
            ExprKind::Binary { op, lhs, rhs } => {
                let a = self.try_const_index(lhs)?;
                let b = self.try_const_index(rhs)?;
                match op {
                    BinOp::Add => Some(a + b),
                    BinOp::Sub => Some(a - b),
                    BinOp::Mul => Some(a * b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Format a CONSTANT generate-array index as a decimal scope-segment string. A
    /// decimal literal (`g[0]`), literal arithmetic, or a literal-valued
    /// `localparam` (`g[P]`) folds; anything else (a `parameter`, a runtime value)
    /// is a loud parse error.
    pub(crate) fn const_index_string(&mut self, idx: &Expr) -> String {
        if let Some(v) = self.try_const_index(idx) {
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
                    self.pos = save; // not a bare type → restore for normal expr-arg parse
                }
                let args = if self.peek() == Some(T::LParen) {
                    self.call_args()
                } else {
                    Vec::new()
                };
                Expr {
                    kind: ExprKind::SysCall { name, args },
                    span: start.to(self.prev_span()),
                }
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
                if let Some((base, off, w, asc, sgn, dbase, stride)) =
                    self.struct_field_select(&path)
                {
                    return self.struct_member_expr(
                        base,
                        (off, w, asc, sgn, dbase, stride),
                        path.span,
                    );
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
        Expr {
            kind: ExprKind::PkgScoped {
                pkg: path.segments.into_iter().next().unwrap(),
                name,
            },
            span: start.to(self.prev_span()),
        }
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

    /// Fold a constant-literal expression to `i64` at parse time (decimal literals
    /// and +/-/* of them). Returns `None` for anything non-constant.
    pub(crate) fn const_lit(e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw,
            } => raw
                .chars()
                .filter(|c| *c != '_')
                .collect::<String>()
                .parse::<i64>()
                .ok(),
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => Self::const_lit(operand)?.checked_neg(),
            ExprKind::Binary { op, lhs, rhs } => {
                let a = Self::const_lit(lhs)?;
                let b = Self::const_lit(rhs)?;
                // Checked arithmetic: an overflowing constant fold returns None
                // (→ caller treats the value as non-foldable / loud) rather than
                // panicking in debug or silently wrapping in release.
                match op {
                    BinOp::Add => a.checked_add(b),
                    BinOp::Sub => a.checked_sub(b),
                    BinOp::Mul => a.checked_mul(b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Fold an ENUM LABEL expression to `i64`, including sized / unsized-based
    /// literals (`4'h3`, `8'shFF`, `'b1011`) that [`Self::const_lit`] cannot see.
    ///
    /// `base_signed` is the ENUM BASE TYPE's signedness, and it — not the literal's
    /// own `s` marker — decides how the literal's bit pattern is read. §6.19: a
    /// label is a value OF the base type, so `enum integer { A = 32'hDEADBEEF }` is
    /// −559038737 (iverilog agrees) even though the literal is unsigned, and
    /// `enum bit [7:0] { A = 8'shFF }` is 255 even though the literal is signed.
    /// Folding the marker's sign instead rejected the first as out of range and gave
    /// the second a `.name()` table keyed on −1 while the constant was 255 — the
    /// label's NAME came back empty.
    ///
    /// Deliberately a SEPARATE entry point rather than a widening of `const_lit`:
    /// that one also decides packed-struct member layout and typedef ranges at parse
    /// time, so folding more there would move layout decisions across the front end.
    ///
    /// This is a SECOND place that turns literal text into a value — the first is
    /// `elaborate`'s `parse_int_literal`, unreachable from here (it lives in a crate
    /// that depends on this one). Two predicates for one value is a hazard: a
    /// disagreement shows up as a `.name()` table pointing at a different label than
    /// the constant, so only the NAME is wrong, silently. The accept-set is therefore
    /// narrowed to inputs where the two provably agree — see
    /// [`Self::based_lit_pattern`] — and `enum_sized_label.rs` pins every accepted
    /// form by printing `x.name()` and `x` together.
    pub(crate) fn const_lit_enum(e: &Expr, base_signed: bool) -> Option<i64> {
        match &e.kind {
            ExprKind::IntLit {
                kind: kind @ (IntLitKind::Sized | IntLitKind::UnsizedBased),
                raw,
            } => {
                let (pat, w) = Self::based_lit_pattern(raw, matches!(kind, IntLitKind::Sized))?;
                Self::read_pattern(pat, w, base_signed)
            }
            // `-4'sd1` negates WITHIN the literal's width (→ the 4-bit pattern
            // `1111`), then the base type reads it: 15 in an unsigned base.
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => match &operand.kind {
                ExprKind::IntLit {
                    kind: kind @ (IntLitKind::Sized | IntLitKind::UnsizedBased),
                    raw,
                } => {
                    let (pat, w) = Self::based_lit_pattern(raw, matches!(kind, IntLitKind::Sized))?;
                    let mask = if w == 64 { !0u64 } else { (1u64 << w) - 1 };
                    Self::read_pattern((pat as i64).checked_neg()? as u64 & mask, w, base_signed)
                }
                _ => Self::const_lit(e),
            },
            _ => Self::const_lit(e),
        }
    }

    /// Read a `w`-bit pattern as a value of a signed / unsigned type.
    fn read_pattern(pat: u64, w: u32, signed: bool) -> Option<i64> {
        if w >= 64 {
            // Every 64-bit pattern is an i64 bit-for-bit. An UNSIGNED 64-bit value
            // above i64::MAX therefore lands here as a negative i64 — the label
            // table's own representation limit, tracked in ROADMAP §3.
            return Some(pat as i64);
        }
        if signed && (pat >> (w - 1)) & 1 == 1 {
            return Some((pat | (!0u64 << w)) as i64);
        }
        i64::try_from(pat).ok()
    }

    /// The `w`-bit unsigned PATTERN of a based literal `[W]'[s]B digits`, with its
    /// width. No sign extension happens here: the caller's type decides that.
    ///
    /// Declines (None) every input whose value this cannot reproduce EXACTLY the way
    /// `elaborate::parse_int_literal` does, which leaves the caller where it was:
    ///   * x/z/? digits, a fill literal (`'0`/`'1`/`'x`/`'z`) — no integer value;
    ///   * width 0 or above 64, or digits overflowing u64;
    ///   * digits that do NOT already fit the declared width. Reproducing truncation
    ///     is exactly where the two implementations part company (an UNSIZED based
    ///     literal grows past 32 bits there and would be masked to 32 here), so it
    ///     is refused rather than guessed;
    ///   * an UNSIZED literal carrying an `s` marker. Its width is 32 here but
    ///     `natural.max(32)` there, and the two only differ in whether bit 31 is a
    ///     sign bit — `'sd2147483648` would be −2147483648 here and +2147483648
    ///     there. Unsized WITHOUT `s` is safe: no sign bit is consulted, and the
    ///     fits-the-width rule above already forces the value below 2^32.
    fn based_lit_pattern(raw: &str, sized: bool) -> Option<(u64, u32)> {
        let tick = raw.find('\'')?;
        let width: u32 = if sized {
            let w: String = raw[..tick].chars().filter(|c| *c != '_').collect();
            let w = w.parse::<u32>().ok()?;
            if w == 0 || w > 64 {
                return None;
            }
            w
        } else {
            if !raw[..tick].is_empty() {
                return None;
            }
            32
        };
        let mut rest = raw[tick + 1..].chars().peekable();
        let signed_marker = matches!(rest.peek(), Some('s') | Some('S'));
        if signed_marker {
            rest.next();
            if !sized {
                return None; // unsized + `s`: width rules differ (see above)
            }
        }
        let base: u32 = match rest.next()? {
            'b' | 'B' => 2,
            'o' | 'O' => 8,
            'd' | 'D' => 10,
            'h' | 'H' => 16,
            _ => return None, // single-char fill: context-sized, no fixed value
        };
        let mut acc: u64 = 0;
        let mut any = false;
        for c in rest {
            if c == '_' {
                continue;
            }
            let d = c.to_digit(base)?; // x/z/? decline here
            acc = acc.checked_mul(base as u64)?.checked_add(d as u64)?;
            any = true;
        }
        if !any {
            return None;
        }
        let masked = if width == 64 {
            acc
        } else {
            acc & ((1u64 << width) - 1)
        };
        if masked != acc {
            return None; // truncation is declined, not reproduced
        }
        Some((masked, width))
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
        Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: v.to_string(),
            },
            span,
        }
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
            let v = self.cur_text().replace('_', "").parse::<u32>().ok();
            self.bump();
            if let Some(v) = v {
                return v;
            }
        }
        self.error(what);
        1
    }
}
