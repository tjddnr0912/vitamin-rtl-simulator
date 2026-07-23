//! frame call sites — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    pub(crate) fn emit_discarded_call(&mut self, b: &mut ProcessBuilder, call: u32) {
        let tmp = match self.cur_discard {
            Some(d) => d,
            None => {
                let w = self.ir_bits_of(call).unwrap_or(32).max(1);
                self.fresh_ia_tmp(w)
            }
        };
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(tmp),
            rhs: call,
        });
        b.push_stmt_id(sid);
    }

    /// v7 P2-C: is `name` a formal DECLARED `string` in the body being lowered?
    /// Innermost-wins (a shadowing inner non-string formal returns `false`, so an
    /// outer string formal of the same name never leaks in). `false` if not a formal.
    pub(crate) fn formal_is_string(&self, name: &str) -> bool {
        self.formal_str
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, s)| *s)
            .unwrap_or(false)
    }

    /// Inline a user-function call at an expression site → the ExprId of its return
    /// value (SD2 inline path; a 0-time function = zero schema cost). The common
    /// case is a combinational function whose body reduces to the return expression
    /// once the formals are substituted by the actual-arg ExprIds. Returns a
    /// placeholder ExprId on any unsupported shape (after emitting the diagnostic)
    /// so arena edges stay valid.
    /// IEEE §13.5.3: build the effective actual-argument list for a tf call, filling
    /// omitted TRAILING formals with their default values. Returns None (after a loud
    /// diagnostic) on too many actuals, or a missing actual for a formal that has no
    /// default. The default expressions are lowered in the CALLER scope at the call
    /// site, like any other actual (so a constant / module-scope default just works;
    /// a default that references an earlier FORMAL resolves in the caller's scope,
    /// not the formal — out of scope here).
    /// G10 (IEEE §13.5.4): reorder named arguments (`.formal(v)` / `.formal()`) to
    /// positional using the callee's formal list. Leading positional args fill slots
    /// 0..k; each named arg scatters to its formal's index; every unbound slot uses the
    /// formal's default. Loud (correct-or-loud) on: an unknown / duplicated formal, a
    /// positional arg after a named one, a `.formal()` with no default, a default that
    /// references another formal, a missing actual, or too many positionals. Returns the
    /// fully-positional args (owned) so both the frame and inline call paths see a plain
    /// list. Only invoked when at least one arg is a `NamedArg`.
    pub(crate) fn resolve_named_args(
        &mut self,
        fname: &str,
        ports: &[ast::TfPort],
        args: &[ast::Expr],
    ) -> Option<Vec<ast::Expr>> {
        let mut slots: Vec<Option<ast::Expr>> = vec![None; ports.len()];
        let mut seen_named = false;
        let mut pos = 0usize;
        for a in args {
            if let ast::ExprKind::NamedArg { formal, value } = &a.kind {
                seen_named = true;
                let Some(idx) = ports.iter().position(|p| p.name.name == formal.name) else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("call to `{fname}`: no formal named `{}`", formal.name),
                    );
                    return None;
                };
                if slots[idx].is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "call to `{fname}`: formal `{}` is bound more than once",
                            formal.name
                        ),
                    );
                    return None;
                }
                match value {
                    Some(v) => slots[idx] = Some((**v).clone()),
                    None => match &ports[idx].default {
                        Some(def) => slots[idx] = Some(def.clone()),
                        None => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "call to `{fname}`: `.{}()` has no default value",
                                    formal.name
                                ),
                            );
                            return None;
                        }
                    },
                }
            } else {
                if seen_named {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "call to `{fname}`: a positional argument cannot follow a named one"
                        ),
                    );
                    return None;
                }
                if pos >= ports.len() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("call to `{fname}`: too many positional arguments"),
                    );
                    return None;
                }
                slots[pos] = Some(a.clone());
                pos += 1;
            }
        }
        let mut out = Vec::with_capacity(ports.len());
        for (i, slot) in slots.into_iter().enumerate() {
            match slot {
                Some(e) => out.push(e),
                None => match &ports[i].default {
                    Some(def) => {
                        // Same guard as `fill_default_args`: a default referencing another
                        // formal would wrongly bind to a caller variable (silent-wrong).
                        if ports.iter().any(|q| expr_reads_ident(def, &q.name.name)) {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "function/task `{fname}`: a default argument value that references another formal is unsupported"
                                ),
                            );
                            return None;
                        }
                        out.push(def.clone());
                    }
                    None => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "call to `{fname}`: missing actual for formal `{}` (no default value)",
                                ports[i].name.name
                            ),
                        );
                        return None;
                    }
                },
            }
        }
        Some(out)
    }

    pub(crate) fn fill_default_args<'a>(
        &mut self,
        fname: &str,
        ports: &'a [ast::TfPort],
        args: &'a [ast::Expr],
    ) -> Option<Vec<&'a ast::Expr>> {
        if args.len() > ports.len() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "function/task `{fname}`: {} args for {} formals",
                    args.len(),
                    ports.len()
                ),
            );
            return None;
        }
        let mut eff: Vec<&'a ast::Expr> = args.iter().collect();
        for p in &ports[args.len()..] {
            match &p.default {
                Some(def) => {
                    // The default is lowered in the CALLER scope; a default that
                    // references another FORMAL (`int b = a + 1`) would wrongly bind to
                    // a same-named caller variable (a silent-wrong vs iverilog, which
                    // resolves it in the subroutine scope). Loud-reject that case.
                    if ports.iter().any(|q| expr_reads_ident(def, &q.name.name)) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "function/task `{fname}`: a default argument value that references another formal is unsupported"
                            ),
                        );
                        return None;
                    }
                    eff.push(def);
                }
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "function/task `{fname}`: missing actual for formal `{}` (no default value)",
                            p.name.name
                        ),
                    );
                    return None;
                }
            }
        }
        Some(eff)
    }

    // ── B1 frame-call: automatic/recursive function lowering ────────────────

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
                &format!("function `{fname}` has an output/inout formal (illegal)"),
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
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "function `{fname}`: a dynamic-array formal `{}` is supported only \
                             as the DIRECT rhs of a blocking assignment at module-process level \
                             (`x = {fname}(arr);`), where the caller array is snapshotted",
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
                                    "frame task `{tname}`: dynamic-array formal `{}` needs a \
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
                                    "frame task `{tname}`: unpacked-array formal `{}` is \
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
                                    "frame task `{tname}`: output/inout dynamic-array formal \
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
                                            "frame task `{tname}`: output array formal `{}` \
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
                                            "frame task `{tname}`: output array formal `{}` \
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
                                    "frame task `{tname}` output/inout arg cannot be a select of an automatic (frame-local) variable"
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
                                    "frame task `{tname}` output/inout arg must be a simple net or select"
                                ),
                            ),
                        },
                        _ => self.error(
                            MsgCode::ElabUnsupported,
                            &format!("frame task `{tname}` output/inout arg must be a simple net"),
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

    /// R5-B: a fresh named temp holding an inout-function call's RETURN value. The
    /// name lets a synthetic `Ident` reference it (module-scoped, like
    /// `fresh_string_temp`); the net id builds the return-capture out-bind lvalue.
    pub(crate) fn fresh_ret_temp(
        &mut self,
        func: &ast::FunctionDef,
        rw: u32,
        rsig: bool,
    ) -> (u32, String) {
        if func.ret_string {
            let name = self.fresh_string_temp();
            ((self.nets.len() - 1) as u32, name)
        } else {
            let w = rw.max(1);
            let name = format!("$ia_ret${}", self.nets.len());
            let net = self.nets.len() as u32;
            self.add_net(
                &name,
                ir::NetVar {
                    kind: if w == 32 && rsig {
                        ir::NetKind::Integer
                    } else {
                        ir::NetKind::Reg
                    },
                    width: w,
                    msb: w.saturating_sub(1),
                    lsb: 0,
                    signed: rsig,
                    array_len: 1,
                    dir: ir::PortDir::Internal,
                    init: default_init(ast::NetVarKind::Reg, w),
                },
            );
            (net, name)
        }
    }

    /// Declared return self-width + signedness of a function (`function [15:0]`,
    /// `function integer`, `function signed [7:0]`, bare `function`).
    pub(crate) fn func_return_dims(&mut self, func: &ast::FunctionDef) -> (u32, bool) {
        let kind = match func.ret_type {
            ast::ParamType::Integer => ast::NetVarKind::Integer,
            ast::ParamType::Real => ast::NetVarKind::Real,
            ast::ParamType::Realtime => ast::NetVarKind::Realtime,
            ast::ParamType::Time => ast::NetVarKind::Time,
            ast::ParamType::Implicit => ast::NetVarKind::Reg,
        };
        let (w, _msb, _lsb, signed) = self.range_to_dims(kind, func.range.as_ref(), func.signed);
        (w, signed)
    }

    /// True if `a` is a bare net Ident or an integer/string literal — i.e. a thing
    /// `lower_expr` can lower without a fatal unresolved-name. A hierarchical /
    /// scope name (`top.dut`) or anything else returns false (dump-family skips it).
    pub(crate) fn is_net_or_const_arg(&self, a: &ast::Expr) -> bool {
        match &a.kind {
            ast::ExprKind::Ident(path) => {
                path.segments.len() == 1
                    && self.symbols.contains_key(&self.fq(&path.segments[0].name))
            }
            ast::ExprKind::IntLit { .. } | ast::ExprKind::StrLit { .. } => true,
            _ => false,
        }
    }
}
