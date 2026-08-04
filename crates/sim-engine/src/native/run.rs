//! S1d-4c-2c (doc-21 §5 S1d-4) — **the tier-3 run loop.**
//!
//! The first time this backend runs a DESIGN. Everything before it produced a
//! part and a differential for that part: the arena (S1a/b/c), the changed set
//! (S1d-2), the wake decision (S1d-3), the statement kernel (4a/4b), the NBA
//! drain (4c-1), re-arming (4c-2a), the body walk (4c-2b). This is the loop that
//! calls them in the order IEEE 1364-2005 §5 prescribes, and it is what makes
//! the earlier pieces observable end-to-end: until now `k_rearm` wrote a table
//! nothing read and `k_schedule_nba_at` filed updates nothing drained.
//!
//! ## Where the slice boundary came from — measurement, not tidiness
//!
//! The plan called this slice "region queues + delta loop + in-body waiters +
//! `busy` + `flush_postponed`". Measuring the corpus first moved the line:
//!
//! - **0 of 72 designs have every process suspend-free.** A whole-timestep
//!   differential without SOME suspension model therefore has zero corpus
//!   coverage — it could only ever run against designs written for it, which is
//!   the exact shape of gate the last four slices kept finding holes in.
//! - **All 138 suspending terminators in the corpus are `Delay`, in
//!   `DelayRegion::Active`.** `Wait{Edge|Level|Expr}`, `Fork` and `Call`: zero.
//!
//! So the unit with an oracle is *the loop plus `Delay`*, and the in-body waiter
//! model — which has no corpus coverage at all — is the slice after. Splitting
//! it the other way (loop first, suspension later) would have produced a gate
//! that could not run a single corpus design.
//!
//! ⚠️ **What the accepted class covers, re-measured after S1d-4d-2.** All four
//! of this repo's `examples/*.sv` now RUN natively and match the VM byte for
//! byte, stdout and VCD alike — they were refused by the system-task row until
//! `$dumpfile`/`$dumpvars` were wired, which is what this paragraph used to
//! say and no longer should. `bench/picorv32` likewise. Still refused: a
//! delayed/multi-driven/wired continuous assign, `$monitor`/`$strobe`,
//! `$dumpall`/`$dumpon`, `final`, `fork`, and frame-local storage (which is
//! what keeps `bench/keccak`'s subroutine variants out).

use sim_ir::SimIr;

use crate::exec::{Kernel, Step};
use crate::native::body::{body_is_walkable, run_body};
use crate::native::kernel::{push_sorted_native, NativeKernel, NativeReady};
use crate::sched::FinishReason;

/// The THIRD gate layer: can the executor that exists TODAY run this design?
///
/// `design_eligibility` answers v1's scope and `NetArena::buildable` answers
/// today's storage; neither answers this, and folding it into either would make
/// an unbuilt piece read as out of scope (the reason `kpred::rhs_routes_to_worker`
/// is likewise kept out of the design gate).
///
/// Every row is something the loop below does not do, stated as a refusal rather
/// than left to be discovered as a wrong answer.
///
/// TEST-ONLY, and that is not a demotion: production asks the same conjunction,
/// spelled as `design_eligibility().refused` (which already ANDs scope with
/// storage) `.or_else(executor_rows)`. This is the two-call convenience, so the
/// gates verify exactly what `simulate` decides rather than a parallel one.
#[cfg(test)]
pub(crate) fn runnable(
    ir: &SimIr,
    opts: &crate::SimOpts,
    md_nets: usize,
) -> Result<(), &'static str> {
    crate::native::runtime_gate(ir, opts)?;
    executor_rows(ir, opts, md_nets)
}

