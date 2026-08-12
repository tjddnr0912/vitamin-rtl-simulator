//! split part of `state` (mechanical move).

use super::*;

impl SimState<'_> {
    /// B2 frame-call: execute TASK `callee` against a fresh frame. `in_vals` are
    /// `(callee input-formal slot, value)` pairs already evaluated in the CALLER
    /// context; `out_slots` are the callee output-formal slots to read back at
    /// `Return`. Returns those slots' values (the caller writes them to its
    /// lvalues). Same `&self` frame arena + borrow discipline as `run_frame_call`;
    /// a NESTED task call (`Terminator::Call` in the task body) recurses and writes
    /// its outputs to the CALLING task's frame-local lvalues. `None` ⇒ empty
    /// sidecar (no frame tasks).
    pub(crate) fn run_task(
        &self,
        callee: u32,
        in_vals: &[(u32, Value)],
        out_slots: &[u32],
    ) -> Option<Vec<Value>> {
        use sim_ir::{Stmt, Terminator};
        if self.func_table.is_empty() {
            return None;
        }
        let m = self.func_table[callee as usize];
        // runaway guard (shared with run_frame_call).
        let d = self.call_depth.get();
        if d >= MAX_CALL_DEPTH {
            if !self.call_fatal.get() {
                self.call_fatal.set(true);
                self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Fatal,
                    code: MsgCode::RunFatal,
                    message: format!(
                        "frame-task recursion exceeded the depth limit ({MAX_CALL_DEPTH})"
                    ),
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(TimeStamp { ticks: self.now }),
                }));
            }
            return Some(
                out_slots
                    .iter()
                    .map(|&s| {
                        let nv = &self.ir.nets[(m.base_net + s) as usize];
                        Value::xs(nv.width.max(1), nv.signed)
                    })
                    .collect(),
            );
        }
        self.call_depth.set(d + 1);
        let _g = DepthGuard(&self.call_depth);
        // N1: expose this task (FuncId `callee`) to `%m` rendered inside its body.
        self.frame_scope.borrow_mut().push(callee);
        let _sg = FrameScopeGuard(&self.frame_scope);
        let fd = self.ir.funcs[callee as usize];
        // R5-B: a FUNCTION with output/inout formals is ALSO driven here — elaborate
        // emits it as a `Terminator::Call` whose `out_slots` copy out the inout/output
        // formal slots PLUS the return slot (captured into a caller temp). run_task is
        // generic over `out_slots`, so it runs the function body (which sets the return
        // slot via its `return`) and copies out every requested slot uniformly.
        let _ = &fd;
        let base = m.base_net;
        let nloc = m.locals_len;
        // B4: per-func storage needs (window for automatic slots, slab for static).
        let has_auto = self.func_has_auto[callee as usize];
        let has_static = self.func_has_static[callee as usize];

        // ── FRAME SETUP (window for automatic slots; persistent slab for static). ──
        // Each fresh frame slot defaults to its NET's declared init — X for 4-state
        // (reg/logic/integer), 0 for 2-state (bit/byte/int/shortint/longint) per IEEE
        // §6.4. (Using `Value::xs` unconditionally mis-defaulted 2-state locals to X,
        // silently corrupting reads/branches of an unassigned 2-state local.)
        let fresh: Vec<Value> = (0..nloc)
            .map(|s| {
                let nv = &self.ir.nets[(base + s) as usize];
                // N1: a frame-local `string` slot (an output/inout string formal or a
                // string return var) defaults to the EMPTY string, not a width-1
                // non-is_str value — the latter renders as a stray space and makes
                // `s==""` / `s.len()==0` FALSE on an unwritten path (a silent-wrong
                // caught by adversarial review; string-return t03 already matched
                // iverilog because it happened to write, output formals did not).
                if nv.kind == NetKind::String {
                    Value::from_str_bytes(&[])
                } else {
                    Value::from_packed(&nv.init, nv.width.max(1), nv.signed)
                }
            })
            .collect();
        // `run_task` is the SYNCHRONOUS (`&self`) subset-task executor — a task with a
        // `fork` is suspendable and never routed here (it goes through `enter_task_frame`),
        // so its window is always `Owned` (byte-identical to the pre-arena path).
        match (has_auto, has_static) {
            (true, true) => {
                self.frame_stack
                    .borrow_mut()
                    .push(WindowSlot::Owned(fresh.clone()));
                self.static_store
                    .borrow_mut()
                    .entry(callee)
                    .or_insert(fresh);
            }
            (true, false) => self.frame_stack.borrow_mut().push(WindowSlot::Owned(fresh)),
            (false, _) => {
                self.static_store
                    .borrow_mut()
                    .entry(callee)
                    .or_insert(fresh);
            }
        }

        // ── COPY-IN the input args (sized to each formal's TYPE — §13.4.3). ──
        // One rule for all three frame entries; see `bind_formal`.
        for (slot, v) in in_vals {
            let bound = self.bind_formal(callee, *slot, base, v.clone());
            self.frame_slot_write(
                callee,
                self.frame_slot_auto[(base + *slot) as usize],
                *slot,
                bound,
            );
        }

        // V5 (§4.5.194) / T1-9: open this task's own dyn LOCALS for this activation
        // (formals `[0, np)` are caller-managed, first_slot=np). A subset dyn-local task
        // is normally LIFTED to the suspendable path; this keeps run_task self-consistent
        // if one is routed here (the DynNew arm below executes its `new[]`). The stash is
        // a LOCAL: this path is straight-line to its `frame_dyn_exit` below, so it cannot
        // interleave with another activation of the same callee.
        let np = self.ir.funcs[callee as usize].n_params;
        let dyn_stash = self.frame_dyn_enter_from(callee, np);

        // ── BB LOOP over the GLOBAL func arena from fd.entry. ──
        let mut cur = fd.entry;
        loop {
            debug_assert!(
                (cur as usize) < self.ir.blocks.len(),
                "task CFG target in range"
            );
            let blk = &self.ir.blocks[cur as usize];
            for &sid in &blk.stmts {
                match &self.ir.stmts[sid as usize] {
                    Stmt::BlockingAssign { lhs, rhs } => {
                        // §4.5.176: a dyn/queue/assoc `foreach` step advances its key via the
                        // frame-aware path in this `&self` subset-task executor (was a stall/0).
                        if self.rhs_is_assoc_iter(*rhs) {
                            self.frame_assoc_iter(lhs, *rhs);
                            continue;
                        }
                        // N1: `$sformatf` renders through the shared formatter here.
                        let v = self.frame_rhs_value(lhs, *rhs);
                        self.frame_or_class_write(lhs, v);
                    }
                    // V5 (§4.5.194): a frame-local dyn `new[]` / `delete()` — parity with
                    // run_frame_call (the classifier now admits DynNew in a subset body, so
                    // execute it here rather than silently skip it).
                    Stmt::SysTask {
                        which: sim_ir::SysTaskId::DynNew,
                        args,
                        ..
                    } => self.frame_dyn_new(args),
                    Stmt::SysTask {
                        which: sim_ir::SysTaskId::DynDelete,
                        args,
                        ..
                    } => {
                        if let Some(&a0) = args.first() {
                            if let sim_ir::Expr::Signal { net, .. } = &self.ir.exprs[a0 as usize] {
                                self.dyn_heap.borrow_mut()[*net as usize].take();
                            }
                        }
                    }
                    // Family C (r17): dyn-array-formal snapshot marker for a `x = f(arr)`
                    // call inside this (subset) TASK body — parity with run_frame_call.
                    // Deep-copy caller(src) → formal(dst); only handle-copy markers match.
                    Stmt::SysTask {
                        which: sim_ir::SysTaskId::Display,
                        ..
                    } if self.handle_copy_stmts.contains_key(&sid) => {
                        if let Some(&(dst, src)) = self.handle_copy_stmts.get(&sid) {
                            self.frame_dyn_copy_out(src, dst);
                            self.enforce_queue_bound(dst);
                        }
                    }
                    // r18 (F2): a SEVERITY task in this subset TASK body — parity with
                    // run_frame_call. MUST precede the plain Display|Write print arm.
                    Stmt::SysTask {
                        which: sim_ir::SysTaskId::Display,
                        fmt,
                        args,
                    } if self.severities.contains_key(&sid) => {
                        if let Some(&sev) = self.severities.get(&sid) {
                            self.frame_emit_severity(sev, *fmt, args);
                        }
                    }
                    // Family D (r17): a genuine `$display`/`$write` in this subset TASK
                    // body — parity with run_frame_call (render + emit as RtlOutput).
                    Stmt::SysTask { which, fmt, args }
                        if matches!(
                            which,
                            sim_ir::SysTaskId::Display | sim_ir::SysTaskId::Write
                        ) =>
                    {
                        let radix = self.radixes.get(&sid).copied();
                        let mut s = crate::builtins::format_args_str(self, *fmt, args, radix);
                        if matches!(which, sim_ir::SysTaskId::Display) {
                            s.push('\n');
                        }
                        self.sink.emit(LogEvent::RtlOutput(diag::RtlText {
                            text: s,
                            sim_time: None,
                        }));
                    }
                    _ => {}
                }
            }
            match &blk.term {
                Terminator::Goto { target } => cur = *target,
                Terminator::Branch {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    cur = if self.mk_eval_ctx().truthy(*cond) {
                        *then_bb
                    } else {
                        *else_bb
                    };
                }
                // NESTED task call — keyed by THIS block's global index.
                Terminator::Call { ret_bb, .. } => {
                    let Some(info) = self.task_calls_func.get(&cur).cloned() else {
                        break; // malformed sidecar: stop defensively
                    };
                    let cm = self.func_table[info.callee as usize];
                    // V2A-dyn (§4.5.194): split scalar copy-ins from dyn-array INPUT formals.
                    // A dyn formal slot (a DynArray net) is bound to a bare Signal reading the
                    // caller's dyn net (elaborate contract) — recover that src net and
                    // snapshot it into the callee's per-activation heap slot (pass-by-value,
                    // IEEE §13.5.1). This mirrors the process path's `split_frame_in_binds` +
                    // `frame_dyn_snapshot_formals`, but for a NESTED SUBSET call driven inside
                    // this `&self` run_task (which does not go through exec.rs's Call arm).
                    let mut in_v: Vec<(u32, Value)> = Vec::with_capacity(info.in_binds.len());
                    let mut dyn_snaps: Vec<(u32, u32)> = Vec::new();
                    for &(slot, e) in &info.in_binds {
                        let fnet = (cm.base_net + slot) as usize;
                        if self.ir.nets[fnet].kind == NetKind::DynArray {
                            if let sim_ir::Expr::Signal { net, .. } = &self.ir.exprs[e as usize] {
                                dyn_snaps.push((slot, *net));
                            }
                        } else {
                            let nv = &self.ir.nets[fnet];
                            let sw = self.wt.get(e);
                            let v = self.mk_eval_ctx().eval_ctx(
                                e,
                                nv.width.max(1).max(sw.width),
                                nv.signed,
                            );
                            in_v.push((slot, v));
                        }
                    }
                    let out_s: Vec<u32> = info.out_binds.iter().map(|&(s, _)| s).collect();
                    // V2A/V2B (§4.5.194): snapshot dyn INPUT formals IN; deep-copy OUTPUT/INOUT
                    // dyn formals OUT — BEFORE the exit restores the formal slot. Self-gating,
                    // so call enter/snapshot/exit unconditionally.
                    // T1-9: the stash is a LOCAL over a straight-line nested call.
                    // T1-9: CAPTURE the actuals before the stash — a recursive call can
                    // pass the formal as its own actual, and the stash is about to take it.
                    let captured = self.frame_dyn_capture_formals(info.callee, &dyn_snaps);
                    let nested_stash = self.frame_dyn_enter(info.callee);
                    self.frame_dyn_install_formals(captured);
                    let outs = self
                        .run_task(info.callee, &in_v, &out_s)
                        .unwrap_or_default();
                    // callee popped → top frame is the calling task again; write its
                    // frame-local output lvalues (a dyn out-formal deep-copies to the heap).
                    // T1-9: CAPTURE the dyn out/inout results before the restore, and
                    // install them AFTER — under recursion the caller's array net IS the
                    // callee's formal net, so an in-place copy-out would be discarded.
                    let outs_dyn = self.frame_dyn_capture_out(info.callee, &info.out_binds);
                    for ((s, lval), val) in info.out_binds.iter().zip(outs) {
                        if self.frame_dyn_out_bind(info.callee, *s, lval) {
                            continue;
                        }
                        self.frame_write_lvalue(lval, val);
                    }
                    self.frame_dyn_exit(nested_stash);
                    self.frame_dyn_install_formals(outs_dyn);
                    cur = *ret_bb;
                }
                Terminator::Return => break,
                _ => break,
            }
        }

        // V5 (§4.5.194) / T1-9: close this activation's dyn LOCALS, restoring whatever the
        // outer activation held (all `None` for a non-reentrant call — byte-identically
        // the old free).
        self.frame_dyn_exit(dyn_stash);

        // ── COPY-OUT the requested output slots (resize to slot width). ──
        let outs: Vec<Value> = out_slots
            .iter()
            .map(|&s| {
                let nv = &self.ir.nets[(base + s) as usize];
                self.frame_slot_read(callee, self.frame_slot_auto[(base + s) as usize], s)
                    .resize_keep_sign(nv.width.max(1), nv.signed)
            })
            .collect();
        if has_auto {
            self.frame_stack.borrow_mut().pop();
        }
        Some(outs)
    }

    /// B2 frame-call: public &self entry for the executor's `Terminator::Call`
    /// (the `run_task` core is private to the frame-eval impl).
    pub(crate) fn run_task_call(
        &self,
        callee: u32,
        in_vals: &[(u32, Value)],
        out_slots: &[u32],
    ) -> Option<Vec<Value>> {
        self.run_task(callee, in_vals, out_slots)
    }

    /// A3-i: the whole engine-side of a SUBSET task call — the dyn book-keeping,
    /// the synchronous run, and the dyn half of the copy-out — returning the
    /// SCALAR copy-outs for the caller's own write funnel.
    ///
    /// ⭐ ONE implementation, and that is the point. `run_process`'s subset arm
    /// and the tier-3 walk's `Terminator::Call` arm used to be destined to be two
    /// spellings of these nine calls; instead both kernels' `k_run_subset_task`
    /// delegate here in a line. It can be `&self` on `SimState` — rather than a
    /// method of either executor — because none of it touches a NET: the inputs
    /// arrive already evaluated in the caller's context, and the scalar outputs
    /// leave unwritten, precisely so the two callers can write them through their
    /// own stores.
    ///
    /// The call ORDER is `run_process`'s, unchanged, including the two subtleties
    /// its comments record: the actuals are CAPTURED before `frame_dyn_enter`
    /// takes the callee's slots (a recursive call can pass the formal as its own
    /// actual), and the dyn out/inout results are captured before `frame_dyn_exit`
    /// restores them but installed after.
    pub(crate) fn run_subset_task(
        &self,
        callee: u32,
        in_vals: &[(u32, Value)],
        dyn_snaps: &[(u32, u32)],
        out_binds: &[(u32, Lvalue)],
    ) -> Vec<(Lvalue, Value)> {
        let out_s: Vec<u32> = out_binds.iter().map(|&(s, _)| s).collect();
        let captured = self.frame_dyn_capture_formals(callee, dyn_snaps);
        let dyn_stash = self.frame_dyn_enter(callee);
        self.frame_dyn_install_formals(captured);
        let mut scalar_writes = Vec::new();
        let mut outs_dyn = Vec::new();
        if let Some(outs) = self.run_task_call(callee, in_vals, &out_s) {
            outs_dyn = self.frame_dyn_capture_out(callee, out_binds);
            for ((s, lval), val) in out_binds.iter().zip(outs) {
                if self.frame_dyn_out_bind(callee, *s, lval) {
                    continue; // dyn formal → heap deep-copy, not the scalar value
                }
                scalar_writes.push((lval.clone(), val));
            }
        }
        self.frame_dyn_exit(dyn_stash);
        self.frame_dyn_install_formals(outs_dyn);
        scalar_writes
    }

    /// Stage-2/3 fork-in-frame: allocate a Case-B shared-window arena slot pre-filled with
    /// `init`, reusing a freed handle when available (the `dyn_heap`/`class_heap` free-list
    /// pattern). Returns the handle a `WindowSlot::Shared` carries. Stage 3: seed the slot's
    /// refcount to `1` (the parent frame's initial reference) on BOTH the free-list-reuse and
    /// the push path — `frame_window_rc` is index-parallel to `frame_windows`, so the push
    /// path grows both in lockstep.
    pub(crate) fn alloc_frame_window(&self, init: Vec<Value>) -> u32 {
        if let Some(h) = self.frame_window_free.borrow_mut().pop() {
            self.frame_windows.borrow_mut()[h as usize] = Some(init);
            self.frame_window_rc.borrow_mut()[h as usize] = 1; // parent's initial reference
            h
        } else {
            let mut g = self.frame_windows.borrow_mut();
            g.push(Some(init));
            let h = (g.len() - 1) as u32;
            drop(g);
            self.frame_window_rc.borrow_mut().push(1); // parallel arena; seed rc = 1
            h
        }
    }

    /// Stage-3 fork-in-frame: a fork ARM (a spawned Case-B child) takes a NEW reference to
    /// the parent's shared window. Called ONCE per spawned child in `exec_fork` (the child's
    /// arm `FrameRec` holds a `WindowSlot::Shared(h)`). Balanced by the child's
    /// `release_frame_window` at completion (`exit_arm_frame`).
    pub(crate) fn retain_frame_window(&self, h: u32) {
        let mut rc = self.frame_window_rc.borrow_mut();
        debug_assert!(
            rc[h as usize] > 0,
            "retain of a freed shared frame window (h={h})"
        );
        rc[h as usize] += 1;
    }

    /// Stage-3 fork-in-frame: DROP one reference to a Case-B shared window and free the arena
    /// slot only when the LAST reference goes (rc → 0). Replaces the Stage-2 direct
    /// `free_frame_window` in `exit_task_frame` (parent `Return`) and `exit_arm_frame` (child
    /// completion). For `join`-all this frees at exactly the same moment as Stage 2 (every
    /// child releases before the parent resumes, so the parent's release is the one that
    /// hits 0); for `join_any`/`join_none` a surviving child defers the free past the parent.
    pub(crate) fn release_frame_window(&self, h: u32) {
        let mut rc = self.frame_window_rc.borrow_mut();
        debug_assert!(
            rc[h as usize] > 0,
            "shared frame window rc underflow (h={h})"
        );
        rc[h as usize] -= 1;
        if rc[h as usize] == 0 {
            drop(rc); // free_frame_window re-asserts rc == 0; release the borrow first
            self.free_frame_window(h);
        }
    }

    /// Stage-2/3 fork-in-frame: free a Case-B shared window slot (return it to the free-list).
    /// Stage 3 calls this ONLY from `release_frame_window` at rc == 0 (never a live slot); the
    /// `debug_assert` locks in that no reference is dropped early (a use-after-free guard).
    pub(crate) fn free_frame_window(&self, h: u32) {
        debug_assert_eq!(
            self.frame_window_rc.borrow()[h as usize],
            0,
            "freeing a shared frame window with live references (h={h})"
        );
        self.frame_windows.borrow_mut()[h as usize] = None;
        self.frame_window_free.borrow_mut().push(h);
    }

    /// Round-14 V3/V4 (suspendable path): push a fresh call frame for `callee` and copy
    /// in the input actuals — the MANUAL, suspend-safe twin of `run_task`'s setup (which
    /// uses RAII guards that assume a single synchronous scope). `run_process` drives the
    /// callee's CFG in between (so NBA/$systask run natively and a `Delay`/`Wait` in the
    /// body suspends the calling process); `exit_task_frame` pops it at `Return`. The
    /// window rides the shared `frame_stack` — correct for a non-suspending call (the
    /// callee runs to completion before any other activity interleaves); a call that
    /// actually suspends needs the per-activity window (a follow-on phase).
    pub(crate) fn enter_task_frame(&self, callee: u32, in_vals: &[(u32, Value)]) {
        let m = self.func_table[callee as usize];
        let base = m.base_net;
        let nloc = m.locals_len;
        self.call_depth.set(self.call_depth.get() + 1);
        self.frame_scope.borrow_mut().push(callee);
        let has_auto = self.func_has_auto[callee as usize];
        let has_static = self.func_has_static[callee as usize];
        let fresh: Vec<Value> = (0..nloc)
            .map(|s| {
                let nv = &self.ir.nets[(base + s) as usize];
                if nv.kind == NetKind::String {
                    Value::from_str_bytes(&[])
                } else {
                    Value::from_packed(&nv.init, nv.width.max(1), nv.signed)
                }
            })
            .collect();
        // Stage-2 fork-in-frame: a Case-B `join` task (`contains_shared_fork`) with
        // automatic locals gets an interior-mutable ARENA window (a `WindowSlot::Shared`)
        // so its fork arms share the parent's locals by handle; every other task keeps the
        // byte-identical `Owned` window. (A shared task with only STATIC slots needs no
        // arena — its locals already live in the shared `static_store`.)
        let shared = self
            .func_contains_shared_fork
            .get(callee as usize)
            .copied()
            .unwrap_or(false);
        match (has_auto, has_static, shared) {
            (true, _, true) => {
                let h = self.alloc_frame_window(fresh);
                self.frame_stack.borrow_mut().push(WindowSlot::Shared(h));
            }
            (true, true, false) => {
                self.frame_stack
                    .borrow_mut()
                    .push(WindowSlot::Owned(fresh.clone()));
                self.static_store
                    .borrow_mut()
                    .entry(callee)
                    .or_insert(fresh);
            }
            (true, false, false) => self.frame_stack.borrow_mut().push(WindowSlot::Owned(fresh)),
            (false, _, _) => {
                self.static_store
                    .borrow_mut()
                    .entry(callee)
                    .or_insert(fresh);
            }
        }
        for (slot, v) in in_vals {
            let bound = self.bind_formal(callee, *slot, base, v.clone());
            self.frame_slot_write(
                callee,
                self.frame_slot_auto[(base + *slot) as usize],
                *slot,
                bound,
            );
        }
    }

    /// Round-14 V3/V4 (suspendable path): copy out `callee`'s output/inout slots and pop
    /// its frame — the MANUAL twin of `run_task`'s teardown. Returns the out-slot values
    /// (the caller writes them to its lvalues). Mirrors `enter_task_frame`'s push order.
    pub(crate) fn exit_task_frame(&self, callee: u32, out_slots: &[u32]) -> Vec<Value> {
        let m = self.func_table[callee as usize];
        let base = m.base_net;
        let has_auto = self.func_has_auto[callee as usize];
        let outs: Vec<Value> = out_slots
            .iter()
            .map(|&s| {
                let nv = &self.ir.nets[(base + s) as usize];
                self.frame_slot_read(callee, self.frame_slot_auto[(base + s) as usize], s)
                    .resize_keep_sign(nv.width.max(1), nv.signed)
            })
            .collect();
        if has_auto {
            // Stage-2/3 fork-in-frame: a Case-B task's window is a `Shared(h)` arena slot —
            // DROP the parent's reference here (`release_frame_window`). For `join`-all every
            // fork arm has already completed (each dropped its own reference in
            // `exit_arm_frame`), so the parent's release is the one that hits 0 → freed now,
            // byte-identical to Stage 2's direct free. For `join_any`/`join_none` a surviving
            // child still holds a reference, so the slot is NOT freed here — the last child's
            // `exit_arm_frame` frees it (the surplus/detached child reads a valid window
            // meanwhile). An `Owned` window just drops its `Vec`.
            if let Some(WindowSlot::Shared(h)) = self.frame_stack.borrow_mut().pop() {
                self.release_frame_window(h);
            }
        }
        self.frame_scope.borrow_mut().pop();
        self.call_depth.set(self.call_depth.get().saturating_sub(1));
        outs
    }

    /// Stage-1 fork-in-frame: tear down a completing fork child's ARM frame. The arm rode
    /// an OWNED window on the shared `frame_stack` while running (Case A → an empty window,
    /// pushed by `run_process`'s `Some(..)`→restore path on each resume), so pop it here —
    /// gated on `func_has_auto` exactly like `exit_task_frame`. Unlike `exit_task_frame`
    /// this does NOT pop `frame_scope` / decrement `call_depth`: the arm frame is spawned
    /// by `exec_fork` (it is never passed through `enter_task_frame`), so it pushed
    /// NEITHER — its window reaches `frame_stack` only via the restore machinery. Popping
    /// them here would corrupt the still-parked PARENT task's frame context. There is no
    /// out-copy (a fork arm has no out-binds). `callee` is the enclosing task's FuncId
    /// (the arm frame's `callee`).
    ///
    /// Stage-2/3 fork-in-frame: a completing fork child's arm window is a `WindowSlot::Shared(h)`
    /// that ALIASES the parent's handle. Pop it (balancing the restore push) and, in Stage 3,
    /// DROP this arm's reference (`release_frame_window`). Under `join` the parent still holds
    /// its own reference and outlives every arm, so this release never hits 0 here (the
    /// parent's `Return` frees the slot — byte-identical to Stage 2's pop-without-free). Under
    /// `join_any`/`join_none` the parent may have ALREADY returned (releasing its reference),
    /// so a SURVIVING arm can be the last reference — then the release frees the slot HERE,
    /// deferring the free correctly past the parent (the refcount's whole purpose). A Case-A
    /// arm carries an `Owned(empty)` window and just drops its `Vec` (no rc).
    pub(crate) fn exit_arm_frame(&self, callee: u32) {
        if self.func_has_auto[callee as usize] {
            // Pop first (releasing the `frame_stack` borrow), THEN release — the arena
            // free-list / refcount RefCells are distinct from `frame_stack`.
            let popped = self.frame_stack.borrow_mut().pop();
            if let Some(WindowSlot::Shared(h)) = popped {
                self.release_frame_window(h);
            }
        }
    }

    /// T1-9: OPEN this activation's frame-local dyn-array slots and hand back the
    /// caller's contents as a STASH the exit restores.
    ///
    /// The heap object lives in `dyn_heap[net]`, keyed by NET — one slot per declared
    /// local, shared by every activation of the same func/task. That is why a RECURSIVE
    /// or CONCURRENT entry used to be a fatal: the inner activation would write the
    /// outer one's array. Taking the slot here and putting it back at exit makes the
    /// slot per-ACTIVATION without a per-activation heap: the inner activation starts
    /// from `None` (an unallocated array, exactly as a first call does) and the outer
    /// one gets its own object back untouched.
    ///
    /// The returned stash travels WITH the activation — a local for the synchronous
    /// paths, the `FrameRec` for the suspendable one — never a shared stack. That is the
    /// load-bearing choice: a global stack would be LIFO-violated by two activities
    /// interleaving in the same task (A enters, suspends; B enters; A resumes and exits),
    /// which is precisely the concurrency this is meant to make safe.
    ///
    /// Cheap: skips the scan entirely unless the callee actually has a dyn-array local,
    /// and returns an empty stash then — so every design without one is untouched.
    pub(crate) fn frame_dyn_enter(&self, callee: u32) -> Vec<(u32, Option<DynObj>)> {
        self.frame_dyn_enter_from(callee, 0)
    }

    /// §4.5.194: `first_slot` skips FORMAL slots (`[0, n_params)`) for the FUNCTION path —
    /// a function's `input` dyn formal is snapshotted by the caller BEFORE `run_frame_call`,
    /// so its slot is legitimately live at entry and is NOT this activation's to take;
    /// only the function's own dyn LOCALS (slots >= n_params) are per-activation. The
    /// suspendable/task callers pass 0 (their snapshot runs AFTER this).
    pub(crate) fn frame_dyn_enter_from(
        &self,
        callee: u32,
        first_slot: u32,
    ) -> Vec<(u32, Option<DynObj>)> {
        if !self
            .func_has_dyn_local
            .get(callee as usize)
            .copied()
            .unwrap_or(false)
        {
            return Vec::new();
        }
        let m = self.func_table[callee as usize];
        let mut heap = self.dyn_heap.borrow_mut();
        let mut saved = Vec::new();
        for s in first_slot..m.locals_len {
            let net = (m.base_net + s) as usize;
            if self.ir.nets[net].kind == NetKind::DynArray {
                saved.push((net as u32, heap[net].take()));
            }
        }
        saved
    }

    /// T1-9: the one shape stash/restore does NOT make safe — two SUSPENDABLE frames of
    /// the same callee whose `[enter, exit]` intervals OVERLAP rather than nest.
    ///
    /// Within a single activity the intervals are always properly nested: a synchronous
    /// call is straight-line, and a recursive suspendable call sits on the same
    /// `call_stack` and returns before its parent. Across activities they need not be —
    /// A can enter, suspend, let B enter, then resume and exit FIRST, at which point A's
    /// restore would overwrite B's live array. So that shape keeps the fatal, and the
    /// caller puts the stash back untouched before raising it.
    ///
    /// Latches `call_fatal` (a `Cell`) rather than `fatal_run`, because this is reachable
    /// from the `&self` paths too; the scheduler converts it to a fatal run end at the
    /// next region seam.
    pub(crate) fn fatal_frame_dyn_concurrent(&self) {
        if self.call_fatal.get() {
            return;
        }
        self.call_fatal.set(true);
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Fatal,
            code: MsgCode::RunFatal,
            message: "internal: a frame-local dynamic array (or dynamic-array input \
                      formal) was still held at frame entry by an activation that is not \
                      on this activity's call stack. Concurrent activations are supported \
                      — each parks its arrays off the shared heap while suspended — so \
                      reaching this means that park/unpark invariant was broken rather \
                      than a construct being unsupported. Please report the design"
                .to_string(),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.now }),
        }));
    }

    /// T1-9: close the activation `frame_dyn_enter*` opened — RESTORE the stashed outer
    /// contents into exactly the slots that were taken.
    ///
    /// Replaces the old free-at-exit. For a non-reentrant call the stash is all `None`,
    /// so this is byte-identically the old `.take()`: the next call still starts from an
    /// unallocated array. For a nested one it hands the parent its array back.
    ///
    /// Restoring the RECORDED slots (rather than re-deriving them from `callee`/
    /// `first_slot`) is what keeps enter and exit symmetric even if the two are called
    /// with different `first_slot` — the function path takes locals only, the task path
    /// takes formals too, and each puts back precisely what it took.
    pub(crate) fn frame_dyn_exit(&self, saved: Vec<(u32, Option<DynObj>)>) {
        if saved.is_empty() {
            return;
        }
        let mut heap = self.dyn_heap.borrow_mut();
        for (net, obj) in saved {
            heap[net as usize] = obj;
        }
    }

    /// V2A-frame (§4.5.173): DEEP-COPY each caller dynamic-array actual into the callee's
    /// per-activation `input`-formal heap slot (pass-by-VALUE, IEEE §13.5.1) — called
    /// right after `enter_task_frame` on the suspendable path. `dyn_snaps[i] = (formal
    /// slot, caller src net)`; the source's heap object is cloned so later writes to
    /// either side never show through (a never-allocated source copies as the empty
    /// array). The formal's slot is freed at `exit_task_frame` (`frame_dyn_free`) and
    /// guarded against recursive/concurrent reentry (`frame_dyn_reentry_ok`), exactly
    /// like a frame-local dyn-array LOCAL (§4.5.171). No dyn formals ⇒ no-op.
    /// T1-9: the READ half of the caller→formal snapshot — deep-copy each actual's heap
    /// object OUT of the heap without writing anything.
    ///
    /// Split from the write half because a RECURSIVE call can pass the formal AS its own
    /// actual (`task rec(input int b[], input int n); … rec(b, n-1);`). Then the snapshot
    /// SOURCE is the formal's own net, which `frame_dyn_enter` is about to take — so the
    /// three must be ordered capture → stash → install. Snapshotting AFTER the stash
    /// copied an already-emptied slot and the inner activations read `sz=0` with a bogus
    /// OOB warn at exit 0 (measured against iverilog, which reads the full array at every
    /// level); the old code never hit it because that shape was a fatal.
    pub(crate) fn frame_dyn_capture_formals(
        &self,
        callee: u32,
        dyn_snaps: &[(u32, u32)],
    ) -> Vec<(u32, Option<DynObj>)> {
        if dyn_snaps.is_empty() {
            return Vec::new();
        }
        let base = self.func_table[callee as usize].base_net;
        let heap = self.dyn_heap.borrow();
        dyn_snaps
            .iter()
            .map(|&(slot, src_net)| {
                let obj = heap.get(src_net as usize).and_then(|o| o.as_ref().cloned());
                (base + slot, obj)
            })
            .collect()
    }

    /// T1-9: the WRITE half — install captured actuals into the callee's formal slots.
    /// Run AFTER `frame_dyn_enter` has stashed whatever those slots held.
    pub(crate) fn frame_dyn_install_formals(&self, captured: Vec<(u32, Option<DynObj>)>) {
        if captured.is_empty() {
            return;
        }
        let mut heap = self.dyn_heap.borrow_mut();
        for (dst, obj) in captured {
            if let Some(cell) = heap.get_mut(dst as usize) {
                *cell = obj;
            }
        }
    }

    /// V2B (§4.5.194): deep-copy an OUTPUT/INOUT dyn-array FORMAL's heap array OUT to the
    /// caller's array at the subroutine's exit — the mirror of `frame_dyn_snapshot_formals`'s
    /// caller→formal copy-IN. Must run BEFORE `frame_dyn_free` frees the formal slot.
    pub(crate) fn frame_dyn_copy_out(&self, formal_net: u32, caller_net: u32) {
        let obj = self
            .dyn_heap
            .borrow()
            .get(formal_net as usize)
            .and_then(|o| o.as_ref().cloned());
        if let Some(cell) = self.dyn_heap.borrow_mut().get_mut(caller_net as usize) {
            *cell = obj;
        }
    }

    /// V2B (§4.5.194): if out-slot `slot` of `callee` is a DYNAMIC-array formal, deep-copy
    /// it out to the caller net (`lval.chunks[0].net`) and return `true` (the caller skips
    /// its scalar `write_lvalue`/`frame_write_lvalue`, whose value would be a meaningless
    /// read of the unused scalar frame slot). A non-dyn out-slot returns `false`.
    pub(crate) fn frame_dyn_out_bind(&self, callee: u32, slot: u32, _lval: &Lvalue) -> bool {
        let formal_net = self.func_table[callee as usize].base_net + slot;
        // The COPY itself moved to `frame_dyn_capture_out` — see there for why. This is
        // now only the "the scalar slot value is not the payload, skip that write"
        // decision it always also was.
        self.ir.nets[formal_net as usize].kind == NetKind::DynArray
    }

    /// T1-9: capture what a callee's dyn OUTPUT/INOUT formals produced, as
    /// `(caller net, cloned object)` pairs — to be installed AFTER `frame_dyn_exit`.
    ///
    /// The copy-out used to write the caller's net directly, in place, before the exit.
    /// Under RECURSION that is wrong, because the caller's array net and the callee's
    /// formal net are THE SAME net: the restore then puts the parent's pre-call array
    /// back over the copy-out and the result is discarded. Measured — a recursive
    /// `inout int b[]` incrementing `b[0]` at each of three levels printed 3/2/1 and
    /// left the actual at 1, where iverilog prints 3/3/3 and leaves 3 (§13.5.2
    /// copy-out propagates back up the chain).
    ///
    /// Capturing first and installing after the restore is correct for BOTH cases: with
    /// distinct nets the install writes somewhere the restore never touched, and with
    /// the same net it deliberately lands on top of the restored parent array, which is
    /// exactly what copy-out means.
    pub(crate) fn frame_dyn_capture_out(
        &self,
        callee: u32,
        out_binds: &[(u32, Lvalue)],
    ) -> Vec<(u32, Option<DynObj>)> {
        let base = self.func_table[callee as usize].base_net;
        let heap = self.dyn_heap.borrow();
        out_binds
            .iter()
            .filter_map(|(slot, lval)| {
                let formal_net = (base + slot) as usize;
                if self.ir.nets[formal_net].kind != NetKind::DynArray {
                    return None;
                }
                let dst = lval.chunks.first()?.net;
                Some((dst, heap.get(formal_net).and_then(|o| o.as_ref().cloned())))
            })
            .collect()
    }

    /// §4.5.194 (V5): execute a `loc = new[n]` (`Stmt::SysTask { DynNew, args }`) inside a
    /// frame body from the `&self` executor — args = [handle Signal, size, opt src Signal].
    /// Mirrors the `&mut` builtin's size handling (X/Z → empty + warn, cap) then defers to
    /// the shared `alloc_dyn_array` core. A non-dyn / malformed handle is a defensive no-op.
    pub(crate) fn frame_dyn_new(&self, args: &[u32]) {
        let Some(&a0) = args.first() else {
            return;
        };
        let sim_ir::Expr::Signal { net, .. } = &self.ir.exprs[a0 as usize] else {
            return;
        };
        let net = *net;
        if self.ir.nets.get(net as usize).map(|nv| nv.kind) != Some(NetKind::DynArray) {
            self.dyn_warn_once_at(net, "new[] on a non-dynamic-array handle (ignored)");
            return;
        }
        let n = match args.get(1) {
            Some(&a) => {
                let v = self.mk_eval_ctx().eval(a);
                if v.has_xz() {
                    self.dyn_warn_once_at(net, "new[] size is X/Z; array degraded to empty");
                    0
                } else {
                    let raw = v.to_u64().unwrap_or(0);
                    if raw > MAX_DYN_ELEMS as u64 {
                        self.dyn_warn_once_at(
                            net,
                            "new[] size exceeds the element cap (1<<24); clamped",
                        );
                    }
                    raw.min(MAX_DYN_ELEMS as u64) as usize
                }
            }
            None => 0,
        };
        let src_net = args.get(2).and_then(|&a| match &self.ir.exprs[a as usize] {
            sim_ir::Expr::Signal { net, .. } => Some(*net),
            _ => None,
        });
        self.alloc_dyn_array(net, n, src_net);
    }
}
/// NetKind → VCD VarType.
pub(crate) fn vcd_var_type(kind: NetKind) -> VarType {
    match kind {
        NetKind::Reg => VarType::Reg,
        NetKind::Integer => VarType::Integer,
        NetKind::Real => VarType::Real, // VCD `$var real`
        NetKind::Wire | NetKind::Logic => VarType::Wire,
        // v5/v6/v7 dyn + string handles are NEVER declared to the VCD (design
        // doc: variable length has no $var form) — filtered upstream; defensive map.
        NetKind::DynArray
        | NetKind::Queue
        | NetKind::Assoc
        | NetKind::AssocStr
        | NetKind::String => VarType::Wire,
    }
}

