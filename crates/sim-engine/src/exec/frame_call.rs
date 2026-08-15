//! A3-i (ROADMAP §5.1-n) — **the SUBSET task call, once, for both backends.**
//!
//! `run_process`'s `Terminator::Call` arm has two halves. The suspendable half
//! pushes a `FrameRec` and drives the callee's CFG across suspensions; the SUBSET
//! half runs the callee synchronously through `SimState::run_task_call` and
//! writes its output formals back to the caller's lvalues. This file is the
//! second half, lifted out of that arm and made generic over [`Kernel`] so the
//! tier-3 walk performs it through ITS store instead of the engine's.
//!
//! ## What was actually store-dependent, measured rather than assumed
//!
//! Almost nothing. The frame window (`frame_stack` / `static_store`), the dyn
//! heap, the file table, the RNG and the diagnostic sink all live in `SimState`,
//! which BOTH kernels borrow — one object, not two stores — exactly as A1-iv-b
//! found for the file table. So `run_task_call` and the six `frame_dyn_*`
//! operations around it need no routing at all.
//!
//! Two things did:
//!
//!  * the COPY-IN. `Scheduler::split_frame_in_binds` sizes each actual to its
//!    formal's declared type and evaluates it with `eval_ctx_top`, which is
//!    hard-wired to the engine's nets. On a tier-3 run every scalar actual would
//!    have read X. That is the A1-ii defect verbatim, so the fix is the same one:
//!    the body moves here and the read becomes `k_eval_ctx`.
//!  * the COPY-OUT. The engine writes each output formal with
//!    `sched.resolve_lvalue_offsets` + `sched.st.write_lvalue` — the engine's
//!    funnel. Here they are `k_resolve_lvalue_offsets` + `k_write_lvalue`.
//!
//! For `K = Scheduler` both reduce to the calls that arm already made, so the
//! engine path is mechanically byte-identical rather than merely equivalent.
//!
//! ## Why the writes are collected instead of interleaved
//!
//! [`Kernel::k_run_subset_task`] takes `&mut self`, and so does `k_write_lvalue`;
//! they cannot nest. So the seam performs the DYN copy-outs in place (they land
//! in the shared heap) and RETURNS the scalar ones, which this file then writes.
//! Deferring them is unobservable, and the argument is a disjointness one rather
//! than a hope: a dyn out-formal's destination is a heap-handle net and a scalar
//! out-formal's is not, so no write in the returned list can be read or clobbered
//! by one performed inside; and the returned list preserves `out_binds` order, so
//! two scalar out-formals aliasing the same net still resolve last-wins the same
//! way. It is the same shape, and the same argument, as A1-iii's `TaskWrites`.

use sim_ir::NetKind;

use crate::exec::Kernel;
use crate::value::Value;

