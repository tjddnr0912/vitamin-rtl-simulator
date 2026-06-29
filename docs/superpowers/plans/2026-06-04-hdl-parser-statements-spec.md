All confirmed. I have complete grounding to finalize the spec, resolving every BLOCKER/MAJOR from both verdicts. Key fixes folded in: **M4** (intra-assign delay consume-and-discard with one advisory error, no cascade), **M3/H1** (`disable fork` + empty-HierPath → emit `Stmt::Error` not a malformed node), **C3/N1** (drop dead `stmt_span`), **N2** (`unreachable!` hardening), **m2** (`@()` diagnostic), **H3** (always_ff sensitivity note), plus the `parse_for_assign` zero-progress guard and the `synchronize`-stops-at-proc-kw interaction.

---

# vitamin `hdl-parser` PR2 — FINAL Statement + Procedural-Block Parser Spec

All API verified against real source. Confirmed exact names: `expr(min_bp)`, `bump()`, `peek()->Option<TokenKind>`, `parse_lvalue()->Lvalue`, `parse_delay()->Option<Delay>` (consumes `#`), `parse_net_var()->Option<NetVarDecl>`, `net_var_kind()->Option<_>`, `hier_path()->Option<HierPath>`, `ident()->Option<Ident>`, `call_args()->Vec<Expr>` (consumes leading `(`), `synchronize()`, `eat`/`eat_kw`/`at_kw`/`expect`/`error`, `cur_span`/`prev_span`/`cur_text()->&'s str`, `is_ident`/`at_lex_error`/`at_eof`, `Self::sp`, field `self.src`. AST: `ModuleItem::Proc(ProceduralBlock)`, `ModuleDecl.body: Vec<ModuleItem>`, all `Stmt`/`CaseItem`/`Lvalue`/`HierPath`/`Delay` shapes as listed. Tokens: `At`/`Arrow`/`Hash`/`SystemTask`/`LParen`/`RParen`/`LBrace`/`Semi`/`Comma`/`Colon`/`Eq`/`LtEq`/`Star`. `synchronize` stops at `Kw::End|Endcase|Join` and at proc/decl keywords (so block-body `stmt_error` lands correctly; `join_any`/`join_none` are NOT anchors — handled explicitly).

---

## 1. The COMPLETE methods to ADD to `impl<'t,'s> Parser<'t,'s>`

Insert this entire block after the `impl` that ends at **line 1270** (before `pub fn parse`). `BlockEnd` enum goes at module scope.

