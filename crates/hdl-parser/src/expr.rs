//! Pratt expression core — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

pub(crate) fn infix_bp(k: TokenKind) -> Option<(u8, u8)> {
    use TokenKind as T;
    Some(match k {
        T::PipePipe => (5, 6),                                // ||   lvl13
        T::AmpAmp => (7, 8),                                  // &&   lvl12
        T::Pipe => (9, 10),                                   // |    lvl11
        T::Caret | T::TildeCaret | T::CaretTilde => (11, 12), // ^ ~^ ^~ lvl10
        T::Amp => (13, 14),                                   // &    lvl9
        T::EqEq | T::BangEq | T::EqEqEq | T::BangEqEq | T::EqEqQ | T::BangEqQ => (15, 16), // == != === !== ==? !=? lvl8
        T::Lt | T::LtEq | T::Gt | T::GtEq => (17, 18), // < <= > >= lvl7
        T::Shl | T::Shr | T::ShlA | T::ShrA => (19, 20), // << >> <<< >>> lvl6
        T::Plus | T::Minus => (21, 22),                // + -  lvl5
        T::Star | T::Slash | T::Percent => (23, 24),   // * / % lvl4
        T::StarStar => (26, 27), // **   lvl3 LEFT-assoc (IEEE Table 11-2 / iverilog: `2**2**3` = (2**2)**3)
        _ => return None,
    })
}

pub(crate) const TERNARY_LBP: u8 = 4; // lvl14, right-assoc; rbp = 3

pub(crate) const TERNARY_RBP: u8 = 3;

pub(crate) const UNARY_BP: u8 = 27; // lvl2, prefix right-assoc — binds tighter than **

/// Flat bit index used to encode a DROPPED struct-member bit-select WRITE (OOB
/// source index): any net is at most `MAX_NET_WIDTH` (1<<20) bits, so a write to
/// bit `1<<21` always falls off the end and the engine drops it (no-op) — exactly
/// iverilog's behaviour for `s.f[i]` with `i` past the member, with no risk of
/// leaking into a neighbouring member however wide the struct.
pub(crate) const OOB_DROP_BIT: u32 = 1 << 21;

pub(crate) fn bin_op(k: TokenKind) -> BinOp {
    use TokenKind as T;
    match k {
        T::StarStar => BinOp::Pow,
        T::Star => BinOp::Mul,
        T::Slash => BinOp::Div,
        T::Percent => BinOp::Mod,
        T::Plus => BinOp::Add,
        T::Minus => BinOp::Sub,
        T::Shl => BinOp::Shl,
        T::Shr => BinOp::Shr,
        T::ShlA => BinOp::AShl,
        T::ShrA => BinOp::AShr,
        T::Lt => BinOp::Lt,
        T::LtEq => BinOp::Le,
        T::Gt => BinOp::Gt,
        T::GtEq => BinOp::Ge,
        T::EqEq => BinOp::Eq,
        T::BangEq => BinOp::Ne,
        T::EqEqEq => BinOp::CaseEq,
        T::BangEqEq => BinOp::CaseNe,
        T::EqEqQ => BinOp::WildEq,
        T::BangEqQ => BinOp::WildNe,
        T::Amp => BinOp::BitAnd,
        T::Caret => BinOp::BitXor,
        T::TildeCaret | T::CaretTilde => BinOp::BitXnor,
        T::Pipe => BinOp::BitOr,
        T::AmpAmp => BinOp::LogAnd,
        T::PipePipe => BinOp::LogOr,
        _ => unreachable!("bin_op called on non-binary token"),
    }
}

/// Build a binary `Expr` spanning its operands (used by the `inside` desugar).
pub(crate) fn mk_bin(op: BinOp, l: Expr, r: Expr) -> Expr {
    let span = l.span.to(r.span);
    Expr {
        kind: ExprKind::Binary {
            op,
            lhs: Box::new(l),
            rhs: Box::new(r),
        },
        span,
    }
}

pub(crate) fn prefix_op(k: TokenKind) -> Option<UnOp> {
    use TokenKind as T;
    Some(match k {
        T::Plus => UnOp::Plus,
        T::Minus => UnOp::Minus,
        T::Bang => UnOp::LogNot,
        T::Tilde => UnOp::BitNot,
        T::Amp => UnOp::RedAnd,
        T::TildeAmp => UnOp::RedNand,
        T::Pipe => UnOp::RedOr,
        T::TildePipe => UnOp::RedNor,
        T::Caret => UnOp::RedXor,
        T::TildeCaret | T::CaretTilde => UnOp::RedXnor,
        _ => return None,
    })
}