/// The VCD `$var` reference field. A bit-vector carries its DECLARED `[msb:lsb]`
/// range, so a waveform viewer labels an ascending `[0:3]` or non-zero-base
/// `[7:4]` vector correctly instead of defaulting to `[width-1:0]` (IEEE
/// 1364-2005 §18.2.3.2 `<reference> ::= id [ [msb:lsb] ]`; iverilog-pinned —
/// omitting it silently mislabels every non-`[N:0]` vector, e.g. `[0:3]` and
/// `[3:0]` become the indistinguishable `reg 4`).
///
/// A "vector" here is anything that is NOT a true scalar and NOT `[0:0]`:
/// `width > 1`, OR a 1-bit net at a NON-ZERO bit index (`reg [5:5]` records
/// `[5:5]`; iverilog-pinned). A true scalar (`reg a` → msb==lsb==0, width 1) and
/// a `[0:0]` net both collapse to a bare name, as iverilog does. A dimensionless
/// `real`/`realtime` never carries a range. `base` is the leaf name, already
/// array-indexed for a per-element word.
/// The `$var` reference text for a net, with the DECLARED `(msb, lsb)` when the net's
/// stored range cannot express it.
///
/// A net declared with a negative low bound (`logic [3:-2] x`) is stored NORMALIZED as
/// `[w-1:0]`, because `NetVar.msb`/`lsb` are frozen `u32` — so the `$var` line said
/// `x [5:0]` where iverilog says `x [3:-2]`. Same bits, but a waveform viewer then labels
/// every bit with the wrong index. `decl` comes from the `net_decl_ranges` sidecar and is
/// `None` for every ordinary net (byte-identical output).
pub(crate) fn vcd_var_reference_decl(
    vt: VarType,
    base: &str,
    width: u32,
    msb: u32,
    lsb: u32,
    decl: Option<(i64, i64)>,
) -> String {
    let is_vector = width > 1 || msb != 0 || lsb != 0;
    if !is_vector || matches!(vt, VarType::Real | VarType::Realtime) {
        return base.to_string();
    }
    if let Some((dm, dl)) = decl {
        // Trusted only when span-consistent, the same rule the stored range gets below.
        if (dm - dl).unsigned_abs() as u32 + 1 == width {
            return format!("{base} [{dm}:{dl}]");
        }
    }
    // A packed multi-dim net (`logic [0:3][7:0]`) stores msb = width-1 but a STALE
    // OUTER-dim lsb (elaborate's packed override resets width+msb, never lsb), so
    // |msb-lsb|+1 can disagree with the flat `width`. VCD flattens a packed vector
    // to `[width-1:0]` (iverilog-pinned: `[0:3][7:0]` and `[7:4][7:0]` both dump
    // `[31:0]`); emitting the raw `[width-1:staleLsb]` would be an internally
    // inconsistent range (size ≠ span). Use the declared `[msb:lsb]` only when it
    // is span-consistent (every single packed-dim net, incl. ascending/non-zero
    // base); otherwise fall back to the flat range.
    let span = (msb as i64 - lsb as i64).unsigned_abs() as u32 + 1;
    if span == width {
        format!("{base} [{msb}:{lsb}]")
    } else {
        format!("{base} [{}:0]", width.saturating_sub(1))
    }
}