/// The rows `runnable` adds on TOP of the runtime gate — split out because
/// `simulate` has already computed the gate's half and recomputing it there
/// would let the published verdict and the executed decision come from two
/// evaluations of the same predicate.
pub(crate) fn executor_rows(
    ir: &SimIr,
    opts: &crate::SimOpts,
    md_nets: usize,
) -> Result<(), &'static str> {
    // CONTINUOUS ASSIGNS (S1d-4d-1): the ZERO-DELAY fixpoint is built; the three
    // families around it are not, and each would be a different wrong answer
    // rather than a slower one.
    if ir.cont_assigns.iter().any(|c| c.delay.is_some()) {
        // `assign #d` needs the inertial wheel (`delayed_ca`, `ca_gen`,
        // `last_ca_drv`, the sole-driver x-drive) — a pulse narrower than `d`
        // must never reach the LHS, and without the generation check it would.
        return Err("a delayed continuous assign (`assign #d`)");
    }
    // WIRED BEFORE MULTI-DRIVER, and the order is load-bearing rather than
    // cosmetic: a `wand`/`wor` net is multi-driven BY CONSTRUCTION, so with the
    // rows the other way round the wired row could never be the answer and its
    // message would name a family no design ever sees.
    if !opts.wired_and_nets.is_empty() || !opts.wired_or_nets.is_empty() {
        return Err("a `wand`/`wor` net (wired resolution)");
    }
    // MULTI-DRIVER: a net resolved from several whole-net drivers by 4-state
    // wire resolution, which a plain fixpoint would silently turn into
    // last-write-wins. Asked through the SCHEDULER's own `md_nets`, computed
    // once in `Scheduler::new`, rather than re-derived here.
    //
    // ⚠️ The first version of this row counted net occurrences across every
    // cont-assign lvalue and refused at two. That over-approximated badly: the
    // per-bit generate idiom (`for (g…) assign y[g] = ~x[g];`), a part-select
    // pair (`y[7:4]` / `y[3:0]`), and even a single assign whose LHS concat
    // names one net twice all read as multi-driven — and every one of them is a
    // single logical driver the settle already handles by last-write-wins in
    // the same ascending order the engine uses. That is the most common way
    // real RTL drives a bus, so the proxy capped exactly the coverage this
    // slice exists to unlock.
    if md_nets > 0 {
        return Err("a multi-driven net (wire resolution)");
    }
    if !opts.final_procs.is_empty() {
        return Err("`final` blocks (the post-loop drain is not restated)");
    }
    for pi in 0..ir.processes.len() as u32 {
        if !body_is_walkable(ir, pi, ir.processes[pi as usize].entry) {
            // Names the REACHABLE cause first. The previous wording led with
            // `fork`/subroutine — both normally refused an entire layer earlier
            // (the S0 `fork` row, the arena's `func_table`) — and advertised a
            // named-event wait, which elaborate never constructs. The one thing
            // that actually arrives here in an eligible, buildable design is a
            // bare `wait fork;`, which populates no `fork_modes` entry.
            //
            // A plain non-`automatic` task containing `@(posedge clk)` is NOT
            // refused: elaborate inlines it, so there is no `Terminator::Call`.
            // What the subroutine half really refuses is frame-local storage,
            // and the arena says so in its own words.
            return Err("a `wait fork`, or a `fork`/subroutine-call body whose sidecar was lost");
        }
        if !body_dispatch_ok(ir, pi) {
            return Err("a system task the tier-3 kernel refuses (VCD, $monitor/$strobe, file)");
        }
    }
    Ok(())
}

/// Does this body reach only system tasks the tier-3 kernel will dispatch?
///
/// The SECOND caller obligation `run_body`'s doc names, and the one the design
/// gate structurally cannot answer: a `$dumpvars` design is `eligible: true,
/// buildable: true`, because the refusal lives in `k_dispatch_systask` rather
/// than in any eligibility row. Asked here, or it becomes a panic mid-run.
fn body_dispatch_ok(ir: &SimIr, proc: u32) -> bool {
    ir.processes[proc as usize].body.iter().all(|blk| {
        blk.stmts.iter().all(|&sid| match &ir.stmts[sid as usize] {
            sim_ir::Stmt::SysTask { which, .. } => {
                crate::native::kernel::systask_refusal(*which).is_none()
            }
            _ => true,
        })
    })
}

/// Can the walk run THIS process — both conditions, as one question.
///
/// Exported because the body differential asks exactly the same thing, and it
/// asked only the first half until the moment `Delay` became walkable turned
/// that omission into a panic: the corpus bodies that suspend are also the ones
/// carrying `$dumpfile`. A second spelling in the test would have been a gate
/// that admits designs production refuses, or the reverse.
#[cfg(test)]
pub(crate) fn body_admissible(ir: &SimIr, proc: u32) -> bool {
    body_is_walkable(ir, proc, ir.processes[proc as usize].entry) && body_dispatch_ok(ir, proc)
}