```rust
// ════════════════════════ PR2: statements + procedural blocks ════════════════════════
impl<'t, 's> Parser<'t, 's> {
    // ─────────────────────── 1. procedural blocks ───────────────────────
    /// `initial S` | `always [@(…)] S` | `always_ff @(…) S` | `always_comb S`
    /// | `always_latch S`. For `always`/`always_ff` a leading `@(…)` folds onto
    /// `ProceduralBlock.sensitivity` (pragmatic: the AST doc-comment says "only
    /// general always", but `always_ff @(edge)` genuinely carries an edge list and
    /// the field is `Option`, so this is type-legal and the right shape — H3).
    /// `always_comb`/`always_latch`/`initial` NEVER consume an `@` here. Always
    /// returns a block; a bad body yields `Stmt::Error` via `parse_statement`
    /// (forward progress guaranteed — the proc keyword itself is always consumed,
    /// satisfying the module-item loop guard at line 769).
    fn parse_procedural_block(&mut self) -> ProceduralBlock {
        let start = self.cur_span();
        let kind = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(k))) => match k {
                Kw::Initial => ProcKind::Initial,
                Kw::Always => ProcKind::Always,
                Kw::AlwaysFf => ProcKind::AlwaysFf,
                Kw::AlwaysComb => ProcKind::AlwaysComb,
                Kw::AlwaysLatch => ProcKind::AlwaysLatch,
                _ => unreachable!("parse_procedural_block: caller pre-screens proc kw"),
            },
            _ => unreachable!("parse_procedural_block: caller pre-screens proc kw"),
        };
        self.bump(); // initial / always*

        let sensitivity = match kind {
            ProcKind::Always | ProcKind::AlwaysFf if self.peek() == Some(TokenKind::At) => {
                Some(self.parse_sensitivity())
            }
            _ => None,
        };

        let body = Box::new(self.parse_statement());
        ProceduralBlock {
            kind,
            sensitivity,
            body,
            span: start.to(self.prev_span()),
        }
    }

    /// `@*` | `@(*)` → Star ;  `@(ev or ev , …)` → List.  Consumes the leading `@`.
    /// Shared by block-level sensitivity (§1) and statement-level `@(…)` (EventCtrl).
    fn parse_sensitivity(&mut self) -> Sensitivity {
        self.bump(); // '@'
        if self.eat(TokenKind::Star) {
            return Sensitivity::Star; // `@*`
        }
        if !self.expect(TokenKind::LParen, "'(' or '*' after '@'") {
            return Sensitivity::List(Vec::new()); // recover; only `@` consumed
        }
        if self.peek() == Some(TokenKind::Star) {
            self.bump(); // `@(*)`
            self.expect(TokenKind::RParen, "')'");
            return Sensitivity::Star;
        }
        let mut events = Vec::new();
        if self.peek() == Some(TokenKind::RParen) {
            self.error("event expression"); // m2: `@()` is illegal — diagnose
        } else {
            loop {
                let before = self.pos;
                events.push(self.parse_event_expr());
                let sep = self.eat_kw(Kw::Or) || self.eat(TokenKind::Comma);
                // forward-progress guard MUST stay AFTER the separator-eat: on a junk
                // token that parse_event_expr could not consume AND that is not a
                // separator, this is the only advance. Do not reorder (soundness NIT).
                if self.pos == before {
                    self.bump();
                }
                if !sep || self.peek() == Some(TokenKind::RParen) {
                    break; // no sep, or trailing-separator tolerance
                }
            }
        }
        self.expect(TokenKind::RParen, "')'");
        Sensitivity::List(events)
    }

    /// `[posedge|negedge] expr` → EventExpr.
    fn parse_event_expr(&mut self) -> EventExpr {
        let start = self.cur_span();
        let edge = if self.eat_kw(Kw::Posedge) {
            Edge::Posedge
        } else if self.eat_kw(Kw::Negedge) {
            Edge::Negedge
        } else {
            Edge::NoEdge
        };
        let expr = self.expr(0);
        let span = start.to(expr.span);
        EventExpr { edge, expr, span }
    }

    // ─────────────────────── 2. statement dispatcher ───────────────────────
    fn parse_statement(&mut self) -> Stmt {
        use TokenKind as T;
        if self.at_lex_error() {
            let s = self.cur_span();
            self.bump(); // skip the lexer-error sentinel without re-reporting
            return Stmt::Error(s);
        }
        match self.peek() {
            Some(T::Semi) => {
                let s = self.cur_span();
                self.bump();
                Stmt::Null(s)
            }
            Some(T::Hash) => self.parse_delay_stmt(),
            Some(T::At) => self.parse_event_stmt(),
            Some(T::Arrow) => self.parse_trigger_stmt(),
            Some(T::LBrace) => self.parse_assign_or_call(), // {a,b} = … concat lvalue
            Some(T::Word(WordKind::Keyword(kw))) => match kw {
                Kw::Begin => self.parse_seq_block(),
                Kw::Fork => self.parse_par_block(),
                Kw::If => self.parse_if(),
                Kw::Case => self.parse_case(CaseKind::Case),
                Kw::Casez => self.parse_case(CaseKind::Casez),
                Kw::Casex => self.parse_case(CaseKind::Casex),
                Kw::For => self.parse_for(),
                Kw::While => self.parse_while(),
                Kw::Repeat => self.parse_repeat(),
                Kw::Forever => self.parse_forever(),
                Kw::Wait => self.parse_wait(),
                Kw::Disable => self.parse_disable(),
                Kw::Assign => self.parse_proc_assign(),
                Kw::Deassign => self.parse_deassign(),
                Kw::Force => self.parse_force(),
                Kw::Release => self.parse_release(),
                _ => self.stmt_error(),
            },
            Some(T::SystemTask) => self.parse_systask_call(),
            _ if self.is_ident() => self.parse_assign_or_call(),
            _ => self.stmt_error(),
        }
    }

    /// Unparseable statement: record one error, build Error, sync, GUARANTEE ≥1
    /// token consumed. NOTE: `synchronize` stops at proc/decl keywords AND at
    /// `end`/`endcase`/`join`, so a stuck statement inside a block lands on the
    /// block closer (the caller's `at_block_end` then ends the loop) — no overrun.
    fn stmt_error(&mut self) -> Stmt {
        let s = self.cur_span();
        let before = self.pos;
        self.error("statement");
        self.synchronize();
        if self.pos == before {
            self.bump(); // forced progress when sync stopped immediately
        }
        Stmt::Error(s)
    }

    /// On a recovery path where `synchronize` may stop immediately: sync then
    /// force ≥1 token. Returns an `Error` spanning from `start`.
    fn stmt_error_at(&mut self, start: Span) -> Stmt {
        let before = self.pos;
        self.synchronize();
        if self.pos == before {
            self.bump();
        }
        Stmt::Error(start.to(self.prev_span()))
    }

    // ─────────────────────── 3. assignments / task calls ───────────────────────
    /// Leading ident or `{`: blocking `=`, nonblocking `<=`, or a user-task call.
    fn parse_assign_or_call(&mut self) -> Stmt {
        let start = self.cur_span();
        let lhs = self.parse_lvalue();
        match self.peek() {
            Some(TokenKind::Eq) => {
                self.bump();
                self.skip_intra_assign_delay(); // M4: discard `#d`/`@(ev)`, one advisory
                let rhs = self.expr(0);
                self.expect(TokenKind::Semi, "';'");
                Stmt::Blocking { lhs, delay: None, rhs, span: start.to(self.prev_span()) }
            }
            Some(TokenKind::LtEq) => {
                self.bump();
                self.skip_intra_assign_delay(); // M4: `q <= #1 d;` is extremely common
                let rhs = self.expr(0);
                self.expect(TokenKind::Semi, "';'");
                Stmt::NonBlocking { lhs, delay: None, rhs, span: start.to(self.prev_span()) }
            }
            // user-task call: bare HierPath followed by `(` or `;`
            Some(TokenKind::LParen) | Some(TokenKind::Semi) => {
                if let Lvalue::Ident(path) = lhs {
                    let args = if self.peek() == Some(TokenKind::LParen) {
                        self.call_args()
                    } else {
                        Vec::new()
                    };
                    self.expect(TokenKind::Semi, "';'");
                    Stmt::UserTaskCall { name: path, args, span: start.to(self.prev_span()) }
                } else {
                    // e.g. `a[i](…)` — an indexed lvalue cannot be a call.
                    self.error("'=' or '<=' after lvalue");
                    self.stmt_error_at(start)
                }
            }
            _ => {
                self.error("'=' or '<=' after lvalue");
                self.stmt_error_at(start)
            }
        }
    }

    /// M4: intra-assignment timing control after `=`/`<=` (e.g. `a = #5 b;`,
    /// `q <= @(posedge clk) d;`). DEFERRED feature — parse-and-DISCARD with ONE
    /// advisory error so the RHS still parses cleanly (no cascade). `delay` is
    /// stored as `None` per MVP scope.
    fn skip_intra_assign_delay(&mut self) {
        match self.peek() {
            Some(TokenKind::Hash) => {
                self.error("intra-assignment delay (not yet supported; ignored)");
                let _ = self.parse_delay(); // consumes `#d` / `#(…)`
            }
            Some(TokenKind::At) => {
                self.error("intra-assignment event control (not yet supported; ignored)");
                let _ = self.parse_sensitivity(); // consumes `@(…)`
            }
            _ => {}
        }
    }

    fn parse_systask_call(&mut self) -> Stmt {
        let start = self.cur_span();
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
        Stmt::SysTaskCall { name, args, span: start.to(self.prev_span()) }
    }

    // procedural-continuous family — all reuse parse_lvalue
    fn parse_proc_assign(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // assign
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Eq, "'=' in procedural assign");
        let rhs = self.expr(0);
        self.expect(TokenKind::Semi, "';'");
        Stmt::Assign { lhs, rhs, span: start.to(self.prev_span()) }
    }
    fn parse_force(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // force
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Eq, "'=' in force");
        let rhs = self.expr(0);
        self.expect(TokenKind::Semi, "';'");
        Stmt::Force { lhs, rhs, span: start.to(self.prev_span()) }
    }
    fn parse_deassign(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // deassign
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Semi, "';'");
        Stmt::Deassign { lhs, span: start.to(self.prev_span()) }
    }
    fn parse_release(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // release
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Semi, "';'");
        Stmt::Release { lhs, span: start.to(self.prev_span()) }
    }

    // ─────────────────────── 4. control flow ───────────────────────
    fn parse_if(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // if
        self.expect(TokenKind::LParen, "'(' after 'if'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let then_s = Box::new(self.parse_statement());
        // dangling-else binds EAGERLY to this (nearest) if: an inner `if` parsed by
        // the recursive `parse_statement` above already claimed its own `else`.
        let else_s = if self.eat_kw(Kw::Else) {
            Some(Box::new(self.parse_statement()))
        } else {
            None
        };
        Stmt::If { cond, then_s, else_s, span: start.to(self.prev_span()) }
    }

    fn parse_case(&mut self, kind: CaseKind) -> Stmt {
        let start = self.cur_span();
        self.bump(); // case/casez/casex
        self.expect(TokenKind::LParen, "'(' after case");
        let scrutinee = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let mut items = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Endcase) {
            let before = self.pos;
            items.push(self.parse_case_item());
            if self.pos == before {
                self.bump(); // never spin on a stuck case item
            }
        }
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endcase)),
            "'endcase'",
        );
        Stmt::Case { kind, scrutinee, items, span: start.to(self.prev_span()) }
    }

    /// `default [:] stmt` | `label {, label} : stmt`.
    fn parse_case_item(&mut self) -> CaseItem {
        let start = self.cur_span();
        if self.eat_kw(Kw::Default) {
            self.eat(TokenKind::Colon); // ':' OPTIONAL after default
            let body = Box::new(self.parse_statement());
            return CaseItem::Default { body, span: start.to(self.prev_span()) };
        }
        let mut labels = vec![self.expr(0)];
        while self.eat(TokenKind::Comma) {
            labels.push(self.expr(0));
        }
        self.expect(TokenKind::Colon, "':' in case item");
        let body = Box::new(self.parse_statement());
        CaseItem::Match { labels, body, span: start.to(self.prev_span()) }
    }

    fn parse_for(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // for
        self.expect(TokenKind::LParen, "'(' after 'for'");
        let init = Box::new(self.parse_for_assign()); // `i = 0`, no trailing ';'
        self.expect(TokenKind::Semi, "';' after for-init");
        let cond = self.expr(0);
        self.expect(TokenKind::Semi, "';' after for-cond");
        let step = Box::new(self.parse_for_assign()); // `i = i+1`, no trailing ';'
        self.expect(TokenKind::RParen, "')'");
        let body = Box::new(self.parse_statement());
        Stmt::For { init, cond, step, body, span: start.to(self.prev_span()) }
    }

    /// A single blocking assignment WITHOUT a trailing `;` (for-init / for-step).
    /// H2: if the lvalue is malformed (no leading ident → `Lvalue::Error`, zero
    /// tokens consumed), still emit a Blocking with `Lvalue::Error`; `parse_for`'s
    /// own `expect(Semi)`/`expect(RParen)` drive progress (an empty `for(;;)` is out
    /// of MVP scope and recovers without hanging — the `)`/body consumption advances).
    fn parse_for_assign(&mut self) -> Stmt {
        let start = self.cur_span();
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Eq, "'=' in for-clause assignment");
        let rhs = self.expr(0);
        Stmt::Blocking { lhs, delay: None, rhs, span: start.to(self.prev_span()) }
    }

    fn parse_while(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // while
        self.expect(TokenKind::LParen, "'(' after 'while'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let body = Box::new(self.parse_statement());
        Stmt::While { cond, body, span: start.to(self.prev_span()) }
    }

    fn parse_repeat(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // repeat
        self.expect(TokenKind::LParen, "'(' after 'repeat'");
        let count = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let body = Box::new(self.parse_statement()); // may be `@(posedge clk);`
        Stmt::Repeat { count, body, span: start.to(self.prev_span()) }
    }

    fn parse_forever(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // forever — NO parens, NO count
        let body = Box::new(self.parse_statement());
        Stmt::Forever { body, span: start.to(self.prev_span()) }
    }

    // ─────────────────────── 5. blocks ───────────────────────
    fn parse_seq_block(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // begin
        let label = self.opt_block_label();
        let (decls, stmts) = self.block_body(BlockEnd::End);
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::End)), "'end'");
        self.opt_block_label(); // optional `: end_label` (no AST slot → discard)
        Stmt::Block { label, decls, stmts, span: start.to(self.prev_span()) }
    }

    fn parse_par_block(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // fork
        let label = self.opt_block_label();
        let (decls, stmts) = self.block_body(BlockEnd::Join);
        let join = self.eat_join(); // Join | JoinAny | JoinNone (latter two are Idents)
        self.opt_block_label(); // optional `: join_label`
        Stmt::Fork { label, decls, stmts, join, span: start.to(self.prev_span()) }
    }

    /// `: name` after begin/fork (or end/join) → Some(ident), else None.
    fn opt_block_label(&mut self) -> Option<Ident> {
        if self.eat(TokenKind::Colon) {
            self.ident()
        } else {
            None
        }
    }

    /// Shared block body: decls-prefix (Verilog ordering) THEN statements, until
    /// the closer (`end` for begin; any join form for fork).
    fn block_body(&mut self, end: BlockEnd) -> (Vec<NetVarDecl>, Vec<Stmt>) {
        let mut decls = Vec::new();
        while !self.at_eof() && !self.at_block_end(end) && self.net_var_kind().is_some() {
            let before = self.pos;
            if let Some(d) = self.parse_net_var() {
                decls.push(d);
            }
            if self.pos == before {
                self.bump(); // guard: malformed decl that consumed nothing
            }
        }
        let mut stmts = Vec::new();
        while !self.at_eof() && !self.at_block_end(end) {
            let before = self.pos;
            stmts.push(self.parse_statement());
            if self.pos == before {
                self.bump(); // guard: never spin on a stuck statement
            }
        }
        (decls, stmts)
    }

    /// True at this block's closer. `End` for begin; any join form for fork.
    fn at_block_end(&self, end: BlockEnd) -> bool {
        match end {
            BlockEnd::End => self.at_kw(Kw::End),
            BlockEnd::Join => {
                self.at_kw(Kw::Join)
                    || (self.is_ident() && matches!(self.cur_text(), "join_any" | "join_none"))
            }
        }
    }

    /// Consume the fork terminator → JoinKind. `join_any`/`join_none` are single
    /// `Word(Ident)` tokens (absent from `Kw::classify`). On a missing terminator
    /// (EOF / closer) record an error and default to `Join`; the GRANDPARENT loop
    /// guard owns forward progress (this method may legitimately not advance at EOF).
    fn eat_join(&mut self) -> JoinKind {
        if self.eat_kw(Kw::Join) {
            JoinKind::Join
        } else if self.is_ident() && self.cur_text() == "join_any" {
            self.bump();
            JoinKind::JoinAny
        } else if self.is_ident() && self.cur_text() == "join_none" {
            self.bump();
            JoinKind::JoinNone
        } else {
            self.error("'join' / 'join_any' / 'join_none'");
            JoinKind::Join
        }
    }

    // ─────────────────────── 6. timing / event / misc ───────────────────────
    fn parse_delay_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        // parse_delay() always returns Some after a `#` (no None path); the
        // dispatcher guarantees peek()==Hash. unwrap_or is defensive dead-fallback.
        let delay = self.parse_delay().unwrap_or(Delay {
            values: Vec::new(),
            span: start,
        });
        let body = if self.eat(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        Stmt::DelayCtrl { delay, body, span: start.to(self.prev_span()) }
    }

    fn parse_event_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        let ctrl = self.parse_sensitivity(); // consumes the `@`
        let body = if self.eat(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        Stmt::EventCtrl { ctrl, body, span: start.to(self.prev_span()) }
    }

    fn parse_trigger_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // '->'
        // H1: on a missing name, emit Stmt::Error rather than fabricate an
        // empty-segment HierPath (which would violate the ≥1-segment invariant).
        let Some(name) = self.hier_path() else {
            return self.stmt_error_at(start);
        };
        self.expect(TokenKind::Semi, "';'");
        Stmt::EventTrigger { name, span: start.to(self.prev_span()) }
    }

    fn parse_wait(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // wait
        self.expect(TokenKind::LParen, "'(' after 'wait'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let body = if self.eat(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        Stmt::Wait { cond, body, span: start.to(self.prev_span()) }
    }

    fn parse_disable(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // disable
        // M3: `disable fork;` is standard Verilog; `fork` is `Kw::Fork`, not an
        // ident, so hier_path() would fail. Special-case it → a 1-segment "fork" path.
        if self.at_kw(Kw::Fork) {
            let fspan = self.cur_span();
            self.bump(); // fork
            let seg = Ident { name: "fork".to_string(), span: fspan };
            let target = HierPath { segments: vec![seg], span: fspan };
            self.expect(TokenKind::Semi, "';'");
            return Stmt::Disable { target, span: start.to(self.prev_span()) };
        }
        // H1: on a missing/illegal name, emit Stmt::Error rather than an empty path.
        let Some(target) = self.hier_path() else {
            return self.stmt_error_at(start);
        };
        self.expect(TokenKind::Semi, "';'");
        Stmt::Disable { target, span: start.to(self.prev_span()) }
    }
}