/// Does this subroutine body actually PARK — reach a terminator that ends the
/// activation before `Return`?
///
/// The question `suspendable_set` does NOT answer. That set names the executor
/// the engine picks (the `&self` synchronous one cannot print, write out of
/// frame, or schedule an NBA); this names the property tier-3's walk turns on,
/// because a frame that never parks has an activation that begins and ends
/// inside one `run_body` call.
///
/// TRANSITIVE through nested calls: a body whose own terminators are all
/// `Goto`/`Branch`/`Return` still parks if it CALLS something that parks. The
/// callee is reached through `Terminator::Call.target`, which is the callee's
/// entry block — the same mapping `compute_suspendable_tasks` builds.
///
/// ⚠️ **The transitive arm is REDUNDANT TODAY, and that was measured rather than
/// argued.** A mutation that drops it survives the whole suite, and the reason is
/// structural: `frames_admitted` asks this question of EVERY func in the table,
/// so a design whose only parking task is a nested callee is refused by that
/// callee's own row — the caller's answer never decides anything. It is kept
/// because it is the fail-closed direction and because the redundancy is a
/// property of the CALLER (`frames_admitted` visiting everything), not of this
/// function: a future gate that asks only about called funcs would need it back.
/// Recorded as redundant instead of presented as load-bearing.
///
/// Reachability from `entry` only, like every other walk in this file: a block
/// the CFG cannot reach cannot park.
///
/// ⚠️ ONE block-visited set across the WHOLE transitive walk, not one per callee,
/// and it is what terminates on a RECURSIVE task. A first draft recursed per
/// callee with a fresh set; `task automatic t(...); … t(n-1); … endtask` would
/// have re-entered `t`'s entry block forever. The blocks of every func live in
/// one arena, so one set is both correct and sufficient.
/// ⚠️⚠️ **A4-b DELETED `frame_park_kinds`, `ParkKinds` and `frame_forks`, and the
/// deletion is the record of a family closing.** A3-ii-b built the walk to split
/// one refusal in two — "does this frame park on a `Delay`/`Wait`" (which it then
/// admitted) versus "does it reach a `Fork`" (which it kept refusing) — and its
/// own doc measured the split: 65 frames park on a `Wait` edge, 19 on a `Delay`,
/// 13 reach a `Fork`. A4-b admits the third, so nothing asks the question and
/// both fields were write-only.
///
/// Its two fail-closed early returns are NOT lost. Both — an out-of-range block
/// id and an unresolvable nested `Call` target, each a malformed sidecar —
/// are refused by `native::frames::driven_body_is_runnable`, which runs under
/// exactly the condition this walk's caller used (`fd.is_task && susp`).
///
/// Can the walk run the call described by `site`? ONE spelling for the GATE
/// (`native::frames::callee_mode`) and the walk's own `debug_assert`.
///
/// ⚠️ It exists because the two got out of step the moment A3-ii-a widened the
/// gate: the runtime twin still asked A3-i's question (`is the callee
/// synchronous`), so a driven frame the gate had just admitted tripped
/// `run_body`'s precondition assert. A predicate with two spellings does not stay
/// in agreement across a widening — it only looks like it does until one moves.
pub(crate) fn site_runnable(
    ir: &sim_ir::SimIr,
    susp: &std::collections::BTreeSet<u32>,
    site: Option<&crate::TaskCallInfo>,
) -> bool {
    let Some(info) = site else {
        return false;
    };
    if !susp.contains(&info.callee) {
        return true; // A3-i: synchronous, delegated whole
    }
    // A3-ii-a drove it "provided it never parks"; A3-ii-b drove it even when it
    // does; A4-b drives it when it FORKS, which was the last clause. A callee
    // with no `ir.funcs` entry is still refused — that is a malformed sidecar,
    // and fail-closed is the direction for an accept gate.
    ir.funcs.get(info.callee as usize).is_some()
}

/// One OPEN driven frame.
///
/// ⚠️ A3-ii-b DELETED the reduced twin this used to be. A3-ii-a defined an
/// `OpenFrame` as "the three fields `run_process` keeps in a `FrameRec` and
/// nothing more — this walk's frames never park, so there is no `window`, no
/// `dyn_parked`, no `forked`, no `is_arm`", and said the ABSENCE was the point:
/// each missing field existed in the engine for a suspension the gate refused.
/// This slice admits that suspension, so the absent fields are exactly what is
/// now needed and the two types have collapsed into one — which is also what
/// lets both executors share `frame_window`'s stash rather than spell it twice.
pub(crate) type OpenFrame = crate::sched::FrameRec;

/// What the walk should do with the `Terminator::Call` it is standing on.
pub(crate) enum Taken {
    /// The call is finished; advance to `ret_bb`.
    Done,
    /// A frame was opened; drive it, and its `Return` will land the parent.
    Opened(OpenFrame),
}