/// Build a net's storage from its declared init. For scalars the init IS the
/// value; for arrays the init plane is replicated per word (elaborate emits one
/// init plane; v1 broadcasts it to every element).
pub(crate) fn expand_init(
    init: &BitPacked,
    width: u32,
    array_len: u32,
    total_bits: usize,
) -> BitPacked {
    let total_words = nwords(total_bits as u32).max(1);
    if array_len <= 1 {
        let mut val = init.val.clone();
        let mut unk = init.unk.clone();
        val.resize(total_words, 0);
        unk.resize(total_words, 0);
        return BitPacked { val, unk };
    }
    // broadcast the width-wide init to each element
    let mut out = BitPacked {
        val: vec![0; total_words],
        unk: vec![0; total_words],
    };
    let elem = Value::from_packed(init, width, false);
    for w in 0..array_len {
        let base = w * width;
        for i in 0..width {
            let (v, u) = elem.get_vu(i);
            set_bit(&mut out, base + i, v, u);
        }
    }
    out
}

/// Slice `width` bits starting at word `word`*width from a packed store.
pub(crate) fn slice_word(store: &BitPacked, width: u32, word: u32) -> BitPacked {
    let base = word * width;
    // WORD-PARALLEL fast path: a 64-aligned element read — copy whole store words and
    // mask the top partial word (which discards any neighbouring element's bits that
    // share that word). Covers every scalar net (base 0) and 64-aligned array elements;
    // an unaligned base (array element with a non-64-aligned offset) falls to bit-serial.
    if base % 64 == 0 {
        let n = nwords(width).max(1);
        let wbase = (base / 64) as usize;
        let mut val = vec![0u64; n];
        let mut unk = vec![0u64; n];
        for k in 0..n {
            val[k] = store.val.get(wbase + k).copied().unwrap_or(0);
            unk[k] = store.unk.get(wbase + k).copied().unwrap_or(0);
        }
        let m = top_mask(width);
        val[n - 1] &= m;
        unk[n - 1] &= m;
        return BitPacked { val, unk };
    }
    let mut tmp = Value::zeros(width.max(1), false);
    tmp.width = width;
    for i in 0..width {
        let (v, u) = bit_of(store, base + i);
        tmp.set_vu(i, v, u);
    }
    tmp.into_bitpacked(width)
}

