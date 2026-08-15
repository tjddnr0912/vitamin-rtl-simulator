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
//! `Terminator::Call` is refused by the scan below.
//!
//! ⚠️ **The `Call` half of that sentence changed in S3a and the reason matters.**
//! It used to read "`NetArena::build` refuses a non-empty `func_table`, so there
//! is no frame" — a DESIGN-level refusal, which made the `Call` arm unreachable
//! for free. S3a admits subroutine designs (`native::frames`), so frames now
//! exist on this path, and an `Expr::Call` is ANSWERED — through the kernel's
//! composite reader, entirely inside a statement this loop never looks into.
//! What is gone is the free part, not the property.
//!
//! **For a PROCESS body, `body_is_walkable` is the owner of the `Terminator::Call`
//! refusal** — `native::frames` refuses one too, but only inside a FUNCTION body
//! (its `func_body` walk), which is a different block space. A TASK is refused a
//! layer earlier by the `is_task` row; a FUNCTION WITH OUTPUT FORMALS is not:
//! it is not a task, its own body stays inside its frame, and the
//! `Terminator::Call` lives in the PROCESS body — which `frames_admitted` never
//! scans. `frames_tests::s3a_a_call_statement_is_refused_by_the_executor_layer`
//! asserts exactly that (`buildable` says `None`; the executor layer refuses).
//!
//! The cost of that shortness is that both simplifications are LOAD-BEARING. If
//! either family is ever admitted, this loop is wrong rather than incomplete —
//! so both refused terminators are explicit arms that fatal, not a `_`.
//!
//! ⚠️ And the `fork` argument leans on a sidecar the engine treats as loseable:
//! `fork_modes` is a `.velab` trailer, and `fatal_fork_mode_missing` exists
//! precisely because a truncated one can arrive. With it lost, the gate sees an
//! empty table and calls a `Fork`-carrying design eligible. What actually
//! protects THIS walk is `body_is_walkable` returning `false` on `Fork` and
//! `Call` — the gate is the argument, the scan is the guard.
//!
//! ## Suspension (S1d-4c-2c) and what is still NOT here
//!
//! `Delay` now has an arm, and it is written HERE rather than in the tier-3 run
//! loop on purpose: the decision it encodes — Inactive iff `#0` or zero ticks,
//! resume at `now + ticks` saturating — is an IEEE rule, not a storage question,
//! and the two things it needs from the implementor (`k_now`,
//! `k_schedule_resume`) are kernel calls. So both backends get the rule from one
//! spelling, and the engine's copy in `run_process` is the one this was checked
//! against rather than a twin that could drift.
//!
//! `Wait` got its arm in S1d-4c-2d, and it is here for the same reason:
//! WHEN to suspend and what the `wait(expr)` already-true fall-through does are
//! IEEE rules, while WHERE the waiter is filed (and what an `@(sig)` snapshots)
//! is the implementor's — `k_suspend_on`. The corpus contains exactly zero
//! in-body waiters (measured: 138 suspending terminators, all `Delay`), so that
//! model is tested entirely against dedicated designs.
//!
//! `body_is_walkable(ir, proc, entry)` is the precondition, and it must be
//! asked about the SAME entry the walk will use. There is a second, separate
//! obligation the caller also owns: a body containing a `SysTask` this kernel
//! refuses (`$dumpvars`, `$monitor`, `$strobe`, `$dumpfile`, `$fclose`,
//! `$writemem*` — all `eligible: true, buildable: true`) panics inside
//! `k_dispatch_systask`. `native::run::runnable` is where both are now asked.

use sim_ir::{SimIr, Terminator};

use crate::exec::frame_call::OpenFrame;
use crate::exec::{apply_effect, compute_effect, Kernel, Step};