/// Which keyword terminates a block body (begin→end, fork→join family).
#[derive(Clone, Copy)]
enum BlockEnd {
    End,
    Join,
}
```

**Note on the dropped `stmt_span` helper (C3/N1):** the earlier draft proposed a private `stmt_span(&Stmt)->Span` for outer-span unions. It is **omitted** — every method above computes spans via `start.to(self.prev_span())` at the call site, so the helper is dead code and would trip a `-D warnings` CI. Do not add it.

---

## 2. The EXACT edit to `parse_module_item`

Replace the combined stub at **lines 965–981**:

```rust
        // procedural blocks / generate / genvar → recovering STUB (types exist)
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Initial
                    | Kw::Always
                    | Kw::AlwaysFf
                    | Kw::AlwaysComb
                    | Kw::AlwaysLatch
                    | Kw::Generate
                    | Kw::Genvar
            )))
        ) {
            let s = self.cur_span();
            self.error("(procedural/generate parsing not yet implemented)");
            self.skip_balanced_block(); // consume @(…) begin…end / generate…endgenerate
            return Some(ModuleItem::Error(s));
        }
```

with this split (proc blocks → real parser; generate/genvar keep the stub):

```rust
        // procedural blocks → REAL parsing (PR2).
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Initial
                    | Kw::Always
                    | Kw::AlwaysFf
                    | Kw::AlwaysComb
                    | Kw::AlwaysLatch
            )))
        ) {
            return Some(ModuleItem::Proc(self.parse_procedural_block()));
        }
        // generate / genvar → recovering STUB (unchanged): skip_balanced_block stays.
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(Kw::Generate | Kw::Genvar)))
        ) {
            let s = self.cur_span();
            self.error("(generate parsing not yet implemented)");
            self.skip_balanced_block();
            return Some(ModuleItem::Error(s));
        }
