//! SVA sequences — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// SVA-REST: `let NAME [(formals)] = expr ;` (IEEE 1800 §11.13). Cursor on `let`.
    pub(crate) fn parse_let_decl(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `let`
        let name = self.ident()?;
        let formals = self.parse_sva_formals();
        self.expect(TokenKind::Eq, "'=' in a let declaration");
        let body = self.expr(0);
        self.expect(TokenKind::Semi, "';' after a let declaration");
        Some(ModuleItem::LetDecl(LetDecl {
            name,
            formals,
            body,
            span: start.to(self.prev_span()),
        }))
    }

    /// A synthetic `1` literal expr (the bare-property `1'b1 |-> e` sentinel and
    /// the named-instance placeholder consequent).
    pub(crate) fn sva_true_lit(span: Span) -> Expr {
        Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: "1".to_string(),
            },
            span,
        }
    }

    /// Disambiguate `sequence IDENT ( … )` (cursor on `sequence`) between an SVA
    /// sequence DECLARATION and a module instantiation of a V2005 module literally
    /// named `sequence`. The decl shape is exactly `sequence NAME ( formals ) ; BODY
    /// ; endsequence`, so AFTER the formals-terminator `;` the body is a single
    /// sequence expression with NO top-level `;`, and its terminating `;` is
    /// immediately followed by `endsequence`. An instantiation `sequence u(…) ;` has
    /// no such body+`endsequence`. We therefore (1) skip the balanced `( … )`, (2)
    /// require the formals `;`, then (3) scan the body to its next depth-0 `;` and
    /// accept ONLY if `endsequence` follows it. DECISIVE and bounded to the candidate
    /// construct — it cannot be poisoned by an unrelated later `sequence … endsequence`
    /// (review 2026-06-16: a content-independent scan-until-`endsequence` mis-routed a
    /// positional `sequence u(o);` merely followed by a real decl, and a fixed token
    /// budget flipped long decls). Lets `sequence u(.o(o));` stay an instantiation.
    pub(crate) fn is_sequence_decl_ahead(&self) -> bool {
        const BUDGET: usize = 65536;
        // (1) Skip the balanced `( … )` — peek_at(2) is the opening `(`.
        let mut i = 2usize;
        let mut depth = 0usize;
        loop {
            match self.peek_at(i) {
                None => return false,
                Some(TokenKind::LParen) => {
                    depth += 1;
                    i += 1;
                }
                Some(TokenKind::RParen) => {
                    i += 1;
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => i += 1,
            }
            if i > BUDGET {
                return false;
            }
        }
        // (2) The formals list must be terminated by `;`.
        if self.peek_at(i) != Some(TokenKind::Semi) {
            return false;
        }
        i += 1;
        // (3) Scan the body to its next depth-0 `;`; a decl has `endsequence` after it.
        let mut bdepth = 0usize;
        loop {
            match self.peek_at(i) {
                None => return false,
                Some(TokenKind::LParen | TokenKind::LBracket) => {
                    bdepth += 1;
                    i += 1;
                }
                Some(TokenKind::RParen | TokenKind::RBracket) => {
                    bdepth = bdepth.saturating_sub(1);
                    i += 1;
                }
                Some(TokenKind::Semi) if bdepth == 0 => {
                    return self.text_at(i + 1) == "endsequence";
                }
                // A hard module boundary before the body terminator ⇒ not a decl.
                Some(TokenKind::Word(WordKind::Keyword(Kw::Module | Kw::Endmodule)))
                    if bdepth == 0 =>
                {
                    return false
                }
                _ => i += 1,
            }
            if i > BUDGET {
                return false;
            }
        }
    }

    /// Named SVA `sequence NAME [(formals)]; <seq>; endsequence` (IEEE §16.8).
    /// Formal arguments (slice A1) bind by position at the use site; the body reuses
    /// the existing `parse_sequence`, so every sequence operator is available by name.
    pub(crate) fn parse_sequence_decl(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `sequence` (contextual keyword)
        let name = self.ident()?;
        let formals = self.parse_sva_formals();
        self.expect(TokenKind::Semi, "';' after sequence name");
        let body = self.parse_sequence();
        self.expect(TokenKind::Semi, "';' after sequence body");
        if self.at_ident_kw("endsequence") {
            self.bump();
        } else {
            self.error("`endsequence`");
        }
        self.eat_end_label();
        Some(ModuleItem::SequenceDecl(SeqDecl {
            name,
            formals,
            body,
            span: start.to(self.prev_span()),
        }))
    }

    /// Parse an SVA formal-argument list `( formal {, formal} )` after a named
    /// sequence/property name (slice A1, IEEE 1800 §16.8/§16.12). A formal is
    /// `[data_type] name [= default]`; the formal NAME is what elaborate substitutes,
    /// so we capture the LAST identifier before a top-level `,` / `)` / `=` and skip
    /// the optional type prefix and any default value (defaults are unsupported — all
    /// actuals must be passed; an arity mismatch is loud at the use site). No `(` →
    /// an empty list (a non-parameterized decl, byte-identical to before this slice).
    pub(crate) fn parse_sva_formals(&mut self) -> Vec<Ident> {
        let mut out = Vec::new();
        if self.peek() != Some(TokenKind::LParen) {
            return out;
        }
        self.bump(); // `(`
        if self.eat(TokenKind::RParen) {
            return out; // empty `()`
        }
        loop {
            match self.parse_one_sva_formal() {
                Some(id) => out.push(id),
                // An empty entry (`(,)`, `(,x)`, `(x,)`) is malformed — loud rather
                // than silently normalized (review 2026-06-16). Arity is still
                // enforced at the use site; this is recovery, not fatal.
                None => self.error("a formal name in the sequence/property formal list"),
            }
            if self.eat(TokenKind::Comma) {
                continue;
            }
            if !self.eat(TokenKind::RParen) {
                self.error("',' or ')' in a sequence/property formal list");
            }
            break;
        }
        out
    }

    /// One SVA formal: scan to the next top-level `,` / `)` / `=`, returning the last
    /// identifier seen (the formal name), regardless of any leading type / range
    /// tokens. A `= default` value is parse-and-dropped (unsupported).
    pub(crate) fn parse_one_sva_formal(&mut self) -> Option<Ident> {
        let mut last: Option<Ident> = None;
        loop {
            match self.peek() {
                Some(TokenKind::Comma) | Some(TokenKind::RParen) | None => break,
                Some(TokenKind::Eq) => {
                    self.bump(); // `=`
                    let _ = self.expr(0); // default value — consumed and ignored
                    break;
                }
                _ if self.is_ident() => {
                    last = self.ident();
                }
                _ => {
                    self.bump(); // a type keyword / `[m:l]` range / etc.
                }
            }
        }
        last
    }

    /// True at a sequence/property body LOCAL-VARIABLE declaration (slice A2): a
    /// data-type keyword (`logic`/`reg`/`integer`/`bit`-via-`logic`/…) or an SV
    /// integral type name lexed as an identifier (`int`/`bit`/`byte`/`shortint`/
    /// `longint`). A property/sequence body must otherwise begin with `@(clk)` (a
    /// property) or a sequence expression, so a type at the body start is a local var.
    pub(crate) fn at_sva_local_var_decl(&self) -> bool {
        if self.net_var_kind().is_some() {
            return true;
        }
        self.is_ident()
            && matches!(
                self.cur_text(),
                "int" | "bit" | "byte" | "shortint" | "longint"
            )
    }

    /// Parse ONE sequence/property local-variable declaration (slice N2c, IEEE
    /// §16.10): `<type> [packed_range] <name> [= init] ;`. The cursor is on the type
    /// keyword (`at_sva_local_var_decl` is true). Returns the resolved width/sign so
    /// elaborate can size the data-tracking register. `None` on a malformed decl (the
    /// caller recovers by skipping to the next `;`). Only the integral atom types and
    /// a literal packed range are supported; the type keyword itself is consumed.
    pub(crate) fn parse_sva_local_decl(&mut self) -> Option<SvaLocalDecl> {
        let start = self.cur_span();
        let kind = self.net_var_kind()?;
        self.bump(); // the type keyword
                     // Atom storage width by kind; a packed range (below) overrides for the
                     // vector-capable kinds (bit/logic/reg).
                     // `unsupported`: the declared type is NOT a synthesizable fixed-width
                     // integral var, so a capture into it has no data-tracking register in this
                     // subset. The width/sign below are a 1-bit placeholder; elaborate's
                     // `synth_local_var_assert` reads this flag and LOUD-rejects the capture —
                     // never a silent 1-bit truncation that flips the assertion verdict.
        let (atom_width, ranged_ok, unsupported): (u32, bool, bool) = match kind {
            NetVarKind::Byte => (8, false, false),
            NetVarKind::Shortint => (16, false, false),
            NetVarKind::Int | NetVarKind::Integer => (32, false, false),
            NetVarKind::Longint | NetVarKind::Time => (64, false, false),
            NetVarKind::Bit | NetVarKind::Logic | NetVarKind::Reg => (1, true, false),
            // Real / realtime / string / event / nets / class are not a
            // synthesizable data-tracking var — flag them so elaborate loud-rejects
            // (carry a 1-bit placeholder width; the type is validated at the capture).
            _ => (1, false, true),
        };
        // Optional packed range `[msb:lsb]` (vector kinds only). Literal bounds only;
        // a non-literal bound recovers via `parse_small_const` (loud) → width fallback.
        let width = if ranged_ok && self.peek() == Some(TokenKind::LBracket) {
            self.bump(); // `[`
            let msb = self.parse_small_const("a packed-range MSB in an SVA local var");
            self.expect(TokenKind::Colon, "':' in an SVA local-var packed range");
            let lsb = self.parse_small_const("a packed-range LSB in an SVA local var");
            self.expect(
                TokenKind::RBracket,
                "']' to close an SVA local-var packed range",
            );
            msb.abs_diff(lsb) + 1
        } else {
            atom_width
        };
        let signed = atom_default_signed(Some(kind));
        let name = self.ident()?;
        let init = if self.eat(TokenKind::Eq) {
            Some(self.expr(0))
        } else {
            None
        };
        self.expect(
            TokenKind::Semi,
            "';' after an SVA local-variable declaration",
        );
        Some(SvaLocalDecl {
            name,
            width,
            signed,
            unsupported_type: unsupported,
            init,
            span: start.to(self.prev_span()),
        })
    }

    /// True if the upcoming `( … )` (cursor on `(`) contains a comma at paren-depth
    /// one — a sequence MATCH-ITEM local-variable list `(bool, x = e, …)` (slice A2).
    /// A parenthesized sequence has no top-level comma; concat/select commas nest
    /// deeper (counted via all bracket kinds), so they are not mistaken for one.
    pub(crate) fn at_sva_match_item_paren(&self) -> bool {
        if self.peek() != Some(TokenKind::LParen) {
            return false;
        }
        const BUDGET: usize = 8192;
        let mut depth = 0usize;
        let mut i = 0usize;
        while i < BUDGET {
            match self.peek_at(i) {
                None => return false,
                Some(
                    TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBrace
                    | TokenKind::LBracketStar
                    | TokenKind::LBracketArrow
                    | TokenKind::LBracketEq,
                ) => {
                    depth += 1;
                    i += 1;
                }
                Some(TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace) => {
                    depth = depth.saturating_sub(1);
                    i += 1;
                    if depth == 0 {
                        return false; // outer paren closed with no top-level comma
                    }
                }
                Some(TokenKind::Comma) if depth == 1 => return true,
                _ => i += 1,
            }
        }
        false
    }

    /// True if the upcoming `( … )` (cursor on `(`) is a parenthesized SUB-SEQUENCE
    /// — it contains a `##` cycle-delay concatenation at paren-depth one (slice A.2
    /// cross-clock multi-term segment, e.g. `@(c1)(a ##1 b)`). A parenthesized boolean
    /// expression `(a && b)` has no top-level `##`, so it is left to `self.expr(0)`
    /// (byte-identical). A match-item paren is detected separately and takes priority.
    pub(crate) fn at_paren_subsequence(&self) -> bool {
        if self.peek() != Some(TokenKind::LParen) {
            return false;
        }
        const BUDGET: usize = 8192;
        let mut depth = 0usize;
        let mut i = 0usize;
        while i < BUDGET {
            match self.peek_at(i) {
                None => return false,
                Some(
                    TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBrace
                    | TokenKind::LBracketStar
                    | TokenKind::LBracketArrow
                    | TokenKind::LBracketEq,
                ) => {
                    depth += 1;
                    i += 1;
                }
                Some(TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace) => {
                    depth = depth.saturating_sub(1);
                    i += 1;
                    if depth == 0 {
                        return false; // outer paren closed with no top-level `##`
                    }
                }
                Some(TokenKind::HashHash) if depth == 1 => return true,
                _ => i += 1,
            }
        }
        false
    }

    /// Parse an SVA sequence (Phase-3 slices S4/S5): `##n` / bounded-range
    /// `##[m:n]` cycle-delay concatenation (left-associative, looser) over
    /// primaries that may carry a `[*n]` / `[*m:n]` consecutive-repetition
    /// postfix (tighter). Unbounded (`[*m:$]`/`##[m:$]`) / goto / nonconsecutive
    /// / `throughout` / `within` forms are deferred — loud at the enclosing
    /// `expect`. (min,max) carry the bound; max==Some(min) is the single-count
    /// form.
    pub(crate) fn parse_sequence(&mut self) -> Sequence {
        // `##` concatenation (tightest of the binary sequence ops).
        let lhs = self.parse_seq_concat();
        // `seq1 within seq2` (slice S9) — contextual keyword, binary over `##`
        // chains, RHS a full sequence.
        if self.at_ident_kw("within") {
            self.bump(); // `within`
            let rhs = self.parse_sequence();
            return Sequence::Within {
                seq1: Box::new(lhs),
                seq2: Box::new(rhs),
            };
        }
        // `cond throughout seq` (slice S7) — `throughout` is a contextual keyword;
        // its left operand must be a boolean leaf, its right operand a full
        // sequence (looser than `##`, so `g throughout a ##2 c` is
        // `g throughout (a ##2 c)`).
        if self.at_ident_kw("throughout") {
            self.bump(); // `throughout`
            let seq = self.parse_sequence();
            return match lhs {
                Sequence::Boolean(cond) => Sequence::Throughout {
                    cond: Box::new(cond),
                    seq: Box::new(seq),
                },
                _ => {
                    self.error("`throughout` requires a boolean left operand");
                    seq
                }
            };
        }
        lhs
    }

    /// `##`-concatenation chain over sequence primaries (left-associative).
    pub(crate) fn parse_seq_concat(&mut self) -> Sequence {
        // A leading `##N` with no left operand — e.g. the consequent of
        // `a |-> ##1 b`. Per IEEE 1800 §16.9, `##N b` ≡ `1 ##N b` (a true leaf
        // delayed by N). Synthesize the implicit `1` so the delay chain has a left
        // operand; this produces the SAME pipeline `a |=> b` / `1 ##1 b` already do
        // (golden-neutral). Without it the primary parser hits `##` as an expression
        // and reports a spurious E2002.
        let mut lhs = if self.peek() == Some(TokenKind::HashHash) {
            Sequence::Boolean(Self::sva_true_lit(self.cur_span()))
        } else {
            self.parse_seq_primary()
        };
        while self.peek() == Some(TokenKind::HashHash) {
            self.bump(); // `##`
            let (min, max) = self.parse_seq_delay();
            let rhs = self.parse_seq_primary();
            lhs = Sequence::Delay {
                min,
                max,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        lhs
    }

    /// A sequence primary: a boolean leaf expression, optionally followed by one
    /// or more repetition postfixes — `[*n]`/`[*m:n]` consecutive, `[->n]` goto,
    /// or `[=n]` nonconsecutive.
    pub(crate) fn parse_seq_primary(&mut self) -> Sequence {
        // A `@(...)` clocking event at a sequence primary is a multi-clock RE-CLOCKING
        // boundary (slice N2a): `a ##1 @(c2) b`. The leading property clock was already
        // consumed by `parse_concurrent_assert`, so a `@` here re-establishes the
        // sampling clock for the following primary from this `##`-boundary onward
        // (IEEE 1800 §16.13/§16.16 clock flow). Wrap the following primary in
        // `Sequence::Clocked`; elaborate's `synth_crossclock` handles the supported
        // `a ##1 @(c2) b` shape and loud-rejects the rest.
        if self.peek() == Some(TokenKind::At) {
            let clock = self.parse_sensitivity();
            let seq = self.parse_seq_primary();
            return Sequence::Clocked {
                clock,
                seq: Box::new(seq),
            };
        }
        // A `( boolean , local_var = expr {, …} )` match-item paren (slice N2c, IEEE
        // §16.10) is a sequence LOCAL-VARIABLE capture — a top-level comma just inside
        // the paren distinguishes it from a parenthesized sequence (which has none).
        // Parse the boolean term + the `name = expr` assignment list into a
        // `Sequence::MatchItem`; elaborate lowers a fixed-delay single capture to a
        // parallel data shift register (and loud-rejects the convergence cases).
        if self.at_sva_match_item_paren() {
            return self.parse_seq_match_item();
        }
        // A parenthesized SUB-SEQUENCE `( … ## … )` (slice A.2 cross-clock multi-term
        // segment) — `self.expr(0)` cannot parse a `##` cycle-delay, so when the group
        // carries a top-level `##` recurse into `parse_sequence`. A parenthesized
        // boolean `(a && b)` has no top-level `##` → falls through to `expr(0)` (byte-
        // identical). The result still flows through the postfix-repetition loop below.
        if self.at_paren_subsequence() {
            self.bump(); // `(`
            let inner = self.parse_sequence();
            self.expect(TokenKind::RParen, "')' to close a parenthesized sequence");
            return self.parse_seq_postfix(inner);
        }
        let e = self.expr(0);
        let seq = Sequence::Boolean(e);
        self.parse_seq_postfix(seq)
    }

    /// Parse a sequence MATCH-ITEM `( <bool> , <name> = <expr> {, <name> = <expr>} )`
    /// (slice N2c, IEEE §16.10): a boolean term that CAPTURES local variable(s) when
    /// it matches. The cursor is on `(` and `at_sva_match_item_paren` is true (a
    /// top-level comma was seen). Emits `Sequence::MatchItem`; the postfix loop still
    /// applies (a `(...)[*n]` repeated capture is parsed but elaborate loud-rejects it
    /// — repetition would converge multiple captures, a data collision).
    pub(crate) fn parse_seq_match_item(&mut self) -> Sequence {
        self.bump(); // `(`
        let term = self.expr(0); // the boolean term
        let mut assigns: Vec<(Ident, Expr)> = Vec::new();
        // At least one `, name = expr` (the detector guaranteed a top-level comma).
        while self.eat(TokenKind::Comma) {
            let name = match self.ident() {
                Some(id) => id,
                None => {
                    self.error("a local-variable name in a sequence match-item `(b, x = e)`");
                    break;
                }
            };
            if !self.eat(TokenKind::Eq) {
                self.error("`=` in a sequence match-item local-variable assignment");
                break;
            }
            let val = self.expr(0);
            assigns.push((name, val));
        }
        self.expect(TokenKind::RParen, "')' to close a sequence match-item");
        let mi = Sequence::MatchItem {
            seq: Box::new(Sequence::Boolean(term)),
            assigns,
        };
        self.parse_seq_postfix(mi)
    }

    /// Apply the trailing repetition postfixes (`[*n]`/`[*m:n]` consecutive, `[+]`,
    /// `[->n]` goto, `[=n]` nonconsecutive) to an already-parsed sequence primary.
    /// Shared by the boolean-leaf and parenthesized-subsequence primary forms.
    pub(crate) fn parse_seq_postfix(&mut self, mut seq: Sequence) -> Sequence {
        loop {
            match self.peek() {
                Some(TokenKind::LBracketStar) => {
                    self.bump(); // `[*`
                    let (min, max) = self.parse_seq_repeat_bounds();
                    self.expect(TokenKind::RBracket, "']' to close `[*n]`");
                    seq = Sequence::Repeat {
                        seq: Box::new(seq),
                        min,
                        max,
                        kind: RepeatKind::Consec,
                    };
                }
                // SVA-REST `seq[+]` consecutive-repetition sugar ≡ `seq[*1:$]`
                // (one-or-more, unbounded — the S13 run-latch). `seq[*]` (≡ `[*0:$]`,
                // a zero-or-more EMPTY match) is `parse_seq_repeat_bounds` → (0, None).
                Some(TokenKind::BracketPlus) => {
                    self.bump(); // `[+]`
                    seq = Sequence::Repeat {
                        seq: Box::new(seq),
                        min: 1,
                        max: None,
                        kind: RepeatKind::Consec,
                    };
                }
                Some(tok @ (TokenKind::LBracketArrow | TokenKind::LBracketEq)) => {
                    self.bump(); // `[->` / `[=`
                    let (which, kind) = if tok == TokenKind::LBracketArrow {
                        ("[->n]", RepeatKind::Goto)
                    } else {
                        ("[=n]", RepeatKind::Nonconsec)
                    };
                    let n = self.parse_seq_count_single(which);
                    self.expect(TokenKind::RBracket, "']' to close goto/nonconsec count");
                    seq = Sequence::Repeat {
                        seq: Box::new(seq),
                        min: n,
                        max: Some(n),
                        kind,
                    };
                }
                _ => break,
            }
        }
        seq
    }

    /// Single positive count for `[->n]` / `[=n]`. Ranges (`[->m:n]`) and `0`
    /// are deferred (loud, recovered to 1).
    pub(crate) fn parse_seq_count_single(&mut self, which: &'static str) -> u32 {
        let n = self.parse_small_const(which);
        if self.peek() == Some(TokenKind::Colon) {
            self.error("a single goto/nonconsec count (ranges are unsupported in this subset)");
        }
        if n == 0 {
            self.error("a positive goto/nonconsec count");
            return 1;
        }
        n
    }

    /// Cycle delay after `##`: `##n` → (n, Some(n)), bounded range `##[m:n]`
    /// → (m, Some(n)), or unbounded `##[m:$]` → (m, None) (slice S6).
    pub(crate) fn parse_seq_delay(&mut self) -> (u32, Option<u32>) {
        if self.peek() == Some(TokenKind::LBracket) {
            self.bump(); // `[`
            let lo = self.parse_small_const("a lower bound in `##[m:n]`");
            self.expect(TokenKind::Colon, "':' in `##[m:n]`");
            if self.peek() == Some(TokenKind::Dollar) {
                self.bump(); // `$` — unbounded upper bound
                self.expect(TokenKind::RBracket, "']'");
                return (lo, None);
            }
            let hi = self.parse_small_const("an upper bound in `##[m:n]`");
            self.expect(TokenKind::RBracket, "']'");
            let (lo, hi) = (lo.min(hi), lo.max(hi));
            return (lo, Some(hi));
        }
        let n = self.parse_small_const("a constant cycle delay after `##`");
        (n, Some(n))
    }

    /// `[*n]` repetition bounds: `[*n]` → (n, Some(n)), bounded range `[*m:n]`
    /// → (m, Some(n)), unbounded `[*m:$]` → (m, None) (slice S13), or a zero
    /// lower bound — `[*0]`/`[*0:0]` → (0, Some(0)) (exactly empty), `[*0:n]` →
    /// (0, Some(n)), bare `[*]`/`[*0:$]` → (0, None) (empty-or-more). The empty
    /// (zero-repetition) match is synthesized for SUFFIX/MIDDLE positions
    /// (`a ##1 b[*0:n]`); a leading/standalone empty is honest-loud at elaborate
    /// (the empty SEED's -1 offset is not expressible). See `sva_empty_match.rs`.
    /// Caller consumed `[*`; this stops before `]`.
    pub(crate) fn parse_seq_repeat_bounds(&mut self) -> (u32, Option<u32>) {
        // Bare `[*]` ≡ `[*0:$]` — zero-or-more (empty-or-more).
        if self.peek() == Some(TokenKind::RBracket) {
            return (0, None);
        }
        let lo = self.parse_small_const("a repetition count in `[*n]`");
        if self.peek() == Some(TokenKind::Colon) {
            self.bump(); // ':'
            if self.peek() == Some(TokenKind::Dollar) {
                self.bump(); // `$` — unbounded upper bound: `[*m:$]` (>= m)
                return (lo, None);
            }
            let hi = self.parse_small_const("an upper bound in `[*m:n]`");
            let (lo, hi) = (lo.min(hi), lo.max(hi));
            return (lo, Some(hi));
        }
        (lo, Some(lo))
    }

    // ─────────────────────── 5. blocks ───────────────────────
    pub(crate) fn parse_seq_block(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // begin
        let label = self.opt_block_label();
        let (decls, stmts) = self.block_body(BlockEnd::End);
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::End)), "'end'");
        self.opt_block_label(); // optional `: end_label` (no AST slot → discard)
        Stmt::Block {
            label,
            decls,
            stmts,
            span: start.to(self.prev_span()),
        }
    }
}