/// True for any operator-class token that can legally appear in INFIX position.
/// Used after the Pratt loop to detect a leftover operator (verdict B1): e.g.
/// `~&`/`~|`/`~` are pure-unary, so `a ~& b` would otherwise silently truncate.
pub(crate) fn is_operatorish(k: TokenKind) -> bool {
    use TokenKind as T;
    infix_bp(k).is_some()
        || matches!(
            k,
            T::Question | T::TildeAmp | T::TildePipe | T::Tilde | T::Bang | T::StarStar
        )
}

impl Parser<'_, '_> {
    /// PARSE-CONCAT-CAP global budget on parsed expression nodes (user decision,
    /// 2026-06-22). 2^21 ≈ 2.1 M nodes × 80 B ≈ 168 MiB of `Expr` — ~80,000× the
    /// largest concat in the test corpus (26 elements), so any realistic v1
    /// single-file design is far below it, while a `{a,a,…,4M}` flood is a loud,
    /// bounded parse error instead of an OOM.
    /// Pratt entry. `min_bp` = caller's right binding power. After the fold loop,
    /// if the next token is operator-class but matched no infix slot, emit one
    /// error (verdict B1: do not silently leave `~& b` unconsumed).
    /// P2-5 guard: cap the expression recursion so deep nesting is a clean
    /// parse error, never a SIGSEGV. 128 is ≫ any real RTL expression (deepest
    /// practical cones are <50) yet fires with margin below the point a default
    /// 2 MiB test-thread stack overflows in debug builds. That point is
    /// OS-sensitive: macOS debug frames are fat enough to SIGABRT at ~241-deep
    /// (measured), so the former 256 sat ABOVE the overflow depth and never got
    /// to fire on macOS — the cap must be < the smallest overflow depth across
    /// build hosts, not merely "large". 128 keeps 47% headroom under that ~241
    /// floor (room for future hot-frame growth) while clearing the 100-deep
    /// `shallow_nesting_still_parses` test. (Statement nesting frames are
    /// smaller, so `MAX_STMT_DEPTH` stays 256.)
    pub(crate) const MAX_EXPR_DEPTH: u32 = 128;
    pub(crate) const MAX_AST_NODES: usize = 1 << 21;

    pub fn expr(&mut self, min_bp: u8) -> Expr {
        self.expr_depth += 1;
        if self.expr_depth > Self::MAX_EXPR_DEPTH {
            self.expr_depth -= 1;
            self.error("expression nesting too deep (cap 128)");
            return Expr {
                kind: ExprKind::Error,
                span: self.cur_span(),
            };
        }
        // PARSE-CONCAT-CAP: count every expression node; past the budget, latch
        // `node_budget_blown` (the expr comma-loops check it and stop pushing) and
        // report once. Returns an Error leaf so no further nodes are built here.
        self.node_count += 1;
        if self.node_count > Self::MAX_AST_NODES {
            if !self.node_budget_blown {
                self.node_budget_blown = true;
                self.error("expression too large (AST node budget 2097152 exceeded)");
            }
            self.expr_depth -= 1;
            return Expr {
                kind: ExprKind::Error,
                span: self.cur_span(),
            };
        }
        let r = self.expr_capped(min_bp);
        self.expr_depth -= 1;
        r
    }