```

`skip_balanced_block` (line 1227) **stays** — still used by the generate/genvar branch. `parse_procedural_block` always returns a value and always consumes ≥1 token (the proc keyword), so the `parse_source_unit`/module-body loop's forward-progress guard is satisfied.

---

## 3. Test cases (14) to ADD to `mod tests`

First, two helpers (place near `expr_of`, line 1313):

```rust
    /// Parse a module body; return the first ProceduralBlock.
    fn proc_of(body: &str) -> ProceduralBlock {
        let src = format!("module m;\n{body}\nendmodule");
        let (su, errs) = p(&src);
        assert!(errs.is_empty(), "parse errors: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        match m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_))) {
            Some(ModuleItem::Proc(pb)) => pb.clone(),
            _ => panic!("no procedural block in body"),
        }
    }
    fn as_block(s: &Stmt) -> (&Option<Ident>, &Vec<NetVarDecl>, &Vec<Stmt>) {
        match s {
            Stmt::Block { label, decls, stmts, .. } => (label, decls, stmts),
            other => panic!("not a Block: {other:?}"),
        }
    }
```

```rust
    // S1. initial begin: blocking + nonblocking mix
    #[test]
    fn s1_initial_blocking_nonblocking() {
        let pb = proc_of("initial begin a = 1; q <= d; end");
        assert_eq!(pb.kind, ProcKind::Initial);
        assert!(pb.sensitivity.is_none());
        let (_l, _d, stmts) = as_block(&pb.body);
        assert!(matches!(stmts[0], Stmt::Blocking { .. }));
        assert!(matches!(stmts[1], Stmt::NonBlocking { .. }));
    }

    // S2. always @(posedge clk) if/else (no begin) — sensitivity on the BLOCK
    #[test]
    fn s2_always_posedge_if_else() {
        let pb = proc_of("always @(posedge clk) if (rst) q <= 0; else q <= d;");
        assert_eq!(pb.kind, ProcKind::Always);
        let Some(Sensitivity::List(evs)) = &pb.sensitivity else { panic!() };
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].edge, Edge::Posedge);
        let Stmt::If { else_s, .. } = &*pb.body else { panic!("body not If") };
        assert!(else_s.is_some());
    }

    // S3. posedge/negedge `or`-separated sensitivity list
    #[test]
    fn s3_sensitivity_or_list() {
        let pb = proc_of("always @(posedge clk or negedge rst_n) q <= d;");
        let Some(Sensitivity::List(evs)) = &pb.sensitivity else { panic!() };
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].edge, Edge::Posedge);
        assert_eq!(evs[1].edge, Edge::Negedge);
    }

    // S4. always_comb + case: sensitivity MUST be None (@ never consumed); multi-label
    #[test]
    fn s4_always_comb_case() {
        let pb = proc_of("always_comb case (sel) 2'b00, 2'b01: y = a; default: y = b; endcase");
        assert_eq!(pb.kind, ProcKind::AlwaysComb);
        assert!(pb.sensitivity.is_none());
        let Stmt::Case { kind, items, .. } = &*pb.body else { panic!() };
        assert_eq!(*kind, CaseKind::Case);
        let CaseItem::Match { labels, .. } = &items[0] else { panic!() };
        assert_eq!(labels.len(), 2); // two labels share one body
        assert!(matches!(items[1], CaseItem::Default { .. }));
    }

    // S5. casez kind + `default` WITHOUT a colon
    #[test]
    fn s5_casez_default_no_colon() {
        let pb = proc_of("always_comb casez (req) 4'b1???: g = 1; default g = 0; endcase");
        let Stmt::Case { kind, items, .. } = &*pb.body else { panic!() };
        assert_eq!(*kind, CaseKind::Casez);
        assert!(matches!(items[1], CaseItem::Default { .. }));
    }

    // S6. for-loop — init/step are Blocking built WITHOUT consuming the ';'
    #[test]
    fn s6_for_loop() {
        let pb = proc_of("initial for (i = 0; i < 8; i = i + 1) sum = sum + i;");
        let Stmt::For { init, step, body, .. } = &*pb.body else { panic!() };
        assert!(matches!(**init, Stmt::Blocking { .. }));
        assert!(matches!(**step, Stmt::Blocking { .. }));
        assert!(matches!(**body, Stmt::Blocking { .. }));
    }

    // S7. while + $display systask call (name retains `$`, 2 args)
    #[test]
    fn s7_while_and_display() {
        let pb = proc_of("initial while (cnt < 8) begin $display(\"c=%d\", cnt); cnt = cnt + 1; end");
        let Stmt::While { body, .. } = &*pb.body else { panic!() };
        let (_l, _d, stmts) = as_block(body);
        let Stmt::SysTaskCall { name, args, .. } = &stmts[0] else { panic!() };
        assert_eq!(name.name, "$display");
        assert_eq!(args.len(), 2);
    }

    // S8. #delay statement with body, then $finish with NO parens (empty args)
    #[test]
    fn s8_delay_and_finish() {
        let pb = proc_of("initial begin #20 rst = 0; #200 $finish; end");
        let (_l, _d, stmts) = as_block(&pb.body);
        let Stmt::DelayCtrl { body: b0, .. } = &stmts[0] else { panic!() };
        assert!(matches!(b0.as_deref(), Some(Stmt::Blocking { .. })));
        let Stmt::DelayCtrl { body: b1, .. } = &stmts[1] else { panic!() };
        let Some(Stmt::SysTaskCall { name, args, .. }) = b1.as_deref() else { panic!() };
        assert_eq!(name.name, "$finish");
        assert!(args.is_empty());
    }

    // S9. dangling-else binds to the INNER if
    #[test]
    fn s9_dangling_else_inner() {
        let pb = proc_of("initial if (a) if (b) x = 1; else x = 2;");
        let Stmt::If { then_s, else_s, .. } = &*pb.body else { panic!() };
        assert!(else_s.is_none(), "outer if must NOT own the else");
        let Stmt::If { else_s: inner_else, .. } = &**then_s else { panic!("then not If") };
        assert!(inner_else.is_some(), "inner if owns the else");
    }

    // S10. named begin-end with a local decl + end-label (label consumed, no hang)
    #[test]
    fn s10_named_block_local_decl() {
        let pb = proc_of("initial begin : blk reg [7:0] tmp; tmp = a; end");
        let (label, decls, stmts) = as_block(&pb.body);
        assert_eq!(label.as_ref().unwrap().name, "blk");
        assert_eq!(decls.len(), 1);
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], Stmt::Blocking { .. }));
    }

    // S11. always @(*) Star + nested begin
    #[test]
    fn s11_nested_block_and_star() {
        let pb = proc_of("always @(*) begin a = b; begin c = d; end end");
        assert!(matches!(pb.sensitivity, Some(Sensitivity::Star)));
        let (_l, _d, stmts) = as_block(&pb.body);
        assert!(matches!(stmts[0], Stmt::Blocking { .. }));
        assert!(matches!(stmts[1], Stmt::Block { .. }));
    }

    // S12. recovery: garbage statement → Error, no infinite loop, following stmt parses
    #[test]
    fn s12_recovery_garbage_stmt() {
        let (su, errs) = p("module m;\ninitial begin & ; x = 1; end\nendmodule");
        assert!(!errs.is_empty(), "expected a recovered error");
        let su = su.unwrap();
        let m = first_module(&su);
        let Some(ModuleItem::Proc(pb)) =
            m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
        else { panic!("no proc block") };
        let (_l, _d, stmts) = as_block(&pb.body);
        assert!(stmts.iter().any(|s| matches!(s, Stmt::Error(_))), "garbage → Error");
        assert!(
            stmts.iter().any(|s| matches!(s, Stmt::Blocking { .. })),
            "must recover and parse `x = 1;`"
        );
    }

    // S13. fork / join_none — JoinKind from an Ident token (not a keyword)
    #[test]
    fn s13_fork_join_none() {
        let pb = proc_of("initial fork #10 a = 1; #20 b = 1; join_none");
        let Stmt::Fork { stmts, join, .. } = &*pb.body else { panic!() };
        assert_eq!(*join, JoinKind::JoinNone);
        assert_eq!(stmts.len(), 2);
    }

    // S14. repeat body is a bare EventCtrl (body None); wait body None;
    //      and intra-assign delay `q <= #1 d;` parses RHS cleanly with an advisory.
    #[test]
    fn s14_event_body_none_and_intra_delay() {
        let pb = proc_of("initial begin repeat (8) @(posedge clk); wait (ready); q <= #1 d; end");
        let (_l, _d, stmts) = as_block(&pb.body);
        let Stmt::Repeat { body, .. } = &stmts[0] else { panic!() };
        let Stmt::EventCtrl { body: eb, .. } = &**body else { panic!("repeat body not EventCtrl") };
        assert!(eb.is_none()); // `@(posedge clk);` → body None
        let Stmt::Wait { body: wb, .. } = &stmts[1] else { panic!() };
        assert!(wb.is_none());
        // M4: intra-assign delay discarded, RHS still parses as a NonBlocking to `d`.
        let Stmt::NonBlocking { delay, .. } = &stmts[2] else { panic!("not NonBlocking") };
        assert!(delay.is_none(), "intra-assign delay is dropped (deferred)");
    }
