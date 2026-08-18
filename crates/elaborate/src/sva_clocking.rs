//! SVA clocking — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    // ── N7 class/OOP: registration + layout + resolution helpers ──────────────

    /// Whole-design class prescan: collect every `class … endclass` (top-level +
    /// module/package body), assign a stable `class_id`, resolve own fields, and
    /// record methods (lowered later). Then flatten the inheritance chain so a
    /// derived class's field list is `[base fields …, own fields …]` (base-first
    /// = field-id stability under up-cast). Forward-reference safe.
    /// N4: whole-design prescan of clocking-block NAMES (diagnostic only). Runs
    /// before any module lowers, so a cross-hierarchy `@(inst.cb)` event control —
    /// lowered in a PARENT before the child module's clocking block — is recognized
    /// for an accurate "unsupported clocking-event" message (not a generic
    /// hier-name error). Soundness is unaffected (the construct is loud either way).
    pub(crate) fn prescan_clocking_names(&mut self, unit: &ast::SourceUnit) {
        for it in &unit.items {
            if let ast::TopItem::Module(m) | ast::TopItem::Interface(m) | ast::TopItem::Package(m) =
                it
            {
                for bi in &m.body {
                    if let ast::ModuleItem::Clocking(cb) = bi {
                        if let Some(n) = &cb.name {
                            self.all_clocking_names.insert(n.name.clone());
                        }
                    }
                }
            }
        }
    }

    /// Multi-clock canonical pattern (slice A3): `@(c1) ante |=> @(c2) cons`. The
    /// (boolean) antecedent is sampled on c1 into a 1-bit handoff reg, and a SECOND
    /// synthesized process — clocked by c2 — consumes the handoff on its next c2 edge
    /// to check the (boolean) consequent:
    ///   always @(c1) handoff <= |ante;
    ///   always @(c2) if (handoff && !cons) $error(...);
    /// Pure IR-0 two-process synthesis (sim-ir untouched). The two sides use SEPARATE
    /// `SvaRegs`, so a `$past` in the antecedent samples on c1 and one in the
    /// consequent samples on c2 — each on its own clock, no cross-clock aliasing.
    ///
    /// TIE SEMANTICS (oracle-free, hand-IEEE pin): when c1 and c2 tick the same
    /// instant the c2 process reads the PRIOR-edge handoff (proc A's `handoff <= |ante`
    /// is an NBA that settles in the NBA region, after the c2 process's Active-region
    /// read). This is the conservative `|=>`-on-next-consume-edge reading; it is
    /// tool-divergent and unverifiable (iverilog rejects SVA), so it is pinned by a
    /// determinism test, not claimed IEEE-conformant.
    ///
    /// Everything outside the canonical shape is LOUD: `|->` with a consequent clock
    /// (parser), an OR-of-clocks / `@(*)` on either side, a multi-term sequence
    /// antecedent/consequent, and `disable iff` / a custom action block combined with
    /// a second clock (their sampling clock is ambiguous — deferred).
    pub(crate) fn synth_multiclock(&mut self, sva: PendingSva, sp: ast::Span) {
        let cons_clock = sva.cons_clock.clone().expect("cons_clock is Some");
        // `|=>` only (the parser only attaches a consequent clock to `|=>`).
        if !matches!(sva.kind, ast::ImplicationKind::NonOverlap) {
            self.error(
                MsgCode::ElabUnsupported,
                "a consequent clocking event requires `|=>` (non-overlapping implication)",
            );
            return;
        }
        // The consequent clock must be a single edge-event (no OR-of-clocks / `@(*)`).
        let c2_single = matches!(&cons_clock, ast::Sensitivity::List(evs) if evs.len() == 1);
        if !c2_single {
            self.error(
                MsgCode::ElabUnsupported,
                "the consequent clocking event must be a single edge (an OR-of-clocks / \
                 `@(*)` consequent clock is unsupported in this subset)",
            );
            return;
        }
        // v1 restricts both sides to a boolean (a multi-term sequence across two clocks
        // is deferred).
        let ast::Sequence::Boolean(ante_e) = &sva.ante else {
            self.error(
                MsgCode::ElabUnsupported,
                "a multi-clock property's antecedent must be a boolean in this subset \
                 (a sequence antecedent with a consequent clock is deferred)",
            );
            return;
        };
        let ast::Sequence::Boolean(cons_e) = &sva.cons else {
            self.error(
                MsgCode::ElabUnsupported,
                "a multi-clock property's consequent must be a boolean in this subset \
                 (a sequence consequent under a second clock is deferred)",
            );
            return;
        };
        // `disable iff` / a custom action block with a second clock is deferred: the
        // sampling/reset clock becomes ambiguous across the two processes.
        if sva.disable_iff.is_some() || sva.pass.is_some() || sva.fail.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "`disable iff` / a custom action block combined with a consequent clock \
                 is unsupported in this subset",
            );
            return;
        }

        // 1-bit handoff reg (X-init like `pend`, so no spurious fire before c1 ticks).
        let handoff = self.fresh_sva_reg(1, "mc_pend");

        let handoff_path = ast::HierPath {
            segments: vec![ast::Ident {
                name: handoff.clone(),
                span: sp,
            }],
            span: sp,
        };

        // PROCESS A @ c1: SET-only `if (|ante) handoff <= 1'b1;` (the `|=>` obligation
        // persists until the CONSUMER discharges it — a later `ante`=0 c1 edge must
        // NOT clear a pending obligation), plus the antecedent's prev-reg NBAs (c1).
        // (review 2026-06-16: a level-held `handoff <= |ante` re-fired on every c2
        // edge in the window when c2 is faster than c1, and dropped an obligation on a
        // c1 edge with ante=0.)
        let mut regs_a = SvaRegs::default();
        let ante_b = self.rewrite_sampled(ante_e, &mut regs_a);
        let set_handoff = ast::Stmt::If {
            cond: sva_unary(ast::UnOp::RedOr, ante_b, sp),
            then_s: Box::new(ast::Stmt::NonBlocking {
                lhs: ast::Lvalue::Ident(handoff_path.clone()),
                delay: None,
                event: None,
                rhs: sva_one(sp),
                span: sp,
            }),
            else_s: None,
            span: sp,
        };
        let mut body_a = vec![set_handoff];
        body_a.extend(regs_a.nbas);
        let proc_a = self.lower_proc_block(&ast::ProceduralBlock {
            kind: ast::ProcKind::Always,
            sensitivity: Some(sva.clock.clone()),
            body: Box::new(sva_block_or_single(body_a, sp)),
            span: sp,
        });
        self.push_process(proc_a);

        // PROCESS B @ c2: CHECK + DISCHARGE — `if (handoff) begin if (!cons)
        // $error(...); handoff <= 1'b0; end` — so each match is consumed at EXACTLY
        // ONE c2 edge (single-shot obligation, not a level flag), plus the
        // consequent's prev-reg NBAs (sampled on c2). The X-init handoff keeps
        // `if (handoff)` from firing/discharging before the first c1 match.
        let mut regs_b = SvaRegs::default();
        let cons_b = self.rewrite_sampled(cons_e, &mut regs_b);
        let fire = ast::Stmt::If {
            // §16.13.5: consequent X/Z = non-match → fire (sva_match makes X a hard 0).
            cond: sva_unary(ast::UnOp::LogNot, sva_match(cons_b, sp), sp),
            then_s: Box::new(ast::Stmt::SysTaskCall {
                name: ast::Ident {
                    name: "$error".to_string(),
                    span: sp,
                },
                args: vec![ast::Expr {
                    kind: ast::ExprKind::StrLit {
                        raw: "\"Assertion property violation\"".to_string(),
                    },
                    span: sp,
                }],
                span: sp,
            }),
            else_s: None,
            span: sp,
        };
        let discharge = ast::Stmt::NonBlocking {
            lhs: ast::Lvalue::Ident(handoff_path),
            delay: None,
            event: None,
            rhs: sva_zero(sp),
            span: sp,
        };
        let consume = ast::Stmt::If {
            cond: sva_ident_expr(&handoff, sp),
            then_s: Box::new(ast::Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts: vec![fire, discharge],
                span: sp,
            }),
            else_s: None,
            span: sp,
        };
        let mut body_b = vec![consume];
        body_b.extend(regs_b.nbas);
        let proc_b = self.lower_proc_block(&ast::ProceduralBlock {
            kind: ast::ProcKind::Always,
            sensitivity: Some(cons_clock),
            body: Box::new(sva_block_or_single(body_b, sp)),
            span: sp,
        });
        self.push_process(proc_b);
    }

    /// Cross-clock SEQUENCE antecedent (slices N2a-1, N2a-2): a `##1`-connected chain
    /// of re-clocked booleans `@(c1) s0 ##1 @(c2) s1 ##1 @(c3) s2 … |-> c` (or `|=>`).
    /// Unlike A3 (`synth_multiclock`), where the IMPLICATION crosses clocks, here the
    /// cross-clock boundaries are INSIDE the antecedent (each `##1 @(ck)`). Each segment
    /// is sampled on its own clock and a 1-bit handoff carries the partial match forward:
    /// segment k's clock arms `hf[k]`, and the FIRST edge of segment k+1's clock consumes
    /// it (the exact one-cycle `##1` window). When the final segment matches, the OVERLAP
    /// `|->` checks `c` on that SAME final edge; the NON-OVERLAP `|=>` checks it on the
    /// NEXT edge of the final clock (one more handoff stage `hf_final`). N2a-1 was the
    /// two-segment overlap case; N2a-2 generalizes to N segments + `|=>`.
    ///
    ///   always @(c1) if (|s0) hf0 <= 1'b1;                 // arm seg 0 (set-only)
    ///   always @(c2) if (hf0) begin                         // first c2 after
    ///     if (|s1) hf1 <= 1'b1; hf0 <= 1'b0; end            //   advance / discharge
    ///   … one process per `##1` boundary …
    ///   always @(cN) if (hf_{N-2}) begin                    // final segment, |->
    ///     if (|s_{N-1} && !c) $error(...); hf_{N-2} <= 1'b0; end
    ///
    /// Pure IR-0. Each process carries its own `SvaRegs` (per-clock `$past` sampling).
    ///
    /// TIE / COUNT FIDELITY (oracle-free hand-IEEE pins, as N2a-1): a coincident
    /// boundary edge reads the PRIOR-edge handoff (the NBA settles after the Active
    /// read) — the conservative "advance to the next DISTINCT edge" reading of `##1`;
    /// and each handoff is a single bit, so several attempts due on the same boundary
    /// edge merge into ONE report (VERDICT-SAFE — they sample identical downstream
    /// booleans, so only duplicate reports are merged, never a missed/spurious verdict).
    /// A redundant same-as-property-clock re-clock is folded upstream by
    /// `strip_redundant_clocks`.
    ///
    /// LOUD (deferred / unsupported): a MULTI-TERM segment (a `##1` operand that is not
    /// a single re-clocked boolean — N2a-2 multi-term lane), a non-`##1` connector
    /// (`##0` across distinct clocks is illegal §16.13.4; `##n` n>1 deferred), an
    /// OR-of-clocks edge, an explicit consequent clock, or `disable iff` / a custom
    /// action.
    pub(crate) fn synth_crossclock(&mut self, sva: PendingSva, sp: ast::Span) {
        let c1 = sva.clock.clone();
        if sva_clock_signal(&c1).is_none() {
            self.error(
                MsgCode::ElabUnsupported,
                "a cross-clock property's leading clock must be a single edge in this subset \
                 (an OR-of-clocks / `@(*)` boundary is unsupported)",
            );
            return;
        }
        // Flatten the `##1`-connected chain into ordered (clock, boolean) segments.
        let Some(segs) = self.collect_xclock_segments(&sva.ante, &c1) else {
            return; // a specific loud diagnostic was already emitted
        };
        let n = segs.len();
        if n < 2 {
            // Reached only via `seq_has_clocked` (a Clocked node exists), so a single
            // segment means a degenerate shape the flattener could not split.
            self.error(
                MsgCode::ElabUnsupported,
                "a cross-clock antecedent must contain at least one `##1 @(ck)` boundary",
            );
            return;
        }
        if sva.cons_clock.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "an explicit consequent clock combined with a cross-clock antecedent is unsupported",
            );
            return;
        }
        let ast::Sequence::Boolean(c_e) = &sva.cons else {
            self.error(
                MsgCode::ElabUnsupported,
                "a cross-clock property's consequent must be a boolean in this subset",
            );
            return;
        };
        if sva.disable_iff.is_some() || sva.pass.is_some() || sva.fail.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "`disable iff` / a custom action block combined with a cross-clock antecedent \
                 is unsupported in this subset",
            );
            return;
        }
        let c_e = c_e.clone();
        let is_overlap = matches!(sva.kind, ast::ImplicationKind::Overlap);

        // One handoff per `##1` boundary (`hf[i]` = "segments 0..=i matched, awaiting
        // segment i+1's clock"); the `|=>` form adds `hf_final` to delay the consequent
        // check one final-clock edge. X-init = no fire before the first arm.
        let handoffs: Vec<String> = (0..n - 1)
            .map(|_| self.fresh_sva_reg(1, "cc_pend"))
            .collect();
        let hf_final = if is_overlap {
            None
        } else {
            Some(self.fresh_sva_reg(1, "cc_pend"))
        };

        // Segment 0 @ its (property) clock: SET-only arm of hf[0] (the obligation
        // persists until consumed — a later arm-clock edge with s0=0 must NOT clear a
        // pending handoff; the A3 set-only lesson).
        {
            let mut regs = SvaRegs::default();
            // A multi-term segment-0 (`(a ##1 b)`) expands to its own shift-register
            // pipeline; the pipeline NBAs MUST land in THIS `always @(c1)` block so the
            // shift regs clock on c1. A lone Boolean reduces to the old `|s0` raw expr
            // (synth_seq_match's len==1 fast-path) → byte-identical to the old path.
            let mut pipeline_nbas: Vec<ast::Stmt> = Vec::new();
            let s0 = self.synth_seq_match(&segs[0].1, &mut regs, &mut pipeline_nbas, sp);
            let arm = ast::Stmt::If {
                cond: sva_unary(ast::UnOp::RedOr, s0, sp),
                then_s: Box::new(sva_nb_set(&handoffs[0], true, sp)),
                else_s: None,
                span: sp,
            };
            let mut body = vec![arm];
            body.extend(regs.nbas);
            body.extend(pipeline_nbas);
            let proc = self.lower_proc_block(&ast::ProceduralBlock {
                kind: ast::ProcKind::Always,
                sensitivity: Some(segs[0].0.clone()),
                body: Box::new(sva_block_or_single(body, sp)),
                span: sp,
            });
            self.push_process(proc);
        }

        // Segments 1..n: one process per `##1` boundary, @ segment i's clock, consuming
        // hf[i-1]. Intermediate boundaries advance to hf[i]; the final segment completes
        // the antecedent (overlap → check `c` now; |=> → arm hf_final, check next edge).
        for i in 1..n {
            let mut regs = SvaRegs::default();
            // Multi-term segment i (`@(ck)(c ##1 e)`): its shift-register pipeline NBAs
            // go into `pipeline_nbas`, appended to THIS `always @(seg[i].clock)` block so
            // the shift regs clock on seg i's clock (a c2-only pipeline advances on c2
            // edges only). A lone Boolean reduces to the old `|si` raw expr (byte-id).
            let mut pipeline_nbas: Vec<ast::Stmt> = Vec::new();
            let si = self.synth_seq_match(&segs[i].1, &mut regs, &mut pipeline_nbas, sp);
            let prev = handoffs[i - 1].clone();
            let is_last = i == n - 1;
            let mut body: Vec<ast::Stmt> = Vec::new();
            // For the |=> final segment, the hf_final consume (the PRIOR edge's
            // completion) runs FIRST so it reads the prior arm before this edge re-arms
            // hf_final (the same check-then-arm ordering as the single-clock |=> pend).
            if is_last && !is_overlap {
                let hf = hf_final.clone().unwrap();
                let c_b = self.rewrite_sampled(&c_e, &mut regs);
                body.push(ast::Stmt::If {
                    cond: sva_ident_expr(&hf, sp),
                    then_s: Box::new(sva_block_or_single(
                        vec![
                            ast::Stmt::If {
                                // §16.13.5: consequent X/Z = non-match → fire.
                                cond: sva_unary(ast::UnOp::LogNot, sva_match(c_b, sp), sp),
                                then_s: Box::new(sva_error_stmt(sp)),
                                else_s: None,
                                span: sp,
                            },
                            sva_nb_set(&hf, false, sp),
                        ],
                        sp,
                    )),
                    else_s: None,
                    span: sp,
                });
            }
            // Consume body of hf[i-1].
            let consume_body: Vec<ast::Stmt> = if !is_last {
                // intermediate: advance to hf[i] on a match, then discharge hf[i-1].
                vec![
                    ast::Stmt::If {
                        cond: sva_unary(ast::UnOp::RedOr, si, sp),
                        then_s: Box::new(sva_nb_set(&handoffs[i], true, sp)),
                        else_s: None,
                        span: sp,
                    },
                    sva_nb_set(&prev, false, sp),
                ]
            } else if is_overlap {
                // antecedent completes on this edge ⇒ overlap `|->` checks `c` now.
                let c_b = self.rewrite_sampled(&c_e, &mut regs);
                vec![
                    ast::Stmt::If {
                        cond: sva_binary(
                            ast::BinOp::LogAnd,
                            sva_unary(ast::UnOp::RedOr, si, sp),
                            // §16.13.5: consequent X/Z = non-match → fire.
                            sva_unary(ast::UnOp::LogNot, sva_match(c_b, sp), sp),
                            sp,
                        ),
                        then_s: Box::new(sva_error_stmt(sp)),
                        else_s: None,
                        span: sp,
                    },
                    sva_nb_set(&prev, false, sp),
                ]
            } else {
                // |=>: antecedent completes ⇒ arm hf_final; the consequent is checked at
                // the NEXT final-clock edge by the hf_final consume prepended above.
                vec![
                    ast::Stmt::If {
                        cond: sva_unary(ast::UnOp::RedOr, si, sp),
                        then_s: Box::new(sva_nb_set(hf_final.as_ref().unwrap(), true, sp)),
                        else_s: None,
                        span: sp,
                    },
                    sva_nb_set(&prev, false, sp),
                ]
            };
            body.push(ast::Stmt::If {
                cond: sva_ident_expr(&prev, sp),
                then_s: Box::new(sva_block_or_single(consume_body, sp)),
                else_s: None,
                span: sp,
            });
            body.extend(regs.nbas);
            body.extend(pipeline_nbas);
            let proc = self.lower_proc_block(&ast::ProceduralBlock {
                kind: ast::ProcKind::Always,
                sensitivity: Some(segs[i].0.clone()),
                body: Box::new(sva_block_or_single(body, sp)),
                span: sp,
            });
            self.push_process(proc);
        }
    }

    /// Flatten a cross-clock antecedent — a left-nested `##1` chain — into ordered
    /// `(clock, sequence)` segments (slice A.2: a segment may now be MULTI-TERM, e.g.
    /// `@(c1)(a ##1 b) ##1 @(c2)(d ##1 e)`). A clock boundary is a top-spine `##1`
    /// whose RIGHT operand is `@(ck) seq` — the segment Sequence is `seq` at `ck`. The
    /// leftmost residue (the first non-clock-boundary subtree) is segment 0 at the
    /// property clock `c1`. Each segment Sequence is expanded by `synth_seq_match` into
    /// its own shift-register pipeline.
    ///
    /// Emits a specific loud diagnostic and returns `None` for: a non-`##1` connector
    /// across a boundary, a NESTED re-clock inside one segment (`@(c2)(c ##1 @(c3) d)`
    /// — a 4th clock boundary, kept LOUD), or an OR-of-clocks edge.
    pub(crate) fn collect_xclock_segments(
        &mut self,
        seq: &ast::Sequence,
        c1: &ast::Sensitivity,
    ) -> Option<Vec<(ast::Sensitivity, ast::Sequence)>> {
        // A top-spine `##1` whose RHS is `@(ck) inner` is a clock boundary: peel it,
        // recurse on the LHS for the preceding chain, then append `(ck, inner)`.
        if let ast::Sequence::Delay { min, max, lhs, rhs } = seq {
            if let ast::Sequence::Clocked { clock, seq: inner } = &**rhs {
                if *min != 1 || *max != Some(1) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "only `##1` connects a cross-clock sequence boundary in this subset \
                         (`##0` across distinct clocks is illegal §16.13.4; `##n` n>1 is deferred)",
                    );
                    return None;
                }
                let mut segs = self.collect_xclock_segments(lhs, c1)?;
                // A NESTED re-clock inside the segment (`@(c2)(c ##1 @(c3) d)`) is a
                // 4th clock boundary — kept LOUD (the segment pipeline is single-clock).
                if seq_has_clocked(inner) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a re-clocked cross-clock segment must not itself contain a nested \
                         `@(ck)` re-clock in this subset (a 4th clock boundary is deferred)",
                    );
                    return None;
                }
                if sva_clock_signal(clock).is_none() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a cross-clock segment clock must be a single edge in this subset \
                         (an OR-of-clocks / `@(*)` boundary is unsupported)",
                    );
                    return None;
                }
                segs.push((clock.clone(), (**inner).clone()));
                return Some(segs);
            }
        }
        // Not a clock-boundary `##1`: the WHOLE subtree is the leftmost segment 0 at the
        // property clock `c1`. It may be multi-term (`(a ##1 b)`) — a same-clock `##1`
        // stays inside the segment. But a `@(ck)` re-clock buried here without a `##1`
        // boundary at the top spine (e.g. `a ##1 @(c2) b ##1 e` → outer rhs `e` is not
        // clocked, so the whole `(a ##1 @(c2) b ##1 e)` lands here) is a same-clock
        // multi-term segment that swallows a re-clock → LOUD.
        if seq_has_clocked(seq) {
            self.error(
                MsgCode::ElabUnsupported,
                "each cross-clock segment after a `##1` must be re-clocked `@(ck) seq` \
                 in this subset (a same-clock multi-term segment that swallows a re-clock \
                 is deferred — the trailing operand must be re-clocked)",
            );
            return None;
        }
        Some(vec![(c1.clone(), seq.clone())])
    }

    /// N4 (§14): lower each `clocking` block in `body` into preponed-sampled
    /// holding nets + a marked commit handler, and register `@(cb)` events. Runs
    /// AFTER the net passes (source nets resolve) and BEFORE the process loop (so
    /// `cb.sig` resolves to its holding net and `@(cb)` to the clocking event).
    ///
    /// v1 = default-skew INPUT sampling + `@(cb)`. Each input gets a holding net
    /// (clean VCD name `__clk_<cb>_<sig>`) plus an alias symbol `cb.sig` so reads
    /// resolve to it (the interface-alias path in `resolve_net`). A synthesized
    /// `always @(clk);` handler is marked in `clocking_commit`; the engine, on the
    /// clocking edge, commits `preponed_buf[source] → holding` (blocking, same-slot
    /// — no NBA lag). HONEST-LOUD (follow-on slices): output/inout DRIVERS (need the
    /// Observed/Reactive region), explicit skews, multi-clock / anonymous blocks,
    /// and a non-net-reference input bind.
    pub(crate) fn lower_clocking_blocks(&mut self, body: &[ast::ModuleItem]) {
        // `@(cb)` resolution is module-local: clear the previous module's map.
        self.clocking_events.clear();
        self.default_clocking = None;
        self.default_disable_iff = None;
        // §16.15: scope-level default reset. Recorded in the same pass and cleared with
        // the same reset, so the two scope defaults can never disagree about which
        // module they belong to.
        for item in body {
            if let ast::ModuleItem::DefaultDisableIff(e) = item {
                // §16.15 allows exactly ONE per scope, and verilator says so out loud
                // ("Only one 'default disable iff' allowed per module"). Silently
                // keeping one of two would make which reset applies depend on
                // declaration order, and neither choice is defensible.
                if self.default_disable_iff.is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a scope may declare only one `default disable iff` \
                         (IEEE 1800-2017 §16.15); this module declares a second",
                    );
                    continue;
                }
                // ⚠️ With that guard above, "first wins" and "last wins" are now
                // PROVABLY the same rule — this line is reachable only while the slot
                // is None. The mutation battery reached that conclusion the other way
                // round: swapping to `get_or_insert` survived, and the only design that
                // could tell them apart is the duplicate this scope now refuses.
                self.default_disable_iff = Some(e.clone());
            }
        }
        for item in body {
            let ast::ModuleItem::Clocking(cb) = item else {
                continue;
            };
            // The clocking event must be an edge list `@(posedge c1 [or posedge c2…])`
            // — one OR MORE edge terms (YELLOW #2: multi-event clock). `@*`, an empty
            // list, or any non-edge (level) term stays loud. The engine arms the
            // commit handler on every listed edge and (with the multi-edge dedup in
            // `propagate_changes`) ticks exactly once per slot even on simultaneous
            // edges, so the preponed sample / `@(cb)` fire once.
            let clock_ok = matches!(&cb.clock, ast::Sensitivity::List(evs)
                if !evs.is_empty() && evs.iter().all(|e| !matches!(e.edge, ast::Edge::NoEdge)));
            if !clock_ok {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a clocking block clocking event must be one or more edges \
                     (`@(posedge c1 [or posedge c2 …])`; `@*` / level events are unsupported)",
                );
                continue;
            }
            // IEEE 1800 §14.12: `default clocking` supplies the clocking event to any
            // concurrent assertion in this scope that gives none. Recorded only AFTER the
            // edge-list check above, so an unsupported clocking event cannot become an
            // invisible default.
            //
            // ⚠️ MEASURED EQUIVALENT TODAY, KEPT FAIL-CLOSED. Moving this above the check
            // changes nothing observable, because the rejection there is an ERROR: the
            // design does not simulate, so no assertion armed with the bad clock can run
            // (mutation battery, measured — `default clocking cb @(clk);` gives errors=1
            // and no output either way). The ordering is what stays correct if that
            // rejection is ever downgraded to a warning, which is exactly when an
            // invisible default would become a silent wrong answer.
            if cb.is_default {
                self.default_clocking = Some(cb.clock.clone());
            }
            let cb_name = match cb.name.as_ref().map(|n| n.name.clone()) {
                Some(n) => n,
                None => {
                    // Anonymous block: synthesize internal name. No `cb.sig` alias
                    // (no name = no user-visible prefix). Preponed infrastructure is
                    // still synthesized for future program-block / default-clocking use.
                    let n = format!("__anon_clk_{}", self.anon_clocking_count);
                    self.anon_clocking_count += 1;
                    n
                }
            };
            // `@(cb)` → the clocking event (module-local). Also record the name
            // design-globally (diagnostic: cross-hier `@(inst.cb)` message).
            self.clocking_events
                .insert(cb_name.clone(), cb.clock.clone());
            self.all_clocking_names.insert(cb_name.clone());
            let mut pairs: Vec<(u32, u32)> = Vec::new();
            let mut out_pairs: Vec<(u32, u32)> = Vec::new(); // (source_net, holding_net)
            for it in &cb.items {
                if let Some(skew) = &it.skew_raw {
                    let s = skew.trim();
                    if s != "#1step" {
                        // Name the SIGNAL: a clocking block has many items and a
                        // block-wide `default` puts one written skew on several
                        // of them, so "which one" is not recoverable from the
                        // source by reading. And name the accepted spelling as
                        // PER-SIGNAL — saying "the explicit default" next to a
                        // rejected `default …;` item reads as an instruction to
                        // write the thing that was just refused.
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "clocking skew `{s}` on `{}` is unsupported in this subset \
                                 (`#1step` is the only accepted skew, written per signal or \
                                 in a `default input #1step;` item; `#0`/`#N`/`##N` need a \
                                 different sampling region — follow-on slice)",
                                it.name.name
                            ),
                        );
                        continue;
                    }
                    // `#1step` IS the default input skew (preponed value entering the slot).
                    // No change to the holding-net synthesis — fall through.
                }
                match it.dir {
                    ast::ClockingDir::Input => {} // INPUT: fall through to holding-net synthesis below
                    ast::ClockingDir::Output => {
                        // OUTPUT: synthesize a WRITABLE holding reg + drive source at each edge.
                        // Simplified synchronous model: `source = holding` in Active region at edge.
                        // (No Observed/Reactive region — hand-IEEE, covers common TB patterns.)
                        let src_name = match &it.expr {
                            None => it.name.name.clone(),
                            Some(e) => match &e.kind {
                                ast::ExprKind::Ident(p) => p
                                    .segments
                                    .iter()
                                    .map(|s| s.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join("."),
                                _ => {
                                    self.error(
                                        MsgCode::ElabUnsupported,
                                        "a clocking output bind must be a net reference in this subset",
                                    );
                                    continue;
                                }
                            },
                        };
                        let Some(src_id) = self.lookup_net_scoped(&src_name) else {
                            self.error(
                                MsgCode::ElabUnresolvedName,
                                &format!(
                                    "undeclared clocking output signal `{}`",
                                    self.fq(&src_name)
                                ),
                            );
                            continue;
                        };
                        // A2a: the clocking-output commit WRITES the source net
                        // engine-side (a sidecar, never an lvalue) — a desugared
                        // array parameter source must stay loud (adversarial
                        // find: `cb.R <= v` silently overwrote the parameter).
                        self.deny_const_param_write(src_id, "drive (clocking output)");
                        let sv = &self.nets[src_id as usize];
                        let (w, msb, lsb, signed) = (sv.width, sv.msb, sv.lsb, sv.signed);
                        let clean = format!("__clkout_{}_{}", cb_name, it.name.name);
                        let nv = ir::NetVar {
                            kind: ir::NetKind::Reg,
                            width: w,
                            msb,
                            lsb,
                            signed,
                            array_len: 1,
                            dir: ir::PortDir::Internal,
                            init: default_init(ast::NetVarKind::Reg, w), // X-init
                        };
                        let hid_before = self.nets.len() as u32;
                        self.add_net(&clean, nv);
                        let hid = self.lookup_net_scoped(&clean).unwrap_or(hid_before);
                        // Alias `cb.sig` → holding net (writable — NOT added to clocking_hold_nets).
                        let alias = self.fq(&format!("{}.{}", cb_name, it.name.name));
                        self.symbols.insert(alias, hid);
                        // Store output pair for this block's commit proc.
                        out_pairs.push((src_id, hid)); // (source_net, holding_net)
                        continue;
                    }
                    ast::ClockingDir::Inout => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "a clocking INOUT driver is unsupported in this subset",
                        );
                        continue;
                    }
                }
                // Source net = the bind expr (a net reference) or the signal name.
                let src_name = match &it.expr {
                    None => it.name.name.clone(),
                    Some(e) => match &e.kind {
                        ast::ExprKind::Ident(p) => p
                            .segments
                            .iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>()
                            .join("."),
                        _ => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a clocking input bind must be a net reference in this subset",
                            );
                            continue;
                        }
                    },
                };
                let Some(src_id) = self.lookup_net_scoped(&src_name) else {
                    self.error(
                        MsgCode::ElabUnresolvedName,
                        &format!("undeclared clocking input signal `{}`", self.fq(&src_name)),
                    );
                    continue;
                };
                // Holding reg: same width/sign as the source, X-init (cb.sig is X
                // before the first clocking edge — iverilog parity).
                let sv = &self.nets[src_id as usize];
                let (w, msb, lsb, signed) = (sv.width, sv.msb, sv.lsb, sv.signed);
                let clean = format!("__clk_{}_{}", cb_name, it.name.name);
                let nv = ir::NetVar {
                    kind: ir::NetKind::Reg,
                    width: w,
                    msb,
                    lsb,
                    signed,
                    array_len: 1,
                    dir: ir::PortDir::Internal,
                    init: default_init(ast::NetVarKind::Reg, w),
                };
                let hid_before = self.nets.len() as u32;
                self.add_net(&clean, nv);
                let hid = self.lookup_net_scoped(&clean).unwrap_or(hid_before);
                // Alias `cb.sig` → the holding net (resolution only; the net keeps its
                // clean VCD name). Manual insert mirrors interface-member aliasing.
                let alias = self.fq(&format!("{}.{}", cb_name, it.name.name));
                self.symbols.insert(alias, hid);
                pairs.push((hid, src_id));
                self.clocking_inputs.insert(src_id);
                self.clocking_hold_nets.insert(hid); // read-only: a write is loud
            }
            if pairs.is_empty() && out_pairs.is_empty() {
                continue; // no valid items in this clocking block
            }
            // Marked commit handler `always @(clk);` (Null body — the engine does the
            // preponed→holding commit when this proc fires on the clocking edge).
            let pb = ast::ProceduralBlock {
                kind: ast::ProcKind::Always,
                sensitivity: Some(cb.clock.clone()),
                body: Box::new(ast::Stmt::Null(cb.span)),
                span: cb.span,
            };
            let proc = self.lower_proc_block(&pb);
            let pid = self.processes.len() as u32;
            self.push_process(proc);
            if !pairs.is_empty() {
                self.clocking_commit.insert(pid, pairs);
            }
            if !out_pairs.is_empty() {
                self.clocking_outputs.insert(pid, out_pairs);
            }
        }
    }

    /// If `sens` is a bare `@(cb)` whose `cb` names a clocking block (registered by
    /// `lower_clocking_blocks`), return the clocking event to substitute. `None`
    /// for any other sensitivity (a normal signal/edge list passes through).
    pub(crate) fn clocking_event_subst(
        &self,
        sens: Option<&ast::Sensitivity>,
    ) -> Option<ast::Sensitivity> {
        let ast::Sensitivity::List(l) = sens? else {
            return None;
        };
        if l.len() != 1 || !matches!(l[0].edge, ast::Edge::NoEdge) {
            return None;
        }
        let ast::ExprKind::Ident(p) = &l[0].expr.kind else {
            return None;
        };
        if p.segments.len() != 1 {
            return None;
        }
        self.clocking_events.get(&p.segments[0].name).cloned()
    }

    /// Require both operands of a property-level `and`/`or` to share a clock skew;
    /// otherwise loud-reject (combining a skew-1 `|=>` with a skew-0 same-clock
    /// operand would pair two different attempt-start clocks — review N2d).
    pub(crate) fn unify_prop_skew(&mut self, sl: u32, sr: u32) -> Option<u32> {
        if sl != sr {
            self.error(
                MsgCode::ElabUnsupported,
                "the operands of a property-level `and`/`or` have different clock \
                 skews (a `|=>` operand mixed with a same-clock operand) — combine \
                 only same-skew operands (all `|=>`, or all `|->`/boolean) in this \
                 subset",
            );
            return None;
        }
        Some(sl)
    }
}
