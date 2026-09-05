//! statement dispatch — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Map a compound-assignment / increment operator token to its binary op
    /// (SV §11.4.1/§11.4.2). `None` for any non-compound token. `++`/`--` reuse
    /// Add/Sub (with a synthesized `1`); `<<<=`/`>>>=` are the arithmetic shifts.
    pub(crate) fn compound_assign_binop(t: TokenKind) -> Option<BinOp> {
        use TokenKind as T;
        Some(match t {
            T::PlusEq | T::PlusPlus => BinOp::Add,
            T::MinusEq | T::MinusMinus => BinOp::Sub,
            T::StarEq => BinOp::Mul,
            T::SlashEq => BinOp::Div,
            T::PercentEq => BinOp::Mod,
            T::AmpEq => BinOp::BitAnd,
            T::PipeEq => BinOp::BitOr,
            T::CaretEq => BinOp::BitXor,
            T::ShlEq => BinOp::Shl,
            T::ShrEq => BinOp::Shr,
            T::ShlAEq => BinOp::AShl,
            T::ShrAEq => BinOp::AShr,
            _ => return None,
        })
    }

    /// If the cursor is on a compound-assign (`+=`…) or postfix `++`/`--`, consume
    /// it (plus the rhs for a compound op) and return the desugared blocking
    /// assignment `lvalue = lvalue <op> operand` (`++`/`--` use a literal `1`).
    /// Does NOT consume a trailing `;` (the caller owns that — statement vs
    /// for-clause). `None` ⇒ not a compound/inc-dec operator (caller handles
    /// `=`/`<=`/task-call). Side-effect-bearing EXPRESSION forms (`a = i++`) are
    /// NOT parsed here — they stay a loud error (correct-or-loud).
    ///
    /// The lvalue appears on both sides, so any index/select sub-expression is
    /// re-read on the rhs — but this is BYTE-IDENTICAL to the explicit
    /// `lvalue = lvalue <op> e` it desugars to (verified by differential), so the
    /// transform itself is exact. (A pure index reads the same value twice; a
    /// side-effecting index like `arr[f()]` follows the SAME path as the explicit
    /// form, where vita's pre-existing index-eval semantics already apply — that
    /// quirk is out of scope here, not introduced by this desugar.)
    pub(crate) fn try_compound_assign(&mut self, lhs: &Lvalue, start: Span) -> Option<Stmt> {
        let t = self.peek()?;
        let op = Self::compound_assign_binop(t)?;
        let is_incdec = matches!(t, TokenKind::PlusPlus | TokenKind::MinusMinus);
        self.bump(); // the operator
        let operand = if is_incdec {
            Expr {
                span: self.prev_span(),
                kind: ExprKind::IntLit {
                    kind: IntLitKind::Decimal,
                    raw: "1".to_string(),
                },
            }
        } else {
            self.expr(0)
        };
        let span = start.to(self.prev_span());
        let lhs_expr = Self::lvalue_to_expr(lhs);
        let rhs = Expr {
            span,
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs_expr),
                rhs: Box::new(operand),
            },
        };
        Some(Stmt::Blocking {
            lhs: lhs.clone(),
            delay: None,
            event: None,
            rhs,
            span,
        })
    }

    /// `++lvalue` / `--lvalue` (prefix). As a STATEMENT the pre/post distinction is
    /// invisible (the value is discarded), so this desugars identically to
    /// `lvalue = lvalue ± 1`. Cursor is on `++`/`--`. Does NOT consume `;`.
    pub(crate) fn parse_pre_incdec(&mut self, start: Span) -> Stmt {
        let t = self.peek().expect("caller checked ++/--");
        let op = Self::compound_assign_binop(t).expect("caller checked ++/--");
        self.bump(); // ++ / --
        let lhs = self.parse_lvalue();
        let span = start.to(self.prev_span());
        let lhs_expr = Self::lvalue_to_expr(&lhs);
        let one = Expr {
            span,
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: "1".to_string(),
            },
        };
        let rhs = Expr {
            span,
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs_expr),
                rhs: Box::new(one),
            },
        };
        Stmt::Blocking {
            lhs,
            delay: None,
            event: None,
            rhs,
            span,
        }
    }

    pub(crate) fn parse_statement(&mut self) -> Stmt {
        self.stmt_depth += 1;
        if self.stmt_depth > Self::MAX_STMT_DEPTH {
            self.stmt_depth -= 1;
            let s = self.cur_span();
            self.error("statement nesting too deep (cap 256)");
            return Stmt::Error(s);
        }
        let r = self.parse_statement_inner();
        self.stmt_depth -= 1;
        r
    }

    pub(crate) fn parse_statement_inner(&mut self) -> Stmt {
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
            // SV §11.4.2 prefix `++i;` / `--i;` (statement form). As a statement the
            // pre/post distinction is invisible → `i = i ± 1`.
            Some(T::PlusPlus) | Some(T::MinusMinus) => {
                let start = self.cur_span();
                let s = self.parse_pre_incdec(start);
                self.expect(TokenKind::Semi, "';'");
                s
            }
            Some(T::Word(WordKind::Keyword(kw))) => match kw {
                Kw::Begin => self.parse_seq_block(),
                Kw::Fork => self.parse_par_block(),
                Kw::If => self.parse_if(),
                Kw::Case => self.parse_case(CaseKind::Case),
                Kw::Casez => self.parse_case(CaseKind::Casez),
                Kw::Casex => self.parse_case(CaseKind::Casex),
                Kw::For => self.parse_for(),
                Kw::While => self.parse_while(),
                // P2-E: `do body while (cond);` — parse-time desugar (no new
                // AST node): { body; while (cond) body }.
                Kw::Do => self.parse_do_while(),
                // P2-E: unique/priority QUALIFIERS on if/case — the violation
                // check desugars to a synthesized `$warning` arm (IEEE
                // §12.4/12.5: a no-match is a runtime violation warning).
                Kw::Unique | Kw::Priority | Kw::Unique0 | Kw::Priority0 => {
                    self.parse_unique_priority()
                }
                Kw::Foreach => self.parse_foreach(),
                Kw::Repeat => self.parse_repeat(),
                Kw::Forever => self.parse_forever(),
                Kw::Wait => self.parse_wait(),
                Kw::Disable => self.parse_disable(),
                Kw::Assign => self.parse_proc_assign(),
                Kw::Deassign => self.parse_deassign(),
                Kw::Force => self.parse_force(),
                Kw::Release => self.parse_release(),
                // SVA-REST: `assume` parses like `assert` (sim-checked the same).
                Kw::Assert | Kw::Assume => self.parse_assert(),
                // N4: `void'(call);` — a discard-cast statement (evaluate the call
                // for its side effects, drop the result). IEEE §13.4.1 explicit-void.
                Kw::Void
                    if self.peek_at(1) == Some(T::Apostrophe)
                        && self.peek_at(2) == Some(T::LParen) =>
                {
                    self.parse_void_cast_stmt()
                }
                _ => self.stmt_error(),
            },
            Some(T::SystemTask) => self.parse_systask_call(),
            // N7: SV `return [expr];` — contextual (not a V2005 reserved word), so
            // a net literally named `return` in legacy code still parses as an
            // assign/call (the `return EXPR;` / `return;` shape is unambiguous in
            // statement position: a V2005 program has no `return` statement).
            _ if self.at_ident_kw("return") => self.parse_return(),
            // SVA-REST: `cover property(@(clk) seq);` — `cover` is contextual (an SV
            // reserved word, never a legal net name) and recognized only when
            // immediately followed by `property`.
            _ if self.at_ident_kw("cover")
                && self.peek_at(1) == Some(TokenKind::Word(WordKind::Keyword(Kw::Property))) =>
            {
                self.parse_cover_property()
            }
            // SV §11.5 `break;` / `continue;` — contextual (not V2005 reserved), so
            // a net literally named `break`/`continue` used as `break = x;` still
            // parses as an assign. Recognized ONLY in the `break;`/`continue;`
            // statement shape (immediately followed by `;`).
            _ if self.at_ident_kw("break") && self.peek_at(1) == Some(TokenKind::Semi) => {
                self.parse_break_continue(true)
            }
            _ if self.at_ident_kw("continue") && self.peek_at(1) == Some(TokenKind::Semi) => {
                self.parse_break_continue(false)
            }
            // IEEE 1800-2017 §9.3.5: a statement label `L: stmt`. See
            // `parse_labeled_stmt` for the desugar (a named block around the statement).
            _ if self.is_ident() && self.peek_at(1) == Some(T::Colon) => self.parse_labeled_stmt(),
            _ if self.is_ident() => self.parse_assign_or_call(),
            _ => self.stmt_error(),
        }
    }

    /// `label : statement` (IEEE 1800-2017 §9.3.5). The label names a scope around
    /// the statement — `L: begin … end` is `begin : L … end` by the LRM's own
    /// equivalence, and the same reading is applied to every other statement: the
    /// statement is wrapped in a named block, so `%m` inside it reports `top.L`, a
    /// `disable L` ends it, and a `$error` from a labelled `assert` names the label's
    /// scope (verilator prints `top.L` for both; iverilog 13 accepts a label on an
    /// immediate assertion only and prints the enclosing scope — recorded as its
    /// leniency, not an oracle). lowRISC's `ASSERT_INIT(name, prop)` is
    /// `initial begin name: assert (prop) else begin … end end` — thirty of them in
    /// ibex were `E2002` before this. A block that already carries its own label
    /// (`L: begin : M`) is illegal (§9.3.5) and both oracles refuse it.
    pub(crate) fn parse_labeled_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        let Some(label) = self.ident() else {
            return self.stmt_error_at(start);
        };
        self.bump(); // ':'
        let inner = self.parse_statement();
        let span = start.to(self.prev_span());
        // A loop that used `break` arrives wrapped in a SYNTHETIC named block
        // (`begin : $break$<lo> loop end`, `wrap_break`); that label is not one the
        // user wrote, so the user's label goes on a block AROUND it (`disable L`
        // still ends the loop) — review A-1: nine `L: for (…) … break;` cells were
        // E2002 "one label on a block" for a label nobody wrote.
        let synthetic =
            matches!(&inner, Stmt::Block { label: Some(l), .. } if l.name.starts_with('$'));
        match inner {
            other if synthetic => Stmt::Block {
                label: Some(label),
                decls: Vec::new(),
                stmts: vec![other],
                span,
            },
            Stmt::Block {
                label: None,
                decls,
                stmts,
                span,
            } => Stmt::Block {
                label: Some(label),
                decls,
                stmts,
                span,
            },
            Stmt::Fork {
                label: None,
                decls,
                stmts,
                join,
                span,
            } => Stmt::Fork {
                label: Some(label),
                decls,
                stmts,
                join,
                span,
            },
            Stmt::Block { label: Some(_), .. } | Stmt::Fork { label: Some(_), .. } => {
                self.error_at(
                    label.span,
                    "one label on a block (a statement label and a block label on the \
                     same block is illegal, IEEE 1800 §9.3.5)",
                );
                inner
            }
            other => Stmt::Block {
                label: Some(label),
                decls: Vec::new(),
                stmts: vec![other],
                span,
            },
        }
    }

    /// `return [expr] ;` (N7).
    pub(crate) fn parse_return(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // 'return'
        let value = if self.peek() == Some(TokenKind::Semi) {
            None
        } else {
            Some(self.expr(0))
        };
        self.expect(TokenKind::Semi, "';' after return");
        Stmt::Return {
            value,
            span: start.to(self.prev_span()),
        }
    }

    /// Unparseable statement: record one error, build Error, sync, GUARANTEE ≥1
    /// token consumed.
    pub(crate) fn stmt_error(&mut self) -> Stmt {
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
    pub(crate) fn stmt_error_at(&mut self, start: Span) -> Stmt {
        let before = self.pos;
        self.synchronize();
        if self.pos == before {
            self.bump();
        }
        Stmt::Error(start.to(self.prev_span()))
    }

    // ─────────────────────── 3. assignments / task calls ───────────────────────
    /// Leading ident or `{`: blocking `=`, nonblocking `<=`, or a user-task call.
    /// N3 SoA: build a blocking / non-blocking assignment `lhs = rhs` (no timing).
    pub(crate) fn assign_stmt(lhs: Lvalue, rhs: Expr, blocking: bool, span: Span) -> Stmt {
        if blocking {
            Stmt::Blocking {
                lhs,
                delay: None,
                event: None,
                rhs,
                span,
            }
        } else {
            Stmt::NonBlocking {
                lhs,
                delay: None,
                event: None,
                rhs,
                span,
            }
        }
    }

    pub(crate) fn parse_assign_or_call(&mut self) -> Stmt {
        let start = self.cur_span();
        let lhs = self.parse_lvalue();
        // SV §11.4.1/§11.4.2 statement form: `lvalue += e;` / `lvalue++;` desugar to
        // a blocking `lvalue = lvalue <op> …`. An expression-embedded `a = i++` is
        // NOT handled (the expr parser has no `++`) → stays a loud parse error.
        if let Some(stmt) = self.try_compound_assign(&lhs, start) {
            self.expect(TokenKind::Semi, "';'");
            return stmt;
        }
        match self.peek() {
            Some(TokenKind::Eq) => {
                self.bump();
                let (delay, event) = self.parse_intra_assign_timing(true);
                let rhs = self.expr(0);
                // N3 SoA: `arr=new[N]` / `arr=other` / `arr[i]='{…}` on a SoA record
                // array → a Block of per-field native dyn ops (only without intra-assign
                // timing, which a whole-array op never has).
                if delay.is_none() && event.is_none() {
                    if let Some(stmt) =
                        self.try_soa_assign(&lhs, &rhs, true, start.to(self.prev_span()))
                    {
                        self.expect(TokenKind::Semi, "';'");
                        return stmt;
                    }
                }
                let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
                self.expect(TokenKind::Semi, "';'");
                Stmt::Blocking {
                    lhs,
                    delay,
                    event,
                    rhs,
                    span: start.to(self.prev_span()),
                }
            }
            Some(TokenKind::LtEq) => {
                self.bump();
                let (delay, event) = self.parse_intra_assign_timing(false);
                let rhs = self.expr(0);
                if delay.is_none() && event.is_none() {
                    if let Some(stmt) =
                        self.try_soa_assign(&lhs, &rhs, false, start.to(self.prev_span()))
                    {
                        self.expect(TokenKind::Semi, "';'");
                        return stmt;
                    }
                }
                let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
                self.expect(TokenKind::Semi, "';'");
                Stmt::NonBlocking {
                    lhs,
                    delay,
                    event,
                    rhs,
                    span: start.to(self.prev_span()),
                }
            }
            // user-task call: bare HierPath followed by `(` or `;`
            Some(TokenKind::LParen) | Some(TokenKind::Semi) => {
                if let Lvalue::Ident(path) = lhs {
                    let args = if self.peek() == Some(TokenKind::LParen) {
                        self.call_args()
                    } else {
                        Vec::new()
                    };
                    let args = self.expand_struct_call_args(args); // R5: struct actual → members
                    let args = self.desugar_container_pattern_args(&path, args);
                    // `obj.randomize() with { … };` as a void statement (§18.7).
                    if self.at_ident_kw("with") && self.peek_at(1) == Some(TokenKind::LBrace) {
                        self.bump(); // `with`
                        let constraints = self.parse_with_constraints();
                        self.expect(TokenKind::Semi, "';'");
                        return Stmt::RandomizeWith {
                            name: path,
                            args,
                            constraints,
                            span: start.to(self.prev_span()),
                        };
                    }
                    self.expect(TokenKind::Semi, "';'");
                    let span = start.to(self.prev_span());
                    // r18 (Fix A): a queue method on a SoA record queue
                    // (`q.push_back(p)`/`insert`/`delete`) fans out to per-field native
                    // queue ops. A non-SoA-queue receiver → the generic call below.
                    if let Some(s) = self.try_soa_queue_method_stmt(&path, &args, span) {
                        s
                    } else {
                        Stmt::UserTaskCall {
                            name: path,
                            args,
                            span,
                        }
                    }
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

    // N4: `void'(EXPR);` discard-cast statement. Parse `void ' ( EXPR ) ;` and lower
    // to the discard form of EXPR: a user call → `UserTaskCall`, a system call →
    // `SysTaskCall` (the two idioms in the TBs: `void'($value$plusargs(...))` and
    // `void'(helper())`). Any other inner expression evaluates for side effects but
    // has no statement form yet → clean loud error (correct-or-loud).
    pub(crate) fn parse_void_cast_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // `void`
        self.bump(); // `'`
        self.bump(); // `(`
        let inner = self.expr(0);
        self.expect(TokenKind::RParen, "')' closing void'()");
        self.expect(TokenKind::Semi, "';'");
        let span = start.to(self.prev_span());
        match inner.kind {
            // R16 §3.6: route through the SoA record-queue fan-out first, exactly as the
            // bare `q.method();` statement path does. Building the `UserTaskCall`
            // directly here meant `void'(q.pop_front());` on a SoA record queue skipped
            // the fan-out that the identical un-wrapped statement would have taken.
            ExprKind::Call { name, args } => self
                .try_soa_queue_method_stmt(&name, &args, span)
                .unwrap_or(Stmt::UserTaskCall { name, args, span }),
            ExprKind::SysCall { name, args } => Stmt::SysTaskCall { name, args, span },
            _ => {
                self.error_at(inner.span, "a call expression inside void'( … )");
                self.stmt_error_at(start)
            }
        }
    }

    // procedural-continuous family — all reuse parse_lvalue
    pub(crate) fn parse_proc_assign(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // assign
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Eq, "'=' in procedural assign");
        let rhs = self.expr(0);
        let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
        self.expect(TokenKind::Semi, "';'");
        Stmt::Assign {
            lhs,
            rhs,
            span: start.to(self.prev_span()),
        }
    }
    /// If `break` was used in this loop, wrap the whole loop in a synthetic named
    /// block `begin : $break$<lo> loop end` (its exit is past the loop). No-op
    /// (byte-identical) when `break` was not used.
    pub(crate) fn wrap_break(&self, loop_stmt: Stmt, break_used: bool, start: Span) -> Stmt {
        if break_used {
            Stmt::Block {
                label: Some(Ident {
                    name: format!("$break${}", start.lo),
                    span: start,
                }),
                decls: Vec::new(),
                stmts: vec![loop_stmt],
                span: start,
            }
        } else {
            loop_stmt
        }
    }

    /// `break;` / `continue;` (SV §11.5). Desugars to `disable <synthetic-label>`
    /// of the innermost enclosing loop — `break` jumps past the loop, `continue`
    /// to the loop's continue point. Records that the wrap is needed. Outside any
    /// loop it is a loud error (correct-or-loud). Reuses the proven `disable`→Goto
    /// lowering, so fork-crossing break/continue is loud-rejected at elaborate.
    pub(crate) fn parse_break_continue(&mut self, is_break: bool) -> Stmt {
        let start = self.cur_span();
        self.bump(); // `break` / `continue`
        self.expect(TokenKind::Semi, "';'");
        let span = start.to(self.prev_span());
        match self.loop_labels.last_mut() {
            Some(ctx) => {
                let label = if is_break {
                    ctx.break_used = true;
                    ctx.break_label.clone()
                } else {
                    ctx.continue_used = true;
                    ctx.continue_label.clone()
                };
                Stmt::Disable {
                    target: HierPath {
                        segments: vec![Ident { name: label, span }],
                        span,
                    },
                    span,
                }
            }
            None => {
                self.error(if is_break {
                    "an enclosing loop for this `break`"
                } else {
                    "an enclosing loop for this `continue`"
                });
                Stmt::Error(span)
            }
        }
    }

    pub(crate) fn parse_disable(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // disable
                     // M3: `disable fork;` — `fork` is `Kw::Fork`, not an ident, so special-case it.
        if self.at_kw(Kw::Fork) {
            let fspan = self.cur_span();
            self.bump(); // fork
            let seg = Ident {
                name: "fork".to_string(),
                span: fspan,
            };
            let target = HierPath {
                segments: vec![seg],
                span: fspan,
            };
            self.expect(TokenKind::Semi, "';'");
            return Stmt::Disable {
                target,
                span: start.to(self.prev_span()),
            };
        }
        // H1: on a missing/illegal name, emit Stmt::Error rather than an empty path.
        let Some(target) = self.hier_path() else {
            return self.stmt_error_at(start);
        };
        self.expect(TokenKind::Semi, "';'");
        Stmt::Disable {
            target,
            span: start.to(self.prev_span()),
        }
    }
}