/// Perform — or OPEN — the call at the position the walk is standing on.
///
/// `in_frame` is the caller's own frame PC when the walk is inside one, and it
/// selects the SIDE TABLE: a call site in a process body is keyed by
/// `(template, process-local block)` in `task_calls_proc`, a nested one by its
/// GLOBAL block id in `task_calls_func`. Reading the wrong table does not fail
/// loudly — it misses, and a miss means "advance past the call", i.e. the call
/// silently does not happen.
pub(crate) fn call_here<K: Kernel>(k: &mut K, proc: u32, bb: u32, in_frame: Option<u32>) -> Taken {
    let site = match in_frame {
        Some(gbb) => k.k_nested_call_site(gbb),
        None => k.k_task_call_site(proc, bb),
    };
    let Some(info) = site else {
        // A site with no sidecar entry is a deferred hierarchical enable whose
        // actuals elaborate could not resolve. The engine advances past it; so do
        // we. The tier-3 gate refuses such a site, and saying so is the point.
        return Taken::Done;
    };
    let (in_vals, dyn_snaps) = split_in_binds(k, &info);
    if k.k_callee_is_driven(info.callee) {
        // A3-ii-a: DRIVEN. Enter the frame here — the window has to be live
        // before the walk fetches the callee's first block — and hand the walk
        // the record that closes it.
        let dyn_stash = k.k_enter_driven_frame(info.callee, &in_vals, &dyn_snaps);
        return Taken::Opened(OpenFrame {
            callee: info.callee,
            bb: k.k_ir().funcs[info.callee as usize].entry,
            ret_bb: bb_after(k, proc, bb, in_frame),
            out_binds: info.out_binds,
            dyn_stash,
            // The three park fields, at their engine defaults. `forked`/`is_arm`
            // stay false for the life of the frame — the S0 gate refuses forks,
            // so tier-3 has neither a forking frame nor an arm frame — and
            // `window`/`dyn_parked` are filled by `stash_windows_in` at the
            // moment the frame parks, exactly as `run_process`'s are.
            window: None,
            dyn_parked: Vec::new(),
            forked: false,
            is_arm: false,
        });
    }
    // A3-i: SYNCHRONOUS — the whole call happens inside the engine's `&self`
    // frame executor and only the copy-out is ours.
    let writes = k.k_run_subset_task(info.callee, &in_vals, &dyn_snaps, &info.out_binds);
    for (lval, val) in writes {
        let offs = k.k_resolve_lvalue_offsets(&lval);
        k.k_write_lvalue(&lval, val, &offs);
    }
    Taken::Done
}

/// The `ret_bb` of the `Terminator::Call` the walk is standing on, read from
/// whichever block space that position lives in.
fn bb_after<K: Kernel>(k: &K, proc: u32, bb: u32, in_frame: Option<u32>) -> u32 {
    let ir = k.k_ir();
    let term = match in_frame {
        Some(gbb) => &ir.blocks[gbb as usize].term,
        None => &ir.processes[proc as usize].body[bb as usize].term,
    };
    match term {
        sim_ir::Terminator::Call { ret_bb, .. } => *ret_bb,
        // Unreachable: the caller only asks while standing on a `Call`.
        _ => bb,
    }
}

/// Split a call site's inputs into SCALAR copy-ins (evaluated here, in the
/// caller's context, through this kernel's store) and DYN-ARRAY snapshots (a
/// formal↔source net pair, resolved against the shared heap by the seam).
///
/// This IS `Scheduler::split_frame_in_binds` — that method delegates here now,
/// so there is one spelling of the sizing rule rather than one per backend. The
/// only substitution the lift needed was `eval_ctx_top` → `k_eval_ctx`. The sizing rule — the formal's declared width
/// widened to the actual's own self-width, and the formal's signedness (IEEE
/// §13.4.3) — is NOT restated; it is the same three terms in the same order,
/// because getting it wrong is a silently narrow argument rather than a loud one.
///
/// A dyn-array formal is bound by elaborate to a bare `Signal` reading the
/// caller's dyn net; a shape that is anything else contributes no snapshot, which
/// is what the engine does too (pass-by-value then has nothing to copy).
pub(crate) fn split_in_binds<K: Kernel + ?Sized>(
    k: &K,
    info: &crate::TaskCallInfo,
) -> crate::sched::FrameInBinds {
    let ir = k.k_ir();
    let base = k.k_frame_base(info.callee);
    let mut in_v: Vec<(u32, Value)> = Vec::with_capacity(info.in_binds.len());
    let mut dyn_snaps: Vec<(u32, u32)> = Vec::new();
    for &(slot, e) in &info.in_binds {
        let fnet = (base + slot) as usize;
        if ir.nets[fnet].kind == NetKind::DynArray {
            if let sim_ir::Expr::Signal { net, .. } = &ir.exprs[e as usize] {
                dyn_snaps.push((slot, *net));
            }
        } else {
            let nv = &ir.nets[fnet];
            let (sw, _) = k.k_self_width(e);
            let v = k.k_eval_ctx(e, nv.width.max(1).max(sw), nv.signed);
            in_v.push((slot, v));
        }
    }
    (in_v, dyn_snaps)
}
