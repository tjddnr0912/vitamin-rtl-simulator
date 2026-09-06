//! blocks / timing controls — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

/// Which keyword terminates a block body (begin→end, fork→join family).
#[derive(Clone, Copy)]
pub(crate) enum BlockEnd {
    End,
    Join,
}

/// Closer selector for function/task bodies (mirrors `BlockEnd` for begin/fork).
#[derive(Clone, Copy)]
pub(crate) enum BlockEnd2 {
    Endfunction,
    Endtask,
}

impl Parser<'_, '_> {
    /// G11: a numeric literal immediately followed (span-adjacent, no whitespace) by a
    /// time-unit ident `fs`/`ps`/`ns`/`us`/`ms`/`s` is a time literal `1ns` (IEEE §5.8).
    /// Adjacency is the discriminator: `1ns` is a time literal, `1 s` / `x + s` are not,
    /// and `1step` is untouched (`step` ∉ the unit set, and it is anyway lexed elsewhere).
    pub(crate) fn maybe_time_literal(&mut self, num: Expr) -> Expr {
        if !self.is_ident() {
            return num;
        }
        let unit_exp: i8 = match self.cur_text() {
            "s" => 0,
            "ms" => -3,
            "us" => -6,
            "ns" => -9,
            "ps" => -12,
            "fs" => -15,
            _ => return num,
        };
        // The unit token must TOUCH the number (no whitespace) — `1 s` is not a literal.
        if self.cur_span().lo != num.span.hi {
            return num;
        }
        let start = num.span;
        self.bump(); // the time-unit ident
        Expr {
            kind: ExprKind::TimeLit {
                num: Box::new(num),
                unit_exp,
            },
            span: start.to(self.prev_span()),
        }
    }
    /// `#5` | `#(d)` | `#(r,f)` | `#(r,f,t)`. Each paren'd value may be mintypmax
    /// `1:2:3` (verdict M2). Uses `parse_delay_value` which accepts `a:b:c`.
    pub(crate) fn parse_delay(&mut self) -> Option<Delay> {
        let start = self.cur_span();
        self.bump(); // '#'
        let mut values = Vec::new();
        if self.eat(TokenKind::LParen) {
            loop {
                values.push(self.parse_delay_value());
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')'");
        } else {
            // bare `#delay_value`: a single number/ident (no parens) — high bp,
            // no mintypmax (a bare `#1:2:3` is not legal V2005 delay).
            values.push(self.expr(UNARY_BP));
        }
        Some(Delay {
            values,
            span: start.to(self.prev_span()),
        })
    }
    /// A delay value inside `#(…)`: `expr` or `min:typ:max` (verdict M2).
    pub(crate) fn parse_delay_value(&mut self) -> Expr {
        let start = self.cur_span();
        let first = self.expr(0);
        if self.peek() == Some(TokenKind::Colon) {
            self.bump();
            let typ = self.expr(0);
            self.expect(TokenKind::Colon, "':' in min:typ:max delay");
            let max = self.expr(0);
            Expr {
                kind: ExprKind::MinTypMax {
                    min: Box::new(first),
                    typ: Box::new(typ),
                    max: Box::new(max),
                },
                span: start.to(self.prev_span()),
            }
        } else {
            first
        }
    }

    // ─────────────────────── 1. procedural blocks ───────────────────────
    /// `initial S` | `always [@(…)] S` | `always_ff @(…) S` | `always_comb S`
    /// | `always_latch S`. For `always`/`always_ff` a leading `@(…)` folds onto
    /// `ProceduralBlock.sensitivity`.
    pub(crate) fn parse_procedural_block(&mut self) -> ProceduralBlock {
        let start = self.cur_span();
        let kind = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(k))) => match k {
                Kw::Initial => ProcKind::Initial,
                Kw::Always => ProcKind::Always,
                Kw::AlwaysFf => ProcKind::AlwaysFf,
                Kw::AlwaysComb => ProcKind::AlwaysComb,
                Kw::AlwaysLatch => ProcKind::AlwaysLatch,
                Kw::Final => ProcKind::Final,
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

    /// `@*` | `@(*)` → Star ;  `@(ev or ev , …)` → List ;  bare `@e` / `@clk`
    /// → single NO-EDGE event (IEEE 1364 `@ hierarchical_identifier`).  Consumes
    /// the leading `@`. The bare reference is parsed identically to the paren form,
    /// so whatever `@(X)` does, `@X` does too: a whole signal/event simulates, while
    /// a form whose feature is unsupported (single-bit level `@a[2]`, hierarchical
    /// `@u.s`, a call `@f(x)`) routes to the SAME loud reject as `@(…)` at elaborate.
    pub(crate) fn parse_sensitivity(&mut self) -> Sensitivity {
        self.bump(); // '@'
        if self.eat(TokenKind::Star) {
            return Sensitivity::Star; // `@*`
        }
        // Bare, paren-free event control `@ hierarchical_event_identifier` — a
        // single NO-EDGE reference (`@e`, `@clk`, `@u.s`, `@a[2]`), equivalent to
        // `@(e)`. Parse a primary+postfix REFERENCE (`expr_postfix`, not a full
        // `expr(0)`): a bare binary form `@a+b` stops after `a`, and the trailing
        // `+b` is a loud statement error — matching iverilog, which rejects
        // `@a+b`/`@a && b`. A bare edge `@posedge clk` is also illegal (parens
        // required); `posedge`/`negedge` are keywords (not idents), so they fall
        // through to the `'(' or '*'` error below.
        if self.is_ident() {
            let start = self.cur_span();
            let expr = self.expr_postfix();
            let span = start.to(self.prev_span());
            return Sensitivity::List(vec![EventExpr {
                edge: Edge::NoEdge,
                expr,
                iff: None,
                span,
            }]);
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
                // forward-progress guard MUST stay AFTER the separator-eat
                if self.pos == before {
                    self.bump();
                }
                if !sep || self.peek() == Some(TokenKind::RParen) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "')'");
        Sensitivity::List(events)
    }

    /// `[posedge|negedge] expr` → EventExpr.
    pub(crate) fn parse_event_expr(&mut self) -> EventExpr {
        let start = self.cur_span();
        let edge = if self.eat_kw(Kw::Posedge) {
            Edge::Posedge
        } else if self.eat_kw(Kw::Negedge) {
            Edge::Negedge
        } else {
            Edge::NoEdge
        };
        let expr = self.expr(0);
        // G7 (IEEE §9.4.2.3): an optional `iff <expr>` guard on this event term. `iff`
        // is a contextual keyword (a plain Ident here), and `expr(0)` stops at it since
        // it is neither an operator nor a term separator. The guard `expr(0)` likewise
        // stops at `or`/`,`/`)`.
        let iff = if self.at_ident_kw("iff") {
            self.bump();
            Some(self.expr(0))
        } else {
            None
        };
        let span = start.to(iff.as_ref().map_or(expr.span, |e| e.span));
        EventExpr {
            edge,
            expr,
            iff,
            span,
        }
    }

    // ─────────────────────── 2. statement dispatcher ───────────────────────
    /// STMT-DEPTH guard: cap statement-recursion so pathological `begin begin …`
    /// nesting is a clean parse error, never a SIGABRT. 256 is ≫ any real RTL
    /// (deepest practical nesting is <30) and the deepest frame reached at the
    /// cap (≈3 frames/level: parse_statement → parse_seq_block → block_body)
    /// fits a 2 MiB test-thread stack even in debug — 1024 overflowed it. The
    /// cap path consumes no token, but the `block_body` loop's `pos == before`
    /// guard bumps one, so recovery always makes progress (no spin).
    pub(crate) const MAX_STMT_DEPTH: u32 = 256;

    /// Intra-assignment timing control after `=`/`<=` (IEEE 1800 §9.4.5): a `#d`
    /// delay (CAPTURED into `delay`), an `@(ev)` event control, or `repeat(n) @(ev)`
    /// (both CAPTURED into `event`). The elaborator lowers the event form as
    /// capture-now/wait/write for a blocking `=` (process blocks), and as a
    /// capture-now/`fork … join_none` desugar for a non-blocking `<=` (slice N1 —
    /// the process does not block). The `blocking` flag is retained for symmetry and
    /// future per-form diagnostics; both forms capture identically here.
    pub(crate) fn parse_intra_assign_timing(
        &mut self,
        _blocking: bool,
    ) -> (Option<Delay>, Option<IntraEvent>) {
        match self.peek() {
            Some(TokenKind::Hash) => (self.parse_delay(), None),
            Some(TokenKind::At) => {
                let ctrl = self.parse_sensitivity(); // consumes `@(…)`
                (None, Some(IntraEvent { repeat: None, ctrl }))
            }
            _ if self.at_kw(Kw::Repeat) => {
                self.bump(); // repeat
                self.expect(TokenKind::LParen, "'(' after 'repeat'");
                let count = self.expr(0);
                self.expect(TokenKind::RParen, "')'");
                if self.peek() == Some(TokenKind::At) {
                    let ctrl = self.parse_sensitivity();
                    (
                        None,
                        Some(IntraEvent {
                            repeat: Some(count),
                            ctrl,
                        }),
                    )
                } else {
                    self.error("`@(event)` after `repeat(n)` in an intra-assignment control");
                    (None, None)
                }
            }
            _ => (None, None),
        }
    }

    pub(crate) fn parse_force(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // force
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Eq, "'=' in force");
        let rhs = self.expr(0);
        let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
        self.expect(TokenKind::Semi, "';'");
        Stmt::Force {
            lhs,
            rhs,
            span: start.to(self.prev_span()),
        }
    }
    pub(crate) fn parse_deassign(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // deassign
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Semi, "';'");
        Stmt::Deassign {
            lhs,
            span: start.to(self.prev_span()),
        }
    }
    pub(crate) fn parse_release(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // release
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Semi, "';'");
        Stmt::Release {
            lhs,
            span: start.to(self.prev_span()),
        }
    }

    /// Consume an optional `: label` after `endsequence`/`endproperty`
    /// (accept-and-ignore — the minimal-surface choice).
    pub(crate) fn eat_end_label(&mut self) {
        if self.peek() == Some(TokenKind::Colon) {
            self.bump();
            let _ = self.ident();
        }
    }

    pub(crate) fn parse_par_block(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // fork
        let label = self.opt_block_label();
        let (decls, stmts) = self.block_body(BlockEnd::Join);
        let join = self.eat_join(); // Join | JoinAny | JoinNone (latter two are Idents)
        self.opt_block_label(); // optional `: join_label`
        Stmt::Fork {
            label,
            decls,
            stmts,
            join,
            span: start.to(self.prev_span()),
        }
    }

    /// `: name` after begin/fork (or end/join) → Some(ident), else None.
    pub(crate) fn opt_block_label(&mut self) -> Option<Ident> {
        if self.eat(TokenKind::Colon) {
            self.ident()
        } else {
            None
        }
    }

    /// Shared block body: decls-prefix THEN statements, until the closer.
    pub(crate) fn block_body(&mut self, end: BlockEnd) -> (Vec<NetVarDecl>, Vec<Stmt>) {
        let mut decls = Vec::new();
        // A block is lexically scoped: snapshot the scope registries at the block's
        // first body-local typedef DEFINITION or struct/enum-typed VAR decl, and
        // restore them when the block ends, so a local name does not leak out of /
        // clobber an outer one. `None` until such a decl appears → zero overhead
        // for the common (plain-var) block.
        // BOXED: `ScopeSnapshot` is ~11 collections (~520 B). `block_body` sits on
        // the `parse_statement → parse_seq_block → block_body` recursion whose
        // `MAX_STMT_DEPTH` budget is FRAME-sized, so an inline `Option<ScopeSnapshot>`
        // put the whole struct on every recursion frame — at the cap that overran
        // the 2 MiB test-thread stack (`depth_guard::deep_stmt_nesting_errors_cleanly`)
        // once one more map was added. A `Box` keeps only an 8-byte pointer on the
        // frame; the common (plain-var) block stays `None` → no heap alloc.
        let mut scope: Option<Box<ScopeSnapshot>> = None;
        while !self.at_eof() && !self.at_block_end(end) {
            let before = self.pos;
            if self.at_kw(Kw::Typedef) {
                if scope.is_none() {
                    scope = Some(self.snapshot_scope_boxed());
                }
                // A bare `begin/end` block has no `body_enums` carrier, so a
                // body-local enum here stays honest-loud (allow_enum = false).
                let _ = self.parse_body_typedef_def(false);
            } else if self.net_var_kind().is_some() {
                // procedural block-local decl: no net delay. Snapshots the scope when
                // the decl shadows a bound outer name (see the helper).
                if let Some(d) = self.parse_block_plain_decl(&mut scope) {
                    decls.push(d);
                }
            } else if self.at_kw(Kw::Automatic) && self.lifetime_prefixes_decl() {
                // GAP-D (IEEE §6.21): a block-local decl with an explicit
                // `automatic` lifetime override (`automatic int unsigned idx;`).
                // Parsed in a cold, non-inlined helper so its locals never
                // enlarge this hot recursive frame — `block_body` sits on the
                // `parse_statement → parse_seq_block → block_body` recursion, and
                // the MAX_STMT_DEPTH stack budget is frame-sized. Growing this
                // frame inline overflowed the 2 MiB test-thread stack at the cap
                // (`depth_guard.rs::deep_stmt_nesting_errors_cleanly`); mirrors
                // `parse_with_postfix` on the expr path.
                if let Some(d) = self.parse_automatic_block_decl(&mut scope) {
                    decls.push(d);
                } else {
                    // R18 §3.3: `automatic` in front of an UNPACKED-STRUCT type. The
                    // helper above has already consumed the keyword and cannot resolve
                    // the name (an unpacked struct lives in `struct_layouts`, not
                    // `typedefs`), so the member fan-out below is what actually parses
                    // the declaration — and it stamped NO lifetime. `automatic rec_t r;`
                    // was therefore downgraded to static in silence, which is why two
                    // same-named struct locals in disjoint blocks shared one flattened
                    // net while the identical `automatic int` / enum / alias pair each
                    // got its own `$blk$` scope (measured: alias ok, enum ok, struct
                    // not gathered at all).
                    let before = decls.len();
                    if self.try_block_unpacked_struct_decl(&mut decls, &mut scope) {
                        for d in &mut decls[before..] {
                            d.lifetime = Some(true);
                        }
                    }
                }
            } else if self.try_block_unpacked_struct_decl(&mut decls, &mut scope) {
                // Round-9: a block-local scalar UNPACKED-struct variable
                // (`p::kat_t k;`) desugars to N member nets `$unp$k$field`. The
                // PEEK and the parse both live in the cold, non-inlined helper so
                // NONE of their locals (the peek's `String` key + the member-
                // `NetVarDecl` construction) enlarge THIS frame — `block_body`
                // sits on the deep `parse_statement → parse_seq_block →
                // block_body` recursion and the `MAX_STMT_DEPTH` budget is
                // frame-sized (mirrors `parse_automatic_block_decl`; enlarging
                // this frame overflowed the 2 MiB CI test-thread stack at the cap
                // — `depth_guard.rs::deep_stmt_nesting_errors_cleanly`).
            } else if let Some(info) = self.peek_block_typedef_decl() {
                // A procedural block-local declaration using a user-defined type
                // name (`my_enum_t state = IDLE;` / `byte_t b;` / `s_t s;`),
                // mirroring the module-item typedef-decl path. This writes the
                // VAR-name-keyed maps, so it too triggers the scope snapshot — a
                // block-local `s_t x` that shadows an outer `s_t x` must not leak
                // its layout binding out of the block.
                if scope.is_none() {
                    scope = Some(self.snapshot_scope_boxed());
                }
                if let Some(d) = self.parse_typed_decl(info) {
                    decls.push(d);
                }
            } else {
                break; // not a declaration → the statement region begins
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
        // Drop block-local typedefs / struct-var bindings (restore the outer scope)
        // AFTER statements are parsed — a statement may reference a local typedef
        // (e.g. a cast `t'(x)`) or a local struct var's `x.field`.
        if let Some(scope) = scope {
            self.restore_scope_boxed(scope);
        }
        (decls, stmts)
    }

    /// True at this block's closer. `End` for begin; any join form for fork.
    pub(crate) fn at_block_end(&self, end: BlockEnd) -> bool {
        match end {
            BlockEnd::End => self.at_kw(Kw::End),
            BlockEnd::Join => {
                self.at_kw(Kw::Join)
                    || (self.is_ident() && matches!(self.cur_text(), "join_any" | "join_none"))
            }
        }
    }

    /// Consume the fork terminator → JoinKind.
    pub(crate) fn eat_join(&mut self) -> JoinKind {
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
    pub(crate) fn parse_delay_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        let delay = self.parse_delay().unwrap_or(Delay {
            values: Vec::new(),
            span: start,
        });
        let body = if self.eat(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        Stmt::DelayCtrl {
            delay,
            body,
            span: start.to(self.prev_span()),
        }
    }

    pub(crate) fn parse_event_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        let ctrl = self.parse_sensitivity(); // consumes the `@`
        let body = if self.eat(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        Stmt::EventCtrl {
            ctrl,
            body,
            span: start.to(self.prev_span()),
        }
    }

    pub(crate) fn parse_trigger_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // '->'
                     // H1: on a missing name, emit Stmt::Error rather than an empty path.
        let Some(name) = self.hier_path() else {
            return self.stmt_error_at(start);
        };
        self.expect(TokenKind::Semi, "';'");
        Stmt::EventTrigger {
            name,
            span: start.to(self.prev_span()),
        }
    }

    pub(crate) fn parse_wait(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // wait
                     // `wait fork;` — `fork` is `Kw::Fork`, not an ident, so special-case it
                     // before the `wait(expr)` path (mirrors `parse_disable`).
        if self.at_kw(Kw::Fork) {
            self.bump(); // fork
            self.expect(TokenKind::Semi, "';'");
            return Stmt::WaitFork {
                span: start.to(self.prev_span()),
            };
        }
        self.expect(TokenKind::LParen, "'(' after 'wait'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let body = if self.eat(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        Stmt::Wait {
            cond,
            body,
            span: start.to(self.prev_span()),
        }
    }
}