#[inline]
pub(crate) fn bit_of(b: &BitPacked, i: u32) -> (u64, u64) {
    let w = (i / 64) as usize;
    let s = i % 64;
    let v = b.val.get(w).map_or(0, |x| (x >> s) & 1);
    let u = b.unk.get(w).map_or(0, |x| (x >> s) & 1);
    (v, u)
}

/// Set bit `i` of a packed store to (v,u); grow as needed; return true if changed.
#[inline]
pub(crate) fn set_bit(b: &mut BitPacked, i: u32, v: u64, u: u64) -> bool {
    let w = (i / 64) as usize;
    let s = i % 64;
    while b.val.len() <= w {
        b.val.push(0);
    }
    while b.unk.len() <= w {
        b.unk.push(0);
    }
    let ov = (b.val[w] >> s) & 1;
    let ou = (b.unk[w] >> s) & 1;
    if ov == v && ou == u {
        return false;
    }
    b.val[w] = (b.val[w] & !(1 << s)) | ((v & 1) << s);
    b.unk[w] = (b.unk[w] & !(1 << s)) | ((u & 1) << s);
    true
}

/// Scalar (bit0) 4-state of a net's current value — for edge detection.
pub(crate) fn scalar_bit0(b: &BitPacked) -> sim_ir::FourState {
    let v = b.val.first().copied().unwrap_or(0) & 1;
    let u = b.unk.first().copied().unwrap_or(0) & 1;
    match (v, u) {
        (0, 0) => sim_ir::FourState::Zero,
        (1, 0) => sim_ir::FourState::One,
        (0, 1) => sim_ir::FourState::X,
        _ => sim_ir::FourState::Z,
    }
}