/// Can this process body be executed by the walk below — i.e. does it reach no
/// terminator the walk has no arm for?
///
/// ⚠️ **`Delay` was in this set until S1d-4c-2c and no longer is.** The name
/// changed with the meaning (it used to be called `body_is_suspend_free`)
/// deliberately: a predicate whose truth set moves while its name stays put is
/// how a caller ends up relying on the OLD property. What it refuses now is
/// `Fork`, `Call`, and a `Wait` on a cause nothing can satisfy (`wait fork`, a
/// named event). `Wait{Edge|Level|Expr}` became walkable in S1d-4c-2d.
///
/// A WHOLE-BODY scan, not a first-block one: a `Delay` behind two `Goto`s
/// suspends just as much as one in the entry block. The engine's own
/// `is_codegen_able` walks the whole body for the same reason.
///
/// Indexes `ir.processes[proc].body`, NOT the global `ir.blocks` — that arena
/// holds SUBROUTINE blocks, and using it here found `len 0` on every design back
/// when the `func_table` rejection meant an eligible design had no frames at
/// all. Two different block spaces with the same index type — and since S3a the
/// other one is no longer empty, so the distinction is now load-bearing rather
/// than merely correct.
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
///
/// ⚠️ `call_ok(bb)` is a PARAMETER rather than a lookup because the two askers
/// hold the same fact in two places and neither can reach the other's. The GATE
/// runs before `simulate` installs `SimState::suspendable_tasks`, so it computes
/// the set itself (`frames::suspendable_set`); the WALK runs after, so its
/// kernel reads the installed one. Both derive from `sim_ir::
/// compute_suspendable_tasks` over the same inputs — the same
/// agree-by-construction argument elaborate and the engine already rely on for
/// this exact set — and passing the answer in is what keeps the SCAN itself a
/// single spelling rather than forking it per caller.
#[allow(dead_code)] // The production consumer is S1d-4c-2c's region loop; today
                    // only the body differential calls this. Saying so beats a fake call
                    // site or a widened visibility.
pub(crate) fn body_is_walkable(
    ir: &SimIr,
    proc: u32,
    entry: u32,
    call_ok: &dyn Fn(u32) -> bool,
) -> bool {
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
            // A `Delay` SUSPENDS but the walk now has an arm for it, so it does
            // not disqualify the body — it only means the walk can return
            // `Step::Suspended` and the caller must have somewhere to resume it
            // from. The resume block is pushed as a scan target for the same
            // reason the `Goto` target is: a `Wait` behind a `#5` is still a
            // `Wait` this walk cannot execute.
            Terminator::Delay { resume, .. } => stack.push(*resume),
            // A `Wait` on a cause the waiter model can satisfy is walkable;
            // its resume block joins the scan for the same reason a `Delay`'s
            // does. `Named` and `Fork` are NOT: nothing fires them, so parking
            // on one is a hang. (`Named` is unconstructible today — elaborate
            // lowers named events to a counter net and `@(ev)` to `Level` — but
            // the arm is explicit so a future lowering change is a compile-time
            // decision rather than a silent hang.)
            Terminator::Wait { cond, resume } => match cond {
                sim_ir::WaitCause::Edge { .. }
                | sim_ir::WaitCause::Level { .. }
                | sim_ir::WaitCause::Expr { .. } => stack.push(*resume),
                // ⚠️ The `Fork` half is REACHABLE and this arm is the ONLY
                // thing refusing it — an earlier version of this comment said
                // the S0 `fork` row got there first, and that was measured
                // false. A bare `wait fork;` lowers to `WaitCause::Fork` and
                // populates NO `fork_modes` entry, so such a design is
                // `eligible: true, buildable: true`. Nothing in `fire_waiters`
                // can ever satisfy the cause, so admitting it would park the
                // process forever — a hang and a lost `$display`, not a wrong
                // value. Covered by `s1d4c2c_each_refusal_row_has_a_design`.
                //
                // `Named` really is unconstructible: elaborate lowers a named
                // event to a 64-bit counter net and `@(ev)` to an ordinary
                // `Level` wait, and never builds this variant.
                sim_ir::WaitCause::Named { .. } | sim_ir::WaitCause::Fork => return false,
            },
            // No arm in the walk: `Fork` is gate-refused.
            Terminator::Fork { .. } => return false,
            // A3-i: a SUBSET call is walkable — the arm below delegates it to the
            // engine's synchronous frame executor, exactly as `Expr::Call` has
            // been delegated since S3a. A suspendable callee, or a site with no
            // sidecar entry, is not: `call_site_runnable` owns both reasons, and
            // it is the SAME call the walk's own arm asserts on.
            Terminator::Call { ret_bb, .. } => {
                if !call_ok(bb) {
                    return false;
                }
                stack.push(*ret_bb);
            }
        }
    }
    true
}

