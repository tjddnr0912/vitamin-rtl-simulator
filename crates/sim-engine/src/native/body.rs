//! S1d-4c-2b (doc-21 §5 S1d-4) — **the tier-3 body walk.**
//!
//! The first time this backend executes a PROCESS rather than a statement.
//! Everything below the terminator is already shared: `compute_effect` and
//! `apply_effect` are generic over `K: Kernel`, so statement meaning comes from
//! the same code the engine runs. What this file adds is the block loop and the
//! terminator decisions — `run_process`'s job, which is `Scheduler`-fixed and so
//! is the one piece that had to be restated.
//!
//! ## Why it is short, and what that costs
//!
//! `run_process` is ~500 lines; this is a fraction of that, and the difference
//! is not cleverness — it is the S0 gate. (An earlier version of this line said
//! 498, which no counting convention produced even before this slice added four
//! lines to it — a line count in a comment is stale the moment anything edits
//! the function it counts.) Fork children, join barriers and the call
//! stack are the bulk of the engine's loop, and every one of them is refused:
//! `fork_modes` non-empty is an S0 reject (so activities are 1:1 with processes,
//! `is_child` is always false, and `Terminator::Fork` cannot appear), and
//! `NetArena::build` refuses a non-empty `func_table` (so there is no frame and
//! `Terminator::Call` cannot appear).
//!
//! The cost of that shortness is that both simplifications are LOAD-BEARING. If
//! either family is ever admitted, this loop is wrong rather than incomplete —
//! so both refused terminators are explicit arms that fatal, not a `_`.
//!
//! ⚠️ And the `fork` argument leans on a sidecar the engine treats as loseable:
//! `fork_modes` is a `.velab` trailer, and `fatal_fork_mode_missing` exists
//! precisely because a truncated one can arrive. With it lost, the gate sees an
//! empty table and calls a `Fork`-carrying design eligible. What actually
//! protects THIS walk is `body_is_suspend_free` returning `false` on `Fork` and
//! `Call` — the gate is the argument, the scan is the guard.
//!
//! ## What is NOT here
//!
//! `Delay` and `Wait` SUSPEND, and a suspension needs somewhere to be resumed
//! from — the region queues and the delta loop, which are S1d-4c-2c. Their arms
//! here are `unreachable!`, so a caller that skips the precondition gets a LOUD
//! panic in every profile, not a quiet stop — an earlier version of this
//! paragraph described the opposite failure mode, and describing a panic as a
//! silent hang is worse than saying nothing.
//!
//! `body_is_suspend_free(ir, proc, entry)` is the precondition, and it must be
//! asked about the SAME entry the walk will use. There is a second, separate
//! obligation the caller also owns: a body containing a `SysTask` this kernel
//! refuses (`$dumpvars`, `$monitor`, `$strobe`, `$dumpfile`, `$fclose`,
//! `$writemem*` — all `eligible: true, buildable: true`) panics inside
//! `k_dispatch_systask`. No corpus body this walk runs contains a `SysTask` at
//! all, so nothing exercises it; 4c-2c's scheduler must check both.

use sim_ir::{SimIr, Terminator};

use crate::exec::{apply_effect, compute_effect, Kernel, Step};

/// Can this process body be executed by the walk below — i.e. does it reach no
/// suspending or gate-refused terminator?
///
/// A WHOLE-BODY scan, not a first-block one: a `Delay` behind two `Goto`s
/// suspends just as much as one in the entry block. The engine's own
/// `is_codegen_able` walks the whole body for the same reason.
///
/// Indexes `ir.processes[proc].body`, NOT the global `ir.blocks` — that arena
/// holds TASK FRAME blocks, and using it here found `len 0` on every design,
/// because the `func_table` rejection means an eligible design has no frames at
/// all. Two different block spaces with the same index type.
///
/// ⚠️ Takes the ENTRY to scan from, and that parameter is the whole point. It
/// used to scan from `ir.processes[proc].entry` while `run_body` walks from the
/// caller's `entry` — and the only reason that parameter exists is to resume
/// somewhere else. The two sets differ whenever a block is unreachable from the
/// process entry but reachable from a resume point, which `disable <named
/// block>` produces on a design the gate reports eligible and buildable
/// (`DisableKind::Scope` is deliberately not counted by the `disable_fork` row).
/// Measured: such a design has a `Delay` in a block unreachable from entry, so
/// the old predicate answered "suspend-free" and the walk then panicked blaming
/// the caller for a check it had passed.
#[allow(dead_code)] // The production consumer is S1d-4c-2c's region loop; today
                    // only the body differential calls this. Saying so beats a fake call
                    // site or a widened visibility.
pub(crate) fn body_is_suspend_free(ir: &SimIr, proc: u32, entry: u32) -> bool {
    let body = &ir.processes[proc as usize].body;
    // A body with no blocks answers "nothing to suspend on" rather than indexing
    // out of range. Not constructible from elaborate today; a `pub(crate)`
    // predicate should still answer instead of panicking.
    if body.is_empty() || entry as usize >= body.len() {
        // A `pub(crate)` predicate answers instead of panicking. Neither shape is
        // constructible from elaborate today; the empty-body guard was added for
        // the same reason and stopping half way through the argument was the
        // inconsistency.
        return true;
    }
    let mut seen = vec![false; body.len()];
    let mut stack = vec![entry];
    while let Some(bb) = stack.pop() {
        if seen[bb as usize] {
            continue;
        }
        seen[bb as usize] = true;
        match &body[bb as usize].term {
            Terminator::Goto { target } => stack.push(*target),
            Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                stack.push(*then_bb);
                stack.push(*else_bb);
            }
            Terminator::Return => {}
            // Suspends (4c-2c) or is gate-refused (fork/call).
            Terminator::Delay { .. }
            | Terminator::Wait { .. }
            | Terminator::Fork { .. }
            | Terminator::Call { .. } => return false,
        }
    }
    true
}