/// WIDE-EDGE: bit0 edge predicates per IEEE 1364 §5.1.2 (Table — confirmed vs
/// iverilog 13.0 across all 12 four-state transitions). A `posedge` is a
/// transition TOWARD 1: `0→1`, `0→x`, `0→z`, `x→1`, `z→1`. A `negedge` is a
/// transition TOWARD 0: `1→0`, `1→x`, `1→z`, `x→0`, `z→0`. (`x→z`/`z→x` are
/// neither.) The old narrow form (`new==1 && prev!=1`) missed the `0→x`/`0→z`
/// rises and `1→x`/`1→z` falls — a silent-wrong for any edge net that goes to an
/// UNKNOWN value mid-sim. The write funnel ORs these per transition into the
/// `slot_edge` mask, the single source of procedural `@(posedge/negedge)` firing.
#[inline]
pub(crate) fn fs_is_posedge(prev: sim_ir::FourState, new: sim_ir::FourState) -> bool {
    use sim_ir::FourState::{One, Zero};
    (prev == Zero && new != Zero) || (new == One && prev != One)
}

#[inline]
pub(crate) fn fs_is_negedge(prev: sim_ir::FourState, new: sim_ir::FourState) -> bool {
    use sim_ir::FourState::{One, Zero};
    (prev == One && new != One) || (new == Zero && prev != Zero)
}
