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
//! ⚠️ **What the accepted class does NOT yet include, stated so 65/72 is not
//! read as coverage of real designs**: all four of this repo's `examples/*.sv`
//! and both `bench/` designs are REFUSED and fall back to the VM. The features
//! doing the refusing are the ones every real testbench has — `$dumpfile`/
//! `$dumpvars`, module ports (which lower to continuous assigns), and an in-body
//! `@(posedge clk);`. The corpus subset this gate runs is real coverage of the
//! LOOP; it is not yet coverage of a design anyone would write.
//!
//! ## What this loop is NOT, and how each absence is kept honest
//!
//! `runnable` below is a THIRD gate layer, under `design_eligibility`
//! (v1 SCOPE) and `NetArena::buildable` (today's STORAGE): it answers "can the
//! executor that exists today run this". Every absence here is a row there, so
//! an unbuilt piece is a REFUSAL rather than a wrong answer:
//!
//! - **cont-assign settle** (`settle_cont_assigns`, the delayed-CA wheel, the
//!   multi-driver resolution) — S1d-4d. 65 of the 72 corpus designs have no
//!   continuous assign at all, so refusing the other 7 costs little; running
//!   them would silently freeze every `assign`.
//! - **the postponed region** (`$strobe`/`$monitor`) and the deferred-assert
//!   regions — the former because `k_dispatch_systask` refuses to REGISTER a
//!   `$monitor` (so a design with one cannot get this far), the latter because
//!   `defer_marks`/`defer_acts` are already S0 rejects.
//! - **`final` blocks** — `run_finals` runs after the loop and is not restated.
//! - **forces** — `reeval_active_forces` has no analogue; `force_release` is an
//!   S0 reject, so `active_forces` is provably empty.
//!
//! ## The one thing that is deliberately NOT restated
//!
//! There is no second copy of "which process does a change wake" here — that is
//! `WakeTable::wake`, whose own differential (S1d-3) pinned it against the
//! engine's `propagate_changes` pass. This loop only decides WHEN to ask.

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
pub(crate) fn runnable(ir: &SimIr, opts: &crate::SimOpts) -> Result<(), &'static str> {
    crate::native::runtime_gate(ir, opts)?;
    executor_rows(ir, opts)
}

/// The rows `runnable` adds on TOP of the runtime gate — split out because
/// `simulate` has already computed the gate's half and recomputing it there
/// would let the published verdict and the executed decision come from two
/// evaluations of the same predicate.
pub(crate) fn executor_rows(ir: &SimIr, opts: &crate::SimOpts) -> Result<(), &'static str> {
    if !ir.cont_assigns.is_empty() {
        // The loop has no `settle_cont_assigns`. Running such a design would not
        // be slightly wrong: every `assign` would simply never evaluate.
        return Err("continuous assigns (S1d-4d settles them)");
    }
    if !opts.final_procs.is_empty() {
        return Err("`final` blocks (the post-loop drain is not restated)");
    }
    for pi in 0..ir.processes.len() as u32 {
        if !body_is_walkable(ir, pi, ir.processes[pi as usize].entry) {
            return Err("an in-body `wait`/`@(…)` waiter, `fork` or subroutine call");
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
    arm_t0(k, ir);
    if k.sched.st.finished {
        // The ONE live `finished` poll: `arm_t0`'s missing-init-proc guard latches
        // it. NOT for a `$finish` in an initializer — an initializer body is
        // straight-line assignments, and `run_body` would return `Step::Finish`
        // rather than set the flag anyway. (An earlier version of this comment
        // said the opposite; review measured it.)
        return finish_kind(k);
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
            return finish_kind(k);
        }
        let mut delta_count: u64 = 0;
        // ── drain the current time to a stable point ──────────────────────
        loop {
            if !k.active.is_empty() {
                // Take the batch so wakes triggered DURING it land in a fresh
                // `active` — the engine's shape, and it is semantic rather than
                // an allocation trick: a process woken by the batch belongs to
                // the NEXT delta, not to the middle of this one.
                let batch = std::mem::take(&mut k.active);
                for r in batch {
                    if k.sched.st.finished {
                        return finish_kind(k);
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
                            return FinishReason::Finish;
                        }
                        Step::Stop => {
                            k.sched.st.finished = true;
                            return FinishReason::Stop;
                        }
                        Step::Fatal => {
                            k.sched.st.finished = true;
                            k.sched.st.had_fatal = true;
                            return FinishReason::Error;
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
                    return FinishReason::DeltaLimit;
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
                    return FinishReason::DeltaLimit;
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
                    return FinishReason::DeltaLimit;
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
            None => return FinishReason::Quiescent,
            Some(t) => t,
        };
        if let Some(lim) = time_limit {
            if next > lim {
                return FinishReason::Quiescent;
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

/// `arm_processes` for the class the gate admits.
///
/// Two halves, and the second is easy to lose: declaration initializers run
/// FIRST and their writes are then UN-DIRTIED. IEEE 1800 §6.21 puts a
/// declaration initializer "before any initial or always block starts", and
/// measurement says that is literal — `reg clk = 0;` must not hand
/// `always @clk` an x→0 edge. Keeping the dirt would hand it exactly that.
fn arm_t0(k: &mut NativeKernel, ir: &SimIr) {
    let inits: Vec<u32> = k.sched.st.init_procs.to_vec();
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
    // Drop what the initializers made dirty — the whole point of running them
    // before arming. The engine splits its dirty list at the mark left by the t0
    // cont-assign settle; here there is no settle (refused), so the initializers
    // are the only writer and the list is drained wholesale into nothing.
    let mut discard = Vec::new();
    k.arena.take_changed(&mut discard);
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
/// (`force_release` is an S0 reject), the clocking commit (`clocking` is an S0
/// reject), and the IN-BODY half of the engine's waiter `retain` (`executor_rows`
/// refuses `Wait`).
///
/// ⚠️ Two corrections to an earlier version of this paragraph, because they are
/// the sentences a maintainer reasons from. (a) The waiter pass is NOT wholly
/// absent: its STATIC `Level` half (`arm = None`) is alive, as the second loop
/// of `WakeTable::wake`. Only the in-body half is refused. (b) There is no
/// "prev-refresh" here to be the arena's `take_changed`, because this store has
/// no `prev` at all — the changed set IS `dirty` membership, and the edge mask
/// resets on a net's first dirtying of the next slot.
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
}

fn finish_kind(k: &NativeKernel) -> FinishReason {
    if k.sched.st.had_fatal {
        FinishReason::Error
    } else {
        FinishReason::Finish
    }
}