/// Run one eligible design to completion over the arena.
///
/// PRECONDITION: `runnable(ir, opts)` returned `Ok`. Violating it is a panic
/// (from the kernel's refused workers or the walk's `unreachable!` arms), never
/// a wrong answer.
///
/// The structure mirrors `Scheduler::run` one-for-one, because that is what the
/// byte gate compares against: an OUTER loop over timesteps and an INNER loop
/// that drains the current time to a stable point through the region cascade.
pub(crate) fn run(k: &mut NativeKernel, ir: &SimIr) -> FinishReason {
    // t0 STRUCTURAL SETTLE, before anything is armed — `simulate` does this for
    // the engine outside `run()`. A design that cannot converge here has a
    // divergent t0 and is stopped rather than run on it.
    let mut t0_deltas: u64 = 0;
    if settle_cont_assigns(k, ir, &mut t0_deltas).is_none() {
        return FinishReason::DeltaLimit;
    }
    arm_t0(k, ir);
    if k.sched.st.finished {
        // The ONE live `finished` poll: `arm_t0`'s missing-init-proc guard latches
        // it. NOT for a `$finish` in an initializer — an initializer body is
        // straight-line assignments, and `run_body` would return `Step::Finish`
        // rather than set the flag anyway. (An earlier version of this comment
        // said the opposite; review measured it.)
        return done(k, finish_kind(k));
    }
    let max_deltas = k.k_delta_budget();
    let time_limit = k.k_time_limit();
    loop {
        // MIRRORS the engine's loop shape and cannot fire for this class: the
        // only setters on this path are the three terminating arms below, which
        // all `return`. Kept rather than deleted because the two loops are read
        // side by side and a missing check reads as a difference; recorded as
        // dead rather than described as a guard.
        if k.sched.st.finished {
            let fk = finish_kind(k);
            return done(k, fk);
        }
        let mut delta_count: u64 = 0;
        // ── drain the current time to a stable point ──────────────────────
        loop {
            // ACTIVE: continuous assigns settle FIRST, then processes drain —
            // the engine's order. A settle that moved nets may have produced an
            // edge on a cont-assign-driven net (a port-bound clock), so change
            // propagation has to run before the timestep is called stable.
            match settle_cont_assigns(k, ir, &mut delta_count) {
                None => return done(k, FinishReason::DeltaLimit),
                Some(true) => propagate(k),
                Some(false) => {}
            }
            if !k.active.is_empty() {
                // Take the batch so wakes triggered DURING it land in a fresh
                // `active` — the engine's shape, and it is semantic rather than
                // an allocation trick: a process woken by the batch belongs to
                // the NEXT delta, not to the middle of this one.
                let batch = std::mem::take(&mut k.active);
                for r in batch {
                    if k.sched.st.finished {
                        let fk = finish_kind(k);
                        return done(k, fk);
                    }
                    // SELF-RETRIG: tag this body's blocking writes with their
                    // author, so it is not re-fired by its own write.
                    // `Scheduler::run_body` does this around `run_process`, and it
                    // is set HERE rather than inside the shared walk because for
                    // `K = Scheduler` the engine already sets it one level up —
                    // a second set there would be a second spelling of a rule
                    // that has exactly one.
                    //
                    // Cleared after: the NBA apply authors its writes as `None`
                    // (= re-fire normally), which is what makes `q <= d` wake
                    // `always @(q)`.
                    k.arena.ch.blocking_writer = Some(r.proc);
                    let step = run_body(k, ir, r.proc, r.block);
                    k.arena.ch.blocking_writer = None;
                    // The walk drains at every statement boundary; this catches
                    // what happens AFTER the last one — an out-of-range read in a
                    // `Branch` condition or a `#(mem[i])` delay amount. Before
                    // the terminating arms below, so a `$finish` in the same body
                    // still reports it.
                    k.drain_range_diags();
                    match step {
                        Step::Finish => {
                            k.sched.st.finished = true;
                            return done(k, FinishReason::Finish);
                        }
                        Step::Stop => {
                            k.sched.st.finished = true;
                            return done(k, FinishReason::Stop);
                        }
                        Step::Fatal => {
                            k.sched.st.finished = true;
                            k.sched.st.had_fatal = true;
                            return done(k, FinishReason::Error);
                        }
                        // `busy` suppresses a static-sensitivity wake for a
                        // process parked mid-body: its registration is permanent
                        // but IEEE does not re-enter an `always` until it
                        // completes. S1d-3 stated this rule and had no maintainer
                        // for it; this is the maintainer.
                        Step::Suspended => k.wake.busy[r.proc as usize] = true,
                        Step::Done => k.wake.busy[r.proc as usize] = false,
                    }
                }
                propagate(k);
                delta_count += 1;
                if delta_count > max_deltas {
                    k.sched.fatal_delta_limit();
                    return done(k, FinishReason::DeltaLimit);
                }
                continue;
            }
            // INACTIVE (#0): promote to Active. A `#0` batch is a NEW event
            // cluster, so the edge-dedup marks reset — an edge it produces must
            // be able to re-fire a process already woken this timestep.
            if !k.inactive.is_empty() {
                k.active = std::mem::take(&mut k.inactive);
                k.wake.reset_edge_seen();
                delta_count += 1;
                if delta_count > max_deltas {
                    k.sched.fatal_delta_limit();
                    return done(k, FinishReason::DeltaLimit);
                }
                continue;
            }
            // NBA: apply the sampled batch. Same cluster argument as `#0`.
            if !k.nba.is_empty() {
                k.wake.reset_edge_seen();
                k.apply_nba();
                // NBA writes go through the same funnel but outside any body, so
                // the per-body drain above does not cover them. `q[i] <= v` with
                // an out-of-range `i` is the shape.
                k.drain_range_diags();
                propagate(k);
                delta_count += 1;
                if delta_count > max_deltas {
                    k.sched.fatal_delta_limit();
                    return done(k, FinishReason::DeltaLimit);
                }
                continue;
            }
            break; // time-step stable
        }

        // ── advance time ──────────────────────────────────────────────────
        // The minimum over BOTH pending sources. `delayed_nba` is in it because
        // of the hazard S1d-4c-1 recorded against itself: a design whose only
        // pending work is a transport NBA would otherwise be called quiescent
        // and its update dropped. (`delayed_ca` is the engine's third source and
        // has no analogue — `runnable` refuses continuous assigns.)
        let next = match [
            k.wheel.keys().next().copied(),
            k.delayed_nba.keys().next().copied(),
        ]
        .into_iter()
        .flatten()
        .min()
        {
            None => return done(k, FinishReason::Quiescent),
            Some(t) => t,
        };
        if let Some(lim) = time_limit {
            if next > lim {
                return done(k, FinishReason::Quiescent);
            }
        }
        k.sched.st.now = next;
        k.wake.reset_edge_seen();
        k.take_due_delayed(next);
        let events = k.wheel.remove(&next).unwrap_or_default();
        for (inactive, ready) in events {
            if inactive {
                push_sorted_native(&mut k.inactive, ready);
            } else {
                push_sorted_native(&mut k.active, ready);
            }
        }
    }
}