/// Run one process body to completion. `Scheduler::run_process` for the class the
/// S0 gate admits, restated over a `Kernel`.
///
/// CALLER OBLIGATION: `body_is_suspend_free(ir, proc, entry)` must hold for the
/// SAME `entry` passed here. Checked in debug rather than assumed — and note the
/// failure mode is a LOUD panic in every profile (the arms below are
/// `unreachable!`), not a quiet stop. An earlier version of this sentence said
/// the opposite and used the old two-argument spelling; both are corrected in
/// the module doc above, and having the corrected and uncorrected versions of
/// one sentence in one file is its own defect.
#[allow(dead_code)] // ditto — nothing SCHEDULES a tier-3 process yet.
pub(crate) fn run_body<K: Kernel>(k: &mut K, ir: &SimIr, proc: u32, entry: u32) -> Step {
    debug_assert!(
        body_is_suspend_free(ir, proc, entry),
        "run_body entered for a body that can suspend — the walk has nowhere to \
         resume from until S1d-4c-2c"
    );
    let mut bb = entry;
    let mut guard: u64 = 0;
    // Per-process context (`$time`'s multiplier, `%m`'s scope) — the engine
    // installs it on every block activation, and a walk that skipped it would
    // render from whatever process ran last.
    k.k_enter_body(proc);
    loop {
        // PROCESS-LOCAL indexing: a process body is `ir.processes[t].body`, not
        // the global `ir.blocks` arena (that one is for task frames, which the
        // `func_table` rejection makes unreachable here).
        let block = &ir.processes[proc as usize].body[bb as usize];
        for &sid in &block.stmts {
            let effect = compute_effect(&*k, &ir.stmts[sid as usize], sid);
            if let Some(step) = apply_effect(k, effect) {
                return step; // a SysTask returned Finish/Stop/Fatal
            }
            // A fatal raised from a `&self` eval context can only latch a Cell;
            // consume it at the statement boundary so the process STOPS where the
            // fatal happened rather than running the rest of its body on state
            // the fatal just declared invalid.
            //
            // ⚠️ CANNOT FIRE for the class this walk admits, by the SAME argument
            // that makes `Terminator::Call` unreachable: every site that latches
            // `call_fatal` is frame machinery (`state/frame_eval.rs`,
            // `state/task_frames.rs`), which needs a non-empty `func_table`, which
            // `NetArena::build` refuses. Measured — a design that latches it
            // reports `buildable: false, refused: "frame-local storage"`, and a
            // module-level `$fatal` takes `apply_effect`'s `Ctl::Fatal` branch
            // instead. It is kept, not deleted, because this loop is generic and
            // the rule IS load-bearing for `K = Scheduler`: with the check
            // removed, a body whose fatal came from a frame runs on to print past
            // it (measured by routing the engine through this walk).
            //
            // Unlike the `unreachable!` arms below, this stays a silent
            // early-return — a `panic!` here would be wrong for the implementor
            // that CAN reach it. No test can pin it until S3 gives the arena
            // frame storage.
            if k.k_call_fatal() {
                return Step::Fatal;
            }
        }
        match &block.term {
            Terminator::Goto { target } => bb = *target,
            Terminator::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                bb = if k.k_truthy(*cond) {
                    *then_bb
                } else {
                    *else_bb
                };
            }
            Terminator::Return => {
                // `proc` is a TEMPLATE id here and `Scheduler::rearm` indexes
                // ACTIVITIES. They are the same number only because the S0 gate
                // refuses forks, so base activities stay 1:1 with processes and
                // `tie == template == declaration index`. `run_process` keeps the
                // two apart (`pi` for re-arm, `activity_template(pi)` for the
                // body); this walk collapses them, and that is safe exactly as
                // long as the fork row holds.
                k.k_rearm(proc);
                return Step::Done;
            }
            // EXPLICIT arms, not a `_`: each is unreachable for a DIFFERENT
            // reason, and a wildcard would let a future gate widening land here
            // silently. `Delay`/`Wait` need the region queues; `Fork`/`Call` are
            // refused by `fork_modes` and by `NetArena::build`'s `func_table`
            // rejection respectively.
            Terminator::Delay { .. } | Terminator::Wait { .. } => {
                unreachable!(
                    "tier-3 body walk reached a suspending terminator — \
                     `body_is_suspend_free` was not consulted (S1d-4c-2c builds the \
                     region queues this would resume from)"
                )
            }
            Terminator::Fork { .. } => unreachable!(
                "tier-3 body walk reached `Fork` — `fork_modes` non-empty is an S0 \
                 reject, so this is a gate widening without a walk to match"
            ),
            Terminator::Call { .. } => unreachable!(
                "tier-3 body walk reached `Call` — `NetArena::build` refuses a \
                 non-empty `func_table`, so there is no frame to call into"
            ),
        }
        // The in-body step budget. NOT `max_deltas`: a long computation that never
        // suspends is not the same failure as a delta loop that never settles, and
        // conflating them once reported an ordinary `for` loop as a combinational
        // oscillation (round-25).
        guard += 1;
        if guard > k.k_max_deltas() {
            k.k_mark_fatal();
            return Step::Fatal;
        }
    }
}
