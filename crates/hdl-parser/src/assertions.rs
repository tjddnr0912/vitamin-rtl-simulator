//! assertions / clocking — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    // ───────────────────────── N5: functional coverage ─────────────────────
    /// `covergroup NAME [(args)] [@(event)]; ([LABEL:] coverpoint EXPR [{..}|iff..];)*
    /// endgroup` — a functional-coverage model. The header tail (args / sampling event)
    /// and any per-coverpoint bins/iff are SKIPPED to `;` in this slice (auto-bins,
    /// explicit `sample()`); only the coverpoint EXPR is captured.
    /// N4: `[default] clocking [NAME] @(event); { clocking_item }
    /// endclocking [: NAME]` (IEEE 1800 §14.3). Two item forms:
    ///
    /// - `default default_skew ;` where `default_skew ::= input SKEW
    ///   | output SKEW | input SKEW output SKEW` — a block-wide default.
    /// - `DIR [SKEW] sig [= expr] {, …} ;` — a per-signal declaration.
    ///
    /// The default is not a second skew mechanism: it is STAMPED onto every
    /// item of that direction which declared no skew of its own, so exactly one
    /// predicate (elaborate's) decides which skews this subset can honour, and
    /// it names the skew that was actually written either way.
    pub(crate) fn parse_clocking(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        let is_default = self.eat_kw(Kw::Default);
        self.eat_kw(Kw::Clocking); // guaranteed by the dispatch in parse_module_item
                                   // Optional clocking-block name (`clocking cb @(...)` vs anonymous `clocking
                                   // @(...)`). Present iff the next token is an identifier (the `@` starts the
                                   // event otherwise).
        let name = if self.is_ident() { self.ident() } else { None };
        let clock = if self.peek() == Some(TokenKind::At) {
            self.parse_sensitivity()
        } else {
            self.error("'@(event)' clocking event");
            Sensitivity::Star
        };
        self.expect(TokenKind::Semi, "';' after clocking header");
        let mut items = Vec::new();
        // Block-wide `default default_skew;` (IEEE §14.3), stamped onto the
        // skew-less items after the loop. `seen_default` exists because two
        // `default` items have no unambiguous reading — the LRM grammar admits
        // one per block, so a second is loud rather than last-wins.
        let mut default_in: Option<String> = None;
        let mut default_out: Option<String> = None;
        let mut seen_default = false;
        loop {
            if self.at_kw(Kw::Endclocking) || self.peek().is_none() {
                break;
            }
            if self.at_kw(Kw::Default) {
                // Report BEFORE consuming, so the offending token in the message
                // is the second `default` itself and not whatever follows it.
                if seen_default {
                    self.error("at most one `default` item in a clocking block (IEEE 1800 §14.3)");
                } else {
                    seen_default = true;
                    self.bump(); // `default`
                                 // `input SKEW [output SKEW]` | `output SKEW` — the direction
                                 // keyword is not optional, so a bare `default #1step;` is a
                                 // syntax error in the LANGUAGE, not a subset cut. Say so
                                 // instead of naming a road the subset does not have.
                    let has_in = self.eat_kw(Kw::Input);
                    if has_in {
                        default_in = self.parse_clocking_skew();
                        if default_in.is_none() {
                            self.error("a skew (`#…`) after `default input`");
                        }
                    }
                    if self.eat_kw(Kw::Output) {
                        default_out = self.parse_clocking_skew();
                        if default_out.is_none() {
                            self.error("a skew (`#…`) after `default output`");
                        }
                    } else if !has_in {
                        self.error(
                            "`input` or `output` after `default` in a clocking block \
                             (IEEE 1800 §14.3: `default input SKEW [output SKEW];` \
                             or `default output SKEW;`)",
                        );
                    }
                }
                while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
                    self.bump();
                }
                self.eat(TokenKind::Semi);
                continue;
            }
            let dir = if self.eat_kw(Kw::Input) {
                ClockingDir::Input
            } else if self.eat_kw(Kw::Inout) {
                ClockingDir::Inout
            } else if self.eat_kw(Kw::Output) {
                ClockingDir::Output
            } else {
                self.error("'input'/'output' in a clocking block");
                while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
                    self.bump();
                }
                self.eat(TokenKind::Semi);
                continue;
            };
            let skew_raw = self.parse_clocking_skew();
            // Signal list: `sig [= expr] {, sig [= expr]}` ;
            loop {
                let isp = self.cur_span();
                let Some(sig) = self.ident() else {
                    self.error("a clocking signal name");
                    break;
                };
                let expr = if self.eat(TokenKind::Eq) {
                    Some(self.expr(0))
                } else {
                    None
                };
                items.push(ClockingItem {
                    dir,
                    skew_raw: skew_raw.clone(),
                    name: sig,
                    expr,
                    span: isp,
                });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Semi, "';' after a clocking item");
        }
        // Stamp the block-wide default onto every item that declared no skew of
        // its own. A `default` is a property of the BLOCK, not of the items that
        // follow it textually, so this runs over the whole list. `inout` has no
        // `default_skew` production and is left alone.
        if seen_default {
            for it in &mut items {
                if it.skew_raw.is_some() {
                    continue;
                }
                it.skew_raw = match it.dir {
                    ClockingDir::Input => default_in.clone(),
                    ClockingDir::Output => default_out.clone(),
                    ClockingDir::Inout => None,
                };
            }
        }
        if !self.eat_kw(Kw::Endclocking) {
            self.error("'endclocking'");
        }
        // Optional `: NAME` label.
        if self.eat(TokenKind::Colon) {
            self.ident();
        }
        Some(ModuleItem::Clocking(ClockingDecl {
            name,
            is_default,
            clock,
            items,
            span: start,
        }))
    }

    /// One clocking skew: `#delay` / `#1step`, captured RAW so exactly one
    /// predicate (elaborate's) decides which skews this subset can honour.
    /// `None` ⇒ no `#` here, which for a per-signal item means "the block
    /// default" and for a `default_skew` production is a syntax error.
    ///
    /// `#1step` is TWO tokens after the `#`: `IntDecimal("1")` immediately
    /// followed by `Word(Ident("step"))` — maximal munch does not merge them.
    fn parse_clocking_skew(&mut self) -> Option<String> {
        if self.peek() != Some(TokenKind::Hash) {
            return None;
        }
        self.bump(); // `#`
        let is_1step = matches!(self.peek(), Some(TokenKind::IntDecimal))
            && self.cur_text() == "1"
            && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
            && self.text_at(1) == "step";
        if is_1step {
            self.bump(); // `1`
            self.bump(); // `step`
            return Some("#1step".to_string());
        }
        // Keep the `#` so the honest-loud downstream quotes the skew the way the
        // user wrote it (`#0`, not `0`).
        let txt = format!("#{}", self.cur_text());
        self.bump();
        Some(txt)
    }

    /// N3: a synthetic `[W-1:0]` range (decimal literals) for a record-array element net.
    pub(crate) fn synth_bit_range(&self, w: u32, span: Span) -> Range {
        let lit = |v: u32| Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: v.to_string(),
            },
            span,
        };
        Range {
            msb: lit(w.saturating_sub(1)),
            lsb: lit(0),
            span,
        }
    }

    /// SV immediate assertion (IEEE 1800 §16.3):
    ///   `assert [final] (expr) [pass_stmt] [else fail_stmt]`
    /// Desugared AT PARSE TIME to `Stmt::If` — the AST `Stmt` variant set is
    /// frozen (verdict M7), and `if` already has the exact assert condition
    /// semantics (0/X/Z cond → else branch = assertion failure). A missing
    /// else clause synthesizes the IEEE default failure action
    /// `$error("Assertion failed")`, which lowers through the severity table
    /// (stderr diagnostic + nonzero exit; run continues).
    ///
    /// DEFERRED immediate assertions (§16.4): `assert #0 (expr)` (Observed
    /// deferred) and `assert final (expr)` (Reactive deferred) are evaluated WHEN
    /// REACHED but their action MATURES in a later scheduling region with
    /// flush-on-re-reach. These parse to `Stmt::DeferredAssert` (carrying the
    /// region); elaborate emits a per-assertion flush marker + records the action
    /// StmtIds in the deferred sidecars, and the engine adds genuine Observed/
    /// Reactive maturation queues. iverilog rejects deferred assertions, so there
    /// is no oracle (hand-IEEE). A non-zero `#<n>` delay on an assert is NOT a
    /// deferred assertion → loud. Concurrent (`assert property`) is handled
    /// separately. Dangling-else: in `assert (c) if (x) a; else b;` the else binds
    /// to the inner if and the assert gets the synthesized default.
    pub(crate) fn parse_assert(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // `assert`
                     // v8 SVA subset: `assert property(@(clk) a |-> b);`
        if self.at_kw(Kw::Property) {
            return self.parse_concurrent_assert(start);
        }
        // Deferred immediate assertion (IEEE 1800-2017 §16.4): `assert #0` is the
        // Observed-deferred form, `assert final` the Reactive-deferred form. Both
        // sample the condition WHEN REACHED but MATURE the pass/fail action in a
        // later scheduling region with flush-on-re-reach (see Stmt::DeferredAssert
        // + the engine's Observed/Reactive maturation queues). A plain `assert`
        // (no `#0`/`final`) stays the immediate `Stmt::If` desugar below.
        let defer: Option<AssertDefer> = if self.peek() == Some(TokenKind::Hash) {
            self.bump(); // `#`
                         // Only `#0` is the Observed deferred form (§16.4). A non-zero delay on
                         // an assert is not a deferred assertion → loud.
            if matches!(self.peek(), Some(TokenKind::IntDecimal)) && self.cur_text() == "0" {
                self.bump(); // `0`
                Some(AssertDefer::Observed)
            } else {
                self.error(
                    "a deferred-assertion delay must be `#0` (the Observed deferred form); \
                     a non-zero `#` delay on an assertion is unsupported",
                );
                return self.stmt_error_at(start);
            }
        } else if self.eat_kw(Kw::Final) {
            Some(AssertDefer::Reactive)
        } else {
            None
        };
        if self.peek() != Some(TokenKind::LParen) {
            self.error("'(' after 'assert'");
            return self.stmt_error_at(start);
        }
        self.bump(); // `(`
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        // action_block ::= statement_or_null | [statement] `else` statement
        let then_s = if self.at_kw(Kw::Else) {
            Box::new(Stmt::Null(start)) // else-only form: no pass action
        } else {
            Box::new(self.parse_statement())
        };
        let else_s = if self.eat_kw(Kw::Else) {
            Box::new(self.parse_statement())
        } else {
            let sp = start.to(self.prev_span());
            Box::new(Stmt::SysTaskCall {
                name: Ident {
                    name: "$error".to_string(),
                    span: sp,
                },
                args: vec![Expr {
                    kind: ExprKind::StrLit {
                        raw: "\"Assertion failed\"".to_string(),
                    },
                    span: sp,
                }],
                span: sp,
            })
        };
        let span = start.to(self.prev_span());
        match defer {
            // Deferred (#0 / final): preserve the region so elaborate emits the
            // flush marker + records the action StmtIds in the deferred sidecars.
            Some(region) => Stmt::DeferredAssert {
                region,
                cond,
                then_s,
                else_s,
                span,
            },
            // Plain immediate assert: the byte-identical `Stmt::If` desugar.
            None => Stmt::If {
                cond,
                then_s,
                else_s: Some(else_s),
                span,
            },
        }
    }

    /// SVA subset (Phase-3): `assert property(@(posedge clk) seq |-> consequent);`
    /// (overlapping `|->` / non-overlapping `|=>`). Single clock. The antecedent
    /// is a `Sequence` — slice S4 added bounded `##n` cycle-delay and `[*n]`
    /// consecutive repetition (ranges/unbounded/goto/throughout/within stay a
    /// loud parse error). The consequent stays a flat boolean. The failure
    /// action is the implicit `$error` synthesized at elaborate time.
    pub(crate) fn parse_concurrent_assert(&mut self, start: Span) -> Stmt {
        self.bump(); // `property`
        self.expect(TokenKind::LParen, "'(' after 'property'");
        // Named-property INSTANCE: `assert property(NAME);` — NAME is a property
        // declared elsewhere, resolved + inlined at elaborate. Detect by a single
        // identifier immediately followed by `)`. A `NAME(args)` form is the
        // parameterized instance, reserved + loud in this subset.
        if self.is_ident() && self.peek_at(1) == Some(TokenKind::RParen) {
            let name = self.ident().unwrap();
            self.expect(TokenKind::RParen, "')'");
            let (pass, fail) = self.parse_assert_action_block();
            return Stmt::ConcurrentAssert {
                // empty clock = "named-property reference"; elaborate splices the
                // declared property's real clock/spec in at collect time.
                clock: Sensitivity::List(Vec::new()),
                disable_iff: None,
                antecedent: Sequence::Instance {
                    name,
                    args: Vec::new(),
                    span: start,
                },
                implication_kind: ImplicationKind::Overlap,
                consequent: Sequence::Boolean(Self::sva_true_lit(start)),
                consequent_clock: None,
                pass,
                fail,
                prop_expr: None,
                local_vars: Vec::new(),
                span: start.to(self.prev_span()),
            };
        }
        if self.is_ident() && self.peek_at(1) == Some(TokenKind::LParen) {
            // `assert property(NAME(args))` — parameterized property instance
            // (slice A1). Parse the positional actual arguments; elaborate binds them
            // to the declared property's formals and substitutes before splicing.
            let name = self.ident().unwrap();
            self.expect(TokenKind::LParen, "'(' before property arguments");
            let mut args = Vec::new();
            if self.peek() != Some(TokenKind::RParen) {
                loop {
                    args.push(self.expr(0));
                    if self.eat(TokenKind::Comma) {
                        continue;
                    }
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')' after property arguments");
            self.expect(TokenKind::RParen, "')'");
            let (pass, fail) = self.parse_assert_action_block();
            return Stmt::ConcurrentAssert {
                clock: Sensitivity::List(Vec::new()),
                disable_iff: None,
                antecedent: Sequence::Instance {
                    name,
                    args,
                    span: start,
                },
                implication_kind: ImplicationKind::Overlap,
                consequent: Sequence::Boolean(Self::sva_true_lit(start)),
                consequent_clock: None,
                pass,
                fail,
                prop_expr: None,
                local_vars: Vec::new(),
                span: start.to(self.prev_span()),
            };
        }
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
        self.expect(TokenKind::RParen, "')'");
        // action_block ::= statement_or_null | [statement] `else` statement_or_null
        // (slice S11). A bare `;` leaves both None (default $error, no pass).
        let (pass, fail) = self.parse_assert_action_block();
        Stmt::ConcurrentAssert {
            clock,
            disable_iff,
            antecedent,
            implication_kind,
            consequent,
            consequent_clock,
            pass,
            fail,
            prop_expr,
            local_vars,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse an assertion action block after the `property(...)` close paren
    /// (slice S11): `[pass_stmt] [else fail_stmt]`. A bare `;` yields
    /// `(None, None)` — the default $error fail and no pass action, kept distinct
    /// from `(Some(Null), None)` so the no-action checker is byte-identical to
    /// before this slice. Each statement consumes its own terminating `;`.
    pub(crate) fn parse_assert_action_block(&mut self) -> (Option<Box<Stmt>>, Option<Box<Stmt>>) {
        // `eat(Semi)` consumes a bare `;` (empty action block); `at_kw(Else)`
        // (else-only form) leaves the `else` for the `fail` arm below — both yield
        // no pass action. Short-circuit `||` keeps the `;`-consuming side effect.
        let pass = if self.eat(TokenKind::Semi) || self.at_kw(Kw::Else) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        let fail = if self.eat_kw(Kw::Else) {
            Some(Box::new(self.parse_statement()))
        } else {
            None
        };
        (pass, fail)
    }

    /// P2-E: `unique`/`priority` qualified if/case. The qualified statement
    /// parses normally; the VIOLATION surface (IEEE §12.4.2/§12.5.3 — no
    /// branch/arm taken) desugars to a synthesized severity-task else/default
    /// arm (iverilog-pinned text class: "value is unhandled..."). A statement
    /// that already HAS an else/default cannot miss — left untouched. The
    /// multi-match uniqueness check is a documented cut (the lowered cascade
    /// is first-match-wins, so overlap is unobservable).
    ///
    /// ⚠️ The task is [`UNIQUE_VIOLATION_TASK`], not `$warning`. A §12.5.3
    /// violation report is a fact the SIMULATOR produces; a `$warning` is a task
    /// the DESIGN called, and IEEE puts them in different clauses. While the
    /// desugar named `$warning` they were literally one diagnostic code, so
    /// `-Wno-`/`-Werror` could not address one without the other — external
    /// round-29 measured both directions. The name is the only channel the AST
    /// offers (a marker field would change the frozen AST shape), and it maps
    /// straight to `SeverityKind::UniqueViolation` in elaborate.
    pub(crate) fn parse_unique_priority(&mut self) -> Stmt {
        let qspan = self.cur_span();
        // §12.4.2: the `0` variants keep the multi-match intent but SUPPRESS
        // the no-match violation — so they parse as the PLAIN if/case with no
        // synthetic warn injection (hand-IEEE: Icarus rejects `unique0 if`
        // outright and ignores the unique/unique0 distinction on case).
        let suppress_no_match = matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Unique0 | Kw::Priority0
            )))
        );
        self.bump(); // unique / priority / unique0 / priority0
        let warn_stmt = |span: Span| Stmt::SysTaskCall {
            name: Ident {
                name: UNIQUE_VIOLATION_TASK.to_string(),
                span,
            },
            args: vec![Expr {
                kind: ExprKind::StrLit {
                    raw: "\"value is unhandled for priority or unique case statement\"".to_string(),
                },
                span,
            }],
            span,
        };
        match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::If))) => {
                let mut s = self.parse_if();
                if let Stmt::If { else_s, span, .. } = &mut s {
                    if else_s.is_none() && !suppress_no_match {
                        *else_s = Some(Box::new(warn_stmt(*span)));
                    }
                }
                s
            }
            Some(TokenKind::Word(WordKind::Keyword(k @ (Kw::Case | Kw::Casez | Kw::Casex)))) => {
                let kind = match k {
                    Kw::Casez => CaseKind::Casez,
                    Kw::Casex => CaseKind::Casex,
                    _ => CaseKind::Case,
                };
                let mut s = self.parse_case(kind);
                if let Stmt::Case { items, span, .. } = &mut s {
                    let has_default = items.iter().any(|i| matches!(i, CaseItem::Default { .. }));
                    if !has_default && !suppress_no_match {
                        items.push(CaseItem::Default {
                            body: Box::new(warn_stmt(*span)),
                            span: *span,
                        });
                    }
                }
                s
            }
            _ => {
                self.error("'if' or 'case' after a unique/priority qualifier");
                Stmt::Error(qspan.to(self.prev_span()))
            }
        }
    }
}