/// `Scheduler::settle_cont_assigns` for the class the gate admits: re-evaluate
/// every continuous assign until no net moves.
///
/// `None` ⇒ it did not converge (a cont-assign oscillator) and the caller must
/// stop the run — otherwise an `assign`-only loop would spin the whole budget
/// on EVERY outer delta while `active` stayed empty, and the outer delta limit
/// would never fire. `Some(changed)` ⇒ converged, and `changed` says whether
/// any net moved, which is what tells the caller to run change propagation (a
/// port-bound clock's edge has to reach the child's `always @(posedge clk)`).
///
/// ONE deliberate simplification: **`eval_for_lvalue`, not `eval_cont_assign`**
/// — the latter picks a tier-2 compiled program when one exists and falls back
/// to exactly this. Same value, one less table.
///
/// ⚠️ There was a SECOND, and measurement killed it. Visiting every assign every
/// pass (no worklist) looked byte-identical, and the engine's own comment
/// appears to license it: the skip is sound "precisely because a certified
/// assign whose inputs did not move recomputes its previous value, and the
/// write funnel drops a same-value write without noting a change". That
/// argument is about VALUES. An assign whose RHS reads out of range emits an
/// `E4002` on every re-read, so the visit it replaces is NOT observationally a
/// no-op — picorv32 went from 6 errors to 9. The worklist is reproduced here
/// (from the scheduler's own `ca_of_net`/`ca_always`) for correctness of the
/// diagnostic stream, not for speed.
///
/// The delta counter is the RUN LOOP's, passed by reference, because the engine
/// shares `self.delta_count` between the settle and the region cascade: a design
/// that settles slowly and oscillates slowly must hit the limit at the same
/// point on both backends.
fn settle_cont_assigns(k: &mut NativeKernel, ir: &SimIr, delta_count: &mut u64) -> Option<bool> {
    if ir.cont_assigns.is_empty() {
        return Some(false);
    }
    let max_deltas = k.k_delta_budget();
    let mut any = false;
    loop {
        let mut changed = false;
        // The engine's visit set, not a superset of it: the assigns whose
        // dependency nets moved (`ca_dirty`, maintained by the write funnel from
        // `ca_of_net`) UNION the ones `levelize::ca_deps` refused to certify
        // (`ca_always`). Ascending index = declaration order, which several
        // goldens depend on.
        let pass: Vec<u32> = {
            let mut v = std::mem::take(&mut k.arena.ch.ca_dirty);
            for &ci in &v {
                k.arena.ch.ca_dirty_flag[ci as usize] = false;
            }
            v.extend_from_slice(k.sched.ca_always());
            v.sort_unstable();
            v.dedup();
            v
        };
        for ci in pass.into_iter().map(|c| c as usize) {
            let lhs = ir.cont_assigns[ci].lhs.clone();
            let rhs = ir.cont_assigns[ci].rhs;
            let v = k.k_eval_for_lvalue(&lhs, rhs);
            let offs = k.k_resolve_lvalue_offsets(&lhs);
            changed |= k.arena.write_lvalue(ir, &lhs, v, &offs);
        }
        // A cont-assign RHS can read an out-of-range array element, and the
        // arena can only COUNT that — same third-producer problem the waiter
        // predicate had. Drained here rather than left to the next body,
        // because the t0 settle runs before any body exists.
        k.drain_range_diags();
        if !changed {
            break;
        }
        any = true;
        *delta_count += 1;
        if *delta_count > max_deltas {
            k.sched.fatal_delta_limit();
            return None;
        }
    }
    Some(any)
}