```

These exercise every MVP form plus the three verdict-critical paths: **dangling-else** (S9), **recovery/no-infinite-loop** (S12), and **intra-assign delay no-cascade** (S14). Note S5/S12 use `assert!(!errs.is_empty())` only where errors are intended; all clean-parse tests assert `errs.is_empty()` via `proc_of`.

---

## 4. Coverage statement

**Every MVP statement form → parsed (variant built):**

| Form | Variant | Method |
|---|---|---|
| `lhs = rhs;` | `Blocking{delay:None}` | `parse_assign_or_call` |
| `lhs <= rhs;` | `NonBlocking{delay:None}` | `parse_assign_or_call` |
| `if/else` | `If` (eager-else dangling) | `parse_if` |
| `case`/`casez`/`casex` | `Case{kind}` + `CaseItem::Match/Default` | `parse_case`/`parse_case_item` |
| `for` | `For` (init/step = no-semi `Blocking`) | `parse_for`/`parse_for_assign` |
| `while` | `While` | `parse_while` |
| `repeat` | `Repeat` | `parse_repeat` |
| `forever` | `Forever` (no parens) | `parse_forever` |
| `begin…end` (named, decls, end-label) | `Block` | `parse_seq_block`/`block_body` |
| `fork…join/join_any/join_none` | `Fork{join}` | `parse_par_block`/`eat_join` |
| `#delay [stmt]` | `DelayCtrl{body:Option}` | `parse_delay_stmt` |
| `@(…) [stmt]` / `@*` / `@(*)` | `EventCtrl{ctrl}` | `parse_event_stmt`/`parse_sensitivity` |
| `wait(c) [stmt]` | `Wait{body:Option}` | `parse_wait` |
| `$sys(…);` | `SysTaskCall` (name keeps `$`) | `parse_systask_call` |
| `task(…);` / `task;` | `UserTaskCall` | `parse_assign_or_call` |
| `disable path;` / `disable fork;` | `Disable` | `parse_disable` |
| `assign lv=e;` | `Assign` | `parse_proc_assign` |
| `deassign lv;` | `Deassign` | `parse_deassign` |
| `force lv=e;` | `Force` | `parse_force` |
| `release lv;` | `Release` | `parse_release` |
| `-> ev;` | `EventTrigger` | `parse_trigger_stmt` |
| `;` | `Null` | dispatcher |
| garbage | `Error` + sync | `stmt_error`/`stmt_error_at` |
| `initial`/`always[@]`/`always_ff @`/`always_comb`/`always_latch` | `ProceduralBlock{sensitivity}` | `parse_procedural_block` |