    pub(crate) fn expr_capped(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.expr_prefix();
        loop {
            let Some(op) = self.peek() else { break };
            if op == TokenKind::Question {
                // ternary, right-assoc
                if TERNARY_LBP < min_bp {
                    break;
                }
                self.bump();
                let then_e = self.expr(0); // reset inside branch
                self.expect(TokenKind::Colon, "':' in conditional");
                let else_e = self.expr(TERNARY_RBP); // right-assoc
                let span = lhs.span.to(else_e.span);
                lhs = Expr {
                    kind: ExprKind::Ternary {
                        cond: Box::new(lhs),
                        then_e: Box::new(then_e),
                        else_e: Box::new(else_e),
                    },
                    span,
                };
                continue;
            }
            // `lhs inside { items }` (IEEE §11.4.13): a SET-membership test that
            // desugars at parse time to an OR of equality / range tests (relational
            // binding power, lvl7) — so there is no new AST node, and it works in
            // constraints AND ordinary `if (x inside {…})` for free.
            if self.at_ident_kw("inside") {
                if 17 < min_bp {
                    break;
                }
                self.bump(); // `inside`
                lhs = self.parse_inside(lhs);
                continue;
            }
            // `value dist { item, … }` (IEEE §18.5.4) — weighted distribution.
            // Relational binding power (lvl7), like `inside`.
            if self.at_ident_kw("dist") {
                if 17 < min_bp {
                    break;
                }
                self.bump(); // `dist`
                lhs = self.parse_dist(lhs);
                continue;
            }
            // A `with` postfix on a method CALL: `obj.randomize() with { c; … }`
            // (IEEE §18.7) OR the array-method `arr.sum()/find() with (expr)`
            // iterator clause (IEEE §7.12). Both dispatch into ONE `#[inline(never)]`
            // helper (`parse_with_postfix`, brace vs paren) so this hot recursive
            // frame stays small — the expr-depth cap relies on `expr_capped` not
            // growing (`depth_guard.rs` deep-nesting test; a second inline branch
            // here tipped it into a stack overflow).
            if self.at_ident_kw("with")
                && matches!(self.peek_at(1), Some(TokenKind::LBrace | TokenKind::LParen))
                && matches!(lhs.kind, ExprKind::Call { .. })
            {
                lhs = self.parse_with_postfix(lhs);
                continue;
            }
            // `a -> b` constraint/property implication ≡ `!a || b`. Lowest binding
            // (below ternary), right-assoc; desugared at parse time (no new node).
            // A LEADING `->` is an event-trigger STATEMENT (handled at stmt level),
            // so reaching here means infix position.
            if op == TokenKind::Arrow {
                const IMP_LBP: u8 = 2;
                const IMP_RBP: u8 = 1;
                if IMP_LBP < min_bp {
                    break;
                }
                self.bump(); // ->
                let rhs = self.expr(IMP_RBP);
                let lspan = lhs.span;
                let not_lhs = Expr {
                    kind: ExprKind::Unary {
                        op: UnOp::LogNot,
                        operand: Box::new(lhs),
                    },
                    span: lspan,
                };
                lhs = mk_bin(BinOp::LogOr, not_lhs, rhs);
                continue;
            }
            let Some((l_bp, r_bp)) = infix_bp(op) else {
                break;
            };
            if l_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.expr(r_bp);
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary {
                    op: bin_op(op),
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            };
        }
        // B1: leftover operator that is not a valid infix continuation
        if min_bp == 0 {
            if let Some(op) = self.peek() {
                if is_operatorish(op) && infix_bp(op).is_none() && op != TokenKind::Question {
                    self.error("operator (got a unary-only operator in infix position)");
                }
            }
        }
        lhs
    }

    pub(crate) fn expr_prefix(&mut self) -> Expr {
        if let Some(op) = self.peek().and_then(prefix_op) {
            let start = self.cur_span();
            self.bump();
            let operand = self.expr(UNARY_BP); // lvl2 right-assoc, tighter than **
            let span = start.to(operand.span);
            return Expr {
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
                span,
            };
        }
        self.expr_postfix()
    }

