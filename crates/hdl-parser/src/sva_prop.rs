//! SVA properties — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Parse a property spec `@(clk) [disable iff(e)] seq [ |-> | |=> ] seq` — the
    /// body shared by an inline `assert property( <spec> )` and a named
    /// `property NAME; <spec>; endproperty`. Does NOT consume the surrounding
    /// parens / terminators; the caller does.
    pub(crate) fn parse_property_spec(&mut self, start: Span) -> PropertySpecParts {
        // Sequence/property LOCAL VARIABLES (slice N2c, IEEE 1800-2017 §16.10): typed
        // declarations at the body start (`property p; int x; @(clk) …`). Captured at
        // a `(b, x = e)` match-item and read at a later term/consequent within the
        // SAME match attempt. Elaborate lowers a FIXED-DELAY single-capture to a
        // parallel DATA shift register (no per-attempt collision); ranges / multi-write
        // / cross-clock are loud-rejected there (the convergence cases). Parse the
        // declarations here into AST; each is `<type> <name> [= e] ;` and ends at its
        // own `;` (which precedes the clock).
        let mut local_vars: Vec<SvaLocalDecl> = Vec::new();
        while self.at_sva_local_var_decl() {
            if let Some(decl) = self.parse_sva_local_decl() {
                local_vars.push(decl);
            } else {
                // Malformed decl — recover by skipping to the next `;` / `@` so the
                // rest of the property still parses (never silently swallow it).
                while !matches!(
                    self.peek(),
                    Some(TokenKind::Semi) | Some(TokenKind::At) | None
                ) {
                    self.bump();
                }
                if !self.eat(TokenKind::Semi) {
                    break;
                }
            }
        }
        // Clocking event `@(...)`. `parse_sensitivity` consumes the leading `@`.
        //
        // A MISSING one is not an error here: IEEE 1800 §14.12 lets a concurrent
        // assertion inherit the enclosing scope's `default clocking`, and whether one
        // exists is a module-level fact this parse cannot see (the `default clocking`
        // item may come later in the body). So leave the ESTABLISHED empty-list
        // sentinel — `assert property(NAME)` already parses to exactly that, and
        // `materialize_sva_checkers` is the one place that resolves it: it substitutes
        // the default clocking, or reports that there is none. Erroring here instead
        // reported the wrong cause (the assertion is fine; the scope may lack a clock)
        // and made `default clocking` unusable even though the block itself parsed.
        let clock = if self.peek() == Some(TokenKind::At) {
            self.parse_sensitivity()
        } else {
            Sensitivity::List(Vec::new())
        };
        // Optional `disable iff (expr)` reset (slice S12), between the clocking
        // event and the property expression. `disable` is a keyword; `iff` is a
        // contextual keyword (a plain identifier elsewhere).
        let disable_iff = if self.at_kw(Kw::Disable) {
            self.bump(); // `disable`
            if self.at_ident_kw("iff") {
                self.bump(); // `iff`
            } else {
                self.error("`iff` after `disable` in a concurrent assertion");
            }
            self.expect(TokenKind::LParen, "'(' after `disable iff`");
            let e = self.expr(0);
            self.expect(TokenKind::RParen, "')' after `disable iff` condition");
            Some(e)
        } else {
            None
        };
        // Sequence/property LOCAL VARIABLES can ALSO follow the clocking event (IEEE
        // §16.10 `property_spec`: the assertion_variable_declaration comes after the
        // clocking_event) — `@(clk) int x; (a, x=d) ##1 b |-> …`. Parse them here too
        // (the before-clock loop above covers the named-property `property p; int x;`
        // form). A type keyword at the property-expression start is unambiguously a
        // local-var decl (a sequence term never begins with a bare type keyword).
        while self.at_sva_local_var_decl() {
            if let Some(decl) = self.parse_sva_local_decl() {
                local_vars.push(decl);
            } else {
                while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
                    self.bump();
                }
                if !self.eat(TokenKind::Semi) {
                    break;
                }
            }
        }
        // A property wrapped whole in parentheses (`assert property (@(posedge c)
        // disable iff (r) (a |-> b))`, lowRISC's `ASSERT` macro shape) is the same
        // property: strip every such pair and parse the flat body between them, so
        // `(a |-> b)` lowers byte-identically to `a |-> b`. Only a group that runs
        // to the property's own end is stripped — `(a |-> b) or (c |-> d)` keeps its
        // groups for the tree path below, and `(a |-> b) ##1 c` is left to fail as
        // it did (a parenthesized property is not a sequence).
        let mut stripped = 0usize;
        while self.peek() == Some(TokenKind::LParen) && self.paren_group_is_whole_property() {
            self.bump(); // `(`
            stripped += 1;
        }
        let (antecedent, implication_kind, consequent, consequent_clock, prop_expr) =
            self.parse_property_expr_parts(start);
        for _ in 0..stripped {
            self.expect(TokenKind::RParen, "')' to close a parenthesized property");
        }
        (
            clock,
            disable_iff,
            antecedent,
            implication_kind,
            consequent,
            consequent_clock,
            prop_expr,
            local_vars,
        )
    }

    /// The property expression of a spec, after the clock, `disable iff` and any
    /// local-variable declarations: `(antecedent, kind, consequent, consequent clock,
    /// property tree)`. The flat fields are the byte-identical legacy path; the tree
    /// is `Some` only when a property-level operator is present.
    #[allow(clippy::type_complexity)]
    pub(crate) fn parse_property_expr_parts(
        &mut self,
        start: Span,
    ) -> (
        Sequence,
        ImplicationKind,
        Sequence,
        Option<Sensitivity>,
        Option<PropExpr>,
    ) {
        // Property-level operators (slice N2d + SVA-REST): when the body uses a
        // top-level (paren-depth-0) property operator — `and`/`or` (N2d), or
        // `not`/`always`/`until`/`s_until`/`implies`/`iff`/`s_eventually`/`nexttime`
        // (SVA-REST) — parse a `PropExpr` TREE instead of the flat `seq impl seq`.
        // The flat fields then hold inert placeholders; elaborate dispatches on
        // `Some(prop_expr)`. This detection keeps every operator-free property (the
        // whole existing flat corpus) on the byte-identical flat path below —
        // including slice A3 multi-clock, whose `@(c2)` consequent clock the tree
        // grammar does NOT carry (combining a tree with multi-clock is out of subset
        // → loud at elaborate). An operator inside the clocking event or a
        // parenthesized sub-expression is at depth > 0 and ignored.
        if self.prop_has_toplevel_op() {
            let pe = self.parse_prop_expr();
            let true_lit = Self::sva_true_lit(start);
            return (
                Sequence::Boolean(true_lit.clone()),
                ImplicationKind::Overlap,
                Sequence::Boolean(true_lit),
                None,
                Some(pe),
            );
        }
        // `seq [ |-> | |=> ] expr` — a bare `property(@(clk) expr)` (no
        // implication) desugars to `1'b1 |-> expr`; `seq [ |-> | |=> ] seq` — the
        // consequent is also a Sequence (slice S14). A leading `@(c2)` on the
        // consequent of a `|=>` is a multi-clock property (slice A3).
        let ante_seq = self.parse_sequence();
        if self.eat(TokenKind::PipeArrow) {
            let cons_clock = self.parse_optional_consequent_clock(true);
            (
                ante_seq,
                ImplicationKind::Overlap,
                self.parse_sequence(),
                cons_clock,
                None,
            )
        } else if self.eat(TokenKind::PipeEqArrow) {
            let cons_clock = self.parse_optional_consequent_clock(false);
            (
                ante_seq,
                ImplicationKind::NonOverlap,
                self.parse_sequence(),
                cons_clock,
                None,
            )
        } else {
            let true_lit = Self::sva_true_lit(start);
            match ante_seq {
                // bare `property(@(clk) expr)` desugars to `1'b1 |-> expr`.
                Sequence::Boolean(e) => (
                    Sequence::Boolean(true_lit),
                    ImplicationKind::Overlap,
                    Sequence::Boolean(e),
                    None,
                    None,
                ),
                other => {
                    self.error("an implication `|->`/`|=>` (a bare sequence property is unsupported in this subset)");
                    (
                        other,
                        ImplicationKind::Overlap,
                        Sequence::Boolean(true_lit),
                        None,
                        None,
                    )
                }
            }
        }
    }

    /// Bounded paren/bracket-balanced lookahead from the cursor (which sits at the
    /// start of a property expression, after the clock + `disable iff`): true iff a
    /// property-level `and`/`or` keyword appears at depth 0 before the property's
    /// closing `)` (inline `assert property( … )`) or its `;` (a `property NAME; …;
    /// endproperty` declaration). Decisive and cannot be poisoned by a later
    /// construct — it stops at the first depth-underflow `)` / depth-0 `;` /
    /// `endproperty` / module boundary / EOF. `and`/`or` nested in the clocking
    /// event or a parenthesized sub-expression is at depth > 0 and ignored.
    pub(crate) fn prop_has_toplevel_op(&self) -> bool {
        const BUDGET: usize = 65536;
        let mut i = 0usize;
        let mut depth: i32 = 0;
        loop {
            match self.peek_at(i) {
                None => return false,
                // SVA repeat-open tokens (`[*` / `[->` / `[=`) open a bracket that
                // closes with a plain `]` (RBracket), so they must count for depth
                // or the `]` underflows and a trailing top-level operator is missed
                // (review N2d — the same new-token-vs-bracket-scan hazard as N2a-1).
                Some(
                    TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBracketStar
                    | TokenKind::LBracketArrow
                    | TokenKind::LBracketEq,
                ) => depth += 1,
                Some(TokenKind::RParen | TokenKind::RBracket) => {
                    if depth == 0 {
                        return false; // the property's closing `)` (inline form)
                    }
                    depth -= 1;
                }
                Some(TokenKind::Semi) if depth == 0 => return false, // decl body terminator
                // N2d keyword property operators (`and`/`or`) + SVA-REST prefix
                // `not`/`always`.
                Some(TokenKind::Word(WordKind::Keyword(
                    Kw::And | Kw::Or | Kw::Not | Kw::Always,
                ))) if depth == 0 => return true,
                Some(TokenKind::Word(WordKind::Keyword(Kw::Module | Kw::Endmodule)))
                    if depth == 0 =>
                {
                    return false
                }
                // SVA-REST contextual property operators (`until`/`implies`/
                // `s_eventually`/`nexttime`/…) — reserved SV words, so a property body
                // identifier never legitimately collides with them.
                Some(TokenKind::Word(WordKind::Ident)) if depth == 0 => {
                    if Self::is_prop_op_text(self.text_at(i)) {
                        return true;
                    }
                }
                _ => {}
            }
            if self.peek_at(i).is_some() && self.text_at(i) == "endproperty" {
                return false;
            }
            i += 1;
            if i > BUDGET {
                return false;
            }
        }
    }

    /// True for a CONTEXTUAL (non-keyword in our lexer, but reserved in SV) property
    /// operator word — the infix `until`/`s_until`/`implies`/`iff` and prefix
    /// `eventually`/`s_eventually`/`nexttime`/`s_nexttime`/`s_always`. These are
    /// IEEE 1800 reserved words, so a property-body identifier never legitimately
    /// shadows them (unlike a hand-rolled keyword guess).
    pub(crate) fn is_prop_op_text(s: &str) -> bool {
        matches!(
            s,
            "until"
                | "s_until"
                | "implies"
                | "iff"
                | "eventually"
                | "s_eventually"
                | "nexttime"
                | "s_nexttime"
                | "s_always"
        )
    }

    /// Parse a property expression (slice N2d + SVA-REST). Precedence loosest→
    /// tightest: `implies`/`iff` < `until`/`s_until` < `or` < `and` < unary prefix
    /// (`not`/`always`/`s_eventually`/`nexttime`) < sequence-implication < primary.
    /// Reached only when `prop_has_toplevel_op` detected a property-level operator.
    pub(crate) fn parse_prop_expr(&mut self) -> PropExpr {
        self.parse_prop_implies()
    }

    /// `lhs implies rhs` / `lhs iff rhs` (SVA-REST) — desugared to the `and`/`or`/`not`
    /// core: `p implies q` ≡ `(not p) or q`; `p iff q` ≡ `(p implies q) and (q implies
    /// p)`. Right-associative (`a implies b implies c` = `a implies (b implies c)`).
    pub(crate) fn parse_prop_implies(&mut self) -> PropExpr {
        let lhs = self.parse_prop_until();
        if self.eat_ident_kw("implies") {
            let rhs = self.parse_prop_implies();
            // p implies q ≡ (not p) or q
            return PropExpr::Or(Box::new(PropExpr::Not(Box::new(lhs))), Box::new(rhs));
        }
        if self.eat_ident_kw("iff") {
            let rhs = self.parse_prop_implies();
            // p iff q ≡ (not p or q) and (not q or p)
            let pq = PropExpr::Or(
                Box::new(PropExpr::Not(Box::new(lhs.clone()))),
                Box::new(rhs.clone()),
            );
            let qp = PropExpr::Or(Box::new(PropExpr::Not(Box::new(rhs))), Box::new(lhs));
            return PropExpr::And(Box::new(pq), Box::new(qp));
        }
        lhs
    }

    /// `lhs until rhs` / `lhs s_until rhs` (SVA-REST, non-associative single use).
    pub(crate) fn parse_prop_until(&mut self) -> PropExpr {
        let lhs = self.parse_prop_or();
        let strong = if self.at_ident_kw("s_until") {
            self.bump();
            true
        } else if self.eat_ident_kw("until") {
            false
        } else {
            return lhs;
        };
        let rhs = self.parse_prop_or();
        PropExpr::Until {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            strong,
        }
    }

    pub(crate) fn parse_prop_or(&mut self) -> PropExpr {
        let mut lhs = self.parse_prop_and();
        while self.at_kw(Kw::Or) {
            self.bump(); // `or`
            let rhs = self.parse_prop_and();
            lhs = PropExpr::Or(Box::new(lhs), Box::new(rhs));
        }
        lhs
    }

    pub(crate) fn parse_prop_and(&mut self) -> PropExpr {
        let mut lhs = self.parse_prop_unary();
        while self.at_kw(Kw::And) {
            self.bump(); // `and`
            let rhs = self.parse_prop_unary();
            lhs = PropExpr::And(Box::new(lhs), Box::new(rhs));
        }
        lhs
    }

    /// Unary prefix property operators (SVA-REST): `not p`, `always p`,
    /// `s_eventually p`, `nexttime p` (right-recursive: `not always p` =
    /// `Not(Always(p))`). `nexttime`/`s_nexttime` desugar to `1'b1 |=> p`. The bounded
    /// forms (`s_eventually [m:n]`, `nexttime [n]`, `s_always`) and weak unbounded
    /// `eventually` are loud-rejected (recovery: parse the operand so the rest syncs).
    pub(crate) fn parse_prop_unary(&mut self) -> PropExpr {
        if self.eat_kw(Kw::Not) {
            return PropExpr::Not(Box::new(self.parse_prop_unary()));
        }
        if self.eat_kw(Kw::Always) {
            return PropExpr::Always(Box::new(self.parse_prop_unary()));
        }
        if self.at_ident_kw("s_eventually") || self.at_ident_kw("eventually") {
            let strong = self.cur_text() == "s_eventually";
            self.bump();
            if self.peek() == Some(TokenKind::LBracket) {
                self.error(
                    "an unbounded `s_eventually` (a bounded `s_eventually [m:n]` range \
                     is unsupported in this subset)",
                );
                // consume the `[ … ]` for recovery.
                let mut d = 0i32;
                while let Some(t) = self.peek() {
                    match t {
                        TokenKind::LBracket => d += 1,
                        TokenKind::RBracket => {
                            d -= 1;
                            if d == 0 {
                                self.bump();
                                break;
                            }
                        }
                        _ => {}
                    }
                    self.bump();
                }
            }
            if !strong {
                self.error(
                    "`s_eventually` (a weak unbounded `eventually` has no bounded-sim \
                     verdict; use `s_eventually`)",
                );
            }
            return PropExpr::Eventually {
                strong: true,
                prop: Box::new(self.parse_prop_unary()),
            };
        }
        if self.at_ident_kw("nexttime") || self.at_ident_kw("s_nexttime") {
            self.bump();
            if self.peek() == Some(TokenKind::LBracket) {
                self.error(
                    "an unbounded `nexttime` (a bounded `nexttime [n]` is unsupported \
                     in this subset)",
                );
            }
            // `nexttime p` ≡ `1'b1 |=> p`.
            let sp = self.prev_span();
            return PropExpr::Impl {
                ante: Sequence::Boolean(Self::sva_true_lit(sp)),
                kind: ImplicationKind::NonOverlap,
                cons: Box::new(self.parse_prop_unary()),
            };
        }
        if self.at_ident_kw("s_always") {
            self.error(
                "`always` (a bounded `s_always` strong-always is unsupported in this \
                 subset)",
            );
            self.bump();
            return PropExpr::Always(Box::new(self.parse_prop_unary()));
        }
        self.parse_prop_impl()
    }

    /// A property primary, optionally the antecedent of a single implication. A
    /// parenthesized PROPERTY `( … |-> … )` / `( … and … )` recurses; a
    /// parenthesized boolean expression `(a && b)` is left to `parse_sequence`
    /// (the implication antecedent). The consequent of `|->`/`|=>` is a full
    /// property expression, so `1'b1 |=> p` (the recursion site) parses with `p`
    /// as a bare `Seq(Boolean(Ident))` leaf resolved at elaborate.
    pub(crate) fn parse_prop_impl(&mut self) -> PropExpr {
        if self.peek() == Some(TokenKind::LParen) && self.paren_group_is_property() {
            self.bump(); // `(`
            let inner = self.parse_prop_expr();
            self.expect(TokenKind::RParen, "')' to close a parenthesized property");
            return inner;
        }
        let ante = self.parse_sequence();
        if self.eat(TokenKind::PipeArrow) {
            PropExpr::Impl {
                ante,
                kind: ImplicationKind::Overlap,
                cons: Box::new(self.parse_prop_expr()),
            }
        } else if self.eat(TokenKind::PipeEqArrow) {
            PropExpr::Impl {
                ante,
                kind: ImplicationKind::NonOverlap,
                cons: Box::new(self.parse_prop_expr()),
            }
        } else {
            PropExpr::Seq(ante)
        }
    }

    /// Cursor on `(`: true iff the balanced paren group contains, at the depth just
    /// inside this paren, a property operator (`|->`/`|=>`/`and`/`or`) — i.e. it is
    /// a parenthesized PROPERTY rather than a parenthesized boolean expression
    /// (which `parse_sequence` handles as an implication antecedent / leaf).
    pub(crate) fn paren_group_is_property(&self) -> bool {
        const BUDGET: usize = 65536;
        let mut i = 0usize;
        let mut depth: i32 = 0;
        loop {
            match self.peek_at(i) {
                None => return false,
                // SVA repeat-open tokens count for depth (see `prop_has_toplevel_andor`).
                Some(
                    TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBracketStar
                    | TokenKind::LBracketArrow
                    | TokenKind::LBracketEq,
                ) => depth += 1,
                Some(TokenKind::RParen | TokenKind::RBracket) => {
                    depth -= 1;
                    if depth == 0 {
                        return false; // closed without a property operator
                    }
                }
                Some(TokenKind::PipeArrow | TokenKind::PipeEqArrow) if depth == 1 => return true,
                Some(TokenKind::Word(WordKind::Keyword(
                    Kw::And | Kw::Or | Kw::Not | Kw::Always,
                ))) if depth == 1 => return true,
                Some(TokenKind::Word(WordKind::Ident)) if depth == 1 => {
                    if Self::is_prop_op_text(self.text_at(i)) {
                        return true;
                    }
                }
                _ => {}
            }
            i += 1;
            if i > BUDGET {
                return false;
            }
        }
    }

    /// Cursor on `(`: true iff the balanced group contains a property operator
    /// (`|->`/`|=>`/`and`/`or`/`not`/…) at ANY depth inside it AND the group runs to
    /// the end of the property — the token after its closing `)` is the `)` of an
    /// inline `assert property( … )` or the `;` of a `property NAME; …; endproperty`
    /// body. Such a group is the whole property in parentheses, which is the same
    /// property; `((a |-> b))` is two of them, and the caller strips them one by one.
    /// A boolean expression never contains a property operator, so the any-depth
    /// test cannot mistake `(a & b)` (an antecedent) for a property.
    pub(crate) fn paren_group_is_whole_property(&self) -> bool {
        const BUDGET: usize = 65536;
        let mut i = 0usize;
        let mut depth: i32 = 0;
        let mut seen_op = false;
        loop {
            match self.peek_at(i) {
                None => return false,
                Some(
                    TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBracketStar
                    | TokenKind::LBracketArrow
                    | TokenKind::LBracketEq,
                ) => depth += 1,
                Some(TokenKind::RParen | TokenKind::RBracket) => {
                    depth -= 1;
                    if depth == 0 {
                        return seen_op
                            && matches!(
                                self.peek_at(i + 1),
                                Some(TokenKind::RParen | TokenKind::Semi)
                            );
                    }
                }
                Some(TokenKind::PipeArrow | TokenKind::PipeEqArrow) => seen_op = true,
                Some(TokenKind::Word(WordKind::Keyword(
                    Kw::And | Kw::Or | Kw::Not | Kw::Always,
                ))) => seen_op = true,
                // The same fences `prop_has_toplevel_op` carries: a malformed
                // (unbalanced) property must not let the scan settle past its
                // construct.
                Some(TokenKind::Word(WordKind::Keyword(Kw::Module | Kw::Endmodule))) => {
                    return false
                }
                Some(TokenKind::Word(WordKind::Ident)) => {
                    if self.text_at(i) == "endproperty" {
                        return false;
                    }
                    if Self::is_prop_op_text(self.text_at(i)) {
                        seen_op = true;
                    }
                }
                _ => {}
            }
            i += 1;
            if i > BUDGET {
                return false;
            }
        }
    }

    /// Parse an optional leading `@(c2)` consequent clocking event (slice A3, after
    /// the implication operator). `|=>` accepts it (multi-clock handoff); `|->` does
    /// NOT (no coherent same-tick cross-clock check) → loud, consume for recovery.
    pub(crate) fn parse_optional_consequent_clock(
        &mut self,
        is_overlap: bool,
    ) -> Option<Sensitivity> {
        if self.peek() != Some(TokenKind::At) {
            return None;
        }
        if is_overlap {
            // `self.error` frames its argument as "expected <X>, found <Y>", so the
            // message must be a noun phrase (review 2026-06-16).
            self.error(
                "a `|=>` for a multi-clock property (an overlapping `|->` cannot take \
                 a consequent clocking event)",
            );
            let _ = self.parse_sensitivity(); // consume `@(c2)` so the rest recovers
            return None;
        }
        Some(self.parse_sensitivity())
    }

    /// Named SVA `property NAME [(formals)]; <property_spec>; endproperty`
    /// (IEEE §16.12). Reuses `parse_property_spec` for the body; spliced at an
    /// `assert property(NAME)` instance by elaborate.
    pub(crate) fn parse_property_decl(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `property` (Kw::Property)
        let name = self.ident()?;
        let formals = self.parse_sva_formals();
        self.expect(TokenKind::Semi, "';' after property name");
        let (
            clock,
            disable_iff,
            antecedent,
            implication_kind,
            consequent,
            consequent_clock,
            prop_expr,
            local_vars,
        ) = self.parse_property_spec(start);
        self.expect(TokenKind::Semi, "';' after property body");
        if self.at_ident_kw("endproperty") {
            self.bump();
        } else {
            self.error("`endproperty`");
        }
        self.eat_end_label();
        Some(ModuleItem::PropertyDecl(PropDecl {
            name,
            formals,
            clock,
            disable_iff,
            antecedent,
            implication_kind,
            consequent,
            consequent_clock,
            prop_expr,
            local_vars,
            span: start.to(self.prev_span()),
        }))
    }
}
