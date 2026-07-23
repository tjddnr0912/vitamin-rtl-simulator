//! split part of `exec` (mechanical move).

use super::*;

/// Execute activity `pi` starting at body block `start`. `pi` is a runtime
/// ACTIVITY id (index into `Scheduler::activities`), NOT a declaration index —
/// the body/sensitivity are resolved through `activities[pi].template`.
/// Round-14 V3/V4 Phase 3: pop this activity's live AUTOMATIC frame windows off the
/// SHARED `frame_stack` into their `FrameRec`s before the activity suspends — so an
/// interleaving activity's frame calls can't corrupt them (a `frame_slot_read` always
/// reads `frame_stack.last()`, which must be THIS activity's window only while it runs).
/// Popped TOP-first (reverse call order), matching `enter_task_frame`'s push order.
pub(crate) fn stash_frame_windows(sched: &mut Scheduler, pi: u32) {
    let n = sched.activities[pi as usize].call_stack.len();
    for i in (0..n).rev() {
        let callee = sched.activities[pi as usize].call_stack[i].callee;
        if sched.st.func_has_auto[callee as usize] {
            let w = sched
                .st
                .frame_stack
                .borrow_mut()
                .pop()
                .expect("frame window to stash");
            sched.activities[pi as usize].call_stack[i].window = Some(w);
        }
    }
}

/// The inverse of [`stash_frame_windows`]: push the stashed windows back onto
/// `frame_stack` in call order (bottom `FrameRec` first) so this activity's live frame
/// context is on top again before it resumes executing the frame CFG.
pub(crate) fn restore_frame_windows(sched: &mut Scheduler, pi: u32) {
    let n = sched.activities[pi as usize].call_stack.len();
    for i in 0..n {
        // Tolerant: only a STASHED (`Some`) window is pushed back; a live/None frame is
        // skipped, so a spurious call during normal in-frame execution is a no-op.
        if let Some(w) = sched.activities[pi as usize].call_stack[i].window.take() {
            sched.st.frame_stack.borrow_mut().push(w);
        }
    }
}

