//! deferred hierarchical TASK-ENABLE resolution (`u1.drive(a, b);`) — the one lane that
//! has to BUILD per-instance IR at resolve time (output-array copy-out, frame-formal
//! array forward) rather than just patch a NetId.
//!
//! Split from `hier_defer.rs` (R17).

use super::*;

impl Elaborator<'_> {
    /// Family D (r18): build each deferred hierarchical TASK enable's `TaskCallInfo` — the
    /// callee's per-instance frame-TASK FuncId + positional input binds — into
    /// `task_calls_proc`, once every instance's frame tasks are in `hier_tasks`.
    /// `hier_resolve` commits the leading segments to the callee instance scope (the same
    /// §23.6 walk the net/param/func resolvers use) and looks up `<inst>.<tname>`. An
    /// unresolved target — a non-framed (static) task, an output/inout/array/string
    /// formal, or a bad instance path — is loud (correct-or-loud). The placeholder
    /// `Terminator::Call.target` is patched to the callee entry (faithful IR; the process
    /// executor resolves the callee via `task_calls_proc`, so `target` is otherwise
    /// unread for a process-body call).
    pub(crate) fn resolve_deferred_hier_task_call(&mut self) {
        let deferred = std::mem::take(&mut self.deferred_hier_task_calls);
        // R16 §4-1: this pass runs long after the enables were lowered, so `cur_span`
        // points at whatever was last elaborated. Anchor each iteration at the enable the
        // user actually wrote, and put the ambient span back when the pass ends. Saved
        // once rather than per iteration because the loop body `continue`s in half a dozen
        // places and each iteration overwrites the previous one anyway.
        let ambient_span = self.cur_span;
        for d in deferred {
            self.cur_span = d.span;
            let Some(fid) = self.hier_resolve(&d.prefix, &d.path, &self.hier_tasks) else {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "unsupported hierarchical task call `{}` (the callee must be a framed \
                         task reached through an instance path, with scalar or fixed \
                         unpacked-array formals — no string / dynamic-array formal)",
                        d.path.join(".")
                    ),
                );
                continue;
            };
            // Arity guard: the engine binds actuals to formal slots BY INDEX, so a wrong
            // count would read past / drop formals (silent-wrong) — loud instead.
            let n_params = self.func_metas[fid as usize].n_params as usize;
            if n_params != d.arg_ids.len() {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "hierarchical task call `{}` passes {} argument(s) but the task takes {}",
                        d.path.join("."),
                        d.arg_ids.len(),
                        n_params
                    ),
                );
                continue;
            }
            // §4.5.201: route each arg by the callee port DIRECTION (`hier_task_port_dirs`,
            // stored when the task was registered — the callee def is not in scope here).
            // INPUT → copy-in (value); OUTPUT → copy-out (caller lvalue); INOUT → both. A
            // non-lvalue output/inout actual is loud (correct-or-loud). `dirs.len()` ==
            // `arg_ids.len()` after the arity guard (both are the callee's formal count).
            let dirs = self
                .hier_task_port_dirs
                .get(&fid)
                .cloned()
                .unwrap_or_default();
            let base_net = self.func_metas[fid as usize].base_net;
            let mut in_binds: Vec<(u32, u32)> = Vec::new();
            let mut out_binds: Vec<(u32, ir::Lvalue)> = Vec::new();
            // item 1: (caller-array net, packed-temp net, formal shape) for every OUTPUT/INOUT
            // array formal — the copy-OUT is UNPACKED into the caller-array elements at the
            // front of the ret block after the loop (the deferred twin of the local §4.5.204).
            let mut out_array_unpacks: Vec<(u32, u32, ArrayFormal)> = Vec::new();
            let mut bind_err = false;
            for (i, dir) in dirs.iter().enumerate() {
                // §4.5.207: the callee formal `i` is the per-instance net `base_net + i`. If it
                // is an md-packed ARRAY slot (`frame_arr_formal_meta`), the arg must be a bare
                // whole-array actual (`arg_arrays[i]`) packed into the slot — INPUT only (an
                // output/inout array copy-out over a hier enable is a deferred follow-on).
                let callee_af = self
                    .frame_arr_formal_meta
                    .get(&(base_net + i as u32))
                    .cloned();
                if let Some(af) = callee_af {
                    // An array formal needs a bare whole-array actual — a static array net
                    // resolved in the caller scope at defer time (`arg_arrays[i]`). A frame-LOCAL
                    // array actual is md-packed (not a static array) ⇒ `None` ⇒ loud (ROADMAP
                    // follow-on #2: forwarding a frame formal array through a nested enable).
                    let Some(caller_net) = d.arg_arrays[i] else {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "hierarchical task call `{}`: an array formal needs a bare \
                                 whole-array actual",
                                d.path.join(".")
                            ),
                        );
                        bind_err = true;
                        continue;
                    };
                    // Shared shape gate — the copy-IN and copy-OUT agree on "matching".
                    if !self.hier_array_shape_ok(caller_net, &af) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "hierarchical task call `{}`: the array argument's shape does \
                                 not match the formal",
                                d.path.join(".")
                            ),
                        );
                        bind_err = true;
                        continue;
                    }
                    match dir {
                        // INPUT (§4.5.207): pack the whole-array actual into the callee's
                        // md-packed slot value (copy-in, IEEE §13.5.1 pass-by-value).
                        ast::PortDir::Input => {
                            if let Some(eid) = self.pack_hier_array_actual(caller_net, &af) {
                                in_binds.push((i as u32, eid));
                            }
                        }
                        // OUTPUT / INOUT (item 1): the body writes the md-packed slot; at exit the
                        // engine copies the WHOLE slot to a fresh packed temp (a scalar out-bind),
                        // then the ret-block UNPACK (built after this loop) writes it into the
                        // caller-array elements — the resolve-time twin of the local §4.5.193/204
                        // copy-out. INOUT additionally PACKS the caller array INTO the slot at
                        // entry (a normal in-bind, §4.5.207). Hand-IEEE §13.5.2 pass-by-value-
                        // result (iverilog rejects unpacked subroutine array ports).
                        ast::PortDir::Output | ast::PortDir::Inout => {
                            // §4.5.210: the copy-OUT unpacks into the caller-array ELEMENTS, which
                            // needs a writable static array. A FORWARDED frame array formal (an
                            // md-packed frame net) would need a frame-executor part-select
                            // writeback — loud (input forwarding is supported above).
                            if !self.net_is_static_array(caller_net) {
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "hierarchical task call `{}`: forwarding a frame array \
                                         formal to an OUTPUT/INOUT array formal is unsupported \
                                         (input forwarding only)",
                                        d.path.join(".")
                                    ),
                                );
                                bind_err = true;
                                continue;
                            }
                            self.deny_const_param_write(
                                caller_net,
                                "connect an output/inout array to",
                            );
                            if matches!(dir, ast::PortDir::Inout) {
                                if let Some(eid) = self.pack_hier_array_actual(caller_net, &af) {
                                    in_binds.push((i as u32, eid));
                                }
                            }
                            let w = af.count.saturating_mul(af.elem_w).max(1);
                            let packed_net = self.nets.len() as u32;
                            let pname = format!("__houtpack${}${packed_net}", d.path.join("$"));
                            self.add_net(
                                &pname,
                                ir::NetVar {
                                    kind: ir::NetKind::Reg,
                                    width: w,
                                    msb: w.saturating_sub(1),
                                    lsb: 0,
                                    signed: false,
                                    array_len: 1,
                                    dir: ir::PortDir::Internal,
                                    init: default_init(ast::NetVarKind::Reg, w),
                                },
                            );
                            out_binds.push((i as u32, whole_net_lvalue(packed_net)));
                            out_array_unpacks.push((caller_net, packed_net, af));
                        }
                    }
                    continue;
                }
                // Scalar formal — a whole-array actual here is a type mismatch (loud).
                if d.arg_arrays[i].is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "hierarchical task call `{}`: a whole-array actual was passed to a \
                             scalar formal",
                            d.path.join(".")
                        ),
                    );
                    bind_err = true;
                    continue;
                }
                if matches!(dir, ast::PortDir::Input | ast::PortDir::Inout) {
                    in_binds.push((i as u32, d.arg_ids[i]));
                }
                if matches!(dir, ast::PortDir::Output | ast::PortDir::Inout) {
                    match &d.arg_lvals[i] {
                        Some(lv) => out_binds.push((i as u32, lv.clone())),
                        None => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "hierarchical task call `{}`: an output/inout argument must \
                                     be a writable net or select",
                                    d.path.join(".")
                                ),
                            );
                            bind_err = true;
                        }
                    }
                }
            }
            if bind_err {
                continue;
            }
            // item 1: build the copy-OUT unpack (`caller[i] = packed[i*ew +: ew]`) for every
            // OUTPUT/INOUT array formal — direct IR, no scope/`lower_stmt`. Position `i` ↔ the
            // caller's flat word `i` is the EXACT reverse of `pack_hier_array_actual` (which
            // packs the caller's flat word `i` at slot position `i`), so the round-trip is
            // correct by construction. Element signedness is the caller array's, applied on
            // later reads (bit-faithful copy here). Prepended to the ret block below.
            let mut unpack_sids: Vec<u32> = Vec::new();
            for (caller_net, packed_net, af) in &out_array_unpacks {
                for pos in 0..af.count {
                    let base_sig = self.push_expr(ir::Expr::Signal {
                        net: *packed_net,
                        word: None,
                    });
                    let off = self.const_u32_expr(pos.saturating_mul(af.elem_w), 32);
                    let wid = self.const_u32_expr(af.elem_w, 32);
                    let rhs = self.push_expr(ir::Expr::Select {
                        base: base_sig,
                        offset: off,
                        width: wid,
                        kind: ir::SelKind::PartIdxUp,
                    });
                    let word = self.const_u32_expr(pos, 32);
                    let lhs = ir::Lvalue {
                        chunks: vec![ir::LvalChunk {
                            net: *caller_net,
                            word: Some(word),
                            offset: None,
                            width: None,
                            kind: ir::SelKind::Bit,
                        }],
                    };
                    unpack_sids.push(self.push_stmt(ir::Stmt::BlockingAssign { lhs, rhs }));
                }
            }
            let info = TaskCallInfo {
                callee: fid,
                in_binds,
                out_binds,
            };
            let entry = self.funcs[fid as usize].entry;
            // §4.5.208: a NESTED-in-frame-body enable's placeholder `Call` lives in
            // `func_blocks` and is keyed into `task_calls_func`; a top-level process enable
            // uses `processes[proc]` + `task_calls_proc`. The engine reads the same two tables.
            // item 1: capture the `ret_bb` while patching the terminator, then PREPEND the
            // copy-out unpack to that block so it runs AFTER the task's exit (the out-bind wrote
            // the packed temp) and BEFORE any statement following the enable.
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
        self.cur_span = ambient_span;
    }
}
