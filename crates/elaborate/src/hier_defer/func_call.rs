//! §3.b: a HIERARCHICAL call `u.fw(x)` to a function whose body writes a module net.
//!
//! The local call to such a function is hoisted to a `Terminator::Call` whose only
//! out-bind is the return slot (`emit_frame_func_out_call`), so the write runs on the
//! statement executor and the expression reads a temp. A hierarchical call has the same
//! route and one obstacle: the callee's FuncId does not exist while the CALLING module is
//! lowered (the child instance comes after). So the call site borrows the shape of a
//! hierarchical TASK enable — a placeholder terminator keyed by `(proc, block)`, the
//! arguments lowered in the caller's scope, a [`DeferredHierTaskCall`] carrying the
//! return lvalue — and `resolve_deferred_hier_task_call` builds the `TaskCallInfo` once
//! every instance's frame functions are in `hier_funcs`. What lets the caller DECIDE the
//! route without the callee is the per-module fact table (`hier_body_write_callee`),
//! read from the declaration.

use super::*;

impl Elaborator<'_> {
    /// The `(return width, return sign)` a copy-out call's temp is minted with:
    /// `func_metas` for a local callee, the declaration (folded in the instance's
    /// parameter environment) for a hierarchical one.
    pub(crate) fn out_call_ret_shape(&self, fid: u32, name: &ast::HierPath) -> (u32, bool) {
        if fid != POISON_FID {
            return self
                .func_metas
                .get(fid as usize)
                .map(|m| (m.ret_width, m.ret_signed))
                .unwrap_or((32, true));
        }
        self.hier_body_write_callee(name)
            .map(|c| (c.ret_width, c.ret_signed))
            .unwrap_or((32, true))
    }

    /// One emitter for every hoist site: a local callee goes to
    /// `emit_frame_func_out_call`, a hierarchical one (`POISON_FID`) is deferred.
    pub(crate) fn emit_out_call(
        &mut self,
        b: &mut ProcessBuilder,
        fid: u32,
        name: &ast::HierPath,
        func: &ast::FunctionDef,
        args: &[ast::Expr],
        ret_lval: ir::Lvalue,
    ) {
        if fid == POISON_FID {
            self.emit_deferred_hier_func_call(b, name, args, ret_lval);
        } else {
            self.emit_frame_func_out_call(b, fid, func, args, ret_lval);
        }
    }

    /// Seal the current block with a placeholder `Terminator::Call` for `u.fw(args)` and
    /// record it for `resolve_deferred_hier_task_call`. Mirrors the task-enable defer in
    /// `inline_task.rs`; the differences are that every actual is an input (the funnel
    /// admits nothing else), each is sized to its formal the way a local call sizes it,
    /// and the return slot is bound to `ret_lval`.
    fn emit_deferred_hier_func_call(
        &mut self,
        b: &mut ProcessBuilder,
        name: &ast::HierPath,
        args: &[ast::Expr],
        ret_lval: ir::Lvalue,
    ) {
        let path: Vec<String> = name.segments.iter().map(|s| s.name.clone()).collect();
        let Some(callee) = self.hier_body_write_callee(name) else {
            // The funnel answered from the same lookup a moment ago.
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "hierarchical call `{}` lost its declaration between the hoist \
                     decision and the emit (vita bug)",
                    path.join(".")
                ),
            );
            return;
        };
        // A `.formal(v)` cannot be reordered without the callee's formals in scope,
        // and an omitted formal's DEFAULT is an expression in the CALLEE's scope, which
        // the caller cannot lower. Both loud; the arity guard at resolve catches the
        // second when it is not caught here.
        if args
            .iter()
            .any(|a| matches!(a.kind, ast::ExprKind::NamedArg { .. }))
        {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "hierarchical call `{}` with a named argument `.formal(v)` is \
                     unsupported across an instance path (write the arguments \
                     positionally)",
                    path.join(".")
                ),
            );
            return;
        }
        if args.len() != callee.def.ports.len() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "hierarchical call `{}` passes {} argument(s) but `{}` takes {} (a \
                     default-filled formal is unsupported across an instance path: its \
                     default is an expression in the callee's scope)",
                    path.join("."),
                    args.len(),
                    path.last().cloned().unwrap_or_default(),
                    callee.def.ports.len()
                ),
            );
            return;
        }
        let formal_widths = callee.formal_widths.clone();
        let mut arg_ids = Vec::with_capacity(args.len());
        for (a, fw) in args.iter().zip(formal_widths) {
            arg_ids.push(self.lower_ctx_or_plain(a, fw));
        }
        let call_block = b.cur_id();
        let ret = b.new_block();
        b.end_block_with(ir::Terminator::Call {
            target: ret.raw(), // placeholder — patched to the callee entry at resolve
            ret_bb: ret.raw(),
        });
        b.start_block(ret);
        let n = arg_ids.len();
        let call = DeferredHierTaskCall {
            span: Some(name.span),
            proc: self.cur_proc,
            call_block,
            func_block: if self.frame_task_lowering {
                Some(call_block)
            } else {
                None
            },
            prefix: self.cur_prefix.clone(),
            path,
            arg_ids,
            arg_lvals: vec![None; n],
            arg_arrays: vec![None; n],
            ret_lval: Some(ret_lval),
        };
        if self.frame_task_lowering {
            self.pending_hier_task_calls.push(call);
        } else {
            self.deferred_hier_task_calls.push(call);
        }
    }

    /// Resolve one hoisted hierarchical FUNCTION call: the callee's per-instance FuncId
    /// from `hier_funcs`, the arity, and the route the callee's own lowering recorded
    /// (`body_write_fids`) — then the same `TaskCallInfo` install as a task enable.
    pub(crate) fn resolve_deferred_hier_func_call_one(&mut self, d: DeferredHierTaskCall) {
        let Some(ret_lval) = d.ret_lval.clone() else {
            return;
        };
        let Some(fid) = self.hier_resolve(&d.prefix, &d.path, &self.hier_funcs) else {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "unsupported hierarchical function call `{}` (the callee must be a \
                     framed function with input-only scalar formals and a non-string \
                     return, reached through an instance path)",
                    d.path.join(".")
                ),
            );
            return;
        };
        let n_params = self.func_metas[fid as usize].n_params as usize;
        if n_params != d.arg_ids.len() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "hierarchical call `{}` passes {} argument(s) but the function takes {}",
                    d.path.join("."),
                    d.arg_ids.len(),
                    n_params
                ),
            );
            return;
        }
        // The hoist decided the route from the declaration; the callee's lowering
        // decided it from the same declaration AND the IR veto (a compiler-minted net).
        // A vetoed body is already refused by `validate_frame_body`; anything else that
        // disagrees is a vita bug, and running it would be a mis-routed write.
        if !self.body_write_fids.contains(&fid) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "hierarchical call `{}` was hoisted as a body-writing call but the \
                     callee's lowering did not route it that way (vita bug)",
                    d.path.join(".")
                ),
            );
            return;
        }
        // Rule B across the instance path (see `hier_body_write_callers`): the callee's
        // body write is a driver of the CHILD's net from THIS process, and two
        // `always_comb` processes calling the same function are the pair the per-module
        // scan refuses for a local call (`multidriver.rs`: a via-call writer joins the
        // `always_comb` × `always_comb` pair only — measured, verilator MULTIDRIVEN for
        // that pair and silent for `always_comb` × `initial`, and the oracles agree on
        // the value there). The identity is the SOURCE kind (`proc_idents`, lockstep
        // with `processes`), not `SensKind::Comb`, which a bare self-timed `always` has
        // too. A caller nested in a frame task body is not an `always_comb`.
        if d.func_block.is_none() {
            let is_comb = self
                .proc_idents
                .get(d.proc as usize)
                .is_some_and(|p| p.kind == "always_comb");
            let callers = self.hier_body_write_callers.entry(fid).or_default();
            let already = callers.iter().any(|&(p, _)| p == d.proc);
            if !already {
                callers.push((d.proc, is_comb));
            }
            let pair = callers.iter().filter(|&&(_, c)| c).count() >= 2;
            if pair && self.hier_body_write_refused.insert(fid) {
                // Once per callee, when the pair first exists; a later caller of a
                // refused callee is not installed either (the IR is discarded).
                let fname = d.path.last().cloned().unwrap_or_default();
                self.error(
                    MsgCode::ElabMultidriver,
                    &format!(
                        "the net `{fname}` assigns from its body is written by two \
                         `always_comb` processes, both through the hierarchical call \
                         `{}(...)`, which is two drivers on one variable (IEEE \
                         §9.2.2.2) — verilator MULTIDRIVEN / xcelium *E,MULAXX; give it \
                         one calling process",
                        d.path.join(".")
                    ),
                );
                return;
            }
            if self.hier_body_write_refused.contains(&fid) {
                return;
            }
        }
        // Deliberately NOT `note_frame_call`: the OBS `subroutines` census keys a row on
        // the module on the instance stack, which is empty in this finish-phase pass — a
        // count here filed a phantom row with an empty module name and left the real
        // callee at `sites: 0`. A hierarchical call is uncounted in that object by its
        // own `uncounted` text (the hierarchical task enable counts nothing either); the
        // runtime `subroutine_calls` object records it per instance.
        let return_slot = self.func_metas[fid as usize].return_slot;
        let in_binds: Vec<(u32, u32)> = d
            .arg_ids
            .iter()
            .enumerate()
            .map(|(i, &eid)| (i as u32, eid))
            .collect();
        let info = TaskCallInfo {
            callee: fid,
            in_binds,
            out_binds: vec![(return_slot, ret_lval)],
        };
        self.install_deferred_hier_call(&d, info, Vec::new());
    }

    /// Patch the placeholder terminator at `d`'s block to the callee entry and key
    /// `info` where the engine reads it — `task_calls_func` for a call nested in a frame
    /// TASK body, `task_calls_proc` for a process — prepending `unpack_sids` (an output
    /// array formal's copy-out, task enables only) to the ret block so it runs after the
    /// callee exits and before any statement following the call.
    pub(crate) fn install_deferred_hier_call(
        &mut self,
        d: &DeferredHierTaskCall,
        info: TaskCallInfo,
        unpack_sids: Vec<u32>,
    ) {
        let entry = self.funcs[info.callee as usize].entry;
        if let Some(fb) = d.func_block {
            self.task_calls_func.insert(fb, info);
            let mut ret_bb = None;
            if let Some(blk) = self.func_blocks.get_mut(fb as usize) {
                if let ir::Terminator::Call { target, ret_bb: rb } = &mut blk.term {
                    *target = entry;
                    ret_bb = Some(*rb);
                }
            }
            if let (Some(rb), false) = (ret_bb, unpack_sids.is_empty()) {
                if let Some(retblk) = self.func_blocks.get_mut(rb as usize) {
                    retblk.stmts.splice(0..0, unpack_sids);
                }
            }
        } else {
            self.task_calls_proc.insert((d.proc, d.call_block), info);
            let mut ret_bb = None;
            if let Some(blk) = self
                .processes
                .get_mut(d.proc as usize)
                .and_then(|p| p.body.get_mut(d.call_block as usize))
            {
                if let ir::Terminator::Call { target, ret_bb: rb } = &mut blk.term {
                    *target = entry;
                    ret_bb = Some(*rb);
                }
            }
            if let (Some(rb), false) = (ret_bb, unpack_sids.is_empty()) {
                if let Some(retblk) = self
                    .processes
                    .get_mut(d.proc as usize)
                    .and_then(|p| p.body.get_mut(rb as usize))
                {
                    retblk.stmts.splice(0..0, unpack_sids);
                }
            }
        }
    }
}