    /// primary, then postfix loop: [idx]/[m:l]/[b+:w]; call(args) handled in primary.
    pub(crate) fn expr_postfix(&mut self) -> Expr {
        let mut e = self.expr_primary();
        // IEEE 1800-2017 §11.5.1: a bit/part select applies to a NET or VARIABLE, i.e.
        // to a name — possibly narrowed further by array indexing or member access,
        // which keeps it one. This parser attaches a select to ANY primary, so
        // `((a^b)>>8)[7:0]`, `f(a)[7:0]`, `{a,b}[7:0]` and `16'hABCD[7:0]` all run here
        // and are a vita extension. Measured: iverilog 13 rejects all four; verilator
        // 5.050 rejects the first and last and ACCEPTS the middle two with vita's value.
        // So the answer is the one §3.2 reached for `\r` — keep the value, say that the
        // spelling is not portable — and NOT a refusal, which would descend the ladder
        // for the two forms that are portable to verilator today.
        //
        // ⚠️ The test is PROVENANCE, and it has to be: a packed-struct member access
        // (`p.hi`) is desugared to a part-select by THIS loop, so by the time the AST
        // exists `p.hi[3:0]` (which every tool accepts) and `a[7:0][3:0]` are the same
        // shape. Only here is it still known that the chain began at a name.
        //
        // ⚠️ A `pkg::x` primary IS a name — §26.3 makes a package-scoped reference a
        // reference to the declaration in that package, and both oracles accept a
        // select on one. Leaving it out reported the ONE spelling of a package
        // constant's select that is spelled with the package: `pk::W[7:0]` drew
        // "a bit/part select here applies to an expression, not to a net or
        // variable" while the bare-imported spelling of the same select drew nothing.
        let from_name = matches!(e.kind, ExprKind::Ident(_) | ExprKind::PkgScoped { .. });
        // ⚠️ `from_name` is decided ONCE, from the primary, and that leaves a hole this
        // loop has to close separately: `a[7:0][3:0]` starts at a name, so every select
        // in the chain inherited "legal" and the second one went unwarned while vita
        // silently returned the whole `a`. iverilog states the real rule precisely —
        // "All but the final index in a chain of indices must be a single value, not a
        // range" — and that is decidable right here with no type information: a
        // non-final select that is a RANGE is the illegal shape. Tracking it separately
        // is what keeps `mem[1][3:0]` (index then part-select, which every tool accepts
        // and which appears everywhere) out of the warning.
        let mut prev_select_was_range = false;
        loop {
            match self.peek() {
                Some(TokenKind::LBracket) => {
                    if !from_name || prev_select_was_range {
                        let span = e.span;
                        self.warn_select_base(span);
                    }
                    e = self.parse_select(e);
                    prev_select_was_range = matches!(
                        e.kind,
                        ExprKind::PartSelect { .. } | ExprKind::IndexedPart { .. }
                    );
                    continue;
                }
                // N6B: a METHOD call on an indexed array ELEMENT (`files[i].len()`,
                // `arr[k].substr(a,b)`). A `name[idx]` select followed by `.ident(` is a
                // method on the element, NOT a hierarchical generate-array reference
                // (`g[0].x` / `bank[3].c.r` carry NO `(` after the member). Checked
                // BEFORE `is_indexed_hier_base` so the element-method wins; the receiver
                // is the `BitSelect`, dispatched at elaborate on the element's type.
                Some(TokenKind::Dot)
                    if matches!(e.kind, ExprKind::BitSelect { .. })
                        && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
                        && self.peek_at(2) == Some(TokenKind::LParen) =>
                {
                    e = self.parse_method_chain(e);
                }
                // N3: `arr[i].field` where `arr` is a record ARRAY var — a member READ
                // on the element (a part-select on the dyn element value), NOT a
                // hierarchical ref. The element-method branch above took the `(` case;
                // this handles the bare `.field` (checked BEFORE `is_indexed_hier_base`
                // so a record-array element member wins over the generate-array path).
                Some(TokenKind::Dot)
                    if self.record_array_member_base(&e)
                        && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
                        && self.peek_at(2) != Some(TokenKind::LParen) =>
                {
                    e = self.parse_record_array_member(e);
                }
                // §4.5.190: `arr[i].field` where `arr` is a PACKED-STRUCT 1-D array
                // (`struct_1d_array_vars`) — a member READ = a part-select on the packed
                // element value `arr[i]`. Checked before `is_indexed_hier_base` so an
                // array-element member wins over the generate-array hierarchical path.
                Some(TokenKind::Dot)
                    if self.struct_1d_array_member_base(&e)
                        && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
                        && self.peek_at(2) != Some(TokenKind::LParen) =>
                {
                    e = self.parse_struct_array_member(e);
                }
                // HIER-REST②: a `.` after a `name[idx]` select is a hierarchical
                // reference into a generate / instance-array element (`g[0].x`,
                // `bank[3].c.r`). Fold the CONSTANT index into the scope-segment name
                // (`g`+`[0]` ⇒ `g[0]`) so the normal hierarchical resolver handles it
                // — no new IR. (A deeper `g[0].sub[2].y` re-enters via this loop.)
                Some(TokenKind::Dot) if Self::is_indexed_hier_base(&e) => {
                    e = self.parse_indexed_hier(e);
                }
                // SV size/typedef cast `N'(e)` / `(W+1)'(e)` / `name'(e)` (§6.24):
                // a `'(` immediately after a primary. The already-parsed primary
                // `e` IS the casting type — do NOT re-parse it. REQUIRES `(` after
                // the apostrophe; a bare `'` or `'{` (assignment pattern, still
                // unsupported) is left for the diagnostic, never silently eaten.
                Some(TokenKind::Apostrophe) if self.peek_at(1) == Some(TokenKind::LParen) => {
                    e = self.parse_size_or_named_cast(e);
                }
                // G8 (§8.13): a method call chained on a CALL / method RESULT —
                // `s.substr(a,b).atoi()`. Fires only when `e` is already a Call/MethodCall
                // (a plain `a.b` hier path is folded in `expr_primary`, not here) and the
                // `.` is followed by `ident (`.
                Some(TokenKind::Dot)
                    if matches!(e.kind, ExprKind::Call { .. } | ExprKind::MethodCall { .. })
                        && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
                        && self.peek_at(2) == Some(TokenKind::LParen) =>
                {
                    e = self.parse_method_chain(e);
                }
                _ => break,
            }
        }
        // §3 ⑤ ⓐ: a select chain on a multi-dimensional packed PARAMETER (declared
        // flat) becomes the flat bit/part-select. A no-op for every other chain.
        self.rewrite_packed_md_select(e)
    }

