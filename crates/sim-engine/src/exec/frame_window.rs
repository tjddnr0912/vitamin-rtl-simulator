//! Per-activity frame WINDOW and dyn-array PARK/UNPARK — split out of `exec/process.rs`
//! (mechanical move; module-size policy). These four run at the two points an activity
//! suspends and resumes, and the entry guard is their fail-closed backstop.

use super::*;

use crate::sched::FrameRec;

/// Execute activity `pi` starting at body block `start`. `pi` is a runtime
/// ACTIVITY id (index into `Scheduler::activities`), NOT a declaration index —
/// the body/sensitivity are resolved through `activities[pi].template`.
/// Round-14 V3/V4 Phase 3: pop this activity's live AUTOMATIC frame windows off the
/// SHARED `frame_stack` into their `FrameRec`s before the activity suspends — so an
/// interleaving activity's frame calls can't corrupt them (a `frame_slot_read` always
/// reads `frame_stack.last()`, which must be THIS activity's window only while it runs).
/// Popped TOP-first (reverse call order), matching `enter_task_frame`'s push order.
pub(crate) fn stash_frame_windows(sched: &mut Scheduler, pi: u32) {
    stash_windows_in(sched.st, &mut sched.activities[pi as usize].call_stack);
}

/// A3-ii-b EXTRACTION — the same operation over a call stack the CALLER owns.
///
/// Threading the stack rather than the activity id is what lets both executors
/// share ONE spelling of the stash — and this is the operation that must not
/// have two, because a wrong pop order does not fail loudly. It leaves the
/// interleaving activity reading another frame's window, i.e. a wrong value at
/// exit 0.
///
/// ⚠️ A3-ii-b's stated REASON for the parameter has expired, and only the
/// parameter survived it. It said tier-3 "has no `Scheduler::activities`: the S0
/// gate refuses forks, so its activities are 1:1 with processes and its run loop
/// keeps the stack itself". A4-a gave tier-3 activities and A4-b moved its
/// parked stacks into `activities[act].call_stack` — the same place the engine
/// keeps them, because `exec_fork_into` reads it to spawn a fork-in-frame's
/// arms. The borrowed-slice form is still the right one: tier-3's walk holds its
/// stack in a LOCAL while it runs and only hands it over at a suspension, so
/// there is a window in which the arena's copy is legitimately empty.
pub(crate) fn stash_windows_in(st: &crate::SimState<'_>, frames: &mut [FrameRec]) {
    for i in (0..frames.len()).rev() {
        if st.func_has_auto[frames[i].callee as usize] {
            let w = st
                .frame_stack
                .borrow_mut()
                .pop()
                .expect("frame window to stash");
            frames[i].window = Some(w);
        }
    }
    park_dyn_in(st, frames);
}

/// The inverse of [`stash_frame_windows`]: push the stashed windows back onto
/// `frame_stack` in call order (bottom `FrameRec` first) so this activity's live frame
/// context is on top again before it resumes executing the frame CFG.
pub(crate) fn restore_frame_windows(sched: &mut Scheduler, pi: u32) {
    restore_windows_in(sched.st, &mut sched.activities[pi as usize].call_stack);
}

/// [`stash_windows_in`]'s inverse, on the same borrowed stack.
pub(crate) fn restore_windows_in(st: &crate::SimState<'_>, frames: &mut [FrameRec]) {
    for f in frames.iter_mut() {
        // Tolerant: only a STASHED (`Some`) window is pushed back; a live/None frame is
        // skipped, so a spurious call during normal in-frame execution is a no-op.
        if let Some(w) = f.window.take() {
            st.frame_stack.borrow_mut().push(w);
        }
    }
    unpark_dyn_in(st, frames);
}

