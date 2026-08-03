//! split part of `sched` (mechanical move).

use super::*;

impl Scheduler<'_, '_> {
    pub(crate) fn propagate_changes(&mut self) {
        // IEEE §9.3.2 continuous force: while a force with an expression RHS is
        // live, re-evaluate it whenever ANYTHING changed this delta and re-pin
        // the target through the force funnel. Over-sensitivity is harmless (a
        // same-value re-pin is dropped by the write funnel) and the re-force
        // lands in the SAME sweep below via the dirty list. Designs without a
        // live force never enter (empty registry).
        if !self.st.active_forces.is_empty() && !self.st.dirty.is_empty() {
            self.reeval_active_forces();
        }
        // Take/restore the scratch buffers: this runs once per delta, and a
        // fresh Vec pair per call was measurable allocator traffic.
        let mut changed_nets = std::mem::take(&mut self.scratch_changed);
        changed_nets.clear();
        // Dirty sweep (scheduler R2): every net that took an ACTUAL bit change
        // since the last sweep is a candidate — the previous full O(nets)
        // `cur != prev` scan taxed every idle net on every delta (512 idle
        // regs ≈ 9x the 8-net wall-clock). Sorting restores the old scan's
        // ascending order (byte-identity). GLITCH FIX: `dirty` membership ALONE
        // is the changed set — a net is on `dirty` iff `note_change` saw a real
        // transition, so an A→B→A round-trip (cur ends == prev) is STILL a
        // change the observer must see (IEEE §9 fires a glitch once). The old
        // `cur != prev` endpoint filter silently dropped exactly those nets,
        // leaving `always @(a)`/`@(posedge a)` un-fired. The dropped set is
        // EMPTY for any design without a same-slot round-trip, so non-glitch
        // designs stay byte-identical; the bit0 edge KIND for the wake is
        // recovered from `slot_edge` (built per transition, not from endpoints).
        let mut cand = std::mem::take(&mut self.st.dirty);
        cand.sort_unstable();
        for &n in &cand {
            self.st.dirty_flag[n as usize] = false;
            changed_nets.push(n);
        }
        cand.clear();
        self.st.dirty = cand; // hand the capacity back for the next writes
        if changed_nets.is_empty() {
            self.scratch_changed = changed_nets;
            return;
        }

        // Precompute the per-net intra-slot edge mask for each changed net. The
        // mask (`slot_edge`) is maintained by the write funnel for every
        // `is_edge_target` net and is fresh for any net on `changed_nets` (the
        // first-dirty reset ran this slot). For a non-edge-target net the mask is
        // unused below (it is in no `net_to_edge` list and no `Edge` waiter), so
        // a stale value is harmless. Snapshotting into `edges` lets the `retain`
        // closure in (b) read the mask without re-borrowing `self.st`.
        let mut edges = std::mem::take(&mut self.scratch_edges);
        edges.clear();
        edges.extend(changed_nets.iter().map(|&net| {
            (
                net,
                self.st.slot_edge[net as usize],
                self.st.last_blocking_writer[net as usize],
            )
        }));

        // (a) wake statically edge-sensitive `always` processes.
        // Multi-edge dedup: a process sensitive to SEVERAL nets that all change in
        // THIS delta must be queued ONCE (IEEE §9: `always @(posedge c1 or posedge
        // c2)` ticks once even when both edges land together). Borrow the markers
        // out for the pass (take/restore avoids a per-delta alloc).
        let mut edge_seen = std::mem::take(&mut self.scratch_edge_seen);
        let mut edge_marked = std::mem::take(&mut self.scratch_edge_marked);
        if edge_seen.len() < self.activities.len() {
            edge_seen.resize(self.activities.len(), false);
        }
        for &(net, mask, writer) in &edges {
            // P3-4: index loop instead of cloning the per-net waiter list every
            // delta — the body only pushes into `cur.active`, never mutates
            // `net_to_edge`, so the indexed re-borrow is sound.
            for k in 0..self.net_to_edge[net as usize].len() {
                let (kind, ready) = self.net_to_edge[net as usize][k];
                // Skip a process that is still SUSPENDED MID-BODY: its static
                // edge entry is permanent (never deregistered), but IEEE does
                // not re-enter an `always` until it completes and re-arms. Its
                // legitimate in-body wake comes via the waiter path (b) below.
                // GLITCH: fire from the intra-slot `mask` (== endpoint for a
                // single write) so an A→B→A clock pulse still ticks once.
                // SELF-RETRIG: skip a process firing on a net IT itself
                // blocking-wrote — it already saw the value before re-arming.
                if edge_fires_slot(mask, kind)
                    && !self.activities[ready.proc as usize].busy
                    && writer != ready.proc
                {
                    // N4: a clocking-commit handler applies `preponed_buf → holding`
                    // HERE — at edge DETECTION, before the Active batch drains — so
                    // EVERY same-slot reader of `cb.sig` sees the committed sample,
                    // independent of process tie order. Crucially this fixes the
                    // cross-hierarchy case (`dut.cb.sig` read from a parent whose
                    // process tie sorts BEFORE the submodule's handler). The handler
                    // carries no real body, so it is not run in Active.
                    // `commit_clocking` returns true iff `proc` was a handler.
                    if self.st.commit_clocking(ready.proc) {
                        continue;
                    }
                    // Dedup: this proc may also be registered on another net that
                    // fired this delta — push it to Active only once.
                    let p = ready.proc as usize;
                    if edge_seen[p] {
                        continue;
                    }
                    edge_seen[p] = true;
                    edge_marked.push(ready.proc);
                    push_sorted(&mut self.cur.active, ready);
                }
            }
        }
        // GATED-CLOCK: do NOT clear the markers here — `edge_seen` is now
        // TIMESTEP-scoped (reset at time advance), so a process woken by one edge
        // this timestep is NOT re-woken by a later-delta edge of its sensitivity
        // (a derived/gated clock). `edge_marked` accumulates this timestep's woken
        // procs across all of its deltas for the O(#woken) reset at time advance.
        // Within a single delta the same-net/multi-net dedup is unchanged (the
        // `edge_seen[p]` guard is set during the pass). Restore for the next pass.
        self.scratch_edge_seen = edge_seen;
        self.scratch_edge_marked = edge_marked;

        // (b) wake in-body waiters. `wait(expr)` (WaitCause::Expr) RE-CHECKS its
        // predicate against the post-change net values and resumes only when it
        // becomes true — pre-evaluated here because the `retain` closure cannot
        // also borrow `&self` for `truthy`. (Previously Expr fell through `_ =>
        // true` and never woke, so a `false→true` transition hung the process.)
        // WAITER-POOL p2: only FILL `expr_now` when an `Expr` waiter actually
        // exists. When `n_expr_waiters == 0` the `retain` `WaitCause::Expr` arm
        // below cannot match any waiter, so `expr_now[wi]` is never indexed —
        // the empty buffer is sound. The fill scanned the FULL waiter vector
        // every non-idle delta even for all-`Edge` designs.
        let mut expr_now = std::mem::take(&mut self.scratch_expr_now); // WAITER-POOL
        expr_now.clear();
        if self.n_expr_waiters > 0 {
            expr_now.extend(self.waiters.iter().map(|w| match &w.cause {
                WaitCause::Expr { expr } => self.truthy(*expr),
                _ => false,
            }));
        }
        // Pre-compute Level firing (the retain closure cannot also borrow `&self`):
        // an in-body `@(sig)` (arm=Some) fires when a net differs from its ARM-TIME
        // value; a static sensitivity (arm=None) fires on any net change.
        // p2: same zero-skip guard — no `Level` waiter ⇒ `level_fire` is unused.
        let mut level_fire = std::mem::take(&mut self.scratch_level_fire); // WAITER-POOL
        level_fire.clear();
        if self.n_level_waiters > 0 {
            level_fire.extend(self.waiters.iter().map(|w| {
                match (&w.cause, &w.arm) {
                    // An in-body `@(sig)`/`@(*)` (arm=Some) fires when a watched net
                    // differs from its ARM-TIME value. NOTE: this intentionally does
                    // NOT consult `changed_nets` — a same-slot change that happened
                    // BEFORE the arm (e.g. `@(*)` arming at t0 right after its inputs
                    // were initialized) is already reflected in `arm`, so firing on
                    // dirtiness would spuriously re-run it in the arming slot. The
                    // cost is one narrow miss: an in-body `@(a)` waiting through a
                    // glitch that RETURNS to the arm value (ROADMAP §4.5.4) — a far
                    // rarer corner than the `@(*)` t0 behavior this preserves.
                    // SELF-RETRIG: a watched net the waiter's OWN process
                    // blocking-wrote does not wake it (it saw the value pre-rearm).
                    (WaitCause::Level { nets }, Some(arm)) => {
                        nets.iter().zip(arm).any(|(&n, av)| {
                            self.st.nets[n as usize].cur != *av
                                && self.st.last_blocking_writer[n as usize] != w.ready.proc
                        })
                    }
                    (WaitCause::Level { nets }, None) => nets.iter().any(|&n| {
                        changed_nets.contains(&n)
                            && self.st.last_blocking_writer[n as usize] != w.ready.proc
                    }),
                    _ => false,
                }
            }));
        }
        let mut woken: Vec<Ready> = Vec::new();
        let mut wi = 0usize;
        // WAITER-POOL p2: tally removed Expr/Level waiters here (the retain
        // closure cannot borrow `&mut self`), apply to the running counts after.
        // A removed Level waiter that RE-ARMS re-pushes via `arm_sensitivity`/
        // `suspend_on` on resume → the count re-increments there; a consumed
        // Expr/Edge does not re-push, so the decrement is net-correct.
        let mut removed_expr = 0usize;
        let mut removed_level = 0usize;
        self.waiters.retain(|w| {
            let keep = match &w.cause {
                // Level + inferred-comb: fire per the pre-computed arm/any-change test.
                WaitCause::Level { .. } => !level_fire[wi],
                // GLITCH: an in-body `@(posedge x)` fires from the intra-slot
                // mask, so a clock pulse that glitches back wakes the waiter.
                // SELF-RETRIG: but not on a net this waiter's own process wrote.
                WaitCause::Edge { net, kind } => !edges.iter().any(|&(en, mask, writer)| {
                    en == *net && edge_fires_slot(mask, *kind) && writer != w.ready.proc
                }),
                // wait(expr): consume + resume only when the predicate is now true.
                WaitCause::Expr { .. } => !expr_now[wi],
                _ => true,
            };
            if !keep {
                woken.push(w.ready); // re-armed on resume (Level/Expr) or consumed (Edge)
                match &w.cause {
                    WaitCause::Expr { .. } => removed_expr += 1,
                    WaitCause::Level { .. } => removed_level += 1,
                    _ => {}
                }
            }
            wi += 1;
            keep
        });
        self.n_expr_waiters -= removed_expr;
        self.n_level_waiters -= removed_level;
        for r in woken {
            push_sorted(&mut self.cur.active, r);
        }

        // (c) refresh prev LAST, now that every edge observer has run.
        // Per-field clone_from reuses prev's Vec allocations (a whole-struct
        // clone allocated two fresh Vecs per changed net per delta).
        for &net in &changed_nets {
            let slot = &mut self.st.nets[net as usize];
            slot.prev.val.clone_from(&slot.cur.val);
            slot.prev.unk.clone_from(&slot.cur.unk);
        }
        self.scratch_changed = changed_nets;
        self.scratch_edges = edges;
        expr_now.clear();
        self.scratch_expr_now = expr_now; // WAITER-POOL: hand the capacity back
        level_fire.clear();
        self.scratch_level_fire = level_fire;
    }

