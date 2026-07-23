//! covergroups — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    pub(crate) fn parse_covergroup(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `covergroup`
        let name = self.ident()?;
        // optional `( ports )` — skip balanced (covergroup args, slice-future).
        if self.peek() == Some(TokenKind::LParen) {
            let mut depth = 0i32;
            loop {
                match self.peek() {
                    Some(TokenKind::LParen) => depth += 1,
                    Some(TokenKind::RParen) => {
                        depth -= 1;
                        if depth == 0 {
                            self.bump();
                            break;
                        }
                    }
                    None => break,
                    _ => {}
                }
                self.bump();
            }
        }
        // optional `@(event)` sampling clock (slice F): auto-sample on this event.
        let clock = if self.peek() == Some(TokenKind::At) {
            Some(self.parse_sensitivity())
        } else {
            None
        };
        // skip any remaining header tail (`with function sample(...)`, etc.) to `;`.
        while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
            self.bump();
        }
        self.expect(TokenKind::Semi, "';' after covergroup header");
        let mut points = Vec::new();
        let mut crosses = Vec::new();
        let mut cg_at_least: Option<Expr> = None;
        loop {
            if self.at_kw(Kw::Endgroup) || self.peek().is_none() {
                break;
            }
            // optional `LABEL :`
            let label = if self.is_ident() && self.peek_at(1) == Some(TokenKind::Colon) {
                let l = self.ident().unwrap();
                self.bump(); // ':'
                Some(l)
            } else {
                None
            };
            if self.at_ident_kw("cross") {
                if let Some(cr) = self.parse_cross(label) {
                    crosses.push(cr);
                }
                continue;
            }
            // covergroup-level `option.NAME = expr;` (slice D): only `at_least` affects
            // the measured %; other options (goal/comment/per_instance/…) are accepted
            // and ignored (they do not change the coverage value in this model).
            if self.at_ident_kw("option") || self.at_ident_kw("type_option") {
                if let Some((name, val)) = self.parse_cover_option() {
                    if name == "at_least" {
                        cg_at_least = Some(val);
                    }
                }
                continue;
            }
            if self.at_kw(Kw::Coverpoint) {
                let cp_start = self.cur_span();
                self.bump(); // `coverpoint`
                let expr = self.expr(0);
                // optional coverpoint-level `iff (G)` guard (slice B).
                let iff = self.parse_cover_iff();
                // optional `{ bin* | option* }` body (else a bare `;`).
                let (bins, at_least, weight) = if self.peek() == Some(TokenKind::LBrace) {
                    let b = self.parse_coverpoint_body();
                    self.eat(TokenKind::Semi); // `;` after `}` is optional
                    b
                } else {
                    self.expect(TokenKind::Semi, "';' after coverpoint");
                    (Vec::new(), None, None)
                };
                points.push(Coverpoint {
                    label,
                    expr,
                    iff,
                    bins,
                    at_least,
                    weight,
                    span: cp_start.to(self.prev_span()),
                });
            } else {
                // an unsupported covergroup item (cross / option / …) — loud, skip to `;`.
                self.error("`coverpoint` in covergroup (cross/option are a follow-on)");
                while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
                    self.bump();
                }
                self.eat(TokenKind::Semi);
            }
        }
        if self.at_kw(Kw::Endgroup) {
            self.bump();
        } else {
            self.error("`endgroup`");
        }
        // optional `: NAME` after endgroup
        if self.peek() == Some(TokenKind::Colon) {
            self.bump();
            let _ = self.ident();
        }
        Some(ModuleItem::Covergroup(CovergroupDecl {
            name,
            points,
            crosses,
            clock,
            at_least: cg_at_least,
            span: start.to(self.prev_span()),
        }))
    }

    /// `CG_TYPE NAME = new [(args)] ;` — a covergroup instance.
    pub(crate) fn parse_cover_instance(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        let cg_type = self.ident()?;
        let name = self.ident()?;
        self.expect(TokenKind::Eq, "'=' in covergroup instance");
        if self.at_ident_kw("new") {
            self.bump();
        } else {
            self.error("`new` in covergroup instance");
        }
        // optional `( args )` — skip balanced.
        if self.peek() == Some(TokenKind::LParen) {
            let mut depth = 0i32;
            loop {
                match self.peek() {
                    Some(TokenKind::LParen) => depth += 1,
                    Some(TokenKind::RParen) => {
                        depth -= 1;
                        if depth == 0 {
                            self.bump();
                            break;
                        }
                    }
                    None => break,
                    _ => {}
                }
                self.bump();
            }
        }
        self.expect(TokenKind::Semi, "';' after covergroup instance");
        Some(ModuleItem::CoverInstance(CoverInstance {
            cg_type,
            name,
            span: start.to(self.prev_span()),
        }))
    }

    /// `[LABEL:] cross cp_a, cp_b [, …] [{ … }] ;` — a cross of named coverpoints
    /// (slice C; the `cross` ident is at the cursor, LABEL already consumed). A cross
    /// SELECT body `{ binsof/intersect }` is loud-rejected and balanced-skipped.
    pub(crate) fn parse_cross(&mut self, label: Option<Ident>) -> Option<CrossSpec> {
        let start = self.cur_span();
        self.bump(); // `cross`
        let mut points = Vec::new();
        loop {
            let before = self.pos;
            if let Some(id) = self.ident() {
                points.push(id);
            }
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        // optional cross SELECT body `{ binsof … }` — follow-on; loud + balanced skip.
        if self.peek() == Some(TokenKind::LBrace) {
            self.error("cross select body (binsof/intersect) (follow-on)");
            let mut depth = 0i32;
            loop {
                match self.peek() {
                    Some(TokenKind::LBrace) => {
                        depth += 1;
                        self.bump();
                    }
                    Some(TokenKind::RBrace) => {
                        depth -= 1;
                        self.bump();
                        if depth == 0 {
                            break;
                        }
                    }
                    None => break,
                    _ => {
                        self.bump();
                    }
                }
            }
        }
        self.expect(TokenKind::Semi, "';' after cross");
        Some(CrossSpec {
            name: label,
            points,
            span: start.to(self.prev_span()),
        })
    }

    /// Optional `iff ( expr )` guard after a coverpoint expr or a bin RHS (slice B).
    /// `iff` is a contextual ident here (not a reserved keyword globally).
    pub(crate) fn parse_cover_iff(&mut self) -> Option<Expr> {
        if !self.at_ident_kw("iff") {
            return None;
        }
        self.bump(); // `iff`
        self.expect(TokenKind::LParen, "'(' after iff");
        let g = self.expr(0);
        self.expect(TokenKind::RParen, "')' after iff guard");
        Some(g)
    }

    /// Parse a coverpoint body `{ (bin | option)* }` (the opening `{` is at the
    /// cursor). Returns `(bins, at_least, weight)`. Each bin is `KIND NAME[array] =
    /// ( {range_list} | default ) [iff(G)] ;`. Unsupported bin forms
    /// (wildcard/transition/`binsof`/`intersect`/junk) are LOUD-rejected and
    /// balanced-skipped — never silently dropped. `option.at_least`/`option.weight`
    /// are captured; other `option.*` are accepted and ignored.
    #[allow(clippy::type_complexity)]
    pub(crate) fn parse_coverpoint_body(&mut self) -> (Vec<BinSpec>, Option<Expr>, Option<Expr>) {
        self.bump(); // `{`
        let mut bins = Vec::new();
        let mut at_least = None;
        let mut weight = None;
        loop {
            if matches!(self.peek(), Some(TokenKind::RBrace) | None) {
                break;
            }
            let before = self.pos;
            if self.at_ident_kw("option") || self.at_ident_kw("type_option") {
                if let Some((name, val)) = self.parse_cover_option() {
                    match name.as_str() {
                        "at_least" => at_least = Some(val),
                        "weight" => weight = Some(val),
                        _ => {} // accepted-ignored (does not change the measured %)
                    }
                }
            } else if let Some(b) = self.parse_bin_spec() {
                bins.push(b);
            }
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
        }
        self.eat(TokenKind::RBrace);
        (bins, at_least, weight)
    }

    /// `option.NAME = expr ;` / `type_option.NAME = expr ;` (the `option` ident is at
    /// the cursor). Returns `(NAME, value-expr)`. Slice D.
    pub(crate) fn parse_cover_option(&mut self) -> Option<(String, Expr)> {
        self.bump(); // `option` / `type_option`
        self.expect(TokenKind::Dot, "'.' after option");
        let name = self.ident()?;
        self.expect(TokenKind::Eq, "'=' in option");
        let val = self.expr(0);
        self.expect(TokenKind::Semi, "';' after option");
        Some((name.name, val))
    }

    /// One `KIND NAME[array] = RHS [iff(G)] ;` bin. Returns `None` (after a loud
    /// diagnostic + balanced skip to the bin's `;`) for unsupported forms.
    pub(crate) fn parse_bin_spec(&mut self) -> Option<BinSpec> {
        let start = self.cur_span();
        // `wildcard bins …` — follow-on; loud-reject.
        if self.at_ident_kw("wildcard") {
            self.error("wildcard coverage bins (follow-on)");
            self.skip_bin_to_semi();
            return None;
        }
        let kind = if self.at_ident_kw("bins") {
            BinKind::Regular
        } else if self.at_ident_kw("ignore_bins") {
            BinKind::Ignore
        } else if self.at_ident_kw("illegal_bins") {
            BinKind::Illegal
        } else {
            // `cross`/`option`/junk inside a coverpoint body — loud-reject.
            self.error("`bins`/`ignore_bins`/`illegal_bins` in coverpoint body");
            self.skip_bin_to_semi();
            return None;
        };
        self.bump(); // the bins-kind ident
        let name = self.ident()?;
        // optional array suffix: `[]` (unsized) or `[N]` (fixed).
        let array = if self.peek() == Some(TokenKind::LBracket) {
            self.bump(); // `[`
            if self.eat(TokenKind::RBracket) {
                BinArray::Unsized
            } else {
                let n = self.expr(0);
                self.expect(TokenKind::RBracket, "']' in bin array size");
                BinArray::Fixed(n)
            }
        } else {
            BinArray::Scalar
        };
        self.expect(TokenKind::Eq, "'=' in bin definition");
        // RHS: `default` | `{ open_range_list }` | `( trans_list )`(loud).
        let (values, is_default) = if self.at_kw(Kw::Default) {
            self.bump(); // `default`
            if self.at_ident_kw("sequence") {
                self.error("default sequence (transition) bins (follow-on)");
                self.skip_bin_to_semi();
                return None;
            }
            (Vec::new(), true)
        } else if self.peek() == Some(TokenKind::LParen) {
            self.error("transition coverage bins (follow-on)");
            self.skip_bin_to_semi();
            return None;
        } else if self.peek() == Some(TokenKind::LBrace) {
            (self.parse_open_range_list(), false)
        } else {
            self.error("bin value set `{...}` or `default`");
            self.skip_bin_to_semi();
            return None;
        };
        let iff = self.parse_cover_iff();
        self.expect(TokenKind::Semi, "';' after bin");
        Some(BinSpec {
            name,
            kind,
            array,
            values,
            is_default,
            iff,
            span: start.to(self.prev_span()),
        })
    }

    /// Parse `{ range (, range)* }` (the opening `{` is at the cursor).
    pub(crate) fn parse_open_range_list(&mut self) -> Vec<CoverRange> {
        self.bump(); // `{`
        let mut out = Vec::new();
        loop {
            if matches!(self.peek(), Some(TokenKind::RBrace) | None) {
                break;
            }
            let before = self.pos;
            if let Some(r) = self.parse_cover_range() {
                out.push(r);
            }
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.eat(TokenKind::RBrace);
        out
    }

    /// One open_range_list element: `[ end : end ]` (inclusive range) or a single
    /// value `expr` (`lo==hi`). A transition arrow `=>` after a value is loud-rejected.
    pub(crate) fn parse_cover_range(&mut self) -> Option<CoverRange> {
        if self.peek() == Some(TokenKind::LBracket) {
            self.bump(); // `[`
            let lo = self.parse_range_end();
            self.expect(TokenKind::Colon, "':' in range");
            let hi = self.parse_range_end();
            self.expect(TokenKind::RBracket, "']' in range");
            Some(CoverRange { lo, hi })
        } else {
            let v = self.expr(0);
            // transition `=>` (lexes as `=` then `>`) — follow-on.
            if self.peek() == Some(TokenKind::Eq) && self.peek_at(1) == Some(TokenKind::Gt) {
                self.error("transition coverage bins (follow-on)");
                return None;
            }
            let end = RangeEnd::Val(v);
            Some(CoverRange {
                lo: end.clone(),
                hi: end,
            })
        }
    }

    /// Balanced skip to the terminating `;` of a malformed bin (recovery). Stops at
    /// a depth-0 `}` (the body terminator) without consuming it.
    pub(crate) fn skip_bin_to_semi(&mut self) {
        let mut depth = 0i32;
        loop {
            match self.peek() {
                None => break,
                Some(TokenKind::RBrace) if depth == 0 => break,
                Some(TokenKind::Semi) if depth == 0 => {
                    self.bump();
                    break;
                }
                Some(TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace) => {
                    depth += 1;
                    self.bump();
                }
                Some(TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace) => {
                    depth -= 1;
                    self.bump();
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// SVA-REST: `cover property(@(clk) [disable iff(e)] seq);` — a coverage
    /// statement (counts sequence matches, reports the hit count at end-of-sim).
    /// Shares the clock + `disable iff` + sequence grammar with a property spec; an
    /// optional cover action block is loud-rejected (unsupported — never silently
    /// dropped). Cursor on `cover`.
    pub(crate) fn parse_cover_property(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // `cover`
        self.bump(); // `property` (Kw::Property)
        self.expect(TokenKind::LParen, "'(' after 'property'");
        let clock = if self.peek() == Some(TokenKind::At) {
            self.parse_sensitivity()
        } else {
            self.error("'@(...)' clocking event in cover property");
            Sensitivity::List(Vec::new())
        };
        let disable_iff = if self.at_kw(Kw::Disable) {
            self.bump(); // `disable`
            if self.at_ident_kw("iff") {
                self.bump();
            } else {
                self.error("`iff` after `disable` in a cover property");
            }
            self.expect(TokenKind::LParen, "'(' after `disable iff`");
            let e = self.expr(0);
            self.expect(TokenKind::RParen, "')' after `disable iff` condition");
            Some(e)
        } else {
            None
        };
        let seq = self.parse_sequence();
        self.expect(TokenKind::RParen, "')'");
        if !self.eat(TokenKind::Semi) {
            // A `cover property(...) <stmt>` success-action block is unsupported in
            // this subset — loud (do not silently drop the action), then skip the
            // statement for recovery.
            self.error(
                "';' after `cover property(...)` (a cover action block is unsupported \
                 in this subset)",
            );
            let _ = self.parse_statement();
        }
        Stmt::CoverProperty {
            clock,
            disable_iff,
            seq,
            span: start.to(self.prev_span()),
        }
    }
}