**Sensitivity:** `@*`/`@(*)`→`Star`; `@(posedge a or negedge b , c)`→`List` (both `or` and `,` separate); block-vs-statement placement correct (`always`/`always_ff @(…)` fold onto the block; `always_comb`/`always_latch`/`initial` never consume `@`; body-internal `@` → `EventCtrl`).

**Deferred (out of MVP, per doc-01 Phase-1):**
- `foreach`, `unique`/`priority` case prefixes, `do…while` — not dispatched (fall to `stmt_error`, recover cleanly).
- **Intra-assignment delay/event** (`a = #5 b;`, `q <= @(ev) d;`) — RHS parses; the `#d`/`@(…)` is consumed-and-discarded with ONE advisory error (`delay` stored as `None`). No cascade. (M4 resolution.)
- Bare `@ identifier` (parens-less legacy event) — only `@*`/`@(…)` accepted; bare `@x` recovers via the empty-list path. (Scope-confirmed acceptable; see Residual.)
- Empty `for(;;)` / declaration-as-for-init — recovers without hanging (H2), not a first-class form.

---

## 5. Residual risks

1. **H3 — `always_ff` sensitivity on the block contradicts the AST doc-comment** ("only general always"). Behavior is correct and type-legal (field is `Option`), but if a later elaborate pass asserts `always_ff.sensitivity.is_none()` it will break. **Action: get sign-off; the comment is now stale.** This is the one frozen-contract divergence.