/// `arm_processes` for the class the gate admits.
///
/// Two halves, and the second is easy to lose: declaration initializers run
/// FIRST and their writes are then UN-DIRTIED. IEEE 1800 §6.21 puts a
/// declaration initializer "before any initial or always block starts", and
/// measurement says that is literal — `reg clk = 0;` must not hand
/// `always @clk` an x→0 edge. Keeping the dirt would hand it exactly that.
fn arm_t0(k: &mut NativeKernel, ir: &SimIr) {
    let inits: Vec<u32> = k.sched.st.init_procs.to_vec();
    // THE MARK. Everything already on the dirty list belongs to the t0
    // cont-assign settle and must SURVIVE; only what the initializer bodies add
    // below is dropped. `NetArena::build` dirties nothing (its `set_elem` is
    // test-only and bypasses the channel), so this is exactly the settle's set.
    let settled = k.arena.ch.dirty.len();
    for &pid in &inits {
        // RANGE GUARD, mirroring `arm_processes`'s `fatal_init_proc_missing`:
        // `init_procs` rides the `.velab` trailer, OUTSIDE the schema gate, so a
        // truncated one can name a process this IR does not have. The engine is
        // loud about it; indexing would have been an out-of-bounds panic, which
        // is a worse answer to the same input.
        if (pid as usize) >= ir.processes.len() {
            k.sched.fatal_init_proc_missing(pid);
            return;
        }
        let entry = ir.processes[pid as usize].entry;
        // The Step is DISCARDED, and so is the engine's (`arm_processes` writes
        // `let _ =` too) — an initializer body is straight-line assignments, so
        // there is no `$finish` to honour and nothing to resume. Not checking
        // `finished` between initializers is likewise the engine's behaviour;
        // an earlier version of this loop returned early, which was a
        // difference with no rule behind it.
        let _ = run_body(k, ir, pid, entry);
        // REDUNDANT TODAY, and said so rather than left looking load-bearing:
        // `run_body` drains at every statement boundary and an initializer body
        // is straight-line assignments, so an out-of-range read in one is
        // already reported before this line runs. Measured — removing it leaves
        // the whole gate green, including a design whose ONLY statement is
        // `reg x = mem[9];`. Kept because it costs one `Cell` read and its
        // absence would be a silent loss the day the walk's drain moves.
        k.drain_range_diags();
    }
    // Drop what the INITIALIZERS made dirty — and only that. The engine splits
    // its dirty list at a mark taken before it runs them, and the reason is
    // written in `arm_processes`: the t0 cont-assign settle's writes are on the
    // same list, and "clearing the list wholesale threw those away too … an
    // `always @(w)` on `assign w = 1'b1;` simply never fired".
    //
    // ⚠️ This code DID clear it wholesale, and the comment that used to sit here
    // said the settle was refused so the initializers were the only writer. That
    // was true until S1d-4d-1 put a settle in front of `arm_t0` — the slice
    // invalidated its own precondition and did not revisit the sentence.
    // Measured: 49 of 270 generated cont-assign designs diverged, silently, at
    // exit 0. The mark is what makes the split possible, so it is taken BEFORE
    // the initializer loop above rather than derived afterwards.
    for n in k.arena.ch.dirty.split_off(settled) {
        k.arena.ch.dirty_flag[n as usize] = false;
    }
    let init_set: std::collections::BTreeSet<u32> = inits.iter().copied().collect();
    for pi in 0..ir.processes.len() as u32 {
        if init_set.contains(&pi) {
            continue;
        }
        match ir.processes[pi as usize].sensitivity.kind {
            // initial + combinational/latch blocks RUN at t0…
            sim_ir::SensKind::Initial | sim_ir::SensKind::Comb | sim_ir::SensKind::Latch => {
                push_sorted_native(
                    &mut k.active,
                    NativeReady {
                        proc: pi,
                        block: ir.processes[pi as usize].entry,
                    },
                );
            }
            // …edge/level blocks WAIT for the first event. `WakeTable::new`
            // already encodes both halves of that asymmetry (`level_armed =
            // kind == Level`), which is why there is no arming call here: the
            // registration is static and was built with the table.
            sim_ir::SensKind::Edge | sim_ir::SensKind::Level => {}
        }
    }
}