/// Give a frame-local dynamic array the same per-activation lifetime the AUTOMATIC
/// window already has: take this activity's live dyn-array contents OUT of the net-keyed
/// heap while it is suspended, and put them back when it resumes.
///
/// Without this, two CONCURRENT activations of one task shared the slot. The entry
/// stash (`frame_dyn_enter` → `frame_dyn_exit`) is sound only when the `[enter, exit]`
/// intervals NEST; with a `fork` they overlap, so B's entry stashed A's live array and A
/// resumed onto B's. That was a graceful F4004 rather than a wrong answer, and it is what
/// this lifts — the same fix shape as the window, at the same two points.
///
/// ONLY the TOP frame is parked. The heap slots always hold the top frame's values; an
/// OUTER activation's values already live in the `dyn_stash` of the frame above it (that
/// is what recursion depends on), so parking every frame would have the outer frame park
/// the INNER frame's array and hand it back on resume.
///
/// Two frames are exempt, both because something ELSE can still reach their slots:
///
/// - a fork ARM frame, for the same reason it stashes nothing: it is not a fresh
///   activation of its callee, it rides the parent's. Otherwise an arm that suspends could
///   carry off the parent's live array and still be holding it when the parent resumes at
///   a `join_any`/`join_none`.
/// - a frame that has FORKED (`FrameRec::forked`). Its arms run in it and read its locals,
///   and unlike the automatic window — which the arms share through a
///   `WindowSlot::Shared` handle — a parked dyn array is simply GONE from the heap, so the
///   arm reads X. Measured: a `fork begin … a[0] … end join` inside a task printed `a0=x`.
///   The mark is sticky rather than cleared at the join, so such a frame gives up its
///   isolation from CONCURRENT activations. That combination — two activations of a task
///   that itself forks — is loud today for an unrelated pre-existing reason (a fork inside
///   a fork child overflows the tie encoding, F4004), so it cannot reach the unisolated
///   path; if that cap is ever raised, the fail-closed entry guard below is what keeps it
///   loud rather than wrong.
fn park_dyn_in(st: &crate::SimState<'_>, frames: &mut [FrameRec]) {
    let Some(top) = frames.last() else {
        return;
    };
    if top.is_arm || top.forked {
        return;
    }
    let parked = st.frame_dyn_enter(top.callee);
    if !parked.is_empty() {
        frames.last_mut().unwrap().dyn_parked = parked;
    }
}

/// Inverse of [`park_frame_dyn`]. Tolerant: an empty park list is a no-op, so a call
/// during normal in-frame execution costs nothing.
fn unpark_dyn_in(st: &crate::SimState<'_>, frames: &mut [FrameRec]) {
    let Some(top) = frames.last_mut() else {
        return;
    };
    if top.dyn_parked.is_empty() {
        return;
    }
    let parked = std::mem::take(&mut top.dyn_parked);
    st.frame_dyn_exit(parked);
}

/// T1-9: may this SUSPENDABLE frame entry proceed on the per-activation dyn stash?
///
/// The stash makes a net-keyed frame-local dyn array per-activation by taking the outer
/// contents at entry and putting them back at exit — sound exactly when the `[enter, exit]`
/// intervals NEST. An empty or all-`None` stash means no outer activation held the slot,
/// so there is nothing to nest inside and the entry is always fine.
///
/// An occupied slot is now expected to mean exactly one thing: RECURSION, where the holder
/// is on this activity's own call stack and returns before its parent by construction.
/// CONCURRENT activations used to land here too and were the reason this fatal existed;
/// they no longer can, because a suspended activity's slots are PARKED off the heap (see
/// `park_frame_dyn`) and the scheduler is single-threaded — so the only activation that
/// can be holding a slot at another's entry is the one currently running, i.e. `pi`.
///
/// This therefore guards an INVARIANT, not a capability, and is kept fail-closed rather
/// than asserted: if a future change breaks parking, a wrong array is a silent-wrong and
/// this is the one place that can still catch it.
pub(crate) fn frame_dyn_park_invariant_ok(
    sched: &Scheduler,
    pi: u32,
    callee: u32,
    stash: &[(u32, Option<crate::state::DynObj>)],
) -> bool {
    if !stash.iter().any(|(_, o)| o.is_some()) {
        return true;
    }
    sched.activities[pi as usize]
        .call_stack
        .iter()
        .any(|f| f.callee == callee)
}
