//! the frame-call emitters — a plain `Expr::Call`, the dyn-array-formal snapshot
//! markers, a task enable, and the output/inout-formal copy-out call. Split from
//! `frames_call.rs` (R19) to keep every module under the 1000-line cap.

use super::*;

impl Elaborator<'_> {
    /// Emit an `Expr::Call` to a reserved frame `FuncId` (the call-site divert).
    /// Args are lowered in the CALLER scope (caller nets / outer subst). Returns a
    /// placeholder on an arity / out-formal violation (after the diagnostic).
    pub(crate) fn emit_frame_call(
        &mut self,
        fid: u32,
        func: &ast::FunctionDef,
        args: &[ast::Expr],
    ) -> u32 {
        let fname = &func.name.name;
        if func
            .ports
            .iter()
            .any(|p| !matches!(p.dir, ast::PortDir::Input))
        {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    // The r19 follow-on general hoister (`hoist/general.rs`) made every
                    // ONCE-EVALUATED expression position work, so listing the supported
                    // positions is no longer useful — the message now names what is left.
                    // Keeping the old enumeration would be a stale claim in the other
                    // direction: it said a `?:` arm and a nested expression were
                    // unsupported, which they no longer are.
                    "function `{fname}` has an output/inout formal, so its copy-out has to be \
                     emitted as a statement before the expression that calls it. That works in \
                     any position evaluated ONCE per statement, but not here. The remaining \
                     cases are: a CONTINUOUSLY re-evaluated expression (`assign`, `force`, a \
                     `wait` condition) — the copy-out cannot re-fire on every change; an \
                     intra-assignment delay (`x = #1 {fname}(...)`); a `min:typ:max` or a \
                     constraint/`with` expression; any position inside a function or task body \
                     lowered as a CALL FRAME that needs the copy-out HOISTED out of an \
                     expression (an assignment rhs, a condition, a `case` scrutinee) — a frame \
                     body's writes have to stay frame-local while the copy-out targets the \
                     caller's net, though a BARE call statement there does work, and so does \
                     the same expression in a module process; and an evaluation-order case the \
                     hoist cannot preserve — \
                     an output actual read to the LEFT of the call when the actual is not a \
                     plain bit-vector net, when two calls in the same expression write it, or \
                     when a call the hoist leaves in place could read it (its body, or an \
                     OMITTED formal's default, or a method on a non-container receiver). \
                     Assign the call to a temporary first (`t = {fname}(...);`) and use `t`."
                ),
            );
            return self.placeholder_expr();
        }
        // §11.6: each arg is in the context of its FORMAL's width (a fill grows to
        // it; non-fill ⇒ byte-identical via lower_expr). Omitted trailing actuals are
        // filled with their formals' default values (§13.5.3).
        let ports = func.ports.clone();
        let Some(eff_args) = self.fill_default_args(fname, &ports, args) else {
            return self.placeholder_expr();
        };
        let mut actual_ids: Vec<u32> = Vec::with_capacity(eff_args.len());
        for (i, &a) in eff_args.iter().enumerate() {
            let p = &ports[i];
            // §4.5.177: an `input` DYNAMIC-array formal (reserved as a `DynArray` net) is
            // supported ONLY on the blessed direct-rhs path (`x = f(arr)` at module-process
            // level), where `lower_stmt` has emitted the `handle_copy` snapshot marker that
            // fills the formal's heap slot. An UNBLESSED call has no marker, so the formal
            // would read an empty array — loud (correct-or-loud by construction). When
            // blessed, the arg is a placeholder (the real data rode the marker; the frame
            // window slot for a dyn-array formal is never read — reads go to the heap).
            if self.is_input_dyn_array_formal(p) {
                if !self.dyn_formal_call_ok {
                    // R16 §3.7: the old wording said "the DIRECT rhs of a blocking
                    // assignment at module-process level". Both halves were stale — r17
                    // lifted the module-process restriction (a function body, a task body
                    // and a loop body all work), and R16 added the `return` and buried
                    // positions. A user reading it moved the call out to a module process
                    // for nothing, which is the misdiagnosis class this report is about.
                    //
                    // R20 §4: it had gone stale again, and in BOTH directions — it named a
                    // `?:` arm as unsupported when r18 made it work, and never mentioned any
                    // of the positions that actually remain. So the round-20 reporter, whose
                    // call sat in a supported position (a comparison) and was loud for an
                    // entirely different reason (the §3.1 crosstalk regression), read three
                    // listed causes and found none of them present.
                    //
                    // Both lists below are measured, not inferred — a 25-position matrix
                    // against iverilog, re-run with a callee that is genuinely FRAMED. That
                    // last part matters: a trivial `function int cnt(input byte b[]); return
                    // b.size(); endfunction` is INLINED, so none of this machinery applies to
                    // it and a matrix built on one measures nothing. The first draft of this
                    // message was written from such a matrix and carried a stale clause.
                    //
                    // Dropped from the list: "two calls to `{fname}` in ONE expression inside a
                    // function/task body". Measured supported and CORRECT — `h = {b2h(d),
                    // b2h(e)}` yields `6162` and `h = {b2h(d), b2h(d)}` yields `6161`, both
                    // matching iverilog, because each call is hoisted to its own temp before
                    // the expression and the single slot is reused in sequence, not shared.
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "function `{fname}`: a dynamic-array formal `{}` is passed by \
                             snapshotting the caller array into the formal's slot, with a \
                             marker emitted just before the enclosing statement — so the call \
                             has to sit where that marker can go: a blocking- or \
                             nonblocking-assign rhs, a `return` value, or an unconditionally \
                             evaluated operand of one (concat, arithmetic, comparison, \
                             system-TASK argument) — and a `?:` arm, but only when \
                             `{fname}` is side-effect free, because a conditionally evaluated \
                             call cannot be hoisted without performing its effect on the arm \
                             that was not taken. It is not supported here. The \
                             remaining positions are: the right side of `&&`/`||`; a `while` \
                             or `for` CONDITION (re-evaluated every iteration, so one snapshot \
                             cannot serve it); a delay expression; an argument of another \
                             call, including a system FUNCTION such as `$sformatf`; the BASE \
                             of a select (`{fname}(arr)[7:0]` — a plain concat part is fine); \
                             an lvalue index; a `case` scrutinee; a `repeat` count; a cast or \
                             replication operand; and a RECURSIVE call inside `{fname}`'s own \
                             body when it is not the whole right-hand side, which would need a \
                             second snapshot slot while the first is still live. Assign the \
                             call to a variable first (`x = {fname}(arr);`) and use `x`",
                            p.name.name
                        ),
                    );
                    actual_ids.push(self.placeholder_expr());
                    continue;
                }
                // The actual must be a bare dyn-array net of matching element type — the
                // marker (`lower_stmt`) deep-copies THAT net into the formal's heap slot.
                if self.dyn_array_actual_net(a, p).is_none() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "function `{fname}`: dynamic-array formal `{}` needs a bare matching \
                             dynamic-array actual",
                            p.name.name
                        ),
                    );
                }
                actual_ids.push(self.const_u32_expr(0, 1));
                continue;
            }
            // §13.3 UARR: an unpacked-array formal takes a whole-array actual packed
            // into its md-packed slot; a formal outside the supported slice is
            // loud-rejected here (the earlier reserve left a sane placeholder net).
            if let Some(cls) = self.classify_array_formal(p) {
                match cls {
                    Ok(af) => {
                        let packed = self.lower_array_actual_packed(a, &af);
                        actual_ids.push(packed);
                    }
                    Err(reason) => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "function `{fname}`: unpacked-array formal `{}` is \
                                 unsupported — {reason}",
                                p.name.name
                            ),
                        );
                        actual_ids.push(self.placeholder_expr());
                    }
                }
                continue;
            }
            let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
            let (w, _, _, _) = self.range_to_dims(kind, p.range.as_ref(), p.signed);
            actual_ids.push(self.lower_ctx_or_plain(a, w));
        }
        self.push_expr(ir::Expr::Call {
            func: fid,
            args: actual_ids,
        })
    }

    /// §4.5.177: for a direct-rhs `x = f(arr)` where `f` is a FRAMED function (reserved
    /// via `reserve_frame_func`) with an `input` dyn-array formal, and we are at
    /// module-process level (`!in_frame_body`, so a `handle_copy` marker CAN run in the
    /// `&mut` process executor), emit one snapshot marker per bare dyn actual — a no-op
    /// `Display` whose StmtId maps (in `handle_copy_stmts`) to `(formal heap net, caller
    /// net)`, deep-copying the caller array into the callee formal's per-activation heap
    /// slot at run time — and set `dyn_formal_call_ok` so `emit_frame_call` binds the
    /// formal. Returns whether the call was blessed (caller clears the flag after lowering
    /// the rhs). Every non-matching case returns `false`, leaving the call loud in
    /// `emit_frame_call` (no marker ⇒ no support; correct-or-loud by construction). A
    /// framed function's `&self` executor can never mutate a dyn-array, so the snapshot is
    /// a sound pass-by-value/alias.
    pub(crate) fn emit_frame_dyn_formal_markers(
        &mut self,
        b: &mut ProcessBuilder,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        // Family C (r17): the `in_frame_body` guard is GONE. §4.5.194 made `dyn_heap`
        // interior-mutable (`RefCell`), so the snapshot marker's heap→heap deep-copy is
        // now an op the `&self` frame executors (`run_frame_call`/`run_task`) can run
        // too (see the marker arm there + `classify_frame_body`'s allow-list). This
        // lifts §4.5.177/179's module-process-only restriction, so a dyn-array-formal
        // function call BURIED in a function/task body (`s = sum(b);` inside a task)
        // works — iverilog-pinned. A `#delay` still blocks (its own timing concern).
        if delay.is_some() {
            return false;
        }
        let ast::ExprKind::Call { name, args } = &rhs.kind else {
            return false;
        };
        if name.segments.len() != 1 {
            return false;
        }
        let fname = &name.segments[0].name;
        let Some(&fid) = self.frame_idx.get(fname.as_str()) else {
            return false;
        };
        let Some(func) = self.func_table.get(fname.as_str()).cloned() else {
            return false;
        };
        if !func.ports.iter().any(|p| self.is_input_dyn_array_formal(p)) {
            return false;
        }
        let base_net = self.func_metas[fid as usize].base_net;
        for (i, p) in func.ports.iter().enumerate() {
            if !self.is_input_dyn_array_formal(p) {
                continue;
            }
            let Some(a) = args.get(i) else { continue };
            // A bare matching dyn-array actual → snapshot it into the formal's heap slot.
            // A non-bare / mismatched actual gets NO marker — `emit_frame_call` louds it.
            if let Some(caller_net) = self.dyn_array_actual_net(a, p) {
                let formal_net = base_net + i as u32;
                let sid = self.push_stmt(ir::Stmt::SysTask {
                    which: ir::SysTaskId::Display,
                    fmt: None,
                    args: Vec::new(),
                });
                self.handle_copy_stmts.insert(sid, (formal_net, caller_net));
                // Family C: record it as a dyn-formal marker so `classify_frame_body`
                // allows this Display in the suspendable/subset frame body (a §7.10
                // whole-handle copy is NOT in this set → stays loud in a frame body).
                self.dyn_formal_marker_stmts.insert(sid);
                b.push_stmt_id(sid);
            }
        }
        self.dyn_formal_call_ok = true;
        true
    }

    /// R16 §3.7: collect the frame ids of every dyn-formal call inside `e`, in source
    /// order. Only the positions [`Self::has_unhoistable_dyn_formal_call`] already
    /// accepts are walked, so a caller that checked that predicate knows this list is
    /// complete.
    fn dyn_formal_call_targets(&self, e: &ast::Expr, out: &mut Vec<(u32, String)>) {
        use ast::ExprKind as K;
        if let Some((fid, func)) = self.dyn_formal_call_target(e) {
            out.push((fid, func.name.name.clone()));
            return;
        }
        match &e.kind {
            K::Unary { operand, .. } => self.dyn_formal_call_targets(operand, out),
            K::Paren { inner } => self.dyn_formal_call_targets(inner, out),
            K::Binary { lhs, rhs, .. } => {
                self.dyn_formal_call_targets(lhs, out);
                self.dyn_formal_call_targets(rhs, out);
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.dyn_formal_call_targets(cond, out);
                self.dyn_formal_call_targets(then_e, out);
                self.dyn_formal_call_targets(else_e, out);
            }
            K::Concat { parts } => {
                for p in parts {
                    self.dyn_formal_call_targets(p, out);
                }
            }
            _ => {}
        }
    }

    /// R16 §3.7: emit snapshot markers for dyn-formal calls BURIED in `e` and bless the
    /// expression, without hoisting anything to a temp.
    ///
    /// This is what makes `return {h(b), "!"}` work INSIDE a frame body, where the temp
    /// hoist cannot go: `fresh_ret_temp` allocates a MODULE net, and a frame body writing
    /// one is outside the frame-call subset (it produced a diagnostic about "an assignment
    /// to a net outside the function" — a true statement about a temp the user never
    /// wrote). Markers need no storage of their own: each fills its callee's formal heap
    /// slot, and the expression then evaluates normally.
    ///
    /// SOUNDNESS. The markers all run BEFORE the expression, so two calls sharing one
    /// formal slot would both read the LAST snapshot. Repeated targets are therefore
    /// refused outright rather than mis-ordered — the temp hoist handles those wherever it
    /// is available. Distinct targets have distinct slots and cannot interfere. The caller
    /// must have checked `has_unhoistable_dyn_formal_call` first: the blessing is a global
    /// flag, so a call in a position this walk does not reach would be blessed without a
    /// marker and would read an empty array.
    pub(crate) fn emit_dyn_formal_markers_nested(
        &mut self,
        b: &mut ProcessBuilder,
        e: &ast::Expr,
    ) -> bool {
        let mut fids = Vec::new();
        self.dyn_formal_call_targets(e, &mut fids);
        if fids.is_empty() {
            return false;
        }
        // RECURSION. A marker writes the callee's formal slot BEFORE the expression
        // evaluates. When the callee is the function being lowered, those are the very
        // formals the rest of the expression reads — `return c[n-1] + f(c, n-1);` would
        // read `c` through a slot the marker had already overwritten for the recursive
        // call. It happens to give the right answer when every level passes the same
        // array and a silently wrong one the moment a level passes a different one, so
        // this is refused outright rather than left to chance. The enclosing frame is
        // the trailing `$func$<name>` / `$itask$<name>` segment of the active prefix.
        let enclosing = self.cur_prefix.rsplit('.').next().and_then(|s| {
            s.strip_prefix("$func$")
                .or_else(|| s.strip_prefix("$itask$"))
        });
        if let Some(encl) = enclosing {
            if fids.iter().any(|(_, n)| n == encl) {
                return false;
            }
        }
        let mut sorted: Vec<u32> = fids.iter().map(|(f, _)| *f).collect();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() != fids.len() {
            return false; // a repeated target would share one formal slot
        }
        let mut any = false;
        for e in Self::dyn_formal_call_exprs(e) {
            any |= self.emit_frame_dyn_formal_markers(b, None, &e);
        }
        any
    }

    /// R16 §3.7: the dyn-formal call sub-expressions of `e`, in source order — the same
    /// positions [`Self::dyn_formal_call_targets`] walks.
    fn dyn_formal_call_exprs(e: &ast::Expr) -> Vec<ast::Expr> {
        use ast::ExprKind as K;
        let mut out = Vec::new();
        fn go(e: &ast::Expr, out: &mut Vec<ast::Expr>) {
            use ast::ExprKind as K;
            match &e.kind {
                K::Call { .. } => out.push(e.clone()),
                K::Unary { operand, .. } => go(operand, out),
                K::Paren { inner } => go(inner, out),
                K::Binary { lhs, rhs, .. } => {
                    go(lhs, out);
                    go(rhs, out);
                }
                K::Ternary {
                    cond,
                    then_e,
                    else_e,
                } => {
                    go(cond, out);
                    go(then_e, out);
                    go(else_e, out);
                }
                K::Concat { parts } => {
                    for p in parts {
                        go(p, out);
                    }
                }
                _ => {}
            }
        }
        if matches!(e.kind, K::Call { .. }) {
            out.push(e.clone());
        } else {
            go(e, &mut out);
        }
        out
    }

    /// B2: emit a frame-TASK call. Seals the current block with `Terminator::Call`
    /// plus a continuation block, and registers the positional arg↔formal binding
    /// into `task_calls_proc` for a process-body call or `pending_task_calls` for a
    /// nested task-body call. Input actuals lower in the caller scope; an output or
    /// inout actual must be a simple net (lowered to a whole-net `Lvalue`).
    pub(crate) fn emit_frame_task_call(
        &mut self,
        b: &mut ProcessBuilder,
        fid: u32,
        task: &ast::TaskDef,
        args: &[ast::Expr],
    ) {
        let tname = &task.name.name;
        let Some(eff_args) = self.fill_default_args(tname.as_str(), &task.ports, args) else {
            return;
        };
        // §11.6: an input/inout actual is in the formal's width context (the frame
        // formal net sits at base_net + slot), so a fill grows to it.
        let base_net = self
            .func_metas
            .get(fid as usize)
            .map(|m| m.base_net)
            .unwrap_or(0);
        let mut in_binds: Vec<(u32, u32)> = Vec::new();
        let mut out_binds: Vec<(u32, ir::Lvalue)> = Vec::new();
        // §4.5.193: (caller array path, packed-temp net name, formal shape) for each
        // OUTPUT unpacked-fixed array formal — unpacked into the caller array elements
        // in `ret` AFTER the call (the out-bind copied the md-packed slot to the temp).
        let mut out_array_unpacks: Vec<(ast::HierPath, String, ArrayFormal)> = Vec::new();
        for (slot, (p, a)) in task.ports.iter().zip(eff_args.iter().copied()).enumerate() {
            let slot = slot as u32;
            let fw = self
                .nets
                .get((base_net + slot) as usize)
                .map(|n| n.width)
                .unwrap_or(32);
            match p.dir {
                ast::PortDir::Input => {
                    // V2A-frame (§4.5.173): a dyn-array input formal is pass-by-VALUE — the
                    // engine deep-copies the caller's array into the formal's per-activation
                    // heap slot at frame entry. Emit the in-bind as a BARE `Signal` reading
                    // the caller's dyn-array net; the engine recovers the source net from
                    // this Signal (formal net kind == DynArray ⇒ snapshot, not a scalar
                    // copy). A select / queue / assoc / non-dyn / element-mismatched actual
                    // is loud (`dyn_array_actual_net` ⇒ None; correct-or-loud).
                    if self.is_input_dyn_array_formal(p) {
                        match self.dyn_array_actual_net(a, p) {
                            Some(caller_net) => {
                                let sig = self.push_expr(ir::Expr::Signal {
                                    net: caller_net,
                                    word: None,
                                });
                                in_binds.push((slot, sig));
                            }
                            None => self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "task `{tname}`: dynamic-array formal `{}` needs a \
                                     bare matching dynamic-array actual",
                                    p.name.name
                                ),
                            ),
                        }
                    } else if let Some(cls) = self.classify_array_formal(p) {
                        // §13.3 UARR (§4.5.188): an `input` unpacked-fixed array formal —
                        // pack the whole-array actual into the md-packed slot value (the
                        // caller-side concat), IDENTICAL to the FUNCTION path
                        // (`emit_frame_call`). The engine copies this value into the frame
                        // slot; body `b[i]` reads route through the md-packed element read.
                        match cls {
                            Ok(af) => {
                                let packed = self.lower_array_actual_packed(a, &af);
                                in_binds.push((slot, packed));
                            }
                            Err(reason) => self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "task `{tname}`: unpacked-array formal `{}` is \
                                     unsupported — {reason}",
                                    p.name.name
                                ),
                            ),
                        }
                    } else {
                        let eid = self.lower_ctx_or_plain(a, fw);
                        in_binds.push((slot, eid));
                    }
                }
                ast::PortDir::Output | ast::PortDir::Inout => {
                    // V2B (§4.5.194): an OUTPUT/INOUT DYNAMIC-array formal. The body writes the
                    // DynArray heap slot (new[]/element); the engine deep-copies it OUT to the
                    // caller's dyn array at the subroutine's exit. INOUT also snapshots the
                    // caller IN at entry — emit a BARE `Signal` in-bind (like the input-dyn
                    // path) so `split_frame_in_binds` routes it to `frame_dyn_snapshot_formals`.
                    // The out-bind is the caller's whole dyn net; the engine detects the
                    // DynArray out-slot and does the heap copy instead of a scalar write.
                    if self.is_output_or_inout_dyn_array_formal(p) {
                        match self.dyn_array_actual_net(a, p) {
                            Some(caller_net) => {
                                self.deny_const_param_write(
                                    caller_net,
                                    "connect an output/inout dynamic array to",
                                );
                                if matches!(p.dir, ast::PortDir::Inout) {
                                    let sig = self.push_expr(ir::Expr::Signal {
                                        net: caller_net,
                                        word: None,
                                    });
                                    in_binds.push((slot, sig));
                                }
                                out_binds.push((slot, whole_net_lvalue(caller_net)));
                            }
                            None => self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "task `{tname}`: output/inout dynamic-array formal \
                                     `{}` needs a bare matching dynamic-array actual",
                                    p.name.name
                                ),
                            ),
                        }
                        continue;
                    }
                    // §4.5.193: an OUTPUT unpacked-fixed array formal (`output byte
                    // digest[N]`). The body writes the md-packed slot; copy the WHOLE
                    // slot to a fresh packed temp at exit (a normal scalar out-bind),
                    // then UNPACK the temp into the caller array elements after the call
                    // (below, in `ret`). Reuses the md-packed slot (§4.5.188) — no heap,
                    // works on the synchronous and suspendable paths. INOUT array = loud.
                    // §4.5.193 (r17 extends to INOUT): an OUTPUT/INOUT unpacked-fixed
                    // array formal. Body writes the md-packed slot; at exit copy the whole
                    // slot to a fresh packed temp (scalar out-bind), then UNPACK it into the
                    // caller array elements in `ret`. INOUT additionally PACKS the caller
                    // array INTO the slot at entry (a normal in-bind, identical to the
                    // §4.5.188 input path) — IEEE §13.5.2 pass-by-value-result. iverilog
                    // rejects unpacked subroutine array ports outright, so this is
                    // hand-IEEE / self-consistent (write→read-back round-trip).
                    if matches!(p.dir, ast::PortDir::Output | ast::PortDir::Inout) {
                        if let Some(cls) = self.classify_array_formal(p) {
                            match cls {
                                Ok(af) => {
                                    if let ast::ExprKind::Ident(path) = &a.kind {
                                        if path.segments.len() == 1 {
                                            let arr_net = self.resolve_net(path);
                                            self.deny_const_param_write(
                                                arr_net,
                                                "connect an output array to",
                                            );
                                            // INOUT copy-IN at entry (pass-by-value-result).
                                            if matches!(p.dir, ast::PortDir::Inout) {
                                                let packed = self.lower_array_actual_packed(a, &af);
                                                in_binds.push((slot, packed));
                                            }
                                            let w = af.count.saturating_mul(af.elem_w).max(1);
                                            let packed_net = self.nets.len() as u32;
                                            let pname = format!(
                                                "__outpack${tname}${}${packed_net}",
                                                p.name.name
                                            );
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
                                            out_binds.push((slot, whole_net_lvalue(packed_net)));
                                            out_array_unpacks.push((path.clone(), pname, af));
                                            continue;
                                        }
                                    }
                                    self.error(
                                        MsgCode::ElabUnsupported,
                                        &format!(
                                            "task `{tname}`: output array formal `{}` \
                                             needs a bare array actual",
                                            p.name.name
                                        ),
                                    );
                                    continue;
                                }
                                Err(reason) => {
                                    self.error(
                                        MsgCode::ElabUnsupported,
                                        &format!(
                                            "task `{tname}`: output array formal `{}` \
                                             unsupported — {reason}",
                                            p.name.name
                                        ),
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                    if matches!(p.dir, ast::PortDir::Inout) {
                        let eid = self.lower_ctx_or_plain(a, fw); // inout reads in too
                        in_binds.push((slot, eid));
                    }
                    match &a.kind {
                        ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                            let net = self.resolve_net(path);
                            // A2a: the copy-out WRITES the actual (selects route
                            // through lower_lvalue below and are covered there).
                            self.deny_const_param_write(net, "connect an output/inout to");
                            out_binds.push((slot, whole_net_lvalue(net)));
                        }
                        // §13.5.3: a part/bit/indexed select or array element actual —
                        // copy out through its lowered lvalue (the inout copy-in above
                        // already read it via lower_ctx_or_plain). A select of a
                        // FRAME-LOCAL (an automatic local) cannot be routed by the
                        // engine's frame copy-out (whole-net only) — loud-reject.
                        ast::ExprKind::PartSelect { .. }
                        | ast::ExprKind::BitSelect { .. }
                        | ast::ExprKind::IndexedPart { .. }
                            if self
                                .actual_root_net(a)
                                .map(|n| self.net_is_frame_local(n))
                                .unwrap_or(false) =>
                        {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "task `{tname}` output/inout arg cannot be a select of an automatic (frame-local) variable"
                                ),
                            );
                        }
                        ast::ExprKind::PartSelect { .. }
                        | ast::ExprKind::BitSelect { .. }
                        | ast::ExprKind::IndexedPart { .. } => match expr_to_lvalue(a) {
                            Some(lv_ast) => {
                                let lv = self.lower_lvalue(&lv_ast);
                                out_binds.push((slot, lv));
                            }
                            None => self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "task `{tname}` output/inout arg must be a simple net or select"
                                ),
                            ),
                        },
                        _ => self.error(
                            MsgCode::ElabUnsupported,
                            &format!("task `{tname}` output/inout arg must be a simple net"),
                        ),
                    }
                }
            }
        }
        let info = TaskCallInfo {
            callee: fid,
            in_binds,
            out_binds,
        };
        let call_block = b.cur_id();
        let ret = b.new_block();
        b.end_block_with(ir::Terminator::Call {
            target: self.funcs[fid as usize].entry,
            ret_bb: ret.raw(),
        });
        b.start_block(ret);
        if self.frame_task_lowering {
            // nested: process-LOCAL block key, rebased to global on append.
            self.pending_task_calls.push((call_block, info));
        } else {
            // top-level: keyed by (process template, process-local block).
            self.task_calls_proc
                .insert((self.cur_proc, call_block), info);
        }
        // §4.5.193: unpack each OUTPUT array formal's packed temp into the caller array
        // elements — `caller[i] = packed[i*ew +: ew]` — emitted in `ret`, which runs
        // AFTER the task's exit (where the out-bind wrote the temp). A bit-faithful copy
        // (element signedness is the CALLER array's, applied on later reads, so no
        // `$signed` wrap here). Built as AST + `lower_stmt` so the array-element write /
        // md-packed part-select read reuse the normal lowering. A NESTED call's writes
        // to a module net are caught by the frame subset check (correct-or-loud).
        // §4.5.204: for a MULTI-DIM formal the LHS is a fully-indexed `caller[i0][i1]…`,
        // decomposing the row-major flat index `i` over `af.dims` (outer→inner) — a partial
        // `caller[i]` on a multi-dim array is a sub-array (loud). The packed temp is flat
        // row-major (`array_formal_ext_dims`), so `packed[i*ew +: ew]` is exactly element
        // `i`; ascending zero-based dims (the only supported multi-dim shape) have declared
        // index == row-major coord, so the decomposed digits index directly. For a 1-D
        // formal `strides == [1]` and the single digit is `i` — byte-identical to §4.5.193.
        for (arr_path, pname, af) in &out_array_unpacks {
            let sp = arr_path.span;
            let dec = |v: u32| ast::Expr {
                kind: ast::ExprKind::IntLit {
                    kind: ast::IntLitKind::Decimal,
                    raw: v.to_string(),
                },
                span: sp,
            };
            let d = af.dims.len();
            let mut strides = vec![1u32; d];
            for k in (0..d.saturating_sub(1)).rev() {
                strides[k] = strides[k + 1].saturating_mul(af.dims[k + 1].1.max(1));
            }
            for i in 0..af.count {
                let packed_read = ast::Expr {
                    kind: ast::ExprKind::IndexedPart {
                        base: Box::new(ast::Expr {
                            kind: ast::ExprKind::Ident(ast::HierPath {
                                segments: vec![ast::Ident {
                                    name: pname.clone(),
                                    span: sp,
                                }],
                                span: sp,
                            }),
                            span: sp,
                        }),
                        offset: Box::new(dec(i * af.elem_w)),
                        width: Box::new(dec(af.elem_w)),
                        dir: ast::PartDir::PlusColon,
                    },
                    span: sp,
                };
                // Fully-indexed caller element for row-major flat position `i`. Each dim's
                // 0-based position maps to the caller's DECLARED index `lo + pos` (§4.5.206;
                // zero-based ⇒ lo=0 ⇒ pos, byte-identical). The read side normalizes `idx-lo`
                // the same way, so position p ↔ declared index lo+p on both sides.
                let mut lhs = ast::Lvalue::Ident(arr_path.clone());
                for (stride, &(lo, size, _)) in strides.iter().zip(af.dims.iter()) {
                    let pos = (i / *stride) % size.max(1);
                    lhs = ast::Lvalue::BitSelect {
                        base: Box::new(lhs),
                        index: Box::new(dec(lo + pos)),
                        span: sp,
                    };
                }
                let stmt = ast::Stmt::Blocking {
                    lhs,
                    delay: None,
                    event: None,
                    rhs: packed_read,
                    span: sp,
                };
                self.lower_stmt(b, &stmt);
            }
        }
    }

    /// R5-B: emit a call to a frame FUNCTION that has output/inout formals as a
    /// `Terminator::Call` (statement context), reusing the task copy-out machinery.
    /// `in_binds` cover input + inout formals; `out_binds` cover output + inout
    /// formals (written back to the caller actual) PLUS the function's return slot,
    /// copied into `ret_lval` — the assignment LHS for a direct `x = f(r)`, or a
    /// hoist temp for a call nested in an expression. Mirrors `emit_frame_task_call`;
    /// the only extra is the return-slot out-bind. The engine runs the body through
    /// `run_task` (which is generic over `out_slots`, so it copies out the return
    /// slot too — see the note there).
    pub(crate) fn emit_frame_func_out_call(
        &mut self,
        b: &mut ProcessBuilder,
        fid: u32,
        func: &ast::FunctionDef,
        args: &[ast::Expr],
        ret_lval: ir::Lvalue,
    ) {
        let fname = func.name.name.clone();
        // R19 §3.3: reorder `.formal(v)` to positional first — the same G10 step the
        // inline and plain-frame paths take. Only THIS path (a frame function WITH an
        // output/inout formal) never did it, so `f(.a(1), .o(x))` reached the loop below
        // with a `NamedArg` node still in place and produced two diagnostics that named
        // neither cause: "a named argument is only valid in a user function/task call"
        // and "output/inout arg must be a simple net".
        let reordered;
        let args: &[ast::Expr] = if args
            .iter()
            .any(|a| matches!(a.kind, ast::ExprKind::NamedArg { .. }))
        {
            match self.resolve_named_args(&fname, &func.ports, args) {
                Some(v) => {
                    reordered = v;
                    &reordered
                }
                None => return,
            }
        } else {
            args
        };
        let Some(eff_args) = self.fill_default_args(&fname, &func.ports, args) else {
            return;
        };
        let (base_net, return_slot) = self
            .func_metas
            .get(fid as usize)
            .map(|m| (m.base_net, m.return_slot))
            .unwrap_or((0, func.ports.len() as u32));
        let mut in_binds: Vec<(u32, u32)> = Vec::new();
        let mut out_binds: Vec<(u32, ir::Lvalue)> = Vec::new();
        for (slot, (p, a)) in func.ports.iter().zip(eff_args.iter().copied()).enumerate() {
            let slot = slot as u32;
            let fw = self
                .nets
                .get((base_net + slot) as usize)
                .map(|n| n.width)
                .unwrap_or(32);
            match p.dir {
                ast::PortDir::Input => {
                    let eid = self.lower_ctx_or_plain(a, fw);
                    in_binds.push((slot, eid));
                }
                ast::PortDir::Output | ast::PortDir::Inout => {
                    // V2B (§4.5.194): an OUTPUT/INOUT DYNAMIC-array formal on a FUNCTION
                    // (driven through run_task, same as a task). The engine deep-copies the
                    // formal's heap array OUT to the caller at exit; INOUT also snapshots the
                    // caller IN (a bare Signal in-bind → frame_dyn_snapshot_formals).
                    if self.is_output_or_inout_dyn_array_formal(p) {
                        match self.dyn_array_actual_net(a, p) {
                            Some(caller_net) => {
                                self.deny_const_param_write(
                                    caller_net,
                                    "connect an output/inout dynamic array to",
                                );
                                if matches!(p.dir, ast::PortDir::Inout) {
                                    let sig = self.push_expr(ir::Expr::Signal {
                                        net: caller_net,
                                        word: None,
                                    });
                                    in_binds.push((slot, sig));
                                }
                                out_binds.push((slot, whole_net_lvalue(caller_net)));
                            }
                            None => self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "function output/inout dynamic-array formal `{}` needs a \
                                     bare matching dynamic-array actual",
                                    p.name.name
                                ),
                            ),
                        }
                        continue;
                    }
                    if matches!(p.dir, ast::PortDir::Inout) {
                        let eid = self.lower_ctx_or_plain(a, fw); // inout reads in too
                        in_binds.push((slot, eid));
                    }
                    match &a.kind {
                        ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                            let net = self.resolve_net(path);
                            self.deny_const_param_write(net, "connect an output/inout to");
                            out_binds.push((slot, whole_net_lvalue(net)));
                        }
                        ast::ExprKind::PartSelect { .. }
                        | ast::ExprKind::BitSelect { .. }
                        | ast::ExprKind::IndexedPart { .. }
                            if self
                                .actual_root_net(a)
                                .map(|n| self.net_is_frame_local(n))
                                .unwrap_or(false) =>
                        {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "frame function `{fname}` output/inout arg cannot be a select of an automatic (frame-local) variable"
                                ),
                            );
                        }
                        ast::ExprKind::PartSelect { .. }
                        | ast::ExprKind::BitSelect { .. }
                        | ast::ExprKind::IndexedPart { .. } => match expr_to_lvalue(a) {
                            Some(lv_ast) => {
                                let lv = self.lower_lvalue(&lv_ast);
                                out_binds.push((slot, lv));
                            }
                            None => self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "frame function `{fname}` output/inout arg must be a simple net or select"
                                ),
                            ),
                        },
                        _ => self.error(
                            MsgCode::ElabUnsupported,
                            &format!("frame function `{fname}` output/inout arg must be a simple net"),
                        ),
                    }
                }
            }
        }
        // Return-capture: the function's return slot is copied out to `ret_lval`.
        out_binds.push((return_slot, ret_lval));
        let info = TaskCallInfo {
            callee: fid,
            in_binds,
            out_binds,
        };
        let call_block = b.cur_id();
        let ret = b.new_block();
        b.end_block_with(ir::Terminator::Call {
            target: self.funcs[fid as usize].entry,
            ret_bb: ret.raw(),
        });
        b.start_block(ret);
        if self.frame_task_lowering {
            self.pending_task_calls.push((call_block, info));
        } else {
            self.task_calls_proc
                .insert((self.cur_proc, call_block), info);
        }
    }
}