pub(crate) fn run_process(sched: &mut Scheduler, pi: u32, mut bb: u32) -> Step {
    let mut guard: u64 = 0;
    loop {
        // Round-14 V3/V4: while this activity is inside a suspendable task call, execute
        // the TOP frame's task CFG (global `ir.blocks`) below instead of the base process
        // body; the base-process `bb` is frozen at the call site and resumes at the
        // frame's `ret_bb` on `Return`. The child-completion intercept keys on the base
        // `bb`, so it applies only when NOT in a frame.
        let in_frame = !sched.activities[pi as usize].call_stack.is_empty();
        if !in_frame {
            // ── CENTRALIZED CHILD-COMPLETION INTERCEPT (terminator-agnostic) ──
            // If this activity is a fork child and the NEXT bb to fetch is its barrier's
            // join_bb, the child has completed. Report + die BEFORE the join_bb block is
            // ever fetched (join_bb is a never-executed sentinel). This catches the child
            // whether it arrives via Goto, Branch, or a resumed Delay/Wait, so a child
            // whose last statement is an if/case/delay/wait into join_bb is handled.
            if sched.activity_is_child(pi) {
                if let Some(jr) = sched.activity_join_ref(pi) {
                    if bb == sched.barrier_join_bb(jr) {
                        sched.on_child_complete(jr, pi);
                        return Step::Done; // child dead; rearm skips it (is_child)
                    }
                }
            }
            // Defense-in-depth: a non-child must NEVER fetch a live barrier's join_bb.
            #[cfg(debug_assertions)]
            sched.assert_not_parent_at_join(pi, bb);
        }
        // Round-14 V3/V4 Phase 3: resuming INTO a suspended frame — restore this
        // activity's stashed windows onto the shared `frame_stack` before executing the
        // frame CFG. Only fires on the first iteration after a resume (windows are `Some`
        // only across a suspend); a no-op otherwise.
        if in_frame
            && sched.activities[pi as usize]
                .call_stack
                .iter()
                .any(|f| f.window.is_some())
        {
            restore_frame_windows(sched, pi);
        }

        // Snapshot the block's stmt ids + terminator (process-local indexing,
        // resolved through this activity's template).
        let tmpl = sched.activity_template(pi) as usize;
        // $time/$realtime evaluated in this process scale by its module multiplier.
        sched.st.cur_time_mult = sched
            .st
            .proc_multipliers
            .get(tmpl)
            .copied()
            .unwrap_or(1)
            .max(1);
        sched.st.cur_prec_mult = sched
            .st
            .proc_prec_mults
            .get(tmpl)
            .copied()
            .unwrap_or(1)
            .max(1);
        // `%m` scope of this process (P2-11); flat "top" when no sidecar. Skip the
        // String alloc when the scope is already current (the common case for a
        // process resumed many times) — `clone_from` reuses capacity otherwise.
        match sched.st.proc_scopes.get(tmpl) {
            Some(s) => {
                if &sched.st.cur_scope != s {
                    sched.st.cur_scope.clone_from(s);
                }
            }
            None => {
                if sched.st.cur_scope != "top" {
                    sched.st.cur_scope.clear();
                    sched.st.cur_scope.push_str("top");
                }
            }
        }
        // `ir` is `&'ir SimIr` (shared, outliving this `&mut sched` borrow), so the
        // block's stmt list and terminator are read IN PLACE. The previous
        // `stmts.clone()`/`term.clone()`/per-stmt `Stmt::clone()` allocated on every
        // block activation — the second-largest malloc source of clock-bound designs.
        let ir = sched.st.ir;
        // Frame-aware fetch: the top task frame's block (global arena) or the base
        // process body block (process-local index).
        let cur_bb = if in_frame {
            sched.activities[pi as usize].call_stack.last().unwrap().bb
        } else {
            bb
        };
        let block = if in_frame {
            &ir.blocks[cur_bb as usize]
        } else {
            &ir.processes[tmpl].body[cur_bb as usize]
        };

        // ── statements (P7a read/write-phase split) ──
        // Each statement executes in two explicit phases: a READ phase
        // (`compute_effect`, pure eval over `&Scheduler` — no mutation) that produces
        // a self-contained [`StmtEffect`], then a WRITE phase (`apply_effect`, the
        // `&mut Scheduler` kernel calls). This is the seam a codegen body needs: it
        // inlines the read phase as native code and routes the write phase through the
        // kernel (P7b puts apply_effect's calls behind a trait). Behaviour is
        // byte-identical to the prior inline form — same evals, same writes, same order.
        for &sid in &block.stmts {
            let stmt = &ir.stmts[sid as usize];
            let effect = compute_effect(&*sched, stmt, sid); // READ phase via Kernel seam
            if let Some(step) = apply_effect(sched, effect) {
                return step; // a SysTask returned Finish/Stop/Fatal
            }
        }

        // ── terminator ── (`set_pos!` writes the base-process `bb` or, in a task
        // frame, the top frame's `bb`; the base-only suspend/fork/delay arms guard on
        // `in_frame` and fatal defensively — a Phase-2 suspendable task is leaf and
        // non-suspending, so those never fire from a frame.)
        macro_rules! set_pos {
            ($t:expr) => {
                if in_frame {
                    sched.activities[pi as usize]
                        .call_stack
                        .last_mut()
                        .unwrap()
                        .bb = $t;
                } else {
                    bb = $t;
                }
            };
        }
        match &block.term {
            Terminator::Goto { target } => {
                set_pos!(*target);
            }
            Terminator::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                let t = if sched.truthy(*cond) {
                    *then_bb
                } else {
                    *else_bb
                };
                set_pos!(t);
            }
            Terminator::Delay {
                amount,
                region,
                resume,
            } => {
                // format_version 4: `amount` is the ExprId of the RAW delay value in
                // module units — evaluate NOW (the frame window is still live) and scale
                // by this process's multiplier (X/Z → 0; real → round(v×M)).
                let ticks = sched.delay_ticks(*amount);
                let inactive = matches!(region, DelayRegion::Inactive) || ticks == 0;
                let tick = sched.now().saturating_add(ticks);
                if in_frame {
                    // Suspend the frame: record its resume PC, stash the window off the
                    // shared stack, wake at the frozen process `bb` (ignored on resume —
                    // in_frame reads the frame's PC).
                    sched.activities[pi as usize]
                        .call_stack
                        .last_mut()
                        .unwrap()
                        .bb = *resume;
                    stash_frame_windows(sched, pi);
                    sched.schedule_resume(pi, bb, tick, inactive);
                } else {
                    sched.schedule_resume(pi, *resume, tick, inactive);
                }
                return Step::Suspended;
            }
            Terminator::Wait { cond, resume } => {
                match cond {
                    WaitCause::Expr { expr } => {
                        // `truthy` reads the condition with the window still live.
                        if sched.truthy(*expr) {
                            if in_frame {
                                sched.activities[pi as usize]
                                    .call_stack
                                    .last_mut()
                                    .unwrap()
                                    .bb = *resume; // already true → fall through in the frame
                            } else {
                                bb = *resume; // already true → fall through
                            }
                            guard += 1;
                            if guard > sched.max_deltas_guard() {
                                sched.mark_fatal();
                                return Step::Fatal;
                            }
                            continue;
                        }
                        if in_frame {
                            sched.activities[pi as usize]
                                .call_stack
                                .last_mut()
                                .unwrap()
                                .bb = *resume;
                            stash_frame_windows(sched, pi);
                            sched.suspend_on(pi, bb, cond.clone());
                        } else {
                            // Suspending: the one place the cause must be OWNED.
                            sched.suspend_on(pi, *resume, cond.clone());
                        }
                    }
                    // `wait fork` (v8): park on the implicit child barrier, or
                    // fall through immediately when there are no live children.
                    WaitCause::Fork => {
                        if in_frame {
                            // `wait fork` inside a task frame is a Phase-4 follow-on.
                            sched.mark_fatal();
                            return Step::Fatal;
                        }
                        if sched.exec_wait_fork(pi, *resume) {
                            bb = *resume; // no outstanding children → fall through
                            guard += 1;
                            if guard > sched.max_deltas_guard() {
                                sched.mark_fatal();
                                return Step::Fatal;
                            }
                            continue;
                        }
                        // parked by exec_wait_fork; on_child_complete resumes it.
                    }
                    _ => {
                        if in_frame {
                            sched.activities[pi as usize]
                                .call_stack
                                .last_mut()
                                .unwrap()
                                .bb = *resume;
                            stash_frame_windows(sched, pi);
                            sched.suspend_on(pi, bb, cond.clone());
                        } else {
                            sched.suspend_on(pi, *resume, cond.clone());
                        }
                    }
                }
                return Step::Suspended;
            }
            Terminator::Return => {
                if in_frame {
                    // Pop the task frame: copy out its output/inout slots to the caller
                    // lvalues, then resume the parent at `ret_bb` (the base process here —
                    // a Phase-2 task is leaf, so the stack is empty after this pop).
                    let frame = sched.activities[pi as usize].call_stack.pop().unwrap();
                    let out_s: Vec<u32> = frame.out_binds.iter().map(|&(s, _)| s).collect();
                    let outs = sched.st.exit_task_frame(frame.callee, &out_s);
                    // V2B (§4.5.194): copy out BEFORE the free — the deep-copy of an OUTPUT/
                    // INOUT dyn formal reads its heap slot, which frame_dyn_free would clear.
                    for ((s, lval), val) in frame.out_binds.iter().zip(outs) {
                        if sched.st.frame_dyn_out_bind(frame.callee, *s, lval) {
                            continue; // dyn formal → heap deep-copy, not the scalar slot value
                        }
                        let offs = sched.resolve_lvalue_offsets(lval);
                        sched.st.write_lvalue(lval, val, &offs);
                    }
                    // V5: free this activation's frame dyn-array formals/locals so a later
                    // call starts fresh (size 0) and the reentry guard next sees None.
                    sched.st.frame_dyn_free(frame.callee);
                    if sched.activities[pi as usize].call_stack.is_empty() {
                        bb = frame.ret_bb;
                    } else {
                        sched.activities[pi as usize]
                            .call_stack
                            .last_mut()
                            .unwrap()
                            .bb = frame.ret_bb;
                    }
                } else {
                    sched.rearm(pi);
                    return Step::Done;
                }
            }
            // fork/join/join_any/join_none: register the barrier, spawn each child as
            // a new activity (runnable THIS instant), then either continue at
            // resume_bb (join_none, or zero children) or suspend on the barrier
            // (join/join_any with ≥1 child). The parent is re-enqueued by
            // on_child_complete when the join condition fires.
            Terminator::Fork {
                children,
                join,
                resume_bb,
            } => {
                if in_frame {
                    sched.mark_fatal();
                    return Step::Fatal;
                }
                match sched.exec_fork(pi, children, *join, *resume_bb) {
                    Some(cont) => {
                        bb = cont;
                    }
                    None => return Step::Suspended,
                }
            }
            // B2 frame-call: a TASK call from a PROCESS body. Evaluate inputs in THIS
            // (caller) scope; then either PUSH a suspendable-task frame (round-14 V3/V4 —
            // `run_process` drives its CFG, handling NBA/$systask, and pops at `Return`)
            // or run a subset task synchronously (unchanged), writing outputs to the
            // caller lvalues (which MAY be module nets). A non-frame Call just advances.
            Terminator::Call { ret_bb, .. } => {
                if in_frame {
                    // Round-14 V3/V4 Phase 4: a NESTED task call from a task frame — keyed
                    // by GLOBAL block (`task_calls_func`). Push a nested suspendable frame
                    // (call-stack depth > 1, incl. recursion) or run a subset callee
                    // synchronously; this frame resumes at `ret_bb` when the callee returns
                    // (the `Return` arm sets the PARENT frame's PC).
                    if let Some(info) = sched.st.task_calls_func.get(&cur_bb).cloned() {
                        // V2A-frame (§4.5.173): split scalar copy-ins from dyn-array input
                        // formals — the latter are pass-by-VALUE and snapshotted after enter.
                        let (in_v, dyn_snaps) = sched.split_frame_in_binds(&info);
                        if sched.st.suspendable_tasks.contains(&info.callee) {
                            if sched.activities[pi as usize].call_stack.len() as u32
                                >= crate::state::MAX_CALL_DEPTH
                            {
                                sched.mark_fatal(); // runaway recursion → loud, never a hang
                                return Step::Fatal;
                            }
                            // V5: a recursive/concurrent frame dyn-array local would share
                            // its per-net heap object with the parent activation — fatal-loud.
                            // (V2A-frame: also guards a recursive dyn-array INPUT formal.)
                            if !sched.st.frame_dyn_reentry_ok(info.callee) {
                                return Step::Fatal;
                            }
                            sched.st.enter_task_frame(info.callee, &in_v);
                            // V2A-frame: deep-copy caller dyn-array actuals into the fresh
                            // per-activation formal slots (pass-by-value, IEEE §13.5.1).
                            sched.st.frame_dyn_snapshot_formals(info.callee, &dyn_snaps);
                            let entry = sched.st.ir.funcs[info.callee as usize].entry;
                            sched.activities[pi as usize]
                                .call_stack
                                .push(crate::sched::FrameRec {
                                    callee: info.callee,
                                    bb: entry,
                                    ret_bb: *ret_bb,
                                    out_binds: info.out_binds.clone(),
                                    window: None,
                                });
                            continue; // execute the nested frame next iteration
                        }
                        // subset nested callee: synchronous (mirrors run_task's nested Call).
                        // V2A-dyn (§4.5.194): snapshot a dyn-array INPUT formal into its heap
                        // slot before the synchronous call, free after (as the top-level arm).
                        let out_s: Vec<u32> = info.out_binds.iter().map(|&(s, _)| s).collect();
                        // V2A/V2B (§4.5.194): snapshot a dyn-array INPUT formal IN before the
                        // synchronous call; deep-copy an OUTPUT/INOUT dyn formal OUT after.
                        // reentry_ok/snapshot/free self-gate on the callee's dyn state, so call
                        // them unconditionally (snapshot no-ops with no input dyn; reentry/free
                        // no-op with no dyn formal/local).
                        if !sched.st.frame_dyn_reentry_ok(info.callee) {
                            return Step::Fatal;
                        }
                        sched.st.frame_dyn_snapshot_formals(info.callee, &dyn_snaps);
                        if let Some(outs) = sched.st.run_task_call(info.callee, &in_v, &out_s) {
                            for ((s, lval), val) in info.out_binds.iter().zip(outs) {
                                if sched.st.frame_dyn_out_bind(info.callee, *s, lval) {
                                    continue; // dyn formal → heap deep-copy, not the scalar value
                                }
                                let offs = sched.resolve_lvalue_offsets(lval);
                                sched.st.write_lvalue(lval, val, &offs);
                            }
                        }
                        sched.st.frame_dyn_free(info.callee);
                    }
                    // advance THIS frame past the (subset / no-info) call.
                    sched.activities[pi as usize]
                        .call_stack
                        .last_mut()
                        .unwrap()
                        .bb = *ret_bb;
                    continue;
                }
                if let Some(info) = sched
                    .st
                    .task_calls_proc
                    .get(&(tmpl as u32, cur_bb))
                    .cloned()
                {
                    // V2A-frame (§4.5.173): split scalar copy-ins from dyn-array input
                    // formals — the latter are pass-by-VALUE and snapshotted after enter.
                    let (in_v, dyn_snaps) = sched.split_frame_in_binds(&info);
                    if sched.st.suspendable_tasks.contains(&info.callee) {
                        // Suspendable: push a frame; do NOT advance `bb` — the base process
                        // resumes at `ret_bb` when the frame returns.
                        // V5: reentry guard for a frame dyn-array local (see the nested arm).
                        // (V2A-frame: also guards a recursive dyn-array INPUT formal.)
                        if !sched.st.frame_dyn_reentry_ok(info.callee) {
                            return Step::Fatal;
                        }
                        sched.st.enter_task_frame(info.callee, &in_v);
                        // V2A-frame: deep-copy caller dyn-array actuals into the fresh
                        // per-activation formal slots (pass-by-value, IEEE §13.5.1).
                        sched.st.frame_dyn_snapshot_formals(info.callee, &dyn_snaps);
                        let entry = sched.st.ir.funcs[info.callee as usize].entry;
                        sched.activities[pi as usize]
                            .call_stack
                            .push(crate::sched::FrameRec {
                                callee: info.callee,
                                bb: entry,
                                ret_bb: *ret_bb,
                                out_binds: info.out_binds.clone(),
                                window: None,
                            });
                        continue; // re-fetch from the new frame next iteration
                    }
                    let out_s: Vec<u32> = info.out_binds.iter().map(|&(s, _)| s).collect();
                    // V2A-dyn (§4.5.194): a subset task's dyn-array INPUT formal is
                    // deep-copied into its per-activation heap slot right before the
                    // synchronous run_task_call (dyn_heap is interior-mutable now) and freed
                    // after — the sync executor READS the formal from the heap, exactly like
                    // the suspendable frame-entry snapshot (§4.5.173).
                    // V2A/V2B (§4.5.194): snapshot a dyn INPUT formal IN before the call;
                    // deep-copy an OUTPUT/INOUT dyn formal OUT after. All three self-gate on
                    // the callee's dyn state → call unconditionally.
                    if !sched.st.frame_dyn_reentry_ok(info.callee) {
                        return Step::Fatal;
                    }
                    sched.st.frame_dyn_snapshot_formals(info.callee, &dyn_snaps);
                    if let Some(outs) = sched.st.run_task_call(info.callee, &in_v, &out_s) {
                        for ((s, lval), val) in info.out_binds.iter().zip(outs) {
                            if sched.st.frame_dyn_out_bind(info.callee, *s, lval) {
                                continue; // dyn formal → heap deep-copy, not the scalar value
                            }
                            let offs = sched.resolve_lvalue_offsets(lval);
                            sched.st.write_lvalue(lval, val, &offs);
                        }
                    }
                    sched.st.frame_dyn_free(info.callee);
                }
                bb = *ret_bb;
            }
        }

        guard += 1;
        if guard > sched.max_deltas_guard() {
            sched.mark_fatal();
            return Step::Fatal;
        }
    }
}