/// The engine's `propagate_changes`, for the class the gate admits: take the
/// changed set, ask the wake table who that wakes, queue them.
///
/// What has no analogue, each provably empty rather than skipped: force re-eval
/// (`force_release` is an S0 reject) and the clocking commit (`clocking` is an
/// S0 reject).
///
/// The engine's waiter pass is NOT absent — it is split in two here, and both
/// halves run: its STATIC `Level` arm (`arm = None`) is the second loop of
/// `WakeTable::wake`, and its IN-BODY arms are `fire_waiters` below (S1d-4c-2d;
/// an earlier version of this paragraph said the in-body half was refused,
/// which stopped being true when that slice landed).
///
/// ⚠️ And there is no "prev-refresh" here to be the arena's `take_changed`:
/// this store has no `prev` at all — the changed set IS `dirty` membership, and
/// the edge mask resets on a net's first dirtying of the next slot.
fn propagate(k: &mut NativeKernel) {
    let mut changed = Vec::new();
    k.arena.take_changed(&mut changed);
    if changed.is_empty() {
        return;
    }
    let mut woken = Vec::new();
    k.wake.wake(&changed, &mut woken);
    for p in woken {
        push_sorted_native(
            &mut k.active,
            NativeReady {
                proc: p,
                block: k.ir.processes[p as usize].entry,
            },
        );
    }
    fire_waiters(k, &changed);
    // THE THIRD PRODUCER. `fire_waiters` evaluates every `wait(expr)` predicate
    // through the arena, so it can leave a deferred out-of-range report behind —
    // and `propagate` is called from two places that both `continue` straight
    // into a region test, with every terminating exit (quiescent, time limit,
    // delta limit) returning without another drain. Measured: a design whose
    // only out-of-range read is in a wait predicate ran `--backend native` to
    // exit 0 while the VM exited 1. Same class as the NBA path §4.5.298 fixed;
    // this is the seam that slice did not have yet.
    k.drain_range_diags();
}