/// Hand this activation's open frames to the kernel so they survive the
/// suspension, or do nothing when the walk is not inside one.
///
/// ⚠️ The empty guard is EQUIVALENT today and that was measured, not assumed:
/// removing it stores an empty vector, `k_take_frames` removes and returns it,
/// and the restore over zero frames is a no-op — the mutation survives the whole
/// suite. It stays for hygiene, not for behaviour: `k_take_frames` is asked on
/// every entry, and a stored empty vector is indistinguishable from "this
/// process is parked inside a frame" to any future reader of that map.
fn park_frames<K: Kernel>(k: &mut K, proc: u32, frames: &mut Vec<OpenFrame>) {
    if frames.is_empty() {
        return;
    }
    let taken = std::mem::take(frames);
    k.k_park_frames(proc, taken);
}

/// Run one process body to completion. `Scheduler::run_process` for the class the
/// S0 gate admits, restated over a `Kernel`.
///
/// CALLER OBLIGATION: `body_is_walkable(ir, proc, entry)` must hold for the
/// SAME `entry` passed here. Checked in debug rather than assumed — and note the
/// failure mode is a LOUD panic in every profile (the arms below are
/// `unreachable!`), not a quiet stop. An earlier version of this sentence said
/// the opposite and used the old two-argument spelling; both are corrected in
/// the module doc above, and having the corrected and uncorrected versions of
/// one sentence in one file is its own defect.
#[allow(dead_code)] // ditto — nothing SCHEDULES a tier-3 process yet.
pub(crate) fn run_body<K: Kernel>(k: &mut K, ir: &SimIr, proc: u32, entry: u32) -> Step {
    debug_assert!(
        body_is_walkable(ir, proc, entry, &|bb| k.k_call_site_runnable(proc, bb)),
        "run_body entered for a body reaching a terminator the walk has no arm \
         for (`Wait`, `Fork`, or a `Call` whose callee suspends)"
    );
    let mut bb = entry;
    let mut guard: u64 = 0;
    // This activation's OPEN task frames, innermost last.
    //
    // ⚠️ A3-ii-b ENDED the argument that used to be written here — "it is a
    // LOCAL, not scheduler state, and that is the whole reason this half could
    // be split off … the gate admits only frames that reach no
    // `Delay`/`Wait`/`Fork`, so every frame this walk opens is closed by a
    // `Return` before `run_body` returns. Nothing to stash, nothing to restore."
    // A frame can park now, so the stack has to outlive this Rust call: the two
    // suspend arms hand it to the kernel and this line takes it back.
    //
    // It is still not `Scheduler::activities`. The S0 gate refuses forks, so a
    // tier-3 activity IS its process and the kernel can key the stack by process
    // id — what the engine needs an activity arena for is fork children, which
    // cannot be here. The WINDOW half is not restated at all: both executors call
    // `frame_window::{stash,restore}_windows_in`.
    //
    // Empty for a design with no frame calls, which is every design before A3-i,
    // and empty on a fresh (non-resume) entry.
    let mut frames: Vec<OpenFrame> = k.k_take_frames(proc);
    // Per-process context (`$time`'s multiplier, `%m`'s scope) — the engine
    // installs it on every block activation, and a walk that skipped it would
    // render from whatever process ran last.
    k.k_enter_body(proc);
    loop {
        // TWO BLOCK SPACES, exactly as `run_process`'s frame-aware fetch has: a
        // task frame's PC indexes the GLOBAL `ir.blocks` arena, a process body's
        // indexes `ir.processes[t].body`. Using one for the other is not a subtle
        // bug — it reads a different design's statements.
        let block = match frames.last() {
            Some(f) => &ir.blocks[f.bb as usize],
            None => &ir.processes[proc as usize].body[bb as usize],
        };
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
            // ⚠️ **S3a MADE THIS LIVE.** It used to be unreachable for tier-3 by
            // the same argument that made `Terminator::Call` unreachable: every
            // site that latches `call_fatal` is frame machinery
            // (`state/frame_eval.rs`, `state/task_frames.rs`), which needs a
            // non-empty `func_table`, which `NetArena::build` refused outright.
            // Admitting subroutines re-opens both producers this walk can reach:
            // `$fatal` inside a function body, and a runaway recursion hitting
            // `MAX_CALL_DEPTH`. The old note said "no test can pin it until S3
            // gives the arena frame storage" — S3a is that slice, and the pins
            // are in `frames_tests.rs`.
            //
            // It stays a silent early-return rather than a `panic!` (unlike the
            // arms below) because the fatal has already been REPORTED by whoever
            // latched it; what this does is stop the body, exactly as it does for
            // `K = Scheduler`.
            if k.k_call_fatal() {
                return Step::Fatal;
            }
            // A store that can only RECORD a diagnostic reports it here, at the
            // statement boundary. No-op for `K = Scheduler`, which emits at the
            // access.
            //
            // ⚠️ This is NOT what gets the ORDER right, and saying so was wrong:
            // `format_args_str_with` drains before every `$display`/`$error`
            // line, and `native::run` drains after the body, so no design has
            // been found that this line alone distinguishes (measured —
            // deleting it leaves the workspace suite green and 25 out-of-range
            // designs byte-identical). It is kept as the tightest available
            // backstop, not as a covered behaviour; treat it as unproven.
            k.k_drain_diags();
        }
        // `run_process`'s `set_pos!`, and it exists for the same reason: every
        // non-suspending terminator has to write the TOP FRAME's PC when one is
        // open and the process's otherwise.
        macro_rules! set_pos {
            ($t:expr) => {
                match frames.last_mut() {
                    Some(f) => f.bb = $t,
                    None => bb = $t,
                }
            };
        }
        match &block.term {
            Terminator::Goto { target } => {
                let t = *target;
                set_pos!(t);
            }
            Terminator::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                let t = if k.k_truthy(*cond) {
                    *then_bb
                } else {
                    *else_bb
                };
                set_pos!(t);
            }
            Terminator::Return => {
                // A3-ii-a: a `Return` INSIDE a driven frame pops it and resumes
                // the parent — it is not the end of the process. `run_process`'s
                // pop, with the two things that must not be reordered kept in
                // order by the seam itself (`k_exit_driven_frame` captures the dyn
                // out/inout results before the restore and installs them after).
                if let Some(f) = frames.pop() {
                    let writes = k.k_exit_driven_frame(f.callee, &f.out_binds, f.dyn_stash);
                    for (lval, val) in writes {
                        let offs = k.k_resolve_lvalue_offsets(&lval);
                        k.k_write_lvalue(&lval, val, &offs);
                    }
                    // The parent's PC. `ret_bb` is a GLOBAL block id when the
                    // parent is another frame and a process-local one when the
                    // stack is now empty — the same overload `FrameRec::ret_bb`
                    // carries, read the same way (by whether anything is left).
                    set_pos!(f.ret_bb);
                    // …and fall through to the loop tail, which charges the step
                    // budget. A `continue` here would let a recursive task that
                    // returns immediately spin without ever being counted.
                    //
                    // ⚠️ The `else` below is LOAD-BEARING and was missing in the
                    // first draft: without it a frame's `Return` popped correctly
                    // and then ran straight into `k_rearm` + `Step::Done`, ending
                    // the whole process at the first task return. `rustc` found it
                    // as "value assigned to `bb` is never read" — the dead store
                    // WAS the bug.
                } else {
                    // `proc` is a TEMPLATE id here and `Scheduler::rearm` indexes
                    // ACTIVITIES. They are the same number only because the S0
                    // gate refuses forks, so base activities stay 1:1 with
                    // processes and `tie == template == declaration index`.
                    // `run_process` keeps the two apart (`pi` for re-arm,
                    // `activity_template(pi)` for the body); this walk collapses
                    // them, and that is safe exactly as long as the fork row
                    // holds.
                    k.k_rearm(proc);
                    return Step::Done;
                }
            }
            // SUSPEND (S1d-4c-2c). `run_process`'s non-frame branch, verbatim —
            // and it is written ONCE here rather than twice because the two
            // things it needs are kernel calls: `k_now` (the clock the resume
            // tick is measured from) and `k_schedule_resume` (whoever owns the
            // region queues). The frame branch is absent because this walk never
            // runs INSIDE a frame: an `Expr::Call` is answered whole, inside one
            // statement, by the engine's frame executor, and an admitted
            // subroutine body may not carry a suspending terminator at all (a
            // `native::frames` row). So `in_frame` is still structurally false —
            // for that reason since S3a, rather than because no frame can exist.
            //
            // The two rules that are NOT arithmetic:
            //  - `#0` AND a `#d` that evaluates to ZERO ticks both go INACTIVE.
            //    An X/Z delay amount yields 0 ticks (`delay_ticks_of`), so
            //    `#(1'bx)` is a `#0` — dropping the `ticks == 0` half would put
            //    it back in Active and let it run before this delta's other
            //    processes.
            //  - the tick is `now + ticks` SATURATING. `delay_ticks_of` returns
            //    `u64::MAX` for a negative real delay, which must mean "never
            //    fires", and a wrapping add would turn it into a resume in the
            //    past.
            Terminator::Delay {
                amount,
                region,
                resume,
            } => {
                let ticks = k.k_delay_ticks(*amount);
                let inactive = matches!(region, sim_ir::DelayRegion::Inactive) || ticks == 0;
                let tick = k.k_now().saturating_add(ticks);
                // A3-ii-b: record WHERE this position resumes before parking.
                // `set_pos!` writes the frame's PC when the walk is inside one
                // and the process's when it is not — the same two-space
                // distinction the fetch at the top of the loop makes, and the
                // reason the resume can be a global block id without the wake
                // table ever learning about frames. With no frames open, `bb`
                // is `*resume` and this is byte-identical to what it replaced.
                set_pos!(*resume);
                k.k_schedule_resume(proc, bb, tick, inactive);
                park_frames(k, proc, &mut frames);
                return Step::Suspended;
            }
            // IN-BODY WAIT (S1d-4c-2d). `run_process`'s non-frame branch again,
            // and again written once because what it needs is kernel calls.
            //
            // The `Expr` arm is the one with control flow in it: `wait(e)` with
            // `e` ALREADY TRUE does not suspend at all, it falls through to the
            // resume block — and that fall-through has to charge the step guard,
            // or `wait(1)` in a loop spins forever instead of reporting F4027.
            Terminator::Wait { cond, resume } => {
                if let sim_ir::WaitCause::Expr { expr } = cond {
                    if k.k_truthy(*expr) {
                        // ⚠️ `set_pos!`, not `bb = …`. This fall-through was
                        // written for a process body because a frame could not
                        // reach a `Wait` at all; A3-ii-b makes it reachable, and
                        // the process spelling would have moved the PROCESS's PC
                        // while the walk kept fetching from the frame — the
                        // wrong design's statements, silently.
                        set_pos!(*resume);
                        guard += 1;
                        if guard > k.k_max_deltas() {
                            k.k_mark_fatal();
                            return Step::Fatal;
                        }
                        continue;
                    }
                }
                // `Named` cannot appear: elaborate lowers a named event to a
                // 64-bit counter net and `@(ev)` to an ordinary `Level` wait on
                // it (`stmt_main.rs` says the variant stays "reserved-unused").
                // `Fork` needs `fork_modes`, an S0 reject. Both would suspend on
                // a cause nothing here can satisfy — a hang, not a wrong value —
                // so they are refused by `body_is_walkable` rather than parked.
                set_pos!(*resume);
                k.k_suspend_on(proc, bb, cond);
                park_frames(k, proc, &mut frames);
                return Step::Suspended;
            }
            Terminator::Fork { .. } => unreachable!(
                "tier-3 body walk reached `Fork` — `fork_modes` non-empty is an S0 \
                 reject, so this is a gate widening without a walk to match"
            ),
            // A3-i: a SUBSET subroutine CALL STATEMENT — a task enable, or a call
            // to a function with output formals. Delegated to the engine's
            // synchronous frame executor through `exec::frame_call`, which is the
            // same body `run_process`'s subset arm runs; only the copy-in read and
            // the copy-out write are this kernel's.
            //
            // No `Step` comes back because a subset callee cannot suspend — that
            // is the definition of the half `call_site_runnable` admits, and the
            // other half is refused a layer up rather than parked here.
            Terminator::Call { ret_bb, .. } => {
                // ⚠️ A call site is keyed by (template, PROCESS-LOCAL block), so a
                // nested call — one whose `Terminator::Call` sits in the GLOBAL
                // arena, inside a frame body — is a different table
                // (`task_calls_func`). The mode split therefore has to be asked of
                // the position we are actually at.
                let in_frame = frames.last().map(|f| f.bb);
                match crate::exec::frame_call::call_here(k, proc, bb, in_frame) {
                    // A3-i: the callee runs synchronously, whole, in the engine's
                    // `&self` frame executor. Nothing is opened.
                    crate::exec::frame_call::Taken::Done => set_pos!(*ret_bb),
                    // A3-ii-a: the callee is DRIVEN by this walk. Push it and let
                    // the loop fetch from `ir.blocks` next iteration; its `Return`
                    // pops and lands the parent on `ret_bb`.
                    crate::exec::frame_call::Taken::Opened(f) => frames.push(f),
                }
            }
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
