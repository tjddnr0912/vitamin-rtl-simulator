//! split part of `sched` (mechanical move).

use super::*;

impl Scheduler<'_, '_> {
    pub fn run(&mut self) -> FinishReason {
        // N4 clocking: seed the preponed snapshot for the FIRST time slot (t=0) —
        // a clocking edge at t=0 then samples the init values. No-op without
        // clocking blocks ⇒ byte-identical.
        self.st.snapshot_preponed();
        loop {
            if self.st.finished {
                return self.finish_kind();
            }
            // Drain the current time to a stable point.
            self.delta_count = 0;
            loop {
                // B1: a frame-call runaway latched during a prior region's eval
                // (or the settle just below) surfaces here — every region action
                // `continue`s back to this loop top, so one check covers them all.
                if self.check_call_fatal() {
                    return FinishReason::Error;
                }
                // ACTIVE: continuous assigns settle, then drain processes.
                match self.settle_cont_assigns() {
                    None => return FinishReason::DeltaLimit, // cont-assign oscillator
                    // A settle that moved nets may have produced an EDGE on a
                    // cont-assign-driven net (a port-bound clock). Run change
                    // propagation so those edges/level-waiters fire before we
                    // decide the timestep is stable.
                    Some(true) => self.propagate_changes(),
                    Some(false) => {}
                }
                // A cont-assign RHS (`assign y = deep_recursive_fn(x);`) can latch
                // the fatal during settle, then `break` out below if all region
                // buckets are empty — so it MUST be caught HERE, before the break.
                if self.check_call_fatal() {
                    return FinishReason::Error;
                }
                if !self.cur.active.is_empty() {
                    // Take the batch so wakes triggered DURING it land in a fresh
                    // `cur.active`; iterate borrowed (`Ready: Copy`) so the Vec can
                    // be handed back below — consuming it dropped one allocation
                    // per delta.
                    let mut batch = std::mem::take(&mut self.cur.active);
                    for &r in &batch {
                        if self.st.finished {
                            return self.finish_kind();
                        }
                        // B1: a runaway latched in a PRIOR process body of THIS
                        // batch ends the run before the next body can read a
                        // corrupt static slab (the residue is never observed).
                        if self.check_call_fatal() {
                            return FinishReason::Error;
                        }
                        let step = self.run_body(r.proc, r.block);
                        // R22 §4: consume a fatal latched BY THIS BODY, before the step it
                        // returned is honored. A frame-body fatal is raised from a `&self`
                        // eval context deep inside an expression, so it cannot return
                        // `Step::Fatal` itself — it sets the `call_fatal` Cell instead. The
                        // three seams above poll that Cell only BEFORE running a body, so a
                        // body that latched a fatal and then reached its own `$finish`
                        // returned `Step::Finish` and the run ended as a clean Finish: the
                        // diagnostic was printed, the simulation carried on past it, and
                        // every judgement after it was made on state the fatal had already
                        // declared invalid. Chronology decides — the fatal happened first,
                        // inside the body, so it wins over a `$finish` reached afterwards.
                        if self.check_call_fatal() {
                            return FinishReason::Error;
                        }
                        match step {
                            // P1-6 (IEEE 1364-2005 §5.4/§17): drain the CURRENT
                            // timestep's postponed region ($strobe/$monitor) before
                            // terminating — Icarus/VCS parity. $fatal/$stop are
                            // $finish-class terminations and drain identically.
                            Step::Finish => {
                                self.st.finished = true;
                                self.drain_deferred_on_finish();
                                self.flush_postponed();
                                return FinishReason::Finish;
                            }
                            Step::Stop => {
                                self.st.finished = true;
                                self.drain_deferred_on_finish();
                                self.flush_postponed();
                                return FinishReason::Stop;
                            }
                            Step::Fatal => {
                                self.st.finished = true;
                                self.st.had_fatal = true;
                                self.drain_deferred_on_finish();
                                self.flush_postponed();
                                return FinishReason::Error;
                            }
                            // Track mid-body suspension so the permanent
                            // edge-sensitivity entry does not re-trigger a
                            // process that is still parked inside its body.
                            Step::Suspended => {
                                if let Some(a) = self.activities.get_mut(r.proc as usize) {
                                    a.busy = true;
                                }
                            }
                            Step::Done => {
                                if let Some(a) = self.activities.get_mut(r.proc as usize) {
                                    a.busy = false;
                                }
                            }
                        }
                    }
                    // Recycle the drained batch Vec when no process was woken
                    // mid-batch (the overwhelmingly common case).
                    batch.clear();
                    if self.cur.active.is_empty() {
                        self.cur.active = batch;
                    }
                    self.propagate_changes();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                // INACTIVE (#0): promote to Active.
                if !self.cur.inactive.is_empty() {
                    self.cur.active = std::mem::take(&mut self.cur.inactive);
                    // GATED-CLOCK: a #0 batch is a NEW event cluster — an edge it
                    // produces (e.g. an independent `negedge rst` scheduled via #0)
                    // must be able to re-fire a process already woken this timestep.
                    self.reset_edge_seen_marks();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                // NBA: apply the sampled batch.
                if !self.nba.is_empty() {
                    // GATED-CLOCK: the NBA region is a NEW event cluster — an edge
                    // an NBA update produces must re-fire a process already woken
                    // this timestep (an independent `rst <= 0` re-fires `@(… or
                    // negedge rst)`). Reset BEFORE the post-apply propagation.
                    self.reset_edge_seen_marks();
                    self.apply_nba();
                    self.propagate_changes();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                // OBSERVED (IEEE 1800 §4.4 / §16.4): `assert #0` deferred reports
                // mature here — Active/Inactive/NBA empty, cont-assigns at
                // fixpoint, nets settled, time NOT advanced. A matured action that
                // re-activates a process re-drains Active/NBA before Reactive (the
                // `continue` re-runs the region cascade from the top).
                if !self.st.postponed.deferred_observed.is_empty() {
                    if let Some(step) = self.mature_deferred(DeferRegion::Observed) {
                        self.st.finished = true;
                        if let Step::Fatal = step {
                            self.st.had_fatal = true;
                        }
                        self.flush_postponed();
                        return match step {
                            Step::Stop => FinishReason::Stop,
                            Step::Fatal => FinishReason::Error,
                            _ => FinishReason::Finish,
                        };
                    }
                    self.propagate_changes();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                // REACTIVE (IEEE 1800 §4.4 / §16.4): `assert final` deferred
                // reports mature AFTER Observed, BEFORE Postponed.
                if !self.st.postponed.deferred_reactive.is_empty() {
                    if let Some(step) = self.mature_deferred(DeferRegion::Reactive) {
                        self.st.finished = true;
                        if let Step::Fatal = step {
                            self.st.had_fatal = true;
                        }
                        self.flush_postponed();
                        return match step {
                            Step::Stop => FinishReason::Stop,
                            Step::Fatal => FinishReason::Error,
                            _ => FinishReason::Finish,
                        };
                    }
                    self.propagate_changes();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                // ⚠️ **A CHANGE THAT NOTHING HAS PROPAGATED YET.** Every ordinary
                // writer is followed by a `propagate_changes`, so `st.dirty` is
                // normally empty here — with ONE producer that is not: the
                // clocking commit runs INSIDE `propagate_changes` pass (a), after
                // that pass has already taken its changed set, so `cb.sig`'s
                // change is left for "the next propagate". Without this line
                // there is no next propagate in this timestep: the loop breaks,
                // POSTPONED runs, time advances, and the change is finally swept
                // in a LATER slot — carrying `slot_edge` from the slot it
                // happened in. Measured: `@(posedge cb.d)` armed at t=8 fired
                // immediately off the t=5 mask, and an `always @(cb.d)` woke a
                // timestep late.
                //
                // Found by the tier-3 flip run (slice #1). The tier-3 loop has
                // always had a `propagate` at its stable point — it needs one for
                // the deferred-region cascade — so it was already correct here,
                // and the two backends disagreed the moment clocking was
                // admitted. Guarded on `dirty` so every design without a
                // late producer is byte-identical.
                if !self.st.dirty.is_empty() {
                    self.propagate_changes();
                    self.delta_count += 1;
                    if self.delta_count > self.max_deltas {
                        self.fatal_delta_limit();
                        return FinishReason::DeltaLimit;
                    }
                    continue;
                }
                break; // time-step stable
            }

            // POSTPONED REGION (IEEE 1364-2005 §5.4): now == settled time, all
            // region buckets (Active/Inactive/NBA) empty, cont-assigns at
            // fixpoint, time NOT yet advanced. Reads settled `cur` net values.
            // Drain strobes (call order) then the monitor (print-on-change).
            self.flush_postponed();

            // Advance time to the next scheduled tick — the earliest of a wheel
            // event OR a pending transport-delay cont-assign write.
            let next = match [
                self.wheel.keys().next().copied(),
                self.delayed_ca.keys().next().copied(),
                self.delayed_nba.keys().next().copied(),
            ]
            .into_iter()
            .flatten()
            .min()
            {
                None => return FinishReason::Quiescent,
                Some(t) => t,
            };
            if let Some(lim) = self.time_limit {
                if next > lim {
                    return FinishReason::Quiescent;
                }
            }
            self.st.now = next;
            // GATED-CLOCK: a new timestep is a fresh edge-collapse cluster.
            self.reset_edge_seen_marks();
            // N4 clocking: take the PREPONED snapshot of clocking inputs at the
            // START of the new time slot (before any slot activity — the true
            // preponed value entering the slot). No-op when no clocking blocks ⇒
            // byte-identical for the whole non-clocking corpus.
            self.st.snapshot_preponed();
            // No prev snapshot needed here (R2): at the settled point every
            // mutation path has been swept — `propagate_changes` step (c)
            // already refreshed `prev` for every changed net, and unchanged
            // nets have `prev == cur` by induction (the only `prev` writers
            // are step (c) and the t0 constructor, both setting prev = cur).
            // The old full-net `snapshot_prev` was therefore a no-op pass
            // costing O(nets) per timestep (512 idle nets ≈ half the
            // nets-heavy wall-clock). Soundness is pinned by the byte-compare
            // suites (staged/threads/corpus/differential).
            // Apply inertial-delay cont-assign writes due at this tick; propagate
            // so edges/level-waiters on the delayed net fire (these are NET writes,
            // not process resumes, so the loop-top settle would not see them).
            // A write whose generation was superseded by a later RHS change is
            // STALE — dropped (the inertial cancel).
            {
                // The generation filter and the `last_ca_drv` baseline live in
                // `take_due_delayed_ca`, shared with the tier-3 loop; only the
                // write is per-store.
                let due = self.take_due_delayed_ca(next);
                let mut moved = false;
                for (lhs, v, offs) in due {
                    moved |= self.st.write_lvalue(&lhs, v, &offs);
                }
                if moved {
                    self.propagate_changes();
                }
            }
            // Transport NBAs due now join this tick's NBA region; the global
            // seq sort in `apply_nba` interleaves them with NBAs scheduled
            // during the tick in original statement order.
            self.take_due_delayed(next);
            let mut events = self.wheel.remove(&next).unwrap_or_default();
            for (region, ready) in events.drain(..) {
                match region {
                    RegionTag::Inactive => push_sorted(&mut self.cur.inactive, ready),
                    _ => push_sorted(&mut self.cur.active, ready),
                }
            }
            // Return the drained bucket to the pool for the next schedule().
            self.bucket_pool.push(events);
            // a fresh time may also need cont-assign settle before draining (loop top).
        }
    }

    pub(crate) fn finish_kind(&self) -> FinishReason {
        if self.st.had_fatal {
            FinishReason::Error
        } else {
            FinishReason::Finish
        }
    }

    /// B1 frame-call: surface a runaway-recursion fatal LATCHED on the `&self`
    /// read path. `eval_call` cannot return a `Step`, so it sets `call_fatal`
    /// (a `Cell`) and finishes its in-flight eval with X; the scheduler polls
    /// this at the region seams where an eval can have fired (cont-assign
    /// settle, a process body, a deferred-action arg, a Branch cond). On the
    /// first consume it mirrors the user-`$fatal` termination (drain deferred,
    /// flush postponed, latch finished/had_fatal) so the run ends before any
    /// subsequent call can read a corrupt static slab. Returns true once it is
    /// consumed (the caller then returns `FinishReason::Error`).
    pub(crate) fn check_call_fatal(&mut self) -> bool {
        if self.st.call_fatal.get() && !self.st.finished {
            self.st.finished = true;
            self.st.had_fatal = true;
            self.drain_deferred_on_finish();
            self.flush_postponed();
            return true;
        }
        false
    }

    // ── §16.4 deferred immediate assertions ───────────────────────────────

    /// `builtins::dispatch` calls this at its top for every SysTask. Returns
    /// `true` if the call was INTERCEPTED as a deferred-assert marker/action (so
    /// `dispatch` must do nothing further), `false` if it should run normally.
    ///
    /// A MARKER cancels any prior pending report for this assertion instance —
    /// keyed `(marker_sid, cur_aid, cur_gen)` so flush-on-re-reach hits only THIS
    /// activation, never a recycled-slot predecessor (§16.4). An ACTION renders
    /// its text NOW (reach-time arg values, §16.4.3) and enqueues/REPLACES it for
    /// region maturation.
    /// A8-b: [`Scheduler::try_defer`] against an ALTERNATE net store.
    ///
    /// ⭐ ONE line of this function is store-bound and it is the RENDER — §16.4.3
    /// says a deferred action's text is produced at REACH, with the argument
    /// values it sees there, so the message is a `String` by the time it is
    /// enqueued. Maturation (`mature_deferred`) therefore reads no net at all:
    /// it emits text that was already rendered.
    ///
    /// That split is why this row was conservative rather than deep. What tier-3
    /// lacked was the two REGIONS, not the machinery.
    ///
    /// `None` is the engine's own store and the render reduces to the call it
    /// made before, so that path is byte-identical by construction.
    pub(crate) fn try_defer_with<N: crate::eval::NetReader + ?Sized>(
        &mut self,
        nets: Option<&N>,
        which: sim_ir::SysTaskId,
        fmt: Option<u32>,
        args: &[u32],
        sid: u32,
    ) -> bool {
        if let Some(&region) = self.st.defer_marks.get(&sid) {
            let key = (sid, self.cur_aid, self.cur_gen);
            match region {
                DeferRegion::Observed => {
                    self.st.postponed.deferred_observed.remove(&key);
                }
                DeferRegion::Reactive => {
                    self.st.postponed.deferred_reactive.remove(&key);
                }
            }
            return true; // marker is a no-op (suppressed empty $display)
        }
        if let Some(&(marker, region)) = self.st.defer_acts.get(&sid) {
            // §16.4.3: render the action text NOW, at reach (sampling reach-time
            // arg values / `$time` / `%m`). A severity task renders with no
            // default radix (matching `run_severity_with`); a plain print uses its
            // b/o/h radix.
            let radix = if self.st.severities.contains_key(&sid) {
                None
            } else {
                self.st.radixes.get(&sid).copied()
            };
            let message = match nets {
                Some(n) => crate::builtins::format_args_str_with(&*self.st, n, fmt, args, radix),
                None => crate::builtins::format_args_str(&*self.st, fmt, args, radix),
            };
            let report = crate::state::DeferredReport {
                action_sid: sid,
                which,
                message,
            };
            let key = (marker, self.cur_aid, self.cur_gen);
            match region {
                DeferRegion::Observed => {
                    self.st.postponed.deferred_observed.insert(key, report);
                }
                DeferRegion::Reactive => {
                    self.st.postponed.deferred_reactive.insert(key, report);
                }
            }
            return true;
        }
        false
    }

    /// §16.4: drain ONE deferred-assert maturation queue, emitting each surviving
    /// pending report (text already rendered at reach). A severity action routes
    /// to the diagnostic stream + exit class ($fatal aborts); a plain print goes
    /// to stdout. Deterministic `(marker, aid, gen)` BTreeMap order. Returns
    /// `Some(step)` if a deferred `$fatal` matured.
    pub(crate) fn mature_deferred(&mut self, region: DeferRegion) -> Option<Step> {
        let map = match region {
            DeferRegion::Observed => std::mem::take(&mut self.st.postponed.deferred_observed),
            DeferRegion::Reactive => std::mem::take(&mut self.st.postponed.deferred_reactive),
        };
        if map.is_empty() {
            return None;
        }
        let mut term: Option<Step> = None;
        for (_key, rpt) in map {
            if let Some(sev) = self.st.severities.get(&rpt.action_sid).copied() {
                match crate::builtins::emit_severity_message(self, sev, rpt.message) {
                    crate::builtins::Ctl::Fatal => {
                        term = Some(Step::Fatal);
                        break;
                    }
                    crate::builtins::Ctl::Finish => {
                        term = Some(Step::Finish);
                        break;
                    }
                    crate::builtins::Ctl::Stop => {
                        term = Some(Step::Stop);
                        break;
                    }
                    crate::builtins::Ctl::Continue => {}
                }
            } else {
                // Plain $display/$write deferred action: stdout, newline for the
                // Display family ($write keeps none).
                let mut line = rpt.message;
                if !matches!(rpt.which, sim_ir::SysTaskId::Write) {
                    line.push('\n');
                }
                write_out(self.st, &line);
            }
        }
        term
    }

    /// §16.4 termination drain: mature the current slot's pending deferred
    /// reports (Observed then Reactive) before a `$finish`/`$stop`/`$fatal`
    /// exit, mirroring the postponed drain — a verdict already evaluated this
    /// slot is not lost. Any further `$fatal` is ignored (already terminating).
    pub(crate) fn drain_deferred_on_finish(&mut self) {
        let _ = self.mature_deferred(DeferRegion::Observed);
        let _ = self.mature_deferred(DeferRegion::Reactive);
    }

    // ── postponed region ($strobe FIFO drain + $monitor change-detect) ─────

    /// IEEE 1364-2005 §5.4 postponed region. Called at the settled point of each
    /// timestep (Active/Inactive/NBA empty, cont-assigns at fixpoint, `now` =
    /// settled time, time NOT yet advanced). Read-only w.r.t. net state: it only
    /// EVALUATES ExprIds (reading settled `cur` net values via `NetReader`) and
    /// writes to `self.st.out`. NOTE: this flush has nothing to do with
    /// `prev`/`snapshot_prev` — those are edge-detection state that `run()` rolls
    /// at the *start of the next* timestep (after time advance); the postponed
    /// render reads only the settled `cur` values and is independent of edge state.
    ///
    /// ORDER (frozen, documented for the golden gate): all strobes first in call
    /// order, then the single monitor line. IEEE leaves this tie-break
    /// implementation-defined; vita freezes strobes-then-monitor for byte-stable
    /// 3-OS golden output.
    pub(crate) fn flush_postponed(&mut self) {
        self.flush_postponed_with::<crate::state::SimState>(None)
    }

    /// A5-b: [`Scheduler::flush_postponed`] against an ALTERNATE net store.
    ///
    /// The POSTPONED region is the last thing tier-3 could not do, and the
    /// reason was never the region — it was the READ. `dispatch`'s `$monitor`
    /// and `$strobe` arms capture ExprIds and metadata and touch no net at all,
    /// so REGISTERING one has always been store-independent; what is store-bound
    /// is rendering the captured args and, for `$monitor`, evaluating them to
    /// decide whether anything changed. On a native run both read `SimState`'s
    /// flat store, which never moves — so a `$monitor` printed its establishment
    /// line and then went silent forever. That is why the two are refused
    /// together rather than one at a time.
    ///
    /// `None` is the engine's own store and every arm then reduces to the call
    /// it made before, so that path is mechanically byte-identical rather than
    /// merely equivalent — the same shape as `dispatch_with`,
    /// `format_args_str_with` and `full_snapshot_with`.
    pub(crate) fn flush_postponed_with<N: crate::eval::NetReader + ?Sized>(
        &mut self,
        nets: Option<&N>,
    ) {
        // `$time`/`$realtime` inside a postponed render must scale by the
        // REGISTERING module's multiplier, not the scheduler's live
        // `cur_time_mult` (which now holds the LAST-run process's `M` — a
        // different module under mixed timescales). Each `FmtCapture` carries its
        // own `time_mult`; we drive `cur_time_mult` from it per render and RESTORE
        // the entering value on exit so nothing downstream (e.g. a cont-assign
        // `$time` at the next loop-top settle) observes a render's multiplier.
        let saved_mult = self.st.cur_time_mult;
        // (a) STROBES — drain the FIFO in call order, render NOW (settled values),
        //     print, then CLEAR (one-shot per call: a strobe never repeats next
        //     step unless its statement re-executes and re-registers).
        if !self.st.postponed.strobes.is_empty() {
            // `mem::take` first to end the immutable borrow that `format_args_str`
            // needs against `&self` (mirrors `apply_nba`'s `mem::take(&mut nba)`).
            let batch = std::mem::take(&mut self.st.postponed.strobes);
            for cap in &batch {
                self.st.cur_time_mult = cap.time_mult; // registering module's M
                self.st.cur_scope = cap.scope.clone(); // registering module's %m
                let mut line = match nets {
                    Some(n) => crate::builtins::format_args_str_with(
                        &*self.st, n, cap.fmt, &cap.args, cap.radix,
                    ),
                    None => format_args_str(&*self.st, cap.fmt, &cap.args, cap.radix),
                };
                line.push('\n');
                // `$fstrobe` routes to its descriptor; `$strobe` to stdout.
                match cap.fd {
                    Some(fd) => crate::builtins::file_write(self, fd, &line),
                    None => write_out(self.st, &line),
                }
            }
            // `batch` dropped here; `self.st.postponed.strobes` is now empty.
        }

        // (b) MONITOR — IEEE 1364-2005 §17.1: reprint whenever any monitored
        //     expression VALUE changes (4-state-aware), NOT when the rendered
        //     string changes. We therefore evaluate the arg ExprIds to a
        //     `Vec<Value>` and compare against the stored baseline with
        //     `Value`'s derived `PartialEq` (exact `(val, unk)` bit-plane
        //     equality). Only when the value list differs (or was never seeded:
        //     establishment / replace) do we render + print and re-seed.
        //
        //     Borrow shape: hoist EVERYTHING out of the `&self.st.postponed`
        //     borrow into locals (copy `enabled`, copy `fmt`, clone `args`,
        //     `take` the previous `last_vals`) and DROP that borrow before any
        //     `&self`-eval / `&mut self.st`-write. No NLL dependence on a binding
        //     surviving across the render — render-then-record, zero overlap.
        //     `Option::take` is used so the old baseline is moved out (not
        //     cloned) and the slot is rewritten unconditionally below.
        // One monitor per DESTINATION. stdout first (so a design with no `$fmonitor` is
        // byte-identical), then each descriptor in ascending fd for determinism.
        let mut keys: Vec<Option<u32>> = vec![None];
        keys.extend(self.st.postponed.file_monitors.keys().map(|&d| Some(d)));
        for mkey in keys {
            self.flush_one_monitor(mkey, nets);
        }
        // Restore the entering multiplier (see the save at the top of this fn).
        self.st.cur_time_mult = saved_mult;
    }

    /// Render + print ONE monitor destination (`None` = stdout, `Some(fd)` = `$fmonitor`).
    /// Body unchanged from the single-monitor version; only the slot it reads and re-seeds
    /// is now selected by `mkey`.
    fn flush_one_monitor<N: crate::eval::NetReader + ?Sized>(
        &mut self,
        mkey: Option<u32>,
        nets: Option<&N>,
    ) {
        let mon = match monitor_slot_mut(self.st, mkey) {
            Some(m) => {
                let fmt = m.cap.fmt;
                let args = m.cap.args.clone();
                let tmult = m.cap.time_mult; // monitoring module's M (see (a) above)
                let radix = m.cap.radix;
                let scope = m.cap.scope.clone();
                let prev = m.last_vals.take(); // moves baseline out; slot now None
                let fd = m.cap.fd;
                Some((fmt, args, tmult, radix, scope, prev, fd))
            }
            // no monitor established → nothing to do. (DISABLED is no longer
            // gated here: the establishment print must fire even when disabled,
            // so the enable check moved INTO the change-detection below.)
            _ => None,
        };
        if let Some((fmt, args, tmult, radix, scope, prev, fd)) = mon {
            if fmt.is_none() && args.is_empty() {
                // No-arg monitor (`$monitor;` → fmt=None, args=[]) prints nothing —
                // not even a bare newline. Guarded so a future bare-`$monitor`
                // lowering cannot silently inject a lone "\n" into golden RTL output
                // (see §7.4). Seed an empty baseline (`[]` == `[]` keeps it silent
                // forever); a zero-expression monitor has no value to track.
                if let Some(m) = monitor_slot_mut(self.st, mkey) {
                    m.last_vals = Some(Vec::new());
                }
            } else {
                // Evaluate every monitored expression to a settled 4-state Value,
                // scaling `$time`/`$realtime` by the monitoring module's M.
                // `self.eval` builds `EvalCtx` from `self.st`, reading settled `cur`.
                // IEEE §21.2.3 / 1364 §17.1.3 (P0-9): a DIRECT `$time`/`$stime`/
                // `$realtime` argument does NOT participate in change detection (it
                // is rendered, but time advancing must not retrigger the monitor) —
                // filter those out of the comparison vector. `$stime` is the low 32
                // bits of `$time` and ticks every step exactly like it; omitting it
                // here re-fired the monitor on EVERY timestep (silent-wrong vs the
                // 2 iverilog lines). Change compare is on the BIT PLANES
                // (width/val/unk) only, not the derived `PartialEq` (which also
                // compares the static signed/is_real metadata).
                self.st.cur_time_mult = tmult;
                self.st.cur_scope = scope;
                let is_direct_time = |eid: u32| {
                    matches!(
                        self.st.ir.exprs[eid as usize],
                        sim_ir::Expr::SysFunc {
                            which: sim_ir::SysFuncId::Time
                                | sim_ir::SysFuncId::Stime
                                | sim_ir::SysFuncId::Realtime,
                            ..
                        }
                    )
                };
                // v9 rank 6: an ESTABLISHMENT (or `$monitoron`-forced) flush —
                // `prev == None` — ALWAYS prints, bypassing `$monitoroff` (the
                // establishment line is bound to the `$monitor` time-step; a
                // same-tick or prior `$monitoroff` does not cancel it). Only
                // CHANGE-triggered reprints (`prev == Some`) obey the global flag.
                let was_establishment = prev.is_none();
                let monitor_disabled = self.st.postponed.monitor_disabled;
                // P3-2: reuse the previous baseline Vec — evaluate each arg,
                // compare against the old slot, then overwrite IN PLACE (one
                // allocation per monitor lifetime, not one per timestep).
                let (changed, cur_vals) = match prev {
                    None => (
                        true, // establishment / replace → print
                        args.iter()
                            .filter(|&&eid| !is_direct_time(eid))
                            .map(|&eid| crate::builtins::eval_task_arg(self, nets, eid))
                            .collect::<Vec<Value>>(),
                    ),
                    Some(mut old) => {
                        let mut changed = false;
                        let mut i = 0usize;
                        for &eid in args.iter().filter(|&&eid| !is_direct_time(eid)) {
                            let v = crate::builtins::eval_task_arg(self, nets, eid);
                            match old.get_mut(i) {
                                Some(slot) => {
                                    if !(slot.width == v.width
                                        && slot.val == v.val
                                        && slot.unk == v.unk)
                                    {
                                        changed = true;
                                    }
                                    *slot = v;
                                }
                                None => {
                                    changed = true;
                                    old.push(v);
                                }
                            }
                            i += 1;
                        }
                        if i != old.len() {
                            changed = true;
                            old.truncate(i);
                        }
                        (changed, old)
                    }
                };
                // establishment / forced reprint prints unconditionally; a
                // change-reprint prints only while the monitor is enabled.
                let do_print = was_establishment || (changed && !monitor_disabled);
                if do_print {
                    let mut line = match nets {
                        Some(n) => {
                            crate::builtins::format_args_str_with(&*self.st, n, fmt, &args, radix)
                        }
                        None => format_args_str(&*self.st, fmt, &args, radix),
                    };
                    line.push('\n');
                    // `$fmonitor` routes to its descriptor; `$monitor` to stdout.
                    match fd {
                        Some(fd) => crate::builtins::file_write(self, fd, &line),
                        None => write_out(self.st, &line),
                    }
                }
                // Re-seed the baseline with the freshly-evaluated values regardless
                // of whether we printed: an unchanged step must keep the same
                // baseline, and a printed step adopts the new one.
                if let Some(m) = monitor_slot_mut(self.st, mkey) {
                    m.last_vals = Some(cur_vals);
                }
            }
        }
    }

    // ── NBA ──────────────────────────────────────────────────────────────

    /// Move the transport updates due at `now` into this tick's NBA batch.
    ///
    /// Called by the run loop AND by the tier-3 differential, which is the point:
    /// the first version of this was `#[cfg(test)]` with the run loop keeping its
    /// own inline `delayed_nba.remove`, and its docstring claimed that avoided a
    /// second spelling of an ordering rule while being exactly that. If this move
    /// ever changes — a range drain, or a different position relative to the
    /// `delayed_ca` apply and the wheel promotion — the tier-3 gate now changes
    /// with it instead of validating against a stale rule.
    pub(crate) fn take_due_delayed(&mut self, now: u64) {
        if let Some(ups) = self.delayed_nba.remove(&now) {
            self.nba.extend(ups);
        }
    }

    pub(crate) fn apply_nba(&mut self) {
        let mut batch = std::mem::take(&mut self.nba);
        batch.sort_by_key(|u| u.seq);
        // `write_lvalue` wants an `&Lvalue`, and an `Lvalue` owns its chunk `Vec`. Rather
        // than let every single-chunk update carry a heap `Vec` from its push site to
        // here (2.4M malloc/free pairs on a 40000-cycle picorv32 run — the free side
        // alone measured 25.8 ms of this region's 117 ms), the update carries the chunk
        // BY VALUE and this one scratch `Lvalue` — allocated once per flush, reused for
        // every update in it — lends it the `Vec` shape the write funnel expects.
        let mut scratch =
            std::mem::replace(&mut self.nba_scratch_lhs, Lvalue { chunks: Vec::new() });
        for u in batch.drain(..) {
            match u.lhs {
                NbaLhs::One(c) => {
                    scratch.chunks.clear();
                    scratch.chunks.push(c);
                    self.st.write_lvalue(&scratch, u.sampled, &u.offsets);
                }
                NbaLhs::Many(lv) => {
                    self.st.write_lvalue(&lv, u.sampled, &u.offsets);
                }
            }
        }
        self.nba_scratch_lhs = scratch;
        // Hand the drained Vec back so the next timestep's NBA pushes reuse its
        // capacity (consuming it dropped one allocation per NBA flush).
        self.nba = batch;
    }

    // ── change propagation + edge detection ──────────────────────────────

    /// Diff prev→cur for every net; fire static edge-sensitive `always` blocks,
    /// in-body `Wait{Edge}`/`Wait{Level}` waiters; then refresh prev. Dependent
    /// continuous assigns are covered by `settle_cont_assigns` (re-evaluated each
    /// delta), so no explicit re-enqueue is needed here.
    ///
    /// ORDER IS LOAD-BEARING: every edge comparison (static map AND waiters) must
    /// read the PRE-change `prev`, so the prev-refresh is deferred to the very end
    /// of this pass. (A previous version refreshed prev per-net inside the static
    /// loop, which silently masked all in-body `@(edge)` waits — their later
    /// `prev`/`cur` read saw `prev == cur` → no edge.)
    /// Re-evaluate every live expression force (BTree order = deterministic)
    /// and re-pin its target. The forcing module's time multiplier is restored
    /// around each eval so `$time` in a force RHS renders with the right scale.
    pub(crate) fn reeval_active_forces(&mut self) {
        // ACTIVE pins only — both force (§9.3.2) and proc-assign (§9.3.1)
        // re-evaluate continuously; a latent (force-displaced) assign does not
        // run until release re-pins it.
        // C-FORCE-REEVAL-p2: select only the forces whose inputs changed this
        // delta (via the net→forces sidecar) PLUS every always-reeval force
        // (volatile $time/$random RHS, or zero-net const RHS — these yield a
        // fresh value with frozen inputs, so a net-sensitivity skip would
        // silently FREEZE them). The selected keys are SORTED, so the executed
        // subset re-evaluates in the exact BTreeMap (ascending-key) order the
        // old all-forces loop used for that subset — a force's re-pin can write
        // a net feeding another force, so order is load-bearing for output.
        //
        // p2 FIX (force-feeds-force CHAIN): `reeval_active_forces` runs ONCE per
        // `propagate_changes`, and the dirty list it triggers off is then CONSUMED
        // by the sweep below. So a force re-pin that writes a net feeding ANOTHER
        // force must re-trigger that downstream force WITHIN THIS CALL — there is
        // no later sweep that re-runs it (the new dirtiness is transient, gone by
        // the next delta). We therefore iterate to a FIXPOINT: each pass re-pins a
        // worklist of forces, records which design nets actually changed value,
        // then re-selects the forces those nets feed and repeats until quiescent.
        // The old all-reeval code converged the whole DAG across successive
        // sweeps; here we converge it in one call (input nets are frozen — only
        // force-target nets move — so the fixpoint always settles, bounded by the
        // number of live forces; a guard caps pathological cyclic forces).
        let mut keys = std::mem::take(&mut self.scratch_force_keys);
        keys.clear();
        // Seed the worklist from the externally-changed nets. `self.st.dirty`
        // (still unconsumed at this point in `propagate_changes`) is the same
        // changed-net set that gates this call; using it raw (not the
        // cur!=prev–filtered set) is a safe superset — it never skips a force the
        // old unconditional loop ran. Always-reeval forces (volatile / zero-net)
        // join the FIRST pass unconditionally.
        for &n in &self.st.dirty {
            if let Some(set) = self.st.force_net_to_forces.get(&n) {
                keys.extend(set.iter().copied());
            }
        }
        keys.extend(self.st.force_always_reeval.iter().copied());
        keys.sort_unstable();
        keys.dedup();

        let saved = self.st.cur_time_mult;
        // Fixpoint bound: each force can re-pin its target at most once per pass,
        // and a pass only re-runs forces fed by a net that CHANGED VALUE in the
        // previous pass. A non-cyclic force DAG settles in ≤ (#live forces) passes;
        // the cap defends against a pathological cyclic force chain (which would
        // also oscillate under the old per-sweep convergence — IEEE leaves such a
        // zero-delay loop unstable). +2 slack over the live-force count.
        let mut budget = self.st.active_forces.len().saturating_add(2);
        while !keys.is_empty() && budget > 0 {
            budget -= 1;
            // `next_changed` collects the design nets whose VALUE actually moved
            // from a force re-pin in THIS pass — the trigger set for the next.
            let mut next_changed: Vec<u32> = Vec::new();
            for &k in &keys {
                // Look up live registry data by key. A key could have been removed
                // by an earlier force's re-pin side effects; skip a vanished entry.
                let Some((lv, rhs, mult, _weak)) = self.st.active_forces.get(&k).cloned() else {
                    continue;
                };
                self.st.cur_time_mult = mult;
                let v = self.eval_for_lvalue(&lv, rhs);
                // `force_write` returns whether the target net's value moved. Only
                // a real change can re-trigger a downstream force; an unchanged
                // re-pin is a no-op (preserves the old same-value-drop behavior).
                if self.st.force_write(&lv, v) {
                    next_changed.push(k);
                }
            }
            // Re-select the forces fed by the nets that just changed value. A
            // force whose source net did not move is NOT re-run (byte-identical to
            // a steady chain). De-dup + sort to keep the ascending-key order.
            keys.clear();
            for &n in &next_changed {
                if let Some(set) = self.st.force_net_to_forces.get(&n) {
                    keys.extend(set.iter().copied());
                }
            }
            keys.sort_unstable();
            keys.dedup();
        }
        self.st.cur_time_mult = saved;
        keys.clear();
        self.scratch_force_keys = keys;
    }

    /// GATED-CLOCK: clear the per-process "edge-woken this CLUSTER" markers. An
    /// edge-sensitive `always` fires at most once per ACTIVE-region settle cluster
    /// (so a cont-assign-derived/gated clock whose edge lands a later delta of the
    /// SAME cluster — `gclk = clk & en` — collapses to one firing, matching
    /// iverilog). But a GENUINELY NEW event batch from a region boundary (#0
    /// Inactive promotion, NBA apply, or time advance) is a fresh cluster: those
    /// edges (e.g. an independent `negedge rst` scheduled via `#0`/NBA into a later
    /// delta) MUST be able to re-fire the process, so the markers reset at each
    /// boundary. Within a cluster the markers persist across deltas (cont-assigns
    /// settle one delta at a time). Clears only the touched entries (O(#woken)).
    pub(crate) fn reset_edge_seen_marks(&mut self) {
        for &p in &self.scratch_edge_marked {
            if let Some(slot) = self.scratch_edge_seen.get_mut(p as usize) {
                *slot = false;
            }
        }
        self.scratch_edge_marked.clear();
    }
}

/// The `MonitorState` slot for a destination: `None` = the stdout `$monitor` singleton,
/// `Some(fd)` = that descriptor's `$fmonitor`. A free function so the flush can re-borrow
/// the slot at its two re-seed points without holding a borrow across the render (the
/// borrow shape the flush's comment describes).
fn monitor_slot_mut<'a>(
    st: &'a mut crate::state::SimState<'_>,
    key: Option<u32>,
) -> Option<&'a mut crate::state::MonitorState> {
    match key {
        None => st.postponed.monitor.as_mut(),
        Some(fd) => st.postponed.file_monitors.get_mut(&fd),
    }
}