/// READ phase: evaluate `stmt` through the read-only half of the [`Kernel`] seam,
/// producing a [`StmtEffect`] that captures everything the write phase will apply. No
/// net state is mutated here. Generic over `K: Kernel`, so the SAME executor serves
/// the interpreter (`Scheduler`) and a Stage-C compiled body.
pub(crate) fn compute_effect<'s, K: Kernel>(k: &K, stmt: &'s Stmt, sid: u32) -> StmtEffect<'s> {
    match stmt {
        Stmt::BlockingAssign { lhs, rhs } => {
            // N7: a `new` allocation site — keyed by StmtId, so check it FIRST so
            // the placeholder rhs (a const 0) is never evaluated. The write phase
            // allocates the heap object and stores its id into `lhs`.
            if let Some(class_id) = k.k_class_new_site(sid) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::ClassNew {
                    lhs,
                    class_id,
                    offsets,
                };
            }
            // v5 ④: a queue-pop rhs is a statement-level EFFECT (it mutates
            // the queue) — defer the pop itself to the write phase.
            if k.k_queue_pop_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::QPop {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v6: an assoc-iteration rhs writes its ref key argument — same
            // statement-level deferral as the pops.
            if k.k_assoc_iter_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::AssocIter {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v7: a seeded $random(seed) rhs writes the seed back — same family.
            if k.k_random_seeded_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::SeededRandom {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v9 rank 6: a $dist_uniform(seed, ...) rhs writes the seed back.
            if k.k_dist_seeded_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::SeededDist {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v9 rank 6: a $cast(dst, src) func-form writes the dst ref arg.
            if k.k_cast_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Cast {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v7: $value$plusargs writes its ref var — same family.
            if k.k_value_plusargs_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::ValuePlusargs {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v7: $fopen mutates the file table — same family.
            if k.k_fopen_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Fopen {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v7: $sformatf renders through the kernel — same family.
            if k.k_sformatf_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Sformatf {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v9 SYS-READ: $fgetc/$feof/$ungetc read/advance the fd read state —
            // same family. Resolve the lhs offsets in the READ phase but do NOT
            // evaluate the rhs (that read must happen in the WRITE phase, after
            // the dest offsets are pinned — the deterministic-ordering rule).
            if k.k_fgetc_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Fgetc {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_feof_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Feof {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_ungetc_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Ungetc {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_fgets_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Fgets {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_fread_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Fread {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_fscanf_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Scanf {
                    lhs,
                    rhs: *rhs,
                    offsets,
                    is_file: true,
                };
            }
            if k.k_sscanf_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Scanf {
                    lhs,
                    rhs: *rhs,
                    offsets,
                    is_file: false,
                };
            }
            let value = k.k_eval_for_lvalue(lhs, *rhs); // CONTEXT-SIZED to lhs width
            let offsets = k.k_resolve_lvalue_offsets(lhs); // dynamic index NOW
            StmtEffect::Blocking {
                lhs,
                value,
                offsets,
            }
        }
        Stmt::NonblockingAssign { lhs, rhs, delay } => {
            let value = k.k_eval_for_lvalue(lhs, *rhs); // CONTEXT-SIZED, sampled now
            let delay_ticks = delay.map(|d| k.k_delay_ticks(d));
            StmtEffect::Nonblocking {
                lhs,
                value,
                delay_ticks,
            }
        }
        Stmt::SysTask { which, fmt, args } => StmtEffect::SysTask {
            which: *which,
            fmt: *fmt,
            args,
            sid,
        },
        Stmt::Disable { scope_kind, .. } => match scope_kind {
            sim_ir::DisableKind::Fork => StmtEffect::DisableFork,
            sim_ir::DisableKind::Scope => StmtEffect::Nop,
        },
        Stmt::Force { lhs, rhs } => {
            // Evaluate NOW (context-sized to the target) for the initial pin;
            // the kernel registers `rhs` for continuous re-evaluation
            // (IEEE §9.3.2 — a force with an expression RHS behaves as a
            // continuous assignment until released).
            let value = k.k_eval_for_lvalue(lhs, *rhs);
            StmtEffect::Force {
                lhs,
                value,
                rhs: *rhs,
                sid,
            }
        }
        Stmt::Release { lhs } => StmtEffect::Release { lhs, sid },
    }
}

/// WRITE phase: apply a [`StmtEffect`] through the mutating half of the [`Kernel`]
/// seam. Returns `Some(Step)` only when a `$finish`/`$stop`/fatal system task ends the
/// activation. Generic over `K: Kernel` (same executor for interpreter + compiled VM).
pub(crate) fn apply_effect<K: Kernel>(k: &mut K, effect: StmtEffect<'_>) -> Option<Step> {
    match effect {
        StmtEffect::Blocking {
            lhs,
            value,
            offsets,
        } => {
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::QPop { lhs, rhs, offsets } => {
            let value = k.k_queue_pop(lhs, rhs); // pop + context-size (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::AssocIter { lhs, rhs, offsets } => {
            let value = k.k_assoc_iter(lhs, rhs); // key write + status (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::SeededRandom { lhs, rhs, offsets } => {
            let value = k.k_random_seeded(rhs); // seed write + draw (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::SeededDist { lhs, rhs, offsets } => {
            let value = k.k_dist_seeded(rhs); // seed write + dist draw (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Cast { lhs, rhs, offsets } => {
            let value = k.k_cast(rhs); // dst ref write + status (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::ValuePlusargs { lhs, rhs, offsets } => {
            let value = k.k_value_plusargs(rhs); // var write + status (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Fopen { lhs, rhs, offsets } => {
            let value = k.k_fopen(rhs); // file-table mutation (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Sformatf { lhs, rhs, offsets } => {
            let value = k.k_sformatf(rhs); // kernel-side render (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Fgetc { lhs, rhs, offsets } => {
            let value = k.k_fgetc(rhs); // byte read + fd advance (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Feof { lhs, rhs, offsets } => {
            let value = k.k_feof(rhs); // lazy-EOF flag read (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Ungetc { lhs, rhs, offsets } => {
            let value = k.k_ungetc(rhs); // pushback mutation (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Fgets { lhs, rhs, offsets } => {
            let value = k.k_fgets(rhs); // line read + str dest write (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Fread { lhs, rhs, offsets } => {
            let value = k.k_fread(rhs); // binary read + target write (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Scanf {
            lhs,
            rhs,
            offsets,
            is_file,
        } => {
            // the parser writes every matched ref arg internally (WRITE phase);
            // the conversion count is written to lhs.
            let value = if is_file {
                k.k_fscanf(rhs)
            } else {
                k.k_sscanf(rhs)
            };
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::DisableFork => {
            k.k_disable_fork();
            None
        }
        StmtEffect::Nonblocking {
            lhs,
            value,
            delay_ticks,
        } => {
            // The NBA queue outlives this activation — the one owned clone left.
            match delay_ticks {
                Some(d) if d > 0 => k.k_schedule_nba_at(lhs.clone(), value, d),
                _ => k.k_schedule_nba(lhs.clone(), value),
            }
            None
        }
        StmtEffect::Force {
            lhs,
            value,
            rhs,
            sid,
        } => {
            k.k_force(lhs, value, rhs, sid);
            None
        }
        StmtEffect::Release { lhs, sid } => {
            k.k_release(lhs, sid);
            None
        }
        StmtEffect::SysTask {
            which,
            fmt,
            args,
            sid,
        } => match k.k_dispatch_systask(which, fmt, args, sid) {
            Ctl::Finish => Some(Step::Finish),
            Ctl::Stop => Some(Step::Stop),
            Ctl::Fatal => Some(Step::Fatal),
            Ctl::Continue => None,
        },
        StmtEffect::ClassNew {
            lhs,
            class_id,
            offsets,
        } => {
            let value = k.k_class_alloc(class_id); // allocate heap object (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Nop => None,
    }
}