    /// P2-3: every delta-limit overflow path (t0/run-loop settle, the
    /// interpreter's in-body activation guard via `mark_fatal`, the VM guard via
    /// `k_mark_fatal`) funnels here — emit ONE `F-RUN-NO-CONVERGE` diagnostic
    /// (was: exit 1 with zero diagnostic lines) and flag the fatal exit class.
    /// The `had_fatal` check keeps it single-shot per run.
    pub(crate) fn fatal_delta_limit(&mut self) {
        if !self.st.had_fatal {
            use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
            self.st.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Fatal,
                code: MsgCode::RunNoConverge,
                message: format!(
                    "did not converge: delta limit ({}) exceeded at time {} \
                     (zero-delay loop / combinational oscillation)",
                    self.max_deltas, self.st.now
                ),
                location: None,
                context: Vec::new(),
                sim_time: Some(TimeStamp { ticks: self.st.now }),
            }));
        }
        self.st.had_fatal = true;
    }

    // ── expression eval + executor-facing API (called from exec.rs) ──────

    pub(crate) fn eval(&self, eid: u32) -> Value {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.eval(eid)
    }

    /// Truthiness of an already-computed value, through the same `truthiness` rule as
    /// [`Self::truthy`] — the native branch-condition path uses this so the tri-valued
    /// X/Z decision is never reimplemented on the compiled side.
    pub(crate) fn truthy_value(&self, v: &Value) -> bool {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        matches!(ctx.truthiness(v), crate::eval::Tri::True)
    }

    pub(crate) fn truthy(&self, eid: u32) -> bool {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.truthy(eid)
    }

    /// Evaluate `rhs` in the context of `lhs`'s width (IEEE assignment rule):
    /// width = max(lhs_width, self_width(rhs)); sign = rhs self-sign (lhs sign
    /// does NOT propagate).
    /// Value of continuous assign `ci`'s RHS, using its pre-compiled native program when
    /// one exists and falling back to [`Self::eval_for_lvalue`] otherwise.
    ///
    /// The program was compiled in exactly the context this function's fallback builds,
    /// so the two are the same (context, expression) pairing whose equivalence the P5
    /// gate already locks for process bodies — this only extends it to the one evaluation
    /// the bytecode backend never reached, because a continuous assign is not a body.
    pub(crate) fn eval_cont_assign(&self, ci: usize, lhs: &Lvalue, rhs: u32) -> Value {
        match self.ca_native.get(ci).and_then(Option::as_ref) {
            Some(p) => self.eval_native(p),
            None => self.eval_for_lvalue(lhs, rhs),
        }
    }

    pub(crate) fn eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value {
        let lw = self.st.lvalue_width(lhs);
        let sw = self.st.wt.get(rhs);
        let ctx_w = lw.max(sw.width);
        self.eval_ctx_top(rhs, ctx_w, sw.signed)
    }

    /// Evaluate each LHS chunk's bit-offset expression NOW (read-only EvalCtx),
    /// returning one offset per chunk (0 for a whole-net `None` chunk). The
    /// `&mut self` write path has no EvalCtx, so dynamic indices like `a[i]` are
    /// resolved here at the correct sampling moment. An X/Z or unresolvable index
    /// yields the out-of-range sentinel → `write_chunk` drops the bit (out-of-range
    /// no-op), matching the READ side where `eval_select` returns X for `a[x]`.
    ///
    /// The RULE itself lives in `eval::resolve_offsets`, shared with the tier-3
    /// arena write funnel — this is the engine's binding of it (see that function
    /// on why there must be exactly one spelling).
    pub(crate) fn resolve_lvalue_offsets(&self, lhs: &Lvalue) -> Offsets {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        crate::eval::resolve_offsets(&ctx, lhs)
    }

    /// Evaluate a `Terminator::Delay` amount (format_version 4: an ExprId of
    /// the raw delay value in module units) into global-precision ticks:
    /// real → round(v × M) clamped at 0; any X/Z → 0 (iverilog parity);
    /// integral → v × M, u64-saturating. `M` = the CURRENT process's
    /// timescale multiplier (`cur_time_mult`, set per activation) — exactly
    /// the scaling the old const-fold path applied at elaborate time.
    pub(crate) fn delay_ticks(&self, eid: u32) -> u64 {
        let v = self.eval(eid);
        crate::eval::delay_ticks_of(&v, self.st.cur_time_mult, self.st.cur_prec_mult)
    }

    /// Run a pre-compiled native expression program against the net table (VM-only
    /// fast path). `self.st` is the same `NetReader` `eval_ctx_top` builds its EvalCtx
    /// over, so a native leaf load reads exactly what the interpreter would.
    pub(crate) fn eval_native(&self, prog: &crate::native_eval::NativeProg) -> Value {
        // The evaluation stacks are a REUSED scratch, not a fresh frame-local pair — see
        // `NativeScratch`. `&self` here (this is the read path), so the scratch is
        // interior-mutable. `try_borrow_mut` is not a fallback for correctness but for
        // re-entrancy: nothing today evaluates a native program from inside another one
        // (a native leaf load only reads the net table), and if that ever changes the
        // inner run gets its own stacks instead of corrupting the outer one's.
        match self.native_scratch.try_borrow_mut() {
            Ok(mut sc) => crate::native_eval::run(prog, self.st, &mut sc),
            Err(_) => {
                let mut own = crate::native_eval::NativeScratch::default();
                crate::native_eval::run(prog, self.st, &mut own)
            }
        }
    }

    /// V2A-frame (§4.5.173): split a frame task call's `in_binds` into SCALAR copy-in
    /// pairs `(slot, Value)` and DYNAMIC-array snapshot pairs `(slot, caller src net)`.
    /// A formal whose frame net is a `DynArray` is pass-by-VALUE — its in-bind ExprId is
    /// a bare `Signal` reading the caller's dyn-array net (elaborate contract), from which
    /// we recover the source net; the caller applies `frame_dyn_snapshot_formals` right
    /// after `enter_task_frame`. Every non-dyn formal evaluates to a scalar in the caller
    /// context (unchanged). No dyn formals ⇒ `dyn_snaps` empty, `in_v` == the old vector.
    pub(crate) fn split_frame_in_binds(&self, info: &crate::TaskCallInfo) -> FrameInBinds {
        let cm = self.st.func_table[info.callee as usize];
        let mut in_v: Vec<(u32, Value)> = Vec::with_capacity(info.in_binds.len());
        let mut dyn_snaps: Vec<(u32, u32)> = Vec::new();
        for &(slot, e) in &info.in_binds {
            let fnet = (cm.base_net + slot) as usize;
            if self.st.ir.nets[fnet].kind == sim_ir::NetKind::DynArray {
                if let sim_ir::Expr::Signal { net, .. } = &self.st.ir.exprs[e as usize] {
                    dyn_snaps.push((slot, *net));
                }
            } else {
                let nv = &self.st.ir.nets[fnet];
                let sw = self.st.wt.get(e);
                let v = self.eval_ctx_top(e, nv.width.max(1).max(sw.width), nv.signed);
                in_v.push((slot, v));
            }
        }
        (in_v, dyn_snaps)
    }

    /// Build an EvalCtx and run eval_ctx (mirror of the `eval` façade).
    pub(crate) fn eval_ctx_top(&self, eid: u32, ctx_width: u32, ctx_signed: bool) -> Value {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.eval_ctx(eid, ctx_width, ctx_signed)
    }

    /// v5 ⑤: READ-phase assoc-key resolution (the scheduler-side mirror of
    /// `EvalCtx::assoc_key` — one recipe, two entry points: lvalue offsets
    /// here, rvalue reads in the eval arm).
    pub(crate) fn assoc_key_of(&self, eid: u32) -> Option<i64> {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.assoc_key(eid)
    }

    /// v6: the byte-string twin of `assoc_key_of` (string-keyed assoc).
    pub(crate) fn assoc_str_key_of(&self, eid: u32) -> Option<Vec<u8>> {
        let ctx = EvalCtx {
            ir: self.st.ir,
            nets: self.st,
            now: self.st.now,
            wt: &self.st.wt,
            time_mult: self.st.cur_time_mult,
            rng: &self.st.rng,
            plusargs: &self.st.plusargs,
        };
        ctx.assoc_str_key(eid)
    }

    /// v6: one assoc-iteration step (`first`/`next`/`last`/`prev`) — the
    /// WRITE-phase body behind `k_assoc_iter`. Returns the int status
    /// (1 found / 0 none / −1 found-but-truncated, hand-IEEE §7.9.4). On a
    /// hit the ref key variable is written through the NORMAL lvalue funnel
    /// (so `@(k)` sensitivity and VCD see it like any blocking assign). On
    /// dyn/queue handles the walk is the DENSE 0..size-1 order (the internal
    /// `foreach` desugar target).
    pub(crate) fn assoc_iter_step(&mut self, rhs: u32) -> i32 {
        // §4.5.176: the read/compute half is shared with the `&self` frame executors
        // (`SimState::assoc_iter_compute`) so both paths locate the key identically; the
        // process path writes the key through the normal `&mut` lvalue funnel (dirty
        // channel included, so `@(k)` sensitivity + VCD see it like any blocking assign).
        let (key_write, status) = self.st.assoc_iter_compute(rhs);
        if let Some((knet, kval)) = key_write {
            let klv = sim_ir::Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net: knet,
                    word: None,
                    offset: None,
                    width: None,
                    kind: sim_ir::SelKind::Bit,
                }],
            };
            self.st.write_lvalue(
                &klv,
                kval,
                &Offsets::Inline {
                    buf: [(0, 0); 2],
                    len: 1,
                },
            );
        }
        status
    }

    pub(crate) fn now(&self) -> u64 {
        self.st.now
    }

    /// The BODY-STEP budget — how many block steps one activation may take without
    /// suspending. Deliberately not `max_deltas`: see `SimOpts::max_body_steps`.
    pub(crate) fn max_deltas_guard(&self) -> u64 {
        self.max_body_steps
    }

    /// The in-body step guard fired (interpreter loop + VM `k_mark_fatal`).
    ///
    /// NOT the same condition as `fatal_delta_limit`, which is the scheduler failing to
    /// reach a fixpoint. This one only knows that a single activation ran a long time
    /// without suspending, so it says that and nothing more — it used to borrow the
    /// converge message and assert "zero-delay loop / combinational oscillation" about
    /// a loop that had neither.
    pub(crate) fn mark_fatal(&mut self) {
        if self.st.had_fatal {
            self.st.had_fatal = true;
            return;
        }
        use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
        self.st.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Fatal,
            code: MsgCode::RunBodyStepLimit,
            message: format!(
                "one process executed {} block steps at time {} without reaching a \
                 `#delay`, `@(…)` or `wait` — either it is an unbounded loop, or it is a \
                 long computation that needs a larger budget (`SimOpts::max_body_steps`)",
                self.max_body_steps, self.st.now
            ),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.st.now }),
        }));
        self.st.had_fatal = true;
    }

    pub(crate) fn schedule_resume(&mut self, proc: u32, block: u32, tick: u64, inactive: bool) {
        // `proc` is an activity id; read its deterministic tie (NOT the id) so two
        // sibling children land in distinct-tie wheel slots in declaration order.
        let tie = self.activities[proc as usize].tie;
        let ready = Ready { tie, proc, block };
        if tick == self.st.now {
            if inactive {
                push_sorted(&mut self.cur.inactive, ready);
            } else {
                push_sorted(&mut self.cur.active, ready);
            }
        } else {
            let region = if inactive {
                RegionTag::Inactive
            } else {
                RegionTag::Active
            };
            // Reuse a pooled bucket on first insert at a new tick (or_default
            // would allocate a fresh Vec per distinct simulation time).
            let pool = &mut self.bucket_pool;
            self.wheel
                .entry(tick)
                .or_insert_with(|| pool.pop().unwrap_or_default())
                .push((region, ready));
        }
    }

    /// Suspend a process on an in-body `@(...)`/`wait(...)`. (The `wait(expr)`
    /// already-true short-circuit is handled in the executor.)
    ///
    /// An in-body `Wait{Edge}` is registered ONLY as a `waiter`, NOT in
    /// `net_to_edge`. `waiters` entries are consumed when they fire
    /// (`propagate_changes` `retain`s them out); `net_to_edge` entries are
    /// permanent. Registering in both would leave an orphan `net_to_edge` entry
    /// that re-fires the process on every future edge of that net for the rest of
    /// the sim (resuming a block it already passed) — an unbounded leak and a
    /// correctness bug for any process that loops back through the same wait. The
    /// `waiters` Edge arm in `propagate_changes` performs the exact same edge test.
    pub(crate) fn suspend_on(&mut self, proc: u32, block: u32, cause: WaitCause) {
        // `proc` is an activity id; carry its distinct tie so two siblings waiting
        // on the same event are distinguishable (neither lost nor double-counted).
        let tie = self.activities[proc as usize].tie;
        let ready = Ready { tie, proc, block };
        // Snapshot the watched nets so an in-body `@(sig)` fires on the next change
        // AFTER this point, not on one already applied this delta before it armed.
        let arm = match &cause {
            WaitCause::Level { nets } => Some(
                nets.iter()
                    .map(|&n| self.st.nets[n as usize].cur.clone())
                    .collect(),
            ),
            _ => None,
        };
        // WAITER-POOL p2: keep the running counts exact at this push site.
        match &cause {
            WaitCause::Expr { .. } => self.n_expr_waiters += 1,
            WaitCause::Level { .. } => self.n_level_waiters += 1,
            _ => {}
        }
        self.waiters.push(Waiter { cause, ready, arm });
    }

    /// v5 increment (A): a transport NBA — index + value sampled NOW, update
    /// due in the NBA region of `now + ticks` (saturating).
    pub(crate) fn schedule_nba_at(&mut self, lhs: &Lvalue, sampled: Value, ticks: u64) {
        let offsets = self.resolve_lvalue_offsets(lhs);
        let seq = self.nba_seq;
        self.nba_seq += 1;
        let at = self.st.now.saturating_add(ticks);
        self.delayed_nba.entry(at).or_default().push(NbaUpdate {
            seq,
            lhs: NbaLhs::of(lhs),
            sampled,
            offsets,
        });
    }

    /// `schedule_nba` for a destination the COMPILER proved is a plain whole-net scalar.
    ///
    /// The only thing the general form does that this skips is `resolve_lvalue_offsets`,
    /// and for this shape that call is a constant: no `word` and no `offset` expression
    /// means `pair()` yields `(0, 0)`, one chunk means `Inline { len: 1 }`. Producing that
    /// constant 2.5 million times is what this removes. The queue entry is likewise
    /// unconditionally `NbaLhs::One` — the shape test inside `NbaLhs::of` is the same test
    /// the compiler already made.
    pub(crate) fn schedule_nba_scalar(&mut self, lhs: &Lvalue, sampled: Value) {
        debug_assert_eq!(
            self.resolve_lvalue_offsets(lhs).as_slice(),
            &[(0, 0)],
            "specialised NBA destination must resolve to the constant offsets"
        );
        let seq = self.nba_seq;
        self.nba_seq += 1;
        let chunk = lhs.chunks[0].clone();
        self.nba.push(NbaUpdate {
            seq,
            lhs: NbaLhs::One(chunk),
            sampled,
            offsets: crate::exec::Offsets::Inline {
                buf: [(0, 0); 2],
                len: 1,
            },
        });
    }

    pub(crate) fn schedule_nba(&mut self, lhs: &Lvalue, sampled: Value) {
        // Sample the dynamic LHS index NOW (Active region), BEFORE the mutable
        // push, so a later same-step write to the index net cannot move the target.
        let offsets = self.resolve_lvalue_offsets(lhs);
        let seq = self.nba_seq;
        self.nba_seq += 1;
        self.nba.push(NbaUpdate {
            seq,
            lhs: NbaLhs::of(lhs),
            sampled,
            offsets,
        });
    }

    /// Re-arm a process after it Returns. The Edge/Level asymmetry is
    /// LOAD-BEARING (verified):
    /// - `SensKind::Edge` registers a *permanent* entry in `net_to_edge` that is
    ///   read-not-consumed on fire (`propagate_changes` clones, never removes).
    ///   Re-pushing it on every Return would duplicate the registration → the
    ///   process fires 2^k times on edge k (wrong results for blocking edge
    ///   bodies + a 2^k termination hazard). So Edge MUST NOT re-arm.
    /// - `SensKind::Level` waiters ARE consumed on fire (`waiters.retain` returns
    ///   `false`), so Level MUST re-arm or it would never wake again — and the
    ///   `infinite_delta_guard_trips` test depends on this re-registration.
    /// - `Initial` is one-shot (dead after its single run).
    pub(crate) fn rearm(&mut self, proc: u32) {
        // Fork children NEVER re-arm: a child's reaching its join is a one-shot
        // completion, routed by the run_process loop-top intercept to
        // on_child_complete. (A child has no static sensitivity of its own anyway.)
        if self.activities[proc as usize].is_child {
            return;
        }
        let tmpl = self.activities[proc as usize].template as usize;
        let kind = self.st.ir.processes[tmpl].sensitivity.kind;
        match kind {
            // permanent net_to_edge entry / one-shot: do NOT re-register.
            SensKind::Edge | SensKind::Initial => {}
            // consumed waiter: must re-register to wake on the next change.
            SensKind::Comb | SensKind::Latch | SensKind::Level => self.arm_sensitivity(proc),
        }
    }

    // ── fork/join support (activity arena + join barriers) ────────────────

    /// Recover a fork's join mode from the elaborate side table. TOTAL-OR-FATAL:
    /// a miss is impossible after the `arm_processes` gate, but we never default to
    /// a blocking join — a fabricated `All` would silently turn a lost-side-channel
    /// `join_none`/`join_any` into a deadlock with no diagnostic. Panic instead.
    pub(crate) fn fork_mode(&self, template: u32, join_bb: u32) -> Option<JoinMode> {
        self.fork_modes.get(&(template, join_bb)).copied()
    }

    /// P1-7: a `Terminator::Fork` with no ForkModeTable entry means the `.velab`
    /// fork-mode trailer was lost/stale (it rides OUTSIDE the schema gate, so a
    /// hand-truncated artifact can reach here). Fabricating a mode would silently
    /// turn a `join_none` into a deadlock — emit a FATAL artifact diagnostic and
    /// end the run instead (was: panic!).
    /// A declaration-initializer ProcId the sidecar names but the IR does not have.
    /// Same class as `fatal_fork_mode_missing`: the trailer rides outside the schema gate,
    /// so a truncated `.velab` can reach here, and skipping it would silently drop the
    /// design's initializers.
    pub(crate) fn fatal_init_proc_missing(&mut self, pid: u32) {
        use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
        self.st.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Fatal,
            code: MsgCode::ArtFormatMismatch,
            message: format!(
                "declaration-initializer sidecar names process {pid}, which this IR does \
                 not have — .velab trailer lost or stale; re-run velab"
            ),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.st.now }),
        }));
        self.st.had_fatal = true;
        self.st.finished = true;
    }

    pub(crate) fn fatal_fork_mode_missing(&mut self, template: u32, join_bb: u32) {
        use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
        self.st.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Fatal,
            code: MsgCode::ArtFormatMismatch,
            message: format!(
                "fork join-mode sidecar entry missing for (template={template}, \
                 join_bb={join_bb}) — .velab trailer lost or stale; re-run velab"
            ),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.st.now }),
        }));
        self.st.had_fatal = true;
        self.st.finished = true;
    }

    /// Accessors the executor's loop-top intercept + body fetch use (the
    /// `activities`/`barriers` fields are private to sched.rs).
    pub(crate) fn activity_template(&self, aid: u32) -> u32 {
        self.activities[aid as usize].template
    }
    pub(crate) fn activity_is_child(&self, aid: u32) -> bool {
        self.activities[aid as usize].is_child
    }
    /// The barrier's completion-sentinel join_bb (for the loop-top intercept).
    pub(crate) fn barrier_join_bb(&self, jr: u32) -> u32 {
        self.barriers[jr as usize].join_bb
    }
    pub(crate) fn activity_join_ref(&self, aid: u32) -> Option<u32> {
        self.activities[aid as usize].join_ref
    }

    /// Debug-only: assert no LIVE barrier (same template) has its join_bb equal to
    /// `bb` for a non-child activity. A non-child fetching a live join_bb would mean
    /// the parent walked the never-executed sentinel — a builder/engine bug.
    #[cfg(debug_assertions)]
    pub(crate) fn assert_not_parent_at_join(&self, aid: u32, bb: u32) {
        if self.activities[aid as usize].is_child {
            return;
        }
        let tmpl = self.activities[aid as usize].template;
        let bad = self.barriers.iter().any(|b| {
            !b.fired && b.join_bb == bb && self.activities[b.parent as usize].template == tmpl
        });
        debug_assert!(
            !bad,
            "parent/top-level activity {aid} fetched a live barrier join_bb sentinel {bb}"
        );
    }

    /// Execute a `Terminator::Fork`: register the barrier, spawn each child as a new
    /// activity (sharing the parent's template, entering at its own child-entry BB),
    /// and either suspend the parent (All/Any with ≥1 child) or fall through to
    /// `resume_bb` (None, or zero children). Returns `Some(resume_bb)` when the
    /// parent continues THIS activation (the executor sets `bb = resume_bb`), or
    /// `None` when the parent suspends on the barrier.
    pub(crate) fn exec_fork(
        &mut self,
        parent_aid: u32,
        children: &[u32],
        join: u32,
        resume_bb: u32,
    ) -> Option<u32> {
        let parent_tmpl = self.activities[parent_aid as usize].template;
        // Stage-1 fork-in-frame: is the parent forking from INSIDE a suspendable task
        // frame? Then each child runs one fork ARM as its own in-frame activity, and the
        // fork's mode is keyed by the frame sentinel (its join_bb is a globally-unique
        // func_blocks id, independent of which process runs the task). `arm_callee` is the
        // enclosing task's FuncId, whose CFG the arm blocks live in.
        let parent_in_frame = !self.activities[parent_aid as usize].call_stack.is_empty();
        let arm_callee = if parent_in_frame {
            self.activities[parent_aid as usize]
                .call_stack
                .last()
                .unwrap()
                .callee
        } else {
            0
        };
        let mode_key = if parent_in_frame {
            elaborate::FRAME_FORK_KEY
        } else {
            parent_tmpl
        };
        // P1-7: missing sidecar entry → graceful FATAL stop (never a default mode).
        let Some(mode) = self.fork_mode(mode_key, join) else {
            self.fatal_fork_mode_missing(mode_key, join);
            return None; // parent parks; run loop sees `finished` and ends the run
        };

        // Register the barrier, recycling a drained slot when available (P3-1).
        let barrier = JoinBarrier {
            parent: parent_aid,
            join_bb: join,
            resume_bb,
            mode,
            outstanding: children.len() as u32,
            fired: false,
        };
        let join_ref = match self.free_barriers.pop() {
            Some(id) => {
                self.barriers[id as usize] = barrier;
                id
            }
            None => {
                self.barriers.push(barrier);
                (self.barriers.len() - 1) as u32
            }
        };

        // Spawn each child as a NEW activity. Deterministic: declaration order ==
        // the order of `children`; each child's tie composes parent.tie + child idx.
        // NOTE: nested fork is an elaborate ERROR, so `parent_tmpl` here is always a
        // TOP-LEVEL process and `parent.tie` its dense top-level declaration index —
        // true whether the parent forks from its base process body or from INSIDE a
        // suspendable task frame (the activity running the frame is still top-level).
        let parent_tie = self.activities[parent_aid as usize].tie;
        // FORK-TIE-CAP: `compose_child_tie` packs `(parent_tie + 1)` into the high
        // 16 bits and the child index into the low 16. Past ~65535 top-level
        // processes the high half overflows the u32 (`(parent_tie+1) << 16`); past
        // ~65536 arms the low half aliases (`child_idx & 0xFFFF`). Either silently
        // collides sibling ties → nondeterministic ordering. Reject loud (graceful
        // fatal) — both limits are far above any real testbench.
        if parent_tie >= 0xFFFF || children.len() > 0x1_0000 {
            self.st.fatal_run(&format!(
                "fork exceeds the v1 tie-encoding limit (parent process tie {parent_tie}, \
                 {} arms); deterministic ordering supports \u{2264} 65534 top-level \
                 processes and \u{2264} 65536 arms per fork",
                children.len()
            ));
            return None; // parent parks; run loop sees `finished` and ends the run
        }
        // Stage-2 fork-in-frame: the arm window follows the PARENT's just-stashed window
        // (the Fork terminator arm stashed it into the parent's top `FrameRec.window`
        // immediately before this call). Case B → the parent's window is a `Shared(h)`, so
        // each arm shares the SAME handle `h` (parent + arms reference one arena window);
        // Case A → an EMPTY `Owned` window (the arm touches no parent frame slot). `None`
        // for a static enclosing task (its locals live in `static_store`, not `frame_stack`
        // — so no window rides the stack, matching `stash`/`exit_arm_frame`'s `func_has_auto`
        // gate). Computed ONCE (all arms share the parent's kind).
        let arm_window_kind: Option<u32> =
            if parent_in_frame && self.st.func_has_auto[arm_callee as usize] {
                match &self.activities[parent_aid as usize]
                    .call_stack
                    .last()
                    .unwrap()
                    .window
                {
                    Some(crate::state::WindowSlot::Shared(h)) => Some(*h), // Case B: share handle
                    _ => None, // Case A: empty owned (handled below)
                }
            } else {
                None
            };
        for (child_idx, &child_entry) in children.iter().enumerate() {
            let child_tie = compose_child_tie(parent_tie, child_idx as u32);
            // Build the arm's synthetic frame. The child dies at `join` via the loop-top
            // intercept, so `ret_bb` is unused for teardown.
            let child_call_stack = if parent_in_frame {
                let window = if !self.st.func_has_auto[arm_callee as usize] {
                    None // static enclosing task: no window on the stack
                } else if let Some(h) = arm_window_kind {
                    // Case B: this arm aliases the parent's arena window. Stage 3: take a
                    // reference so the window is not freed while the arm is live — under
                    // `join_any`/`join_none` a surviving arm may outlive the parent's
                    // `Return`, and this refcount is what keeps its window valid. Balanced by
                    // the arm's `exit_arm_frame` release at completion. (For `join` the parent
                    // outlives every arm, so retain/release here is a no-op net-of-parent.)
                    self.st.retain_frame_window(h);
                    Some(crate::state::WindowSlot::Shared(h))
                } else {
                    Some(crate::state::WindowSlot::Owned(Vec::new())) // Case A: empty owned
                };
                vec![crate::sched::FrameRec {
                    callee: arm_callee,
                    bb: child_entry,
                    ret_bb: join,
                    out_binds: Vec::new(),
                    window,
                    // T1-9: a fork ARM frame is not a fresh ACTIVATION of the callee — it
                    // rides the parent's window and the parent's dyn slots, and its own
                    // teardown (`exit_arm_frame`) does not go through the Return handler.
                    // It therefore takes nothing and restores nothing.
                    dyn_stash: Vec::new(),
                    dyn_parked: Vec::new(),
                    forked: false,
                    // The ONE site that builds an arm frame — every other `FrameRec` is a
                    // callee pushed by a `Call`. See `FrameRec::is_arm`.
                    is_arm: true,
                }]
            } else {
                Vec::new()
            };
            let child = Activity {
                call_stack: child_call_stack,
                template: parent_tmpl,
                tie: child_tie,
                join_ref: Some(join_ref),
                is_child: true,
                reported: false,
                dead: false,
                wait_fork: None,
                busy: false,
                gen: 0,
            };
            // Recycle a completed child slot when available (P3-1): a freed slot's
            // old activity has reported (it cannot be queued/waiting anywhere).
            // BUMP the generation on reuse so a §16.4 deferred report still pending
            // under the OLD incarnation's `(marker, aid, gen)` key is not flushed
            // by this fresh incarnation reaching the same marker.
            let child_aid = match self.free_activities.pop() {
                Some(id) => {
                    let next_gen = self.activities[id as usize].gen.wrapping_add(1);
                    self.activities[id as usize] = Activity {
                        gen: next_gen,
                        ..child
                    };
                    id
                }
                None => {
                    self.activities.push(child);
                    (self.activities.len() - 1) as u32
                }
            };
            // Make the child runnable NOW (same instant, Active region); push_sorted
            // by the composed tie keeps siblings in declaration order.
            push_sorted(
                &mut self.cur.active,
                Ready {
                    tie: child_tie,
                    proc: child_aid,
                    block: child_entry,
                },
            );
        }

        match mode {
            JoinMode::None => {
                // join_none: parent does NOT block. Mark fired (never resumes via the
                // barrier) and continue at resume_bb THIS instant; children run on as
                // background activities concurrently with the continuation.
                self.barriers[join_ref as usize].fired = true;
                Some(resume_bb)
            }
            JoinMode::All | JoinMode::Any => {
                if children.is_empty() {
                    // fork join / fork join_any with zero children: resume now.
                    self.barriers[join_ref as usize].fired = true;
                    Some(resume_bb)
                } else {
                    // Parent blocks on the join. It holds NO cur/wheel/waiter entry —
                    // it is parked solely by the barrier and re-enqueued by
                    // on_child_complete when the join condition fires.
                    None
                }
            }
        }
    }
}
