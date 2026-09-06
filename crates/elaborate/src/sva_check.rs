//! SVA checker materialization — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// Synthesize a fresh `Reg` for an SVA helper (`|=>` pending bit / sampled-
    /// value prev register), returning its name so the synthesized checker AST
    /// can reference it by `Ident`. Mirrors `fresh_ia_tmp`. Init is the default
    /// `Reg` X — an X helper makes the first-clock `if (pend && …)` / `$rose`
    /// false, so there is no spurious violation before any value has been sampled.
    pub(crate) fn fresh_sva_reg(&mut self, width: u32, tag: &str) -> String {
        let w = width.max(1);
        let name = format!("__sva_{tag}_{}", self.nets.len());
        let nv = ir::NetVar {
            kind: ir::NetKind::Reg,
            width: w,
            msb: w.saturating_sub(1),
            lsb: 0,
            signed: false,
            array_len: 1,
            dir: ir::PortDir::Internal,
            init: default_init(ast::NetVarKind::Reg, w),
        };
        self.add_net(&name, nv);
        name
    }

    /// A fresh ZERO-initialized SVA reg (liveness `pend`, SVA-REST). Like
    /// `fresh_sva_reg` but 0-init (`fresh_sva_reg`'s X-init would make a never-armed
    /// `pend` poison the end-of-sim `if (pend)` check), mirroring `fresh_cover_counter`.
    pub(crate) fn fresh_sva_reg0(&mut self, width: u32, tag: &str) -> String {
        let w = width.max(1);
        let name = format!("__sva_{tag}_{}", self.nets.len());
        let nwords = w.div_ceil(64).max(1) as usize;
        let nv = ir::NetVar {
            kind: ir::NetKind::Reg,
            width: w,
            msb: w.saturating_sub(1),
            lsb: 0,
            signed: false,
            array_len: 1,
            dir: ir::PortDir::Internal,
            init: ir::BitPacked {
                val: vec![0; nwords],
                unk: vec![0; nwords],
            },
        };
        self.add_net(&name, nv);
        name
    }

    /// v8 SVA: drain `pending_sva` into synthesized clocked checker processes.
    /// `assert property(@(clk) ante |-> cons)` ≡ `always @(clk) if (ante &&
    /// !cons) $error(...)`. For `|=>` (non-overlapping) the antecedent is delayed
    /// one clock through a pending reg: `always @(clk) begin if (pend && !cons)
    /// $error(...); pend <= ante; end` — the check reads the PRIOR clock's
    /// antecedent, then samples this clock's. The `$error` reuses the immediate-
    /// assert severity shape (routes to the diagnostic stream + exit class 1).
    pub(crate) fn materialize_sva_checkers(&mut self) {
        let pending = std::mem::take(&mut self.pending_sva);
        // SVA-REST: while a checker body lowers, every fire `$error`'s StmtId is
        // captured into `assert_fire` (so `$assertoff`/`$assertkill` can suppress it).
        // Scoped to this whole pass — it dispatches to synth_prop_expr/liveness/multi/
        // crossclock, which all emit fires. Cover (a separate pass) keeps it false.
        let saved_synth = self.in_assert_synth;
        self.in_assert_synth = true;
        for sva in pending {
            // Resolve the assertion's names in the scope it was written in.
            let saved_prefix = std::mem::replace(&mut self.cur_prefix, sva.scope.clone());
            self.materialize_one_sva(sva);
            self.cur_prefix = saved_prefix;
        }
        self.in_assert_synth = saved_synth;
    }

    /// One pending concurrent assertion → its synthesized clocked checker (the
    /// body of `materialize_sva_checkers`'s loop; an early `return` is a skipped
    /// assertion whose diagnostic was already emitted).
    fn materialize_one_sva(&mut self, mut sva: PendingSva) {
        // A concurrent assertion with NO clocking event of its own inherits the
        // scope's `default clocking` (IEEE 1800 §14.12). The empty sensitivity list
        // is the sentinel the parser leaves; this is the ONLY place that resolves it,
        // so the multi-clock gate below never sees it. `self.default_clocking` is the
        // CURRENT module's — `lower_clocking_blocks` set it at step (6.5) and this
        // drain runs later in the same `elaborate_instance`.
        // §16.15: an assertion with no `disable iff` of its own inherits the
        // scope's. Applied beside the clock default because it is the same shape of
        // rule and the same drain — and BEFORE the multi-clock gate, so a default
        // reset can never be silently dropped by an early `continue`.
        if sva.disable_iff.is_none() {
            sva.disable_iff = self.default_disable_iff.clone();
        }
        if matches!(&sva.clock, ast::Sensitivity::List(evs) if evs.is_empty()) {
            match self.default_clocking.clone() {
                Some(c) => sva.clock = c,
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a concurrent assertion needs a clocking event: write one \
                         (`assert property (@(posedge clk) …)`) or declare a \
                         `default clocking cb @(posedge clk); endclocking` in this scope",
                    );
                    return;
                }
            }
        }
        let sp = sva.span;
        // A concurrent assertion must have a SINGLE clocking event (slice
        // S15). An OR-of-clocks event `@(posedge c1 or posedge c2)` (a
        // `Sensitivity::List` with >1 term) or `@(*)` is a multi-clock
        // property — the single-`always` checker model does not implement it.
        // Reject it loudly instead of building one (semantically wrong)
        // `always @(c1 or c2)` checker (this closes a silent-accept hole).
        // Mid-property second-`@` events are caught earlier by the parser.
        let single_clock = matches!(&sva.clock, ast::Sensitivity::List(evs) if evs.len() == 1);
        if !single_clock {
            self.error(
                MsgCode::ElabUnsupported,
                "a concurrent assertion must have a single clocking event \
                 (multi-clock / OR-of-clocks property clocks are unsupported \
                 in this subset)",
            );
            return;
        }
        // Sequence/property LOCAL VARIABLES (slice N2c, IEEE §16.10): a property
        // that declares a local var OR carries a `(b, x = e)` capture routes to the
        // data-tracking synthesis (a parallel DATA shift register, shifted in
        // lockstep with the liveness pipeline). Dispatched BEFORE the prop_expr /
        // cross-clock / flat paths so a capture never reaches them. Guarded so an
        // assertion with NO local vars and NO capture is byte-identical (the common
        // case allocates ZERO data-tracking nets).
        if !sva.local_vars.is_empty()
            || seq_has_match_item(&sva.ante)
            || seq_has_match_item(&sva.cons)
        {
            self.synth_local_var_assert(sva, sp);
            return;
        }
        // Property-level operators (slice N2d + SVA-REST): a `prop_expr` tree
        // (the flat `ante/kind/cons` fields hold placeholders). Dispatched FIRST
        // so the placeholder fields never reach the cross-clock / flat paths. A
        // tree containing a LIVENESS operator (`s_eventually` / `s_until`) routes
        // to `synth_liveness` (which emits an end-of-sim `final` obligation check);
        // a SAFETY-only tree (`and`/`or`/`not`/`always`/weak-`until`/recursion)
        // reduces to a per-clock boolean violation by `synth_prop_expr`.
        if sva.prop_expr.is_some() {
            if Self::prop_expr_has_liveness(sva.prop_expr.as_ref().unwrap()) {
                self.synth_liveness(sva, sp);
            } else {
                self.synth_prop_expr(sva, sp);
            }
            return;
        }
        // Cross-clock SEQUENCE antecedent (slice N2a-1): `@(c1) a ##1 @(c2) b |-> c`
        // — a `##1 @(c2)` boundary re-clocks MID-ANTECEDENT (distinct from A3,
        // where the IMPLICATION crosses clocks). Routed to a dedicated dual-clock
        // handoff synthesis. (A `Clocked` node in the consequent is also routed
        // here and loud-rejected — `synth_crossclock` requires a boolean cons.)
        // FIRST fold any redundant same-clock re-clock `@(c1) … @(c1) …` to plain
        // sequence (§16.13 no-op) so it shares the byte-correct single-clock
        // pipeline rather than the lossy handoff (review N2a-1). Guarded so the
        // common no-Clocked antecedent is untouched (byte-identical).
        if seq_has_clocked(&sva.ante) {
            sva.ante = strip_redundant_clocks(&sva.ante, &sva.clock);
        }
        if seq_has_clocked(&sva.ante) || seq_has_clocked(&sva.cons) {
            self.synth_crossclock(sva, sp);
            return;
        }
        // Multi-clock canonical pattern (slice A3): `@(c1) ante |=> @(c2) cons`
        // synthesizes TWO processes (a c1-clocked sampler + a c2-clocked consumer)
        // joined by a 1-bit handoff reg, instead of the single-clock checker below.
        // Single-clock asserts (the common case) keep the byte-identical path.
        if sva.cons_clock.is_some() {
            self.synth_multiclock(sva, sp);
            return;
        }
        // Property-references-property (slice A4): a bare named-PROPERTY consequent
        // `… |-> q` (q an OVERLAP property, same clock) flattens to the boolean
        // `!q.ante || q.cons`. A no-op for a literal / sequence / net consequent
        // (byte-identical), so it runs before the prev-reg allocation below to keep
        // the single-property path's numbering unchanged.
        self.flatten_prop_consequent(&mut sva);
        // Rewrite sampled-value functions ($past/$rose/$fell/$stable) into
        // reads of synthesized prev-registers, collecting the per-clock NBA
        // updates (`prev <= signal`) that maintain them.
        let mut regs = SvaRegs::default();
        // Expand the antecedent Sequence into a disjunction of (boolean-term,
        // hop-delay) alternatives, synthesize each one's match-this-clock
        // signal (a shift-register pipeline for ≥2 terms), and OR them. A
        // single 1-term alternative reproduces the flat-property path
        // byte-for-byte; bounded ranges produce >1 alternative.
        let mut pipeline_nbas: Vec<ast::Stmt> = Vec::new();
        let ante = self.synth_seq_match(&sva.ante, &mut regs, &mut pipeline_nbas, sp);
        // Consequent (slice S14). A boolean consequent is rewritten here (so
        // its prev-reg allocation order — and the byte-identical lowering —
        // is preserved); a sequence consequent is built as an obligation
        // chain AFTER `cond_lhs` is known (it seeds the chain).
        let cons_boolean = match &sva.cons {
            ast::Sequence::Boolean(e) => Some(self.rewrite_sampled(e, &mut regs)),
            _ => None,
        };

        // Action block (slice S11). The fail action — the `else` statement, or
        // the default `$error("Assertion property violation")` when absent —
        // runs on a violation; the optional pass action runs on a non-vacuous
        // success. When both are absent the body is byte-identical to the
        // pre-S11 checker (default $error, no pass branch).
        let fail_stmt_raw = match sva.fail {
            Some(s) => *s,
            None => ast::Stmt::SysTaskCall {
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
            },
        };
        let pass_action_raw = sva.pass;
        // `disable iff (expr)` reset (slice S12): a 1-bit reduction of the
        // (sampled) condition. When present, fire conditions are gated with
        // `!dis` and every obligation NBA is reset to 0 on the dis clock, so
        // in-flight attempts are aborted. Absent → the body is byte-identical
        // to the pre-S12 checker.
        let dis = sva
            .disable_iff
            .as_ref()
            // §16.13.5: an X/Z disable condition is NOT definitely-true → it does
            // NOT disable (X-strict), so a real violation is not silently masked.
            .map(|e| sva_match(self.rewrite_sampled(e, &mut regs), sp));
        // Action-block sampled values (slice A2): rewrite $past/$rose/$fell/$stable
        // inside the fail/pass action statements to the SAME shared prev-regs the
        // property body uses. Done AFTER the antecedent/consequent/disable rewrites
        // so those keep the lower net IDs and an action `$past(sig)` of an
        // already-sampled signal dedups onto the existing prev-reg (regs.by_signal).
        // A no-sampled action allocates ZERO nets → byte-identical to pre-A2.
        let fail_stmt = self.rewrite_sampled_stmt(&fail_stmt_raw, &mut regs);
        let pass_action =
            pass_action_raw.map(|ps| Box::new(self.rewrite_sampled_stmt(&ps, &mut regs)));
        let (cond_lhs, pending_nba) = match sva.kind {
            ast::ImplicationKind::Overlap => (ante, None),
            ast::ImplicationKind::NonOverlap => {
                // 1-bit pending reg: NBA-sampled with the antecedent's BOOLEAN
                // truthiness each clock (reduction-OR, so a multi-bit antecedent
                // is not truncated to its LSB), checked against the consequent
                // on the FOLLOWING clock.
                let pend = self.fresh_sva_reg(1, "pend");
                let pend_path = ast::HierPath {
                    segments: vec![ast::Ident {
                        name: pend.clone(),
                        span: sp,
                    }],
                    span: sp,
                };
                let nba = ast::Stmt::NonBlocking {
                    lhs: ast::Lvalue::Ident(pend_path),
                    delay: None,
                    event: None,
                    rhs: sva_unary(ast::UnOp::RedOr, ante, sp),
                    span: sp,
                };
                (sva_ident_expr(&pend, sp), Some(nba))
            }
        };
        // Consequent core (slice S14): the violation and (non-vacuous)
        // success signals. A boolean consequent is `cond_lhs && !cons` /
        // `cond_lhs && cons` (byte-identical to before S14); a sequence
        // consequent is an obligation chain whose due-delay regs are
        // obligation state (reset by `disable iff` like the antecedent).
        let mut cons_chain_nbas: Vec<ast::Stmt> = Vec::new();
        let (violation_core, success_core) = match cons_boolean {
            Some(cons) => {
                // §16.13.5: the consequent matches only when it is definitely
                // true (X/Z = non-match → violation). `sva_match` makes the X
                // case a hard 0 so `!match` fires; a definitely-nonzero value
                // (e.g. multi-bit `4'b0100`) still matches.
                let cons_match = sva_match(cons, sp);
                (
                    sva_binary(
                        ast::BinOp::LogAnd,
                        cond_lhs.clone(),
                        sva_unary(ast::UnOp::LogNot, cons_match.clone(), sp),
                        sp,
                    ),
                    sva_binary(ast::BinOp::LogAnd, cond_lhs.clone(), cons_match, sp),
                )
            }
            None => {
                self.build_seq_consequent(&sva.cons, &cond_lhs, &mut regs, &mut cons_chain_nbas, sp)
            }
        };
        // violation gated by `!dis`.
        let mut violation = violation_core;
        if let Some(d) = &dis {
            violation = sva_binary(
                ast::BinOp::LogAnd,
                sva_unary(ast::UnOp::LogNot, d.clone(), sp),
                violation,
                sp,
            );
        }
        let if_fail = ast::Stmt::If {
            cond: violation,
            then_s: Box::new(fail_stmt),
            else_s: None,
            span: sp,
        };
        // Clocked body: check FIRST (reads the prior clock's prev/pend), then
        // the NBA updates apply in the NBA region for the next clock.
        let mut stmts = vec![if_fail];
        // Pass action (if any) runs on a NON-VACUOUS success: antecedent
        // matched AND consequent held (vacuous success — antecedent false —
        // does not fire it; a hand-IEEE choice, documented). Also gated `!dis`.
        if let Some(ps) = pass_action {
            let mut success = success_core;
            if let Some(d) = &dis {
                success = sva_binary(
                    ast::BinOp::LogAnd,
                    sva_unary(ast::UnOp::LogNot, d.clone(), sp),
                    success,
                    sp,
                );
            }
            stmts.push(ast::Stmt::If {
                cond: success,
                then_s: ps,
                else_s: None,
                span: sp,
            });
        }
        // `disable iff` reset: clear in-flight obligation state (antecedent
        // pipeline + consequent chain + |=> pend NBAs) when dis is true. The
        // prev-sampling NBAs (regs.nbas) keep sampling — only the attempt
        // obligations are aborted.
        let (pipeline_nbas, cons_chain_nbas, pending_nba) = if let Some(d) = &dis {
            (
                pipeline_nbas
                    .into_iter()
                    .map(|s| gate_nba_with_disable(s, d, sp))
                    .collect::<Vec<_>>(),
                cons_chain_nbas
                    .into_iter()
                    .map(|s| gate_nba_with_disable(s, d, sp))
                    .collect::<Vec<_>>(),
                pending_nba.map(|s| gate_nba_with_disable(s, d, sp)),
            )
        } else {
            (pipeline_nbas, cons_chain_nbas, pending_nba)
        };
        stmts.extend(regs.nbas);
        stmts.extend(pipeline_nbas);
        stmts.extend(cons_chain_nbas);
        if let Some(nba) = pending_nba {
            stmts.push(nba);
        }
        let body = if stmts.len() == 1 {
            stmts.pop().unwrap()
        } else {
            ast::Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts,
                span: sp,
            }
        };
        let pb = ast::ProceduralBlock {
            kind: ast::ProcKind::Always,
            sensitivity: Some(sva.clock),
            body: Box::new(body),
            span: sp,
        };
        let proc = self.lower_synth_proc(&pb, "sva");
        self.push_process(proc);
    }

    /// A fresh DATA-tracking register (slice N2c) of the given width/sign, X-init
    /// (`fresh_sva_reg` semantics — an X stage never spuriously matches a `==`
    /// consequent before any value has been captured). Used for the parallel data
    /// shift register that carries a captured local-variable value alongside the
    /// liveness pipeline.
    pub(crate) fn fresh_sva_data_reg(&mut self, width: u32, signed: bool) -> String {
        let w = width.max(1);
        let name = format!("__sva_lv_{}", self.nets.len());
        let nv = ir::NetVar {
            kind: ir::NetKind::Reg,
            width: w,
            msb: w.saturating_sub(1),
            lsb: 0,
            signed,
            array_len: 1,
            dir: ir::PortDir::Internal,
            init: default_init(ast::NetVarKind::Reg, w),
        };
        self.add_net(&name, nv);
        name
    }

    /// Synthesize a concurrent assertion carrying a sequence/property LOCAL VARIABLE
    /// (slice N2c, IEEE 1800-2017 §16.10): the data-tracking idiom
    /// `(req, d=data) ##1 grant |-> (rdata == d)` — capture a value at one term and
    /// read it at a LATER term/consequent within the SAME match attempt.
    ///
    /// CORRECTNESS (why a single register per var, no thread table): the liveness
    /// pipeline is a SHIFT register — stage k is the attempt that started k clocks
    /// ago. The seed boolean fires at most ONCE per clock, so each stage holds at
    /// most one attempt. A PARALLEL data register `c[0] <= data; c[j] <= c[j-1]`
    /// shifted in lockstep carries the captured value with no collision between
    /// concurrent (pipelined) attempts — they occupy different time-stages of the
    /// same register. The ONLY collision is CONVERGENCE: a RANGED delay lets two
    /// attempts reach one stage via different paths → two data values collide → LOUD.
    ///
    /// SCOPE (correct-or-loud): a SINGLE var, captured at exactly one FIXED-DELAY
    /// term (`##d` / fixed `[*n]`-free) on a SINGLE clock, read in the consequent.
    /// Any range, cross-clock, sequence consequent, multiple write, read-before-
    /// capture, disjunction/throughout/within, or `disable iff` is loud-rejected.
    pub(crate) fn synth_local_var_assert(&mut self, sva: PendingSva, sp: ast::Span) {
        // ── 0. Out-of-scope fast loud-rejects ──────────────────────────────────
        // A `prop_expr` cannot reach here (routed before the local-var gate), but a
        // multi-clock consequent / cross-clock antecedent / `disable iff` would need
        // the data registers threaded through extra handoff/reset machinery — loud.
        if sva.cons_clock.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "a multi-clock (`|=> @(c2)`) consequent with a sequence local variable \
                 is unsupported in this subset",
            );
            return;
        }
        if seq_has_clocked(&sva.ante) || seq_has_clocked(&sva.cons) {
            self.error(
                MsgCode::ElabUnsupported,
                "a cross-clock (`@(c2)`) sequence with a local variable is unsupported \
                 in this subset",
            );
            return;
        }
        if sva.disable_iff.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "`disable iff` combined with a sequence local variable is unsupported \
                 in this subset",
            );
            return;
        }
        // The consequent must be a plain boolean (a SEQUENCE consequent would need the
        // data shifted through the obligation chain too — out of subset).
        let ast::Sequence::Boolean(cons_e) = &sva.cons else {
            self.error(
                MsgCode::ElabUnsupported,
                "a SEQUENCE consequent with a sequence local variable is unsupported \
                 in this subset (use a boolean consequent `|-> (rdata == d)`)",
            );
            return;
        };

        // ── 1. Flatten the antecedent to an ordered fixed-delay term list ───────
        // Each term is (boolean, hop-before-term, captures). A range / goto / nonconsec
        // / repetition / within / throughout / nested capture-under-repeat is loud.
        let mut flat: Vec<FlatLvTerm> = Vec::new();
        if !self.flatten_lv_antecedent(&sva.ante, &mut flat) {
            return; // flatten already emitted the loud diagnostic
        }
        if flat.is_empty() {
            self.error(
                MsgCode::ElabUnsupported,
                "an empty sequence local-variable antecedent is unsupported",
            );
            return;
        }

        // ── 2. Resolve the SINGLE captured var (name → declared width/sign) ─────
        // Collect every capture across the flattened terms. v1 supports exactly ONE
        // write to ONE variable; a multiple-write (same or different var) is a
        // convergence/aliasing hazard → loud.
        let mut captures: Vec<(usize, String, ast::Expr)> = Vec::new(); // (term_idx, name, expr)
        for (idx, t) in flat.iter().enumerate() {
            for (n, e) in &t.captures {
                captures.push((idx, n.name.clone(), e.clone()));
            }
        }
        if captures.len() != 1 {
            self.error(
                MsgCode::ElabUnsupported,
                "exactly one local-variable capture is supported in this subset \
                 (multiple writes converge / alias — unsupported)",
            );
            return;
        }
        let (cap_idx, cap_name, cap_expr) = captures.into_iter().next().unwrap();
        // The var must be DECLARED (so its width/sign govern the data register). An
        // undeclared capture target is loud (never a silent default width).
        let Some(decl) = sva.local_vars.iter().find(|d| d.name.name == cap_name) else {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "sequence local variable `{cap_name}` is captured but not declared \
                     (add a `<type> {cap_name};` at the property body start)"
                ),
            );
            return;
        };
        // The declared type must be a synthesizable FIXED-WIDTH INTEGRAL var
        // (`int`/`integer`/`byte`/`shortint`/`longint`/`bit`/`logic`/`reg`). A
        // `real`/`realtime`/`string`/`event`/class/net type has no data-tracking
        // shift register in this subset; the parser carried a 1-bit placeholder
        // width, so capturing into it would SILENTLY truncate the value to 1 bit and
        // flip the assertion verdict. Loud-reject it (never a silent default width).
        if decl.unsupported_type {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a non-integral sequence local variable `{cap_name}` \
                     (real/realtime/string/event/class) is unsupported in this subset \
                     — only fixed-width integral types (`int`/`byte`/`bit`/…) have a \
                     data-tracking register"
                ),
            );
            return;
        }
        if decl.init.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "an initialized sequence local variable (`int x = e;`) is unsupported \
                 in this subset",
            );
            return;
        }
        // The var must NOT be read AT OR BEFORE its capture term: a read at a term with
        // index ≤ cap_idx is undefined (the value is not yet captured, or is captured
        // combinationally at that same term) → loud (never a silent guess). A read in a
        // LATER antecedent term (index > cap_idx) IS well-defined: it reads the captured
        // value delayed by S = the sum of the FIXED hops from cap_idx+1..=idx clocks
        // (§16.10), substituted in the per-term AND walk below. Reads in the consequent
        // (always after the antecedent completes) continue to resolve at the completion
        // stage. A self-referential capture stays separately loud.
        for (idx, t) in flat.iter().enumerate() {
            if idx <= cap_idx && expr_reads_ident(&t.term, &cap_name) {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "sequence local variable `{cap_name}` read at or before its \
                         capture term is unsupported in this subset (read it in a LATER \
                         antecedent term or the consequent)"
                    ),
                );
                return;
            }
            // A self-referential capture `(b, x = f(x))` has no defined value (x is
            // not yet captured at its own term). Reject it with a targeted message
            // (otherwise the unresolved `x` would surface as a misleading
            // undeclared-net E3010).
            if t.captures
                .iter()
                .any(|(_, e)| expr_reads_ident(e, &cap_name))
            {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "sequence local variable `{cap_name}` read inside its own capture \
                         expression `(b, {cap_name} = … {cap_name} …)` is unsupported \
                         (a self-referential capture has no defined value)"
                    ),
                );
                return;
            }
        }
        // The consequent MUST read the var (otherwise the capture is dead — that is not
        // wrong, but it signals a likely typo; still, a dead capture is harmless, so we
        // do not require it). Substitution below is a no-op if absent.

        // ── 3. Build liveness + parallel data shift register in lockstep ────────
        // The liveness pipeline mirrors `synth_seq_pipeline`'s fixed-delay path. The
        // captured value is COMBINATIONAL at the capture term's clock; each liveness
        // `seq_delay_reg` shift AFTER the capture is mirrored by ONE data register so
        // the value travels in lockstep. `data_chain[k]` reads = the captured value
        // delayed by (k+1) clocks; after S shifts from capture to completion, the read
        // = `data_chain[S-1]` (delay S) — or the COMBINATIONAL captured value when
        // S == 0 (capture and completion are the SAME clock, e.g. an overlap
        // single-term antecedent). This is the load-bearing alignment: the data read
        // stage = the liveness shift count from the capture term to the read site.
        let mut regs = SvaRegs::default();
        let mut pipeline_nbas: Vec<ast::Stmt> = Vec::new();
        let cap_width = decl.width;
        let cap_signed = decl.signed;
        // The combinational captured value at its term's clock (the seed of the chain).
        let cap_value = self.rewrite_sampled(&cap_expr, &mut regs);

        // Liveness seed = |term0 (combinational, this clock).
        let term0 = self.rewrite_sampled(&flat[0].term, &mut regs);
        let mut cur = sva_unary(ast::UnOp::RedOr, term0, sp);
        // `data_chain[k]` read = captured value delayed by (k+1) clocks. Empty until a
        // shift occurs AFTER the capture term. `capturing` becomes true once the
        // capture term has been visited (the seed of the chain is `cap_value`).
        let mut data_chain: Vec<String> = Vec::new();
        let mut capturing = cap_idx == 0;
        // The value feeding the NEXT data shift: combinational `cap_value` for the
        // first shift after capture, else a read of the chain tail.
        let data_feed = |chain: &[String], cap: &ast::Expr| -> ast::Expr {
            match chain.last() {
                Some(r) => sva_ident_expr(r, sp),
                None => cap.clone(),
            }
        };

        // Walk the remaining terms applying hops (shifts) + the boolean AND, mirroring
        // each liveness shift onto the data chain once capturing has begun.
        for (idx, t) in flat.iter().enumerate().skip(1) {
            let hop = t.hop;
            for _ in 0..hop {
                cur = self.seq_delay_reg(cur, &mut pipeline_nbas, sp);
                if capturing {
                    // Shift the data one stage in lockstep with the liveness shift.
                    let feed = data_feed(&data_chain, &cap_value);
                    let next = self.fresh_sva_data_reg(cap_width, cap_signed);
                    pipeline_nbas.push(sva_nb(&next, feed, sp));
                    data_chain.push(next);
                }
            }
            // The capture happens at term `cap_idx`'s OWN clock (after its hops): from
            // here on, shifts carry the captured value.
            if idx == cap_idx {
                capturing = true;
            }
            // Apply the boolean term to the liveness activation. If this term READS the
            // captured local var (only reachable at idx > cap_idx — earlier/at-capture
            // reads were rejected above), substitute the read with the data register at
            // the EXACT shift-stage S = `data_chain.len()` at THIS point = the sum of the
            // FIXED hops from cap_idx+1..=idx (a COMPILE-TIME CONSTANT). `data_chain`'s
            // tail (= `data_chain[S-1]`) is the captured value delayed by S clocks = the
            // value sampled at the capture clock observed at THIS term's clock (§16.10).
            // The chain length is exact because each fixed hop pushed one data reg in
            // lockstep with the liveness shift above; a RANGED hop on the path was
            // already loud-rejected at flatten time, so S is unambiguous (never a guess).
            let tb = if idx > cap_idx && expr_reads_ident(&t.term, &cap_name) {
                match data_chain.last().cloned() {
                    Some(stage_reg) => {
                        let read_v = sva_ident_expr(&stage_reg, sp);
                        let sub = subst_ident_expr(&t.term, &cap_name, &read_v);
                        self.rewrite_sampled(&sub, &mut regs)
                    }
                    None => {
                        // S == 0: an all-`##0` chain coincides the read with the capture
                        // clock — a degenerate read-at-capture. Loud (correct-or-loud;
                        // the same-clock combinational path is out of this subset).
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "sequence local variable `{cap_name}` read at the same \
                                 clock as its capture (an all-`##0` chain to the read) is \
                                 unsupported in this subset"
                            ),
                        );
                        return;
                    }
                }
            } else {
                self.rewrite_sampled(&t.term, &mut regs)
            };
            cur = sva_binary(
                ast::BinOp::BitAnd,
                cur,
                sva_unary(ast::UnOp::RedOr, tb, sp),
                sp,
            );
        }
        let ante = cur;

        // ── 4. Resolve the data-read value at the completion stage ──────────────
        // The chain tail reads = the captured value delayed by `data_chain.len()`
        // clocks = exactly the number of shifts from the capture term to the LAST
        // term (the completion). For `|->` (overlap) the consequent reads at the
        // completion clock → that tail (or the combinational `cap_value` when the
        // chain is empty: capture == completion). For `|=>` (non-overlap) the
        // consequent is one clock later → register the read value ONE more stage.
        let read_value = match sva.kind {
            ast::ImplicationKind::Overlap => data_feed(&data_chain, &cap_value),
            ast::ImplicationKind::NonOverlap => {
                // One extra shift stage so the data survives to the consequent clock.
                let feed = data_feed(&data_chain, &cap_value);
                let extra = self.fresh_sva_data_reg(cap_width, cap_signed);
                pipeline_nbas.push(sva_nb(&extra, feed, sp));
                sva_ident_expr(&extra, sp)
            }
        };

        // ── 5. Substitute the var read in the consequent, then lower the check ──
        let cons_sub = subst_ident_expr(cons_e, &cap_name, &read_value);
        let cons_rw = self.rewrite_sampled(&cons_sub, &mut regs);

        // `cond_lhs` is the antecedent match (overlap) or a 1-clock-delayed pend reg
        // (non-overlap), mirroring the flat path.
        let (cond_lhs, pending_nba) = match sva.kind {
            ast::ImplicationKind::Overlap => (ante, None),
            ast::ImplicationKind::NonOverlap => {
                let pend = self.fresh_sva_reg(1, "pend");
                let nba = sva_nb(&pend, sva_unary(ast::UnOp::RedOr, ante, sp), sp);
                (sva_ident_expr(&pend, sp), Some(nba))
            }
        };
        // §16.13.5: the consequent matches only when DEFINITELY true (X/Z = non-match).
        let cons_match = sva_match(cons_rw, sp);
        let violation = sva_binary(
            ast::BinOp::LogAnd,
            cond_lhs,
            sva_unary(ast::UnOp::LogNot, cons_match, sp),
            sp,
        );
        let fail_stmt = match sva.fail {
            Some(s) => self.rewrite_sampled_stmt(&s, &mut regs),
            None => sva_error_stmt(sp),
        };
        let if_fail = ast::Stmt::If {
            cond: violation,
            then_s: Box::new(fail_stmt),
            else_s: None,
            span: sp,
        };
        // A pass action is supported (non-vacuous success); but to keep this slice
        // tight we loud-reject a pass action (its success signal would need the same
        // completion-stage gating — a follow-on).
        if sva.pass.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "a `pass` action block with a sequence local variable is unsupported \
                 in this subset",
            );
            return;
        }
        let mut stmts = vec![if_fail];
        stmts.extend(regs.nbas);
        stmts.extend(pipeline_nbas);
        if let Some(nba) = pending_nba {
            stmts.push(nba);
        }
        let body = if stmts.len() == 1 {
            stmts.pop().unwrap()
        } else {
            ast::Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts,
                span: sp,
            }
        };
        let pb = ast::ProceduralBlock {
            kind: ast::ProcKind::Always,
            sensitivity: Some(sva.clock),
            body: Box::new(body),
            span: sp,
        };
        let proc = self.lower_synth_proc(&pb, "sva");
        self.push_process(proc);
    }

    /// The per-clock VIOLATION expression of a property expression (true ⇒ the
    /// property fails THIS clock), appending pend-reg NBAs (`pend <= |ante`) to
    /// `pend_nbas` and prev-reg sampling NBAs (via `rewrite_sampled`) to `regs`.
    ///
    /// Recursion: `self_name` is the recursive property's own name. A `… |=> NAME`
    /// consequent is the legal TAIL recursion — its next-clock obligation is
    /// discharged by the per-clock re-attempt that `assert property(NAME)` spawns,
    /// so the whole implication contributes NO violation (drops to `1'b0`). The
    /// reduction is exact for the canonical idioms: `always` (`b and (1'b1 |=> p)`
    /// → `if(!b)`) and weak-until (`q or (b and (1'b1 |=> p))` → `if(!q && !b)`).
    ///
    /// Returns `(viol, skew)` — the per-clock violation expression and the CLOCK
    /// SKEW of the sub-property (the number of clocks by which its verdict lands
    /// AFTER the attempt-start clock: a boolean / overlap leaf is skew 0, a `|=>`
    /// is skew 1). A whole-tree top-level skew only shifts WHEN a verdict is
    /// reported (verdict-safe), but the operands of one `and`/`or` (and an `|->`/
    /// `|=>` consequent) MUST be skew-aligned: combining a skew-1 `|=>` with a
    /// skew-0 sibling would pair two DIFFERENT attempt-start clocks (review N2d:
    /// `(a |=> b) or q` produced both a false pass and a false fire). A skew
    /// mismatch — and a `|=>`/`|->` consequent with a non-zero skew (which would
    /// need a multi-stage pend network, beyond this subset) — is loud-rejected.
    /// Also returns `None` (after a loud diagnostic) for a non-boolean sequence
    /// operand, a recursion reference outside a `|=>` consequent, or a nesting
    /// depth beyond `SVA_SEQ_ALT_CAP` (a robustness cap — the recursive reduction
    /// would otherwise overflow the stack on a pathological `a and a and …` chain).
    pub(crate) fn prop_expr_violation(
        &mut self,
        pe: &ast::PropExpr,
        self_name: Option<&str>,
        regs: &mut SvaRegs,
        pend_nbas: &mut Vec<ast::Stmt>,
        depth: u32,
        sp: ast::Span,
    ) -> Option<(ast::Expr, u32)> {
        if depth as usize > SVA_SEQ_ALT_CAP {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a property-level `and`/`or` nesting exceeds the depth cap ({SVA_SEQ_ALT_CAP}); narrow it"
                ),
            );
            return None;
        }
        match pe {
            ast::PropExpr::Seq(seq) => {
                // A bare sequence used as a property holds iff the (boolean) seq is
                // true → viol = !match(b). Same-clock leaf (skew 0). §16.13.5: an
                // X/Z boolean is a non-match → it does NOT hold → viol = 1 (fire).
                // `sva_match` makes this the single X-strict chokepoint for ALL the
                // property-level combinators (and/or/not/until/implies); since it
                // yields 0/1, no X leaks into the combined violation.
                let b = self.prop_bool_operand(seq, self_name)?;
                Some((sva_unary(ast::UnOp::LogNot, sva_match(b, sp), sp), 0))
            }
            ast::PropExpr::Impl { ante, kind, cons } => {
                // Tail recursion `a |=> NAME` drops to no-violation BEFORE touching
                // the antecedent (no pend reg, no sampled-value reg for `a`). The
                // constant `1'b0` carries skew 0 (it combines with any sibling).
                if matches!(kind, ast::ImplicationKind::NonOverlap)
                    && self.prop_cons_is_self_recursion(cons, self_name)
                {
                    return Some((sva_zero(sp), 0));
                }
                let a = self.prop_bool_operand(ante, self_name)?;
                let a = self.rewrite_sampled(&a, regs);
                let (vc, cons_skew) =
                    self.prop_expr_violation(cons, self_name, regs, pend_nbas, depth + 1, sp)?;
                // A consequent that is itself skewed (a nested `|=>`) would need a
                // multi-stage pend network to stay attempt-aligned — out of subset.
                if cons_skew != 0 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a nested multi-clock-skew implication consequent (e.g. \
                         `a |=> (c |=> d)`) inside a property-level `and`/`or` is \
                         unsupported in this subset",
                    );
                    return None;
                }
                match kind {
                    ast::ImplicationKind::Overlap => {
                        // `a |-> cons` (same clock): viol = a && viol(cons). Skew 0.
                        // A self-reference in `cons` is NOT a `|=>` consequent → it
                        // reaches `prop_bool_operand` and is loud-rejected (an
                        // overlap same-tick recursion is an illegal fixpoint).
                        Some((sva_binary(ast::BinOp::LogAnd, a, vc, sp), 0))
                    }
                    ast::ImplicationKind::NonOverlap => {
                        // Non-recursive `a |=> cons`: a 1-bit pend reg delays the
                        // (reduction-OR) antecedent one clock; viol = pend &&
                        // viol(cons), so `cons` is effectively checked next clock.
                        // Skew 1 (the verdict lands one clock after the attempt).
                        let pend = self.fresh_sva_reg(1, "pend");
                        let pend_path = ast::HierPath {
                            segments: vec![ast::Ident {
                                name: pend.clone(),
                                span: sp,
                            }],
                            span: sp,
                        };
                        pend_nbas.push(ast::Stmt::NonBlocking {
                            lhs: ast::Lvalue::Ident(pend_path),
                            delay: None,
                            event: None,
                            rhs: sva_unary(ast::UnOp::RedOr, a, sp),
                            span: sp,
                        });
                        Some((
                            sva_binary(ast::BinOp::LogAnd, sva_ident_expr(&pend, sp), vc, sp),
                            1,
                        ))
                    }
                }
            }
            // `L and R` holds iff both hold → viol = viol(L) || viol(R).
            ast::PropExpr::And(l, r) => {
                let (vl, sl) =
                    self.prop_expr_violation(l, self_name, regs, pend_nbas, depth + 1, sp)?;
                let (vr, sr) =
                    self.prop_expr_violation(r, self_name, regs, pend_nbas, depth + 1, sp)?;
                let s = self.unify_prop_skew(sl, sr)?;
                Some((sva_binary(ast::BinOp::LogOr, vl, vr, sp), s))
            }
            // `L or R` holds iff either holds → viol = viol(L) && viol(R).
            ast::PropExpr::Or(l, r) => {
                let (vl, sl) =
                    self.prop_expr_violation(l, self_name, regs, pend_nbas, depth + 1, sp)?;
                let (vr, sr) =
                    self.prop_expr_violation(r, self_name, regs, pend_nbas, depth + 1, sp)?;
                let s = self.unify_prop_skew(sl, sr)?;
                Some((sva_binary(ast::BinOp::LogAnd, vl, vr, sp), s))
            }
            // `not p` (SVA-REST) — holds iff `p` does NOT → viol = held(p) = !viol(p).
            // Skew preserved (a `not` of a `|=>` keeps the verdict one clock later).
            ast::PropExpr::Not(p) => {
                let (vp, s) =
                    self.prop_expr_violation(p, self_name, regs, pend_nbas, depth + 1, sp)?;
                Some((sva_unary(ast::UnOp::LogNot, vp, sp), s))
            }
            // Weak `lhs until rhs` (SVA-REST, safety) — at every clock `lhs` must hold
            // until `rhs` first does → viol = `!held(lhs) && !held(rhs)` = viol(lhs) &&
            // viol(rhs). Both operands must be skew-0 (the temporal obligation aligns
            // same-clock verdicts). The STRONG form (`s_until`) is routed to
            // `synth_liveness` and never reaches here.
            ast::PropExpr::Until {
                lhs,
                rhs,
                strong: false,
            } => {
                let (vl, sl) =
                    self.prop_expr_violation(lhs, self_name, regs, pend_nbas, depth + 1, sp)?;
                let (vr, sr) =
                    self.prop_expr_violation(rhs, self_name, regs, pend_nbas, depth + 1, sp)?;
                if sl != 0 || sr != 0 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a `until` operand with a multi-clock (`|=>`) skew is \
                         unsupported in this subset",
                    );
                    return None;
                }
                Some((sva_binary(ast::BinOp::LogAnd, vl, vr, sp), 0))
            }
            // A top-level `always p` is reduced to `p` BEFORE the per-clock reduction
            // (every clock re-checks `p`, so it is exactly `p`'s violation). A NESTED
            // `always` reaching here (e.g. `a |-> always b`, which needs an arm-then-
            // hold-forever latch) is loud-rejected. Likewise a STRONG liveness operator
            // (`s_eventually`/`s_until`) reaching this per-clock reducer is a routing
            // error (it should have gone to `synth_liveness`) → loud.
            ast::PropExpr::Always(_) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a nested `always` (e.g. `a |-> always b`) is unsupported in this \
                     subset (only a top-level `always p` is supported)",
                );
                None
            }
            ast::PropExpr::Until { strong: true, .. } | ast::PropExpr::Eventually { .. } => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a liveness operator (`s_eventually` / `s_until`) nested inside a \
                     property-level `and`/`or`/`not` is unsupported in this subset",
                );
                None
            }
        }
    }
}
