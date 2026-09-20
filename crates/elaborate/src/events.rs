//! timing controls / sensitivity — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// hdl-ast `Edge` → sim-ir `EdgeKind`. A bare signal (`NoEdge`) in an
/// edge-classified or level list arms on `AnyEdge`.
pub(crate) fn map_edge(e: ast::Edge) -> ir::EdgeKind {
    match e {
        ast::Edge::Posedge => ir::EdgeKind::Posedge,
        ast::Edge::Negedge => ir::EdgeKind::Negedge,
        ast::Edge::NoEdge => ir::EdgeKind::AnyEdge,
    }
}

impl Elaborator<'_> {
    /// Lower a blocking intra-assignment EVENT control `lhs = [repeat(n)] @(ev) rhs`
    /// (IEEE 1800 §9.4.5) as capture-now / wait / write:
    ///   `tmp = rhs;  @(ev) × n;  lhs = tmp;`
    /// The RHS is captured NOW into a temp sized EXACTLY to the lvalue (so the rhs
    /// eval context is unchanged), the process waits for the event `n` times, then
    /// the captured value is written. The repeat count is folded scope-aware (so a
    /// `parameter`/`localparam` count works), and the wait is emitted `n` times via
    /// the validated EventCtrl lowering — NOT through `Stmt::Repeat`, whose
    /// scope-blind count fold would silently elide the wait for a non-literal count.
    /// A non-constant or oversized count is LOUD (never a silent 0-event write).
    /// `repeat(0)`/`repeat(<0)` ⇒ zero waits = an immediate write (IEEE). The lvalue
    /// is resolved up front but its index evaluates at the final write — identical to
    /// the `#d` intra-delay path.
    pub(crate) fn lower_intra_event_assign(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        ie: &ast::IntraEvent,
        rhs: &ast::Expr,
        span: ast::Span,
    ) {
        // Fold the repeat count FIRST (loud-and-return before emitting any IR).
        let waits: u32 = match &ie.repeat {
            None => 1,
            Some(n) => match self.const_eval_in_scope(n) {
                Some(c) => {
                    let c = c.max(0); // repeat(0)/repeat(<0) ⇒ zero iterations (IEEE)
                    if c > REPEAT_UNROLL_CAP as i64 {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "an intra-assignment `repeat(n)` count exceeds the unroll cap ({REPEAT_UNROLL_CAP})"
                            ),
                        );
                        return;
                    }
                    c as u32
                }
                None if count_lit_is_xz(n) => 0, // X/Z CONSTANT ⇒ 0 iterations (IEEE)
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a runtime (non-constant) `repeat(n)` count in an intra-assignment \
                         event control is unsupported (n must fold to a constant)",
                    );
                    return;
                }
            },
        };
        let rhs_id = self.lower_expr(rhs);
        let lv = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv, true); // P1-9 (E3018): no proc write to a net
        let w = self.ir_lvalue_width(&lv);
        let tmp = self.fresh_ia_tmp(w);
        let cap = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(tmp),
            rhs: rhs_id,
        });
        b.push_stmt_id(cap);
        // Wait for the event `waits` times (zero ⇒ immediate write). Emitting the
        // EventCtrl `waits` times produces `waits` sequential Wait terminators.
        let evt = ast::Stmt::EventCtrl {
            ctrl: ie.ctrl.clone(),
            body: None,
            span,
        };
        for _ in 0..waits {
            self.lower_stmt(b, &evt);
        }
        // Write the captured value (lvalue index evaluated here, at write time).
        let tmp_read = self.push_expr(ir::Expr::Signal {
            net: tmp,
            word: None,
        });
        let wr = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: lv,
            rhs: tmp_read,
        });
        b.push_stmt_id(wr);
    }

    /// N1: lower a NON-BLOCKING intra-assignment event control
    /// `lhs <= [repeat(n)] @(ev) rhs` (IEEE 1800 §9.4.5). The twin of
    /// [`lower_intra_event_assign`], but the process must NOT block: the RHS — and
    /// any LHS index — are captured NOW, then a detached `fork … join_none` helper
    /// waits for the event `n` times and performs the NBA write of the captured
    /// value, while the parent continues immediately. `repeat(0)` degenerates to a
    /// plain same-tick NBA (the captured value joins the current NBA region).
    ///
    /// Documented DIVERGENCES from iverilog (hand-IEEE pins, like prior slices):
    /// (1) the count must fold to a constant — a genuinely runtime count is LOUD
    /// here (iverilog accepts it), matching the blocking-form precedent (an X/Z
    /// CONSTANT count is 0 iterations per IEEE, NOT runtime — see `count_lit_is_xz`);
    /// (2) same-site self-overlapping in-flight captures share the per-site temp,
    /// whereas iverilog carries an independent value per in-flight assignment;
    /// (3) SAME-TICK region tie (the project's "동시-틱 tie = 도구-발산" zone): at the
    /// helper's write tick, a process woken by the SAME edge that reads the lhs in the
    /// Active/`#0` region sees the OLD value here (LRM-faithful — an NBA is invisible
    /// in the write tick's active region) but the NEW value under iverilog. The
    /// committed value matches exactly ($strobe/postponed + every later tick + final);
    /// the fork+NBA desugar cannot place its write one region earlier without an
    /// engine event-armed-NBA mechanism.
    pub(crate) fn lower_intra_event_nba(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        ie: &ast::IntraEvent,
        rhs: &ast::Expr,
        span: ast::Span,
    ) {
        // Fold the repeat count FIRST (loud-and-return before emitting any IR).
        let waits: u32 = match &ie.repeat {
            None => 1,
            Some(n) => match self.const_eval_in_scope(n) {
                Some(c) => {
                    let c = c.max(0); // repeat(0)/repeat(<0) ⇒ zero iterations (IEEE)
                    if c > REPEAT_UNROLL_CAP as i64 {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "a non-blocking intra-assignment `repeat(n)` count exceeds the unroll cap ({REPEAT_UNROLL_CAP})"
                            ),
                        );
                        return;
                    }
                    c as u32
                }
                None if count_lit_is_xz(n) => 0, // X/Z CONSTANT ⇒ 0 iterations (IEEE)
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a runtime (non-constant) `repeat(n)` count in a non-blocking \
                         intra-assignment event control is unsupported (n must fold to a constant)",
                    );
                    return;
                }
            },
        };

        // repeat(0) ⇒ no wait ⇒ a plain same-tick NBA: the normal NBA path samples
        // the RHS and any LHS index at execution time and joins the current NBA
        // region — exactly the degenerate semantics (no helper needed).
        if waits == 0 {
            let rhs_id = self.lower_expr(rhs);
            let lv = self.lower_lvalue(lhs);
            self.check_lvalue_kind(&lv, true); // P1-9 (E3018)
            let sid = self.push_stmt(ir::Stmt::NonblockingAssign {
                lhs: lv,
                rhs: rhs_id,
                delay: None,
            });
            b.push_stmt_id(sid);
            return;
        }

        // The detached helper is a fork child; a fork nested inside a fork child is
        // the v1 MVP cut (§6.2). Reject loudly rather than emit a nested Fork.
        if self.in_fork {
            self.error_unsupported(
                span,
                "a non-blocking intra-assignment event control inside a fork is \
                 unsupported in v1 (the helper would be a nested fork)",
            );
            return;
        }

        // ── CAPTURE NOW (parent block, before the fork) ──
        // RHS → a private per-site temp.
        let rhs_id = self.lower_expr(rhs);
        let lv = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv, true); // P1-9 (E3018): no proc write to a net
        let w = self.ir_lvalue_width(&lv);
        let tmp = self.fresh_ia_tmp(w);
        let cap = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(tmp),
            rhs: rhs_id,
        });
        b.push_stmt_id(cap);
        // Any LHS index/offset is sampled NOW too (asymmetric with the blocking
        // form, which samples the lvalue at WRITE time — verified vs iverilog).
        let lv_now = self.capture_lvalue_indices_now(b, &lv);

        // ── FORK (join_none): the parent resumes IMMEDIATELY at resume_bb ──
        // INV-2: allocate every named block before sealing with Fork.
        let join_bb = b.new_block();
        let resume_bb = b.new_block();
        let child_entry = b.new_block();
        self.record_fork_mode(ast::JoinKind::JoinNone, join_bb.raw());
        b.end_block_with(ir::Terminator::Fork {
            children: vec![child_entry.raw()],
            join: join_bb.raw(),
            resume_bb: resume_bb.raw(),
        });

        // ── CHILD: wait for the event `waits` times, then NBA-write the capture ──
        let prev_in_fork = self.in_fork;
        self.in_fork = true;
        let prev_floor = self.disable_fork_floor;
        self.disable_fork_floor = self.disable_stack.len();
        b.start_block(child_entry);
        let evt = ast::Stmt::EventCtrl {
            ctrl: ie.ctrl.clone(),
            body: None,
            span,
        };
        for _ in 0..waits {
            self.lower_stmt(b, &evt);
        }
        let tmp_read = self.push_expr(ir::Expr::Signal {
            net: tmp,
            word: None,
        });
        let wr = self.push_stmt(ir::Stmt::NonblockingAssign {
            lhs: lv_now,
            rhs: tmp_read,
            delay: None,
        });
        b.push_stmt_id(wr);
        b.goto(join_bb);
        self.disable_fork_floor = prev_floor;
        self.in_fork = prev_in_fork;

        // Seal join_bb → resume_bb (never-executed sentinel; the engine intercepts
        // the child at join_bb) and open resume_bb as the single continuation.
        b.start_block(join_bb);
        b.goto(resume_bb);
        b.start_block(resume_bb);
    }

    /// Capture every DYNAMIC lvalue index/offset (`Some(ExprId)`) into a fresh temp
    /// (sampled NOW) and rebuild the lvalue to read from those temps. A scalar /
    /// whole-net lvalue (no `word`/`offset`) passes through with no extra IR.
    pub(crate) fn capture_lvalue_indices_now(
        &mut self,
        b: &mut ProcessBuilder,
        lv: &ir::Lvalue,
    ) -> ir::Lvalue {
        let chunks = lv
            .chunks
            .iter()
            .map(|ch| ir::LvalChunk {
                net: ch.net,
                word: ch.word.map(|e| self.capture_index_now(b, e)),
                offset: ch.offset.map(|e| self.capture_index_now(b, e)),
                width: ch.width,
                kind: ch.kind,
            })
            .collect();
        ir::Lvalue { chunks }
    }

    /// Capture an index/offset ExprId NOW into a fresh temp; return a `Signal` read
    /// of that temp (for use in the deferred lvalue rebuild).
    pub(crate) fn capture_index_now(&mut self, b: &mut ProcessBuilder, e: u32) -> u32 {
        let w = self.ir_bits_of(e).unwrap_or(32).max(1);
        let tmp = self.fresh_ia_tmp(w);
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(tmp),
            rhs: e,
        });
        b.push_stmt_id(sid);
        self.push_expr(ir::Expr::Signal {
            net: tmp,
            word: None,
        })
    }

    // ── one ProceduralBlock → one Process ──────────────────────────
    /// G7: the single-term `iff` event guard (IEEE §9.4.2.3) — `Some(guard)` when the
    /// sensitivity list has exactly one term and it carries `iff` (`@(edge sig iff g)`),
    /// else `None`. A multi-term list where any term has `iff` cannot become one
    /// body-wrap (differing per-term guards) → loud, returns `None` (the caller then
    /// lowers unguarded, which fails elaboration anyway — no silent-wrong).
    pub(crate) fn event_iff_guard(&mut self, ctrl: &ast::Sensitivity) -> Option<ast::Expr> {
        let ast::Sensitivity::List(terms) = ctrl else {
            return None;
        };
        if terms.iter().all(|t| t.iff.is_none()) {
            return None;
        }
        if terms.len() != 1 {
            self.error(
                MsgCode::ElabUnsupported,
                "an `iff` guard on a multi-term event control is unsupported (only a \
                 single guarded term `@(edge sig iff cond)` is supported)",
            );
            return None;
        }
        terms[0].iff.clone()
    }

    /// G7: rewrite `always @(edge sig iff g) S` to `always @(edge sig) if (g) S` — the
    /// IR sensitivity has no guard slot, so the guard rides as a body `if`. Strips the
    /// `iff` from the sensitivity (so the recursion in `lower_proc_block` does not
    /// re-desugar). `None` for the common no-`iff` case (and after a multi-term loud).
    pub(crate) fn desugar_event_iff(
        &mut self,
        p: &ast::ProceduralBlock,
    ) -> Option<ast::ProceduralBlock> {
        let sens = p.sensitivity.as_ref()?;
        let guard = self.event_iff_guard(sens)?;
        let ast::Sensitivity::List(terms) = sens else {
            return None;
        };
        let stripped = ast::Sensitivity::List(vec![ast::EventExpr {
            iff: None,
            ..terms[0].clone()
        }]);
        Some(ast::ProceduralBlock {
            kind: p.kind,
            sensitivity: Some(stripped),
            body: Box::new(ast::Stmt::If {
                cond: guard,
                then_s: p.body.clone(),
                else_s: None,
                span: p.span,
            }),
            span: p.span,
        })
    }

    // ── sensitivity mapping ────────────────────────────────────────
    /// `ProcKind` + AST `Sensitivity` → `ir::Sensitivity`. Classification:
    /// any explicit edge ⇒ `Edge`; all bare ⇒ `Level`; `always_ff` forces
    /// `Edge`; `@(*)`/`always_comb` ⇒ `Comb` (read-set inference deferred —
    /// empty edges, no error); `always_latch` ⇒ `Latch`; `initial` ⇒ `Initial`.
    pub(crate) fn lower_sensitivity(
        &mut self,
        kind: ast::ProcKind,
        sens: Option<&ast::Sensitivity>,
        body: &ast::Stmt, // M-C: inspect body for in-body timing on bare `always`
    ) -> ir::Sensitivity {
        use ast::ProcKind::*;
        match kind {
            // P2-E `final`: Initial-shaped in the frozen IR (no sensitivity
            // variant exists and none is needed) — the engine SKIPS arming it
            // via the final_procs side table and runs it at end of simulation.
            Initial | Final => ir::Sensitivity {
                kind: ir::SensKind::Initial,
                edges: Vec::new(),
            },
            AlwaysComb => ir::Sensitivity {
                kind: ir::SensKind::Comb,
                edges: Vec::new(),
            },
            AlwaysLatch => ir::Sensitivity {
                kind: ir::SensKind::Latch,
                edges: Vec::new(),
            },
            AlwaysFf => self.classify_event_list(sens, /* force_edge = */ true),
            Always => match sens {
                None => {
                    if stmt_has_timing(body) {
                        // Legal self-timed `always` (clock generator). The body's
                        // own #/@ drives time; the process re-runs (forever-wrapped
                        // in lower_proc_block). No header edges → Comb-shaped arm.
                        ir::Sensitivity {
                            kind: ir::SensKind::Comb,
                            edges: Vec::new(),
                        }
                    } else {
                        // Truly unschedulable: warn (non-fatal) but still emit a
                        // valid (inert) process rather than killing the whole IR.
                        self.warn(
                            "always with neither @(...) nor in-body timing is \
                             unschedulable; lowered as an inert process",
                        );
                        ir::Sensitivity {
                            kind: ir::SensKind::Comb,
                            edges: Vec::new(),
                        }
                    }
                }
                // ⭐⭐ `always @*` is LEVEL, not Comb — and the whole difference
                // is the time-zero arm. IEEE 1800 §9.2.2.2 gives `always_comb`
                // (and `always_latch`) an implicit execution at time zero;
                // `always @*` has none — it waits for its inferred read set to
                // change, exactly like `always @(a or b)` does.
                //
                // Every other consumer already treats the three alike: the
                // scheduler registers Level, Comb and Latch with the SAME level
                // waiter over `sensitivity.edges`, and the read set is filled
                // from `comb_inferred_procs`, a list of process ids that does not
                // consult this kind. So this is a one-word change with no IR
                // shape change and no `format_version` move.
                //
                // ⚠️ MEASURED, not reasoned: `reg a = 0; always @* out = a;`
                // left `out` at **x** in iverilog and **0** in vita, because
                // nothing ever changes `a` so the block must never run. That
                // 0-for-x is what made `axi_register_wr`'s `m_axi_awvalid_next`
                // definite, and it is every one of the 29 x-cycles by which
                // verilog-axi's digest differed from the oracle's (ROADMAP §2-N).
                //
                // ⚠️ The `None` arm above stays `Comb` on purpose: a self-timed
                // `always` with in-body `#`/`@` is a clock generator and MUST
                // start at time zero, or nothing in the design ever moves.
                Some(ast::Sensitivity::Star) => ir::Sensitivity {
                    kind: ir::SensKind::Level,
                    edges: Vec::new(),
                },
                Some(s @ ast::Sensitivity::List(_)) => {
                    self.classify_event_list(Some(s), /* force_edge = */ false)
                }
            },
        }
    }

    /// Map a `Sensitivity::List` to Edge-or-Level. `force_edge` (always_ff) pins
    /// the kind to Edge. Determinism: edges appended in source order.
    pub(crate) fn classify_event_list(
        &mut self,
        sens: Option<&ast::Sensitivity>,
        force_edge: bool,
    ) -> ir::Sensitivity {
        // N4: a bare `@(cb)` where `cb` names a clocking block lowers to the
        // clocking event (`@(posedge clk)`). Recurse with the substituted event.
        if let Some(ev) = self.clocking_event_subst(sens) {
            return self.classify_event_list(Some(&ev), force_edge);
        }
        let list = match sens {
            Some(ast::Sensitivity::List(l)) => l.as_slice(),
            Some(ast::Sensitivity::Star) | None => {
                if force_edge {
                    self.warn("always_ff requires an explicit @(edge ...) list");
                }
                return ir::Sensitivity {
                    kind: if force_edge {
                        ir::SensKind::Edge
                    } else {
                        ir::SensKind::Comb
                    },
                    edges: Vec::new(),
                };
            }
        };
        let any_edge = force_edge || list.iter().any(|ev| !matches!(ev.edge, ast::Edge::NoEdge));
        // §9.4.2, in two passes because the LEVEL rule is a property of the LIST, not
        // of a term (see below).
        //
        // Pass 1 sorts the terms. An EDGE term on a CONSTANT is dropped outright —
        // [`Self::event_term_never_wakes`], `armed_before_t0 = true` because this is
        // the process-header lane. A NON-EDGE term on a constant is held aside
        // ([`Self::header_level_term_is_const`]); everything else is LIVE.
        let mut live: Vec<&ast::EventExpr> = Vec::new();
        let mut const_level: Vec<(&ast::EventExpr, &str, ast::Span)> = Vec::new();
        for ev in list {
            if self.event_term_never_wakes(ev, /* armed_before_t0 = */ true) {
                continue;
            }
            match self.header_level_term_is_const(ev) {
                Some((name, at)) => const_level.push((ev, name, at)),
                None => live.push(ev),
            }
        }
        // Pass 2. A constant LEVEL term is only a problem when it is the WHOLE
        // sensitivity: alone it decides that the process fires once at t0 and never
        // again, a shape this subset cannot build, so it is refused. Beside a LIVE
        // term it decides nothing the live term does not already decide — drop it and
        // arm the rest, exactly as the EDGE lane does, or the refusal swallows the
        // live sibling (MEASURED: `generate … localparam int V = 99; always @(V or W)`
        // with `W` a real net was refused whole where both oracles print `HDR at 0` /
        // `HDR at 1` / `DONE`; the t0 line is the recorded level-constant gap, the
        // `HDR at 1` one is this defect).
        let mut edges = Vec::new();
        if live.is_empty() {
            for (ev, name, at) in const_level {
                self.error_header_level_const(name, at);
                edges.push(ir::EdgeTerm {
                    net: POISON_NET,
                    kind: map_edge(ev.edge),
                });
            }
        } else {
            for ev in live {
                let net = self.sens_event_net(&ev.expr, any_edge);
                edges.push(ir::EdgeTerm {
                    net,
                    kind: map_edge(ev.edge),
                });
            }
        }
        ir::Sensitivity {
            kind: if any_edge {
                ir::SensKind::Edge
            } else {
                ir::SensKind::Level
            },
            edges,
        }
    }

    /// Is this a PROCESS-HEADER NON-EDGE term whose BARE head binds a constant?
    /// `Some((name, span))` when it is — the two facts
    /// [`Self::error_header_level_const`] needs. PURE: the caller decides whether to
    /// refuse, because that is a property of the whole LIST (a constant level term
    /// beside a live one is dropped, not refused).
    ///
    /// ⚠️ BARE HEAD ONLY, parentheses pierced, no select. `@(K[0] or clk)` keeps the
    /// pre-existing "single-bit level (non-edge) event control is not supported"
    /// refusal, which is a different (recorded) gap — both oracles run it.
    fn header_level_term_is_const<'a>(
        &self,
        ev: &'a ast::EventExpr,
    ) -> Option<(&'a str, ast::Span)> {
        if !matches!(ev.edge, ast::Edge::NoEdge) {
            return None;
        }
        let (name, at) = Self::event_bare_head(&ev.expr)?;
        self.bare_name_binds_constant(name, at)
            .then_some((name, at))
    }

    /// Refuse a header LEVEL term that binds a constant and has no live sibling,
    /// saying which of the two constants it is.
    ///
    /// `event_term_never_wakes` deliberately answers `false` here — a header level
    /// term on a constant fires ONCE at time 0 in both oracles, which this subset
    /// has no shape for — and the UNSHADOWED spelling was already loud through
    /// `resolve_net`. The SHADOWED one was not: `lookup_net_scoped` walks `symbols`
    /// alone, so `generate if (1) begin : g localparam int V = 2; always @(V) …`
    /// armed the OUTER net and fired again when it changed (MEASURED `HDR fired at
    /// 0` + `HDR fired at 1`, where iverilog 13.0 and verilator 5.052 both print
    /// `HDR fired at 0` then `DONE`). One IEEE question was answered loud in one
    /// spelling and silently wrong in its shadow twin; this is the parity.
    ///
    /// The two messages are the §2 🆕 O pair: `error_const_shadows_net` when a net
    /// of the same name is in scope (the reader must know WHICH object vita took),
    /// and the `@(pkg::CONST)` arm's "a constant cannot wake a process" sentence
    /// otherwise — replacing `resolve_net`'s `E3010 undeclared net/variable`, which
    /// was a false statement about the program (`localparam int K = 99;` IS a
    /// declaration).
    ///
    /// ⚠️ ONLY when the term is the WHOLE sensitivity — see the two-pass loop in
    /// `classify_event_list`. Beside a LIVE term the constant is dropped instead,
    /// or the refusal swallows a sibling that does wake the process.
    fn error_header_level_const(&mut self, name: &str, at: ast::Span) {
        if self.bare_const_shadows_net(name, at) {
            self.error_const_shadows_net(
                name,
                &format!(
                    "a constant cannot wake a process, so the level event control \
                     `@({name})` has nothing to sense here"
                ),
            );
        } else {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a level event control `@({name})` must name a net or variable: `{name}` \
                     is a constant (parameter / localparam / genvar / enum label) and a \
                     constant cannot wake a process (an event control waits for a CHANGE). \
                     `posedge`/`negedge` on a constant is accepted and simply never fires"
                ),
            );
        }
    }

    /// The BARE single-segment name at an event term's head, with its span —
    /// parentheses pierced, a select NOT (see `header_level_term_is_const`).
    fn event_bare_head(e: &ast::Expr) -> Option<(&str, ast::Span)> {
        match &e.kind {
            ast::ExprKind::Paren { inner } => Self::event_bare_head(inner),
            ast::ExprKind::Ident(path) => match path.segments.as_slice() {
                [seg] => Some((seg.name.as_str(), path.span)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Can this event-control term NEVER wake its process, because the value it
    /// names can never change? That is "its head binds a CONSTANT" — a parameter /
    /// localparam / genvar / enum label, selected or not — with ONE measured
    /// exception, `armed_before_t0`.
    ///
    /// IEEE 1800 §9.4.2: an event control waits for a CHANGE in the expression. A
    /// constant has none, so an EDGE term on one fires never: MEASURED, iverilog
    /// 13.0 and verilator 5.052 both print only `DONE` for `always @(posedge K[0])`,
    /// `always @(posedge K)`, `always @(negedge K)` and for the in-body
    /// `initial begin @(posedge K); $display("NEVER"); end`. vita had no constant
    /// path here at all: the bit-select arm below refused `@(posedge K[0])` as a
    /// non-LSB select (E3009, because `lsb_bitselect_net` declines a name that is
    /// not a net) and the bare-ident arm sent `@(posedge K)` to `resolve_net` as an
    /// undeclared net (E3010). Routing the SHADOWED spelling through
    /// `lookup_net_unshadowed` turned that pre-existing FALSE LOUD into a REGRESSION
    /// on a design that had run (`generate … localparam int V = 99;` shadowing
    /// `logic [7:0] V`), so the close is the constant path itself, which retires the
    /// shadowed cell and both unshadowed twins at once.
    ///
    /// ⚠️ `armed_before_t0` — a PROCESS-HEADER sensitivity (`always @(…)`) is armed
    /// before the time-zero settle, and a NON-EDGE term there fires ONCE at time 0
    /// even on a constant: MEASURED `always @(K) $display("EDGE at %0t", $time)` →
    /// `EDGE at 0` in BOTH oracles, and `always @(K or clk)` → `EDGE at 0` then
    /// `EDGE at 1`. The IN-BODY `@(K)` is reached after that settle and fires never
    /// (both oracles print `DONE` alone), which is why the flag is a parameter and
    /// not a property of the expression. A header non-edge constant term therefore
    /// keeps whatever it does today (loud) — vita has no "fire once at t0, then
    /// never" shape for an explicit list, and inventing one by re-kinding the
    /// process to `Comb` would move every levelize / native classification that
    /// reads `SensKind::Comb`. Recorded, both oracles' text above.
    ///
    /// The term is DROPPED from the sensitivity rather than refused, so a mixed list
    /// (`@(posedge K or posedge clk)`) still arms on the live term — one constant
    /// term must not silence its siblings (MEASURED: `EDGE at 1` + `DONE`, all three
    /// tools).
    ///
    /// ⚠️ Constant-ness is asked of the HEAD through [`Self::expr_head_binds_constant`],
    /// the one funnel the lowering itself uses, so classifier and lowering cannot
    /// disagree under shadowing. A shape whose head is not a bare name (an operator,
    /// a call, `pkg::K`) answers `false` and keeps whatever diagnostic it had — the
    /// conservative side, since `true` stands a rule down.
    pub(crate) fn event_term_never_wakes(
        &self,
        ev: &ast::EventExpr,
        armed_before_t0: bool,
    ) -> bool {
        if armed_before_t0 && matches!(ev.edge, ast::Edge::NoEdge) {
            return false;
        }
        self.expr_head_binds_constant(&ev.expr)
    }

    /// Resolve an event-control expr to the net it senses. Supported: a bare
    /// signal name (or parenthesized one), and — only in an EDGE-sensitive list
    /// (`edge_ctx`) — a CONSTANT bit-select whose selected bit IS the net's LSB
    /// (packed bit 0). The engine's EDGE model checks bit 0 only, so
    /// `@(posedge clk[lsb])` arms identically to `@(posedge clk)` (IEEE: vector
    /// posedge tracks the LSB). LEVEL sensitivity, by contrast, fires on a
    /// WHOLE-NET change, so a level bit-select (`@(clk[0])`) is NOT representable
    /// (mapping it to the net over-triggers on sibling-bit changes) → rejected.
    /// Everything else (non-LSB bit, part-select, variable/non-const index, array
    /// element, multi-dim packed select, computed base) needs per-bit tracking we
    /// lack → LOUD reject (E3009), NOT a silent POISON_NET that would index
    /// `net_to_edge[u32::MAX]` and panic the scheduler (`error` sets `had_error`,
    /// so the IR is discarded and sim-engine is never reached).
    pub(crate) fn sens_event_net(&mut self, e: &ast::Expr, edge_ctx: bool) -> u32 {
        match &e.kind {
            ast::ExprKind::Ident(path) => {
                // N4: a multi-segment `@(u0.cb)` whose LAST segment names a clocking
                // block is a CROSS-HIERARCHY clocking-event control — give an accurate
                // event-control message (not the lvalue/hier-WRITE text `resolve_net`
                // would emit). The cross-hier VALUE read `u0.cb.sig` IS supported; an
                // in-module `@(cb)` IS supported; cross-hier event wakeup is not (the
                // clocking-event table is keyed by bare module-local name).
                if path.segments.len() > 1
                    && path
                        .segments
                        .last()
                        .is_some_and(|s| self.all_clocking_names.contains(&s.name))
                {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a cross-hierarchy clocking-event control `@(inst.cb)` is unsupported \
                         in this subset (the cross-hierarchy value read `inst.cb.sig` and an \
                         in-module `@(cb)` event are supported)",
                    );
                    return POISON_NET;
                }
                // R28 9-A: an unresolved MULTI-segment path here is a hierarchical name
                // in an EVENT CONTROL, and `resolve_net`'s message for that case names
                // an lvalue ("a hierarchical name in this lvalue context … a whole-net
                // hierarchical write `tb.dut.x = …` is supported; a hierarchical
                // element/part-select write is a follow-on"). An event control is not an
                // lvalue and the design contains no hierarchical write, so the reader is
                // sent looking for an assignment that does not exist — the reporter
                // spent significant time on exactly that, on a foundry ADC model whose
                // `always @(\`TOP.a_uVDC.RTRIM_I)` is the only site. Same defect class as
                // the N4 clocking arm just above, and the same fix: when a caller knows
                // the context, it owns the message. The resolution question is asked
                // with `lookup_dotted_net` — `resolve_net`'s own predicate, not a copy.
                if path.segments.len() > 1 && self.lookup_dotted_net(path).is_none() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "a hierarchical name (`{}`) in an EVENT CONTROL is unsupported \
                             in this subset — reading it is supported, so `@(*)` over a \
                             local copy works: `always @(*) local = {};` then trigger on \
                             `local`. (This is a sensitivity-registration limit, not a \
                             name-resolution one.)",
                            path.segments
                                .iter()
                                .map(|s| s.name.as_str())
                                .collect::<Vec<_>>()
                                .join("."),
                            path.segments
                                .iter()
                                .map(|s| s.name.as_str())
                                .collect::<Vec<_>>()
                                .join("."),
                        ),
                    );
                    return POISON_NET;
                }
                let n = self.resolve_net(path);
                if self.is_dyn_handle_net(n) || self.is_string_net(n) {
                    // v5 ⑥/v7: handles carry no dirty channel — they can
                    // never wake a process (design §4).
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a dynamic-storage handle cannot appear in an event control",
                    );
                }
                n
            }
            // `@(pkg::sig)` — an explicitly scoped package variable in an event
            // control. A package variable is ONE shared net per elaboration, so
            // `pkg::sig` resolves to the SAME net the imported bare `@(sig)` arms
            // on (iverilog-pinned). A package CONSTANT (param/enum-label) or an
            // unknown symbol yields None → loud (a constant cannot wake a process).
            ast::ExprKind::PkgScoped { .. } => match self.pkg_scoped_var_net(e) {
                Some(n) => {
                    if self.is_dyn_handle_net(n) || self.is_string_net(n) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "a dynamic-storage handle cannot appear in an event control",
                        );
                    }
                    n
                }
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "an event control `@(pkg::name)` must name a package variable \
                         (not a constant / enum-label or an unknown symbol)",
                    );
                    POISON_NET
                }
            },
            ast::ExprKind::Paren { inner } => self.sens_event_net(inner, edge_ctx),
            ast::ExprKind::BitSelect { base, index } => {
                if edge_ctx {
                    if let Some(net) = self.lsb_bitselect_net(base, index) {
                        return net; // == the bare-ident net id: bit0 edge is exact
                    }
                    self.error(
                        MsgCode::ElabUnsupported,
                        "edge event-control bit-select must select the net's LSB with a constant \
                         index (non-LSB / part-select / variable index / array / packed need \
                         per-bit edge tracking)",
                    );
                } else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "single-bit level (non-edge) event control is not supported; use \
                         posedge/negedge or the whole signal (level fires on any whole-net change)",
                    );
                }
                POISON_NET
            }
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "event control must be a bare signal name or a constant LSB bit-select",
                );
                POISON_NET
            }
        }
    }

    // ── in-body @(...) / wait → WaitCause; #delay → (amount, region) ─
    /// In-body `@(...)` → ONE `WaitCause`. Single edge term → `Edge`; all bare →
    /// `Level`; multi-edge → ERROR (the frozen `WaitCause::Edge` carries one term;
    /// silently waiting on the FIRST term only changed wake semantics — P1-4).
    /// `@(*)` is handled by the `EventCtrl` arm (read-set patch), not here.
    pub(crate) fn lower_event_wait_cause(&mut self, ctrl: &ast::Sensitivity) -> ir::WaitCause {
        // N4: in-body `@(cb)` → the clocking event (`@(posedge clk)`).
        if let Some(ev) = self.clocking_event_subst(Some(ctrl)) {
            return self.lower_event_wait_cause(&ev);
        }
        match ctrl {
            ast::Sensitivity::Star => {
                unreachable!("in-body @(*) is lowered by the EventCtrl arm")
            }
            ast::Sensitivity::List(list) => {
                // Same rule as the process-header lane (`classify_event_list`), with
                // `armed_before_t0 = false`: this wait is reached AFTER the time-zero
                // settle, so a non-edge constant term fires never here too (measured,
                // both oracles). A term that can never wake is dropped before anything
                // else is decided, so an all-constant `@(…)` becomes `Level { nets: [] }`
                // — the shape an in-body `@(*)` with an empty read set already takes,
                // which the engine never wakes. Dropping FIRST also keeps a constant
                // term from counting toward the multi-term edge refusal below.
                let live: Vec<&ast::EventExpr> = list
                    .iter()
                    .filter(|ev| {
                        !self.event_term_never_wakes(ev, /* armed_before_t0 = */ false)
                    })
                    .collect();
                let n_edges = live
                    .iter()
                    .filter(|ev| !matches!(ev.edge, ast::Edge::NoEdge))
                    .count();
                if n_edges > 0 {
                    if live.len() > 1 {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "multi-term in-body edge wait is unsupported in v1 \
                             (the IR carries a single edge term; move it to a \
                             block-header sensitivity or split the wait)",
                        );
                    }
                    let ev = live
                        .iter()
                        .find(|ev| !matches!(ev.edge, ast::Edge::NoEdge))
                        .expect("n_edges>0 ⇒ at least one edge term");
                    ir::WaitCause::Edge {
                        net: self.sens_event_net(&ev.expr, true),
                        kind: map_edge(ev.edge),
                    }
                } else {
                    let nets = live
                        .iter()
                        .map(|ev| self.sens_event_net(&ev.expr, false))
                        .collect();
                    ir::WaitCause::Level { nets }
                }
            }
        }
    }

    /// `#delay` → `(amount, region)`. Since format_version 4 `amount` is the
    /// **ExprId of the raw delay value in module time units** — the engine
    /// evaluates it at suspension time and scales by the per-process
    /// multiplier (round(v × M); X/Z → 0, iverilog parity). A const `#5`
    /// simply folds to a Const expr, so const and runtime delays share one
    /// path. SD3: a delay that PROVABLY rounds to 0 ticks (`#0`, or a real
    /// under half a precision tick) marks `Inactive`; everything else —
    /// including runtime values that happen to be 0 — is `Active` and the
    /// engine's `ticks == 0` check supplies the inactive nudge at runtime.
    pub(crate) fn lower_delay(&mut self, d: &ast::Delay) -> (u32, ir::DelayRegion) {
        let mult = self.cur_time_mult;
        let Some(e) = d.values.first() else {
            // defensive: parser always supplies a value; treat as `#0`.
            let zero = self.lower_expr(&ast::Expr {
                kind: ast::ExprKind::IntLit {
                    kind: ast::IntLitKind::Decimal,
                    raw: "0".to_string(),
                },
                span: ast::Span { lo: 0, hi: 0 },
            });
            return (zero, ir::DelayRegion::Inactive);
        };
        // min:typ:max picks typ — same branch const_delay_ticks used.
        let pick = match &e.kind {
            ast::ExprKind::MinTypMax { typ, .. } => typ.as_ref(),
            _ => e,
        };
        // §2 "Delays / events" ⓓ: a delay holding a TIME LITERAL is folded in the
        // DELAY domain first — the same `delay_ticks_in_scope` the structural lanes
        // (`assign #(…)`, a net delay, a gate) have used since §4.5.458/459. Before
        // this, the procedural lanes — a statement `#(…)`, a statement prefix, an
        // intra-assign `=`/`<=`, a task body, `always`, a `fork` arm — went straight
        // to `lower_expr`, which reaches `const_eval_in_scope`'s `TimeLit` arm; that
        // arm declines a literal whose unit is finer than the design precision, one
        // with a real numerator, and one that is not a whole multiple of the module's
        // time unit, so `#(2500ps);` under `1ns/1ns` was E3009 where both oracles
        // delay 3 ns. Measured over 6 timescales × 14 literals: 59 false-loud cells,
        // and the STRUCTURAL twin of every one of them was already right.
        //
        // ⚠️ Asked BEFORE `lower_expr`, not as a fallback when it declines. On this
        // class the integer lane does not merely decline — it ANSWERS WRONG, at exit
        // 0: `#(3ns / 2)` under `1ns/1ns` folds integer division to one unit where
        // both oracles delay 2, and `#(5ns / 2ns)` two where both delay 2.5. Eight
        // such cells were silent-wrong, not loud, and a fallback-on-`None` patch
        // would leave every one of them. `delay_ticks_in_scope`'s own comment records
        // the identical ordering decision for the structural lane.
        //
        // ⚠️ Gated on `expr_has_time_lit`, so every delay without a time literal —
        // which is every delay in the existing corpus — keeps the old two lines
        // byte-for-byte, region included. That gate IS the opt-in: `#(ZERO_PARAM)`
        // stays `Active` with the engine's runtime `ticks == 0` nudge.
        //
        // The amount the IR wants is MODULE TIME UNITS (the engine scales by the
        // per-process multiplier), so the folded tick count is divided back by
        // `mult` — which reproduces the structural lane's answer exactly, two-stage
        // rounding and all, instead of re-deriving it here where the two lanes of one
        // language construct could drift apart. An integral quotient stays an integer
        // literal so the 20 already-correct cells do not change shape.
        //
        // ⚠️ NEGATIVE delays are excluded, and that exclusion is a measurement, not
        // caution: `#(1ns - 5ns)` under `1ns/1ns` never fires in iverilog, never
        // fires in verilator, and never fired in the procedural lane before this —
        // three tools agreeing. `delay_ticks_in_scope` returns a `u32` and
        // `real_delay_ticks` CLAMPS a negative amount to 0, so routing one through
        // here would fire it immediately: a cell that matched both oracles turned
        // into one that matches neither. The sign has to be read in the units
        // domain, before the clamp, and a negative one falls through to the path
        // that was already right. (ROADMAP §2 "Delays / events" ⓒ still holds for
        // the STRUCTURAL lane, which does fire at 0 — that row is untouched here.)
        if Self::expr_has_time_lit(pick)
            && self.delay_units_in_scope(pick).is_some_and(|u| u >= 0.0)
        {
            if let Some(ticks) = self.delay_ticks_in_scope(pick) {
                let m = mult.max(1);
                let raw = if u64::from(ticks) % m == 0 {
                    (u64::from(ticks) / m).to_string()
                } else {
                    // Shortest round-tripping decimal; `parse_real_f64` is
                    // `str::parse::<f64>`, so this reads back bit-identical.
                    format!("{}", f64::from(ticks) / m as f64)
                };
                let kind = if raw.contains('.') {
                    ast::ExprKind::RealLit {
                        kind: ast::RealLitKind::Fixed,
                        raw,
                    }
                } else {
                    ast::ExprKind::IntLit {
                        kind: ast::IntLitKind::Decimal,
                        raw,
                    }
                };
                let amount = self.lower_expr(&ast::Expr {
                    kind,
                    span: pick.span,
                });
                // A time literal that PROVABLY rounds to no ticks is `Inactive`, the
                // same rule `#0` and a sub-half-tick real already take.
                let region = if ticks == 0 {
                    ir::DelayRegion::Inactive
                } else {
                    ir::DelayRegion::Active
                };
                return (amount, region);
            }
        }
        let amount = self.lower_expr(pick);
        let region = if const_delay_ticks(pick, mult, self.cur_prec_mult) == Some(0) {
            ir::DelayRegion::Inactive
        } else {
            ir::DelayRegion::Active
        };
        (amount, region)
    }
}