    pub(crate) fn call_args(&mut self) -> Vec<Expr> {
        self.bump(); // '('
        let mut args = Vec::new();
        if self.peek() != Some(TokenKind::RParen) {
            loop {
                // G10 (IEEE §13.5.4): a NAMED argument `.formal(value)` / `.formal()`
                // (may follow positionals). Elaborate reorders these to positional.
                if self.peek() == Some(TokenKind::Dot) {
                    args.push(self.parse_named_call_arg());
                } else {
                    args.push(self.expr(0));
                }
                // PARSE-CONCAT-CAP: stop consuming once the node budget is blown.
                if self.node_budget_blown || !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "')'");
        args
    }

    /// G10: parse one named call argument `.formal(value)` or `.formal()` (empty ⇒ the
    /// formal's default value). Cursor is at the leading `.`.
    pub(crate) fn parse_named_call_arg(&mut self) -> Expr {
        let start = self.cur_span();
        self.bump(); // '.'
        let formal = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        self.expect(TokenKind::LParen, "'(' after named-argument formal");
        let value = if self.peek() == Some(TokenKind::RParen) {
            None
        } else {
            Some(Box::new(self.expr(0)))
        };
        self.expect(TokenKind::RParen, "')' after named-argument value");
        Expr {
            kind: ExprKind::NamedArg { formal, value },
            span: start.to(self.prev_span()),
        }
    }

    /// `{a,b}` concat OR `{n{a,b}}` replication. After parsing `first`, a following
    /// `{` ⇒ replication (first=count); the inner braced list becomes `value:
    /// Vec<Expr>` DIRECTLY (verdict M5 — no Concat wrapper). `{ {a},{b} }` is a
    /// concat-of-concats: `first={a}` then next is `,`, so concat path is taken.
    pub(crate) fn brace_expr(&mut self, start: Span) -> Expr {
        self.bump(); // outer '{'
                     // §11.4.14 streaming concatenation. `<<`/`>>` cannot begin an expression, so
                     // seeing one here decides the production with no lookahead and no ambiguity.
        if matches!(self.peek(), Some(TokenKind::Shl) | Some(TokenKind::Shr)) {
            return self.stream_expr(start);
        }
        let first = self.expr(0);
        if self.peek() == Some(TokenKind::LBrace) {
            // replication: first = count, inner {…} = the repeated element list.
            self.bump(); // inner '{'
            let mut value = vec![self.expr(0)];
            while !self.node_budget_blown && self.eat(TokenKind::Comma) {
                value.push(self.expr(0));
            }
            self.expect(TokenKind::RBrace, "'}' closing replication value");
            self.expect(TokenKind::RBrace, "'}' closing replication");
            return Expr {
                kind: ExprKind::Replicate {
                    count: Box::new(first),
                    value,
                },
                span: start.to(self.prev_span()),
            };
        }
        let mut parts = vec![first];
        while !self.node_budget_blown && self.eat(TokenKind::Comma) {
            parts.push(self.expr(0));
        }
        self.expect(TokenKind::RBrace, "'}'");
        Expr {
            kind: ExprKind::Concat { parts },
            span: start.to(self.prev_span()),
        }
    }

    /// The outer `{` of a concatenation has just been consumed and the cursor is on
    /// `<<`/`>>`: parse the rest of a STREAMING concatenation (IEEE 1800-2017
    /// §11.4.14).
    ///
    ///   `streaming_concatenation ::= { stream_operator [slice_size] { expr {, expr} } }`
    ///
    /// Both directions leave here as a MARKER system call that
    /// `elaborate::stream_concat` expands, because both need something the parser does
    /// not have:
    ///
    /// * `<<` — right-to-left — cuts the operand into `N`-bit blocks and reverses
    ///   them, so the result SHAPE depends on `$bits(operand)`, which is not known
    ///   until elaborate. It becomes [`STREAM_REV_FUNC`]`(N, {parts})`.
    /// * `>>` — left-to-right — is the identity for a packed operand and ignores the
    ///   slice size, so its VALUE is just the concatenation. It still becomes
    ///   [`STREAM_FWD_FUNC`]`({parts})` rather than a bare `Concat`, because
    ///   §11.4.14.3 pads a streaming rhs on the RIGHT when the assignment target is
    ///   wider (measured, verilator 5.050: `{>>{32'hAABBCCDD}}` into a 64-bit variable
    ///   is `aabbccdd00000000`, where the plain concatenation is `00000000aabbccdd`) —
    ///   and a bare `Concat` would erase the one fact that separates them.
    ///
    /// The dispatch that got us here is EXACT, not a heuristic: after `{` the grammar
    /// admits only an expression, and `<<`/`>>` are infix-only in SV — no legal
    /// concatenation, replication or assignment pattern can begin with one. `<<<`/
    /// `>>>` (`ShlA`/`ShrA`) are different tokens and are NOT stream operators, so
    /// they never reach here and keep the ordinary "expected expression" error.
    fn stream_expr(&mut self, start: Span) -> Expr {
        let right_to_left = self.peek() == Some(TokenKind::Shl);
        self.bump(); // `<<` / `>>`
                     // `slice_size ::= simple_type | constant_expression`. A TYPE (`{<<byte{…}}`)
                     // is refused BY NAME here rather than parsed as an expression, which would
                     // otherwise reach elaborate as an undefined identifier and be reported as a
                     // typo. Absent slice size ⇒ 1 (§11.4.14: "if not specified, the default is 1").
        let slice = if self.peek() == Some(TokenKind::LBrace) {
            Self::dec_lit(1, start)
        } else if self.net_var_kind().is_some() || self.peek_typedef_name().is_some() {
            self.error(
                "a constant slice size — vita does not support a TYPE as the slice \
                 size of a streaming concatenation (`{<<byte{…}}`, IEEE 1800-2017 \
                 §11.4.14); write the bit count (`{<<8{…}}`)",
            );
            self.skip_balanced_brace_group();
            return Expr {
                kind: ExprKind::Error,
                span: start.to(self.prev_span()),
            };
        } else {
            self.expr(0)
        };
        if !self.expect(TokenKind::LBrace, "'{' opening the stream expression list") {
            self.skip_balanced_brace_group();
            return Expr {
                kind: ExprKind::Error,
                span: start.to(self.prev_span()),
            };
        }
        let mut parts = vec![self.expr(0)];
        while !self.node_budget_blown && self.eat(TokenKind::Comma) {
            parts.push(self.expr(0));
        }
        // `stream_expression ::= expression [ with [ array_range_expression ] ]` — the
        // `with` form slices an UNPACKED array, which is the part of §11.4.14 vita does
        // not have at all. Named, not "expected '}'".
        if self.at_ident_kw("with") {
            self.error(
                "'}' closing the stream expression list — vita does not support the \
                 `with` range form of a streaming concatenation (`{<<8{a with [3:0]}}`, \
                 IEEE 1800-2017 §11.4.14)",
            );
            self.skip_balanced_brace_group(); // the inner group
            self.skip_balanced_brace_group(); // the outer group
            return Expr {
                kind: ExprKind::Error,
                span: start.to(self.prev_span()),
            };
        }
        self.expect(TokenKind::RBrace, "'}' closing the stream expression list");
        self.expect(TokenKind::RBrace, "'}' closing the streaming concatenation");
        let span = start.to(self.prev_span());
        let operand = Expr {
            kind: ExprKind::Concat { parts },
            span,
        };
        let (name, args) = if right_to_left {
            (STREAM_REV_FUNC, vec![slice, operand])
        } else {
            // `>>` carries no slice size into elaborate (§11.4.14 ignores it) — only
            // the fact that this WAS a stream, which §11.4.14.3's right-padding rule
            // needs at the assignment funnel.
            (STREAM_FWD_FUNC, vec![operand])
        };
        Expr {
            kind: ExprKind::SysCall {
                name: Ident {
                    name: name.to_string(),
                    span,
                },
                args,
            },
            span,
        }
    }

    /// Skip to the `}` that closes a brace group whose OPENING `{` the caller already
    /// consumed, and consume it. Used by the streaming rejections so the cursor ends
    /// exactly where a well-formed concatenation would have left it — an enclosing
    /// `= expr;` / `lhs = …;` then finishes normally and the named diagnostic is the
    /// ONLY line printed, instead of the cascade of follow-on "expected X" errors the
    /// old fall-through produced (three, at one `line:col`, for the lvalue form).
    pub(crate) fn skip_balanced_brace_group(&mut self) {
        let mut depth = 1i32;
        while let Some(k) = self.peek() {
            match k {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        self.bump(); // the closing '}'
                        break;
                    }
                }
                // A `;`/`endmodule` inside the group means the braces never close
                // (truncated source): stop rather than eat the rest of the file.
                TokenKind::Semi | TokenKind::Word(WordKind::Keyword(Kw::Endmodule)) => break,
                _ => {}
            }
            self.bump();
        }
    }

    /// The outer `{` of an assignment TARGET has just been consumed: if what follows
    /// is `<<`/`>>`, the source wrote the streaming UNPACK form (`{>>{b0,b1}} = a;`,
    /// §11.4.14), which vita does not have — the rhs direction ships, this one does
    /// not. Reports it by name and skips the group, so the enclosing `= rhs;` still
    /// parses and this is the only line. Returns whether it fired.
    ///
    /// Same exact `<<`/`>>` predicate as the rhs dispatch in `brace_expr`, for the
    /// same reason: neither token can begin an expression, so no legal concatenation
    /// lvalue can trip it.
    pub(crate) fn reject_streaming_lvalue(&mut self) -> bool {
        if !matches!(self.peek(), Some(TokenKind::Shl) | Some(TokenKind::Shr)) {
            return false;
        }
        self.error(
            "a net or variable — vita supports the streaming operator (pack/unpack, \
             IEEE 1800-2017 §11.4.14) only as a right-hand-side EXPRESSION, not as an \
             assignment target: write `{b0,b1,b2,b3} = {<<8{a}}` rather than \
             `{>>{b0,b1,b2,b3}} = a`",
        );
        self.skip_balanced_brace_group();
        true
    }

    pub(crate) fn parse_systask_call(&mut self) -> Stmt {
        let start = self.cur_span();
        // `$__vita_*` is vita's own namespace: a desugar that needs to say
        // something to elaborate has only the NAME to say it with (a marker
        // field would change the frozen AST shape), so those names are a private
        // channel. Source that writes one is injecting into that channel — e.g.
        // `$__vita_unique_violation` would file an IEEE §12.5.3 violation report
        // the design never violated. Loud, and general: it covers every future
        // desugar without anyone remembering to add a case. Reported BEFORE the
        // bump so the offending token in the message is the name itself.
        if self.cur_text().starts_with("$__vita_") {
            self.error(
                "a system task name outside vita's reserved `$__vita_` namespace \
                 (those names are synthesized by the compiler and cannot be written)",
            );
        }
        let t = self.bump().unwrap(); // SystemTask; lexeme retains `$`
        let name = Ident {
            name: self.src[t.span.clone()].to_string(),
            span: Self::sp(&t.span),
        };
        let args = if self.peek() == Some(TokenKind::LParen) {
            self.call_args()
        } else {
            Vec::new()
        };
        self.expect(TokenKind::Semi, "';'");
        Stmt::SysTaskCall {
            name,
            args,
            span: start.to(self.prev_span()),
        }
    }
}