/// The engine's `propagate_changes` pass (b): in-body waiters.
///
/// Runs AFTER the static-sensitivity pass, as it does there, and pushes through
/// the same sorted insert — so a delta that wakes both an `always @(posedge clk)`
/// and a body parked on `@(posedge clk)` queues them in process order rather
/// than in "static first, in-body second" order.
///
/// A fired waiter is CONSUMED. `Level` and `Expr` re-arm by suspending again
/// when the resumed body reaches the wait a second time; `Edge` is one-shot by
/// nature. That is why there is no re-registration here — the engine has none
/// either, and adding one would fire a waiter the body never re-armed.
fn fire_waiters(k: &mut NativeKernel, changed: &[crate::native::dirty::ChangedNet]) {
    if k.waiters.is_empty() {
        return;
    }
    // Decide FIRST, mutate second: the fire test reads the arena (and, for
    // `Expr`, evaluates through it), which cannot happen while `waiters` is
    // being drained. Same shape as the engine, which pre-computes `expr_now`
    // and `level_fire` for exactly this reason.
    let fires: Vec<bool> = k
        .waiters
        .iter()
        .map(|w| match (&w.cause, &w.arm) {
            // IN-BODY `@(sig)`: fires when a watched net differs from its
            // ARM-TIME value — deliberately NOT "is it in the changed set". A
            // change that landed before the arm is already in the snapshot, so
            // a dirtiness test would re-fire the wait in its own arming slot.
            //
            // SELF-RETRIG: the author guard is the engine's spelling, kept for
            // fidelity — but it CANNOT fire on this arm, and saying so beats a
            // teeth claim no design can back (measured: removing it leaves the
            // gate green). A suspended process cannot write, so `cur != arm`
            // already implies somebody else wrote the net after the arm, which
            // is what `last_blocking_writer` then holds. The guard earns its
            // keep on the STATIC arm (`arm = None`), which lives in the wake
            // table, and on `Edge` below — where the process CAN have made the
            // edge itself, in the same slot, before the wait armed.
            (sim_ir::WaitCause::Level { nets }, Some(arm)) => {
                nets.iter().zip(arm).any(|(&n, av)| {
                    k.arena.net_words(n) != av.as_slice()
                        && k.arena.ch.last_blocking_writer[n as usize] != w.proc
                })
            }
            // GLITCH: an in-body `@(posedge x)` fires from the intra-slot MASK,
            // so a pulse that returns to its old value still wakes the waiter.
            // SELF-RETRIG here is REACHABLE, unlike on the `Level` arm above:
            // `clk = ~clk; @(posedge clk);` leaves the mask set by the waiter's
            // own write, and without the guard the wait resumes on the edge it
            // just caused (measured — the design is in the adversarial set).
            (sim_ir::WaitCause::Edge { net, kind }, _) => changed.iter().any(|&(n, mask, wr)| {
                n == *net && crate::sched::edge_fires_slot(mask, *kind) && wr != w.proc
            }),
            // `wait(e)`: re-check the predicate against the POST-change values.
            (sim_ir::WaitCause::Expr { expr }, _) => k.ctx().truthy(*expr),
            // A static `Level` (arm=None) cannot be here — those live in the
            // wake table — and `Named`/`Fork` are refused by `body_is_walkable`.
            _ => false,
        })
        .collect();
    if !fires.iter().any(|&f| f) {
        return;
    }
    let mut idx = 0usize;
    let mut woken: Vec<NativeReady> = Vec::new();
    k.waiters.retain(|w| {
        let fired = fires[idx];
        idx += 1;
        if fired {
            woken.push(NativeReady {
                proc: w.proc,
                block: w.block,
            });
        }
        !fired
    });
    for r in woken {
        push_sorted_native(&mut k.active, r);
    }
}

/// Every `run` exit funnels through here so the buffered VCD records are
/// written before the kernel is dropped.
///
/// The nine return paths were audited and instrumented (0 losses across the
/// whole suite and 190 designs), but an audit is an argument and this is a
/// guard: `propagate` already contains an early return that skips its own
/// drain, and anything left behind would be a silently truncated waveform at
/// exit 0.
fn done(k: &mut NativeKernel, r: FinishReason) -> FinishReason {
    k.drain_range_diags();
    debug_assert!(
        k.arena.ch.vcd_pending.is_empty(),
        "tier-3 run left VCD records unwritten"
    );
    r
}

fn finish_kind(k: &NativeKernel) -> FinishReason {
    if k.sched.st.had_fatal {
        FinishReason::Error
    } else {
        FinishReason::Finish
    }
}