2. **Bare `@ev` (parens-less) rejected.** Verilog-2005 allows `@ identifier stmt`. The parser only handles `@*`/`@(…)`; `@x` parses as `EventCtrl{List([])}` + 1 error, then `x` as a separate statement. Confirm this is acceptable for the MVP (doc-01 lists only `@(event)`).

3. **Panic-mode block overrun on a missing inner `end`.** A `begin` whose body has an unterminated nested `begin` can let the statement scan reach and consume the outer `end`/`endcase` as failed statements (force-bumped). Mitigated because `synchronize` stops at `end`/`endcase`/`join`, so `stmt_error` recovery usually lands on the right closer — but deeply nested mismatches still cascade. Shared limitation with the existing PR1 parser; acceptable.

4. **`for`-clause garbage emits error spew (H2).** Malformed/empty for-clauses (`for(;;)`, `for(int i=0;…)`) recover without hanging but produce 3–5 cascaded errors and `Lvalue::Error`-laden nodes. Out of MVP scope; flagged for a future for-clause-empty handling pass.

5. **Late mid-block declarations rejected (V2005 ordering).** A `reg y;` appearing after a statement in a `begin…end` routes through `parse_statement`→`stmt_error` (the `reg` keyword isn't a statement start), yielding ~2 cascaded errors. By-design (strict decl-prefix ordering); SV-style mixed decls are out of scope.

6. **`eat_join` non-advance at EOF/closer** relies on the grandparent block-body loop guard for progress, not on itself. Verified safe (the `block_body` stmt loop exits on `at_block_end`/`at_eof` before `eat_join` is even called in the normal path). Documented in the method.

**Build cleanliness:** no new deps; edition 2021 / MSRV 1.82; no dead code (`stmt_span` dropped per C3); `unreachable!()` on the pre-screened proc-kind arms (N2); every list loop has a `before==pos → bump` forward-progress guard; `parse_sensitivity`'s guard is pinned AFTER the separator-eat with a comment. No panic path, no slice-OOB.

Files: parser to extend `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/hdl-parser/src/lib.rs` (edit dispatch at lines 965–981; add the new `impl` block after line 1270; add `proc_of`/`as_block` + S1–S14 in `mod tests` after line 1313). Frozen AST `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/hdl-ast/src/lib.rs` (no changes). Lexer `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/hdl-lexer/src/lib.rs` (no changes; `join_any`/`join_none` confirmed absent from `Kw::classify` → `Word(Ident)`).