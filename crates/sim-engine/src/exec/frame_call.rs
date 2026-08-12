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

/// Perform the `Terminator::Call` at process-local block `bb` of process `proc`.
///
/// PRECONDITION: `native::frames::call_site_runnable` (tier-3) or the engine's
/// own `suspendable_tasks` test (tier-2) said this callee is synchronous. The
/// caller advances to `ret_bb`; this returns nothing because a subset call cannot
/// suspend — that is what makes it the subset.
pub(crate) fn subset_task_call<K: Kernel>(k: &mut K, proc: u32, bb: u32) {
    let Some(info) = k.k_task_call_site(proc, bb) else {
        // A site with no sidecar entry is a deferred hierarchical enable whose
        // actuals elaborate could not resolve. The engine advances past it; so do
        // we. Tier-3 never gets here (`call_site_runnable` refuses the site), and
        // saying so is the point — this arm exists for `K = Scheduler`.
        return;
    };
    let (in_vals, dyn_snaps) = split_in_binds(k, &info);
    let writes = k.k_run_subset_task(info.callee, &in_vals, &dyn_snaps, &info.out_binds);
    for (lval, val) in writes {
        let offs = k.k_resolve_lvalue_offsets(&lval);
        k.k_write_lvalue(&lval, val, &offs);
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
