//! inline task expansion — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// Inline a user-task call into the current process: the body statements join
    /// the caller's CFG via the normal `lower_stmt` machinery (so a task with
    /// if/case/delay just works). INPUT formals substitute a read ExprId; OUTPUT/
    /// INOUT formals bind to the caller's net (reads + writes hit it directly).
    pub(crate) fn inline_task(
        &mut self,
        b: &mut ProcessBuilder,
        name: &ast::HierPath,
        args: &[ast::Expr],
    ) {
        // v5 ⑥: `handle.method(args);` — a 2-segment task enable whose head is
        // a dyn handle is a METHOD statement, not a hierarchical call.
        if name.segments.len() == 2 {
            // N5: covergroup method statement (`c.sample();`).
            if self.cover_inst(&name.segments[0].name).is_some() {
                self.synth_cover_method_stmt(
                    b,
                    &name.segments[0].name,
                    &name.segments[1].name,
                    args,
                );
                return;
            }
            if let Some((net, kind)) = self.dyn_handle(&name.segments[0].name) {
                self.lower_dyn_method_stmt(b, net, kind, &name.segments[1].name, args);
                return;
            }
            // V34-4: the §7.12.2 ORDERING methods on a fixed-size unpacked array.
            // Checked only for those three names, so nothing else changes shape;
            // the pre-slice diagnostic was "unsupported hierarchical task call
            // `a.sort`", which names an instance path that does not exist.
            if matches!(name.segments[1].name.as_str(), "sort" | "rsort" | "reverse") {
                match self.static_array_recv(&name.segments[0].name) {
                    StaticArrayRecv::Integral(net, _) => {
                        self.lower_static_array_order(b, net, &name.segments[1].name, args);
                        return;
                    }
                    StaticArrayRecv::Unsupported(msg) => {
                        self.error(MsgCode::ElabUnsupported, msg);
                        return;
                    }
                    StaticArrayRecv::No => {}
                }
            }
            // v7 P2-C: `s.putc(i, c);`.
            // R6: `string_handle` resolves by SOURCE name, and an inline task's
            // output/inout formal is bound through `out_subst` to a MANGLED
            // formal-local (`__taskarg_<task>_<formal>_<n>`) rather than to a net
            // named `s` — so `s.itoa(42);` inside the body missed here and fell all
            // the way to the hierarchical-enable arm below, reporting the misleading
            // "unsupported hierarchical task call `s.itoa`". It was loud, never
            // silent, but the reason was wrong. The READ half already had this
            // routing (`expr_is_string_ast` consults `out_subst` and `inline_fn`
            // dispatches on the resulting handle), which is why `s.len()` and
            // `s.substr()` worked in the same body that `s.itoa()` refused;
            // this is the WRITE twin of that lookup. Filtered on `is_string_net`, so
            // a non-string out formal still falls through unchanged.
            if let Some(net) = self.string_handle(&name.segments[0].name).or_else(|| {
                self.out_subst_lookup(&name.segments[0].name)
                    .filter(|&n| self.is_string_net(n))
            }) {
                self.lower_string_method_stmt(b, net, &name.segments[1].name, args);
                return;
            }
        }
        if name.segments.len() != 1 {
            // Family D (r18): DEFER a hierarchical TASK enable `u1.tk(x);`. The callee
            // lives in a child instance not yet elaborated at pass 7, so — mirroring the
            // hier FUNCTION call — lower the args in the CALLER scope now, seal this block
            // with a placeholder `Terminator::Call` + a fresh ret block, and record the
            // process key `(cur_proc, call_block)` + instance path.
            // `resolve_deferred_hier_task_call` builds the per-instance `TaskCallInfo`
            // (callee fid + positional in-binds) into `task_calls_proc` (a process enable)
            // OR `task_calls_func` (§4.5.208: a NESTED-in-frame-body enable) after every
            // instance's frame tasks are in `hier_tasks`. A frame-body enable additionally
            // sets `FuncMeta.has_hier_call` so `compute_suspendable_tasks` forces the caller
            // task suspendable consistently (the placeholder `Call.target` is invisible to the
            // pre-resolve elaborate compute). Named args can't be positionally reordered
            // without the callee formals → loud.
            if name.segments.len() >= 2
                && args
                    .iter()
                    .all(|a| !matches!(a.kind, ast::ExprKind::NamedArg { .. }))
            {
                // §4.5.201: lower each arg BOTH as a value (the copy-IN for an input/inout
                // formal) and — when it is an lvalue (a bare var / select) — as a caller
                // lvalue (the copy-OUT target for an output/inout formal). The callee port
                // direction is unknown here, so the resolver picks per port; a non-lvalue arg
                // gets `None` and is loud there if the formal turns out to be output.
                let mut arg_ids = Vec::with_capacity(args.len());
                let mut arg_lvals: Vec<Option<ir::Lvalue>> = Vec::with_capacity(args.len());
                let mut arg_arrays: Vec<Option<u32>> = Vec::with_capacity(args.len());
                for a in args {
                    // §4.5.207: a bare whole-array Ident actual (a static array net) can't be
                    // lowered to a value — resolve its net in the CALLER scope now and pack it
                    // at resolution, once the callee array formal's shape is known. §4.5.210: a
                    // forwarded frame ARRAY FORMAL (`u.tk(a)` inside a frame task/func whose own
                    // formal is `a[]`) is an md-packed frame net (not a static array); accept it
                    // too — its whole net value forwards to the callee slot at resolution. A
                    // scalar / expression actual lowers as before (value + optional caller lvalue).
                    let arr_net = match &a.kind {
                        ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                            self.lookup_net_scoped(&p.segments[0].name).filter(|&n| {
                                self.net_is_static_array(n)
                                    || self.frame_arr_formal_meta.contains_key(&n)
                            })
                        }
                        _ => None,
                    };
                    if let Some(net) = arr_net {
                        arg_arrays.push(Some(net));
                        arg_ids.push(0); // placeholder — the array slot binds at resolution
                        arg_lvals.push(None);
                    } else {
                        arg_arrays.push(None);
                        arg_ids.push(self.lower_expr(a));
                        let lv = expr_to_lvalue(a).map(|lv_ast| self.lower_lvalue(&lv_ast));
                        arg_lvals.push(lv);
                    }
                }
                let call_block = b.cur_id();
                let ret = b.new_block();
                b.end_block_with(ir::Terminator::Call {
                    target: ret.raw(), // placeholder — patched to the callee entry at resolve
                    ret_bb: ret.raw(),
                });
                b.start_block(ret);
                let call = DeferredHierTaskCall {
                    // R16 §4-1: anchor the resolve-time diagnostics at the enable itself.
                    span: self.cur_span,
                    proc: self.cur_proc,
                    call_block,
                    // §4.5.208: a frame-body enable's block is process-LOCAL now; it is
                    // rebased to a global `func_blocks` id at `lower_frame_task_body` finish.
                    func_block: if self.frame_task_lowering {
                        Some(call_block)
                    } else {
                        None
                    },
                    prefix: self.cur_prefix.clone(),
                    path: name.segments.iter().map(|s| s.name.clone()).collect(),
                    arg_ids,
                    arg_lvals,
                    arg_arrays,
                };
                if self.frame_task_lowering {
                    // Collected per-body; rebased + moved to `deferred_hier_task_calls` at the
                    // body's finish (which also sets `FuncMeta.has_hier_call`).
                    self.pending_hier_task_calls.push(call);
                } else {
                    self.deferred_hier_task_calls.push(call);
                }
                return;
            }
            self.error(
                MsgCode::ElabUnsupported,
                "hierarchical task call (deferred)",
            );
            return;
        }
        let bare = name.segments[0].name.clone();
        // Inside a package routine's body a bare callee names that package's own
        // sibling — the FUNCTION half of this was wired and the task half was not, so
        // a package task calling a same-package task reported "undeclared task", and
        // with a module-local task of the same name it called THAT one silently.
        let tname = self.resolve_rtn_key(&bare);
        let task = match self.task_table.get(tname.as_str()) {
            Some(t) => t.clone(),
            None => {
                self.error(
                    MsgCode::ElabUnresolvedName,
                    &format!("call to undeclared task `{bare}`"),
                );
                return;
            }
        };
        // §13.5.4: reorder named arguments (`.formal(v)` / `.formal()`) to positional
        // using the callee's formals, exactly as the FUNCTION path does. Without this a
        // `.formal(v)` actual fell through to `lower_expr`, whose `NamedArg` arm reports
        // "only valid in a user function / task call" — while standing inside one.
        let reordered_args;
        let args: &[ast::Expr] = if args
            .iter()
            .any(|a| matches!(a.kind, ast::ExprKind::NamedArg { .. }))
        {
            match self.resolve_named_args(&tname, &task.ports, args) {
                Some(v) => {
                    reordered_args = v;
                    &reordered_args
                }
                None => return,
            }
        } else {
            args
        };
        // §13.3 UARR: an unpacked-array TASK formal is outside the supported slice
        // (the md-packed representation targets FUNCTION input formals; a task's
        // output/inout array formal is pass-by-reference, not covered). Loud-reject
        // here — before frame/inline dispatch — so it can never silently mis-lower
        // (a whole-array actual would otherwise hit the incidental whole-array guard,
        // but a clear message is better; correct-or-loud).
        // V2A: an `input` DYNAMIC-array formal (`byte b[]`) is EXEMPT — the inline
        // (static-task) path below aliases the caller's `DynArray` handle via
        // `dyn_subst` (read-only), exactly like the R11 function inline path. Every
        // OTHER unpacked-array formal (fixed `[0:N]`, output/inout dyn-array) stays loud.
        // V2A / §13.3 UARR: an `input` DYNAMIC-array formal (`byte b[]`) OR an
        // `input` unpacked-FIXED array formal (`byte b[4]`, §4.5.188) is SUPPORTED
        // — the former aliases/snapshots the caller handle (inline OR frame), the
        // latter is an md-packed value slot on the FRAME path (mirrors the FUNCTION
        // path). The unpacked-fixed exemption is gated on the task being FRAMED
        // (`task_frame_idx`): the INLINE (static-task) binding path has no md-packed
        // slot, so a static unpacked-fixed-formal task stays loud rather than
        // silently truncating the packed actual into a scalar local. Every OTHER
        // unpacked formal (an OUTPUT/INOUT array — pass-by-reference — or a fixed
        // array the classifier rejects) stays loud (correct-or-loud).
        let is_framed = self.task_frame_idx.contains_key(tname.as_str());
        if task.ports.iter().any(|p| {
            // A scalar/vector formal, or a supported `input` DYNAMIC-array formal
            // (aliased/snapshotted on either path), is fine.
            if p.unpacked.is_empty() || self.is_input_dyn_array_formal(p) {
                return false;
            }
            // V2B (§4.5.194): a FRAMED task's OUTPUT/INOUT dyn-array formal is supported —
            // reserved as a DynArray net, deep-copied OUT to the caller at exit (INOUT also
            // copied IN). An inline (non-framed) task has no frame to copy from → loud below.
            if is_framed && self.is_output_or_inout_dyn_array_formal(p) {
                return false;
            }
            // Any OTHER unpacked-array formal is loud UNLESS it is a FRAMED task's
            // `input`/`output`/`inout` unpacked-FIXED array (an md-packed value slot;
            // inout = §4.5.188 copy-in + §4.5.193 copy-out, r17).
            !(is_framed
                && matches!(
                    p.dir,
                    ast::PortDir::Input | ast::PortDir::Output | ast::PortDir::Inout
                )
                && matches!(self.classify_array_formal(p), Some(Ok(_))))
        }) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "task `{tname}` has an unpacked-array formal — unsupported (an \
                     OUTPUT/INOUT array formal is pass-by-reference; pass a packed \
                     vector, or use an `input` array formal / a function)"
                ),
            );
            return;
        }
        // B2 frame-call: a recursive/automatic task is LOWERED to the func arena
        // (reserved in step 6.5) — emit a Terminator::Call + register the binding.
        if let Some(&fid) = self.task_frame_idx.get(tname.as_str()) {
            // V2A-frame (§4.5.173): an input dyn-array formal is now reserved as a
            // per-activation `DynArray` heap slot (`reserve_frame_task`) and DEEP-COPIED
            // from the caller at frame entry (`emit_frame_task_call` → engine snapshot).
            // A dyn-formal task that would take the SYNCHRONOUS (`run_task_call`) path
            // instead of the suspendable-frame path can't be snapshotted (the sync
            // executor is `&self`, can't populate the heap) — that case stays loud in the
            // post-pass (`resolve_frame_task_rejects`). Output/inout dyn-array formals are
            // already loud (the unpacked-array formal guard above).
            self.emit_frame_task_call(b, fid, &task, args);
            return;
        }
        if self.inline_stack.iter().any(|n| n == &tname) {
            // A recursive task not framed (build_task_frame_set missed the cycle):
            // the inline guard still catches it loud rather than looping forever.
            self.error(
                MsgCode::ElabUnsupported,
                &format!("recursive task `{tname}` (frame-call deferred)"),
            );
            return;
        }
        // v7: the INLINE (static-lifetime) task path binds each input formal via a
        // formal-WIDTH local net + copy-in assign. A `string` formal lowers to a
        // 1-bit `Wire` net, so that copy-in TRUNCATES the actual (and no `formal_str`
        // routing happens), giving a silent-wrong compare. The frame path
        // (`automatic`/recursive tasks) handles string formals correctly; a proper
        // inline fix needs a String-kind snapshot local (the local exists to prevent
        // input/output aliasing — direct subst would reintroduce that), so until then
        // loud-reject rather than silently truncate (correct-or-loud). Declaring the
        // task `automatic` diverts to the working frame path.
        // v7 (NARROWED 2026-08-18): an INPUT `string` formal now works — the
        // formal-local below is allocated as a real `NetKind::String` slot, so the
        // copy-in stores a heap string instead of truncating it to one bit.
        // R6 (NARROWED 2026-08-26): OUTPUT/INOUT works too, and the whole-port gate
        // that used to sit here is GONE. Its stated reason — "the copy-out resolver
        // takes a simple net and a `string` actual is not one" — was refuted by
        // measurement: every piece the copy-out needs is already in the Output|Inout
        // arm below (`out_lval` from the bare-Ident case, the inout copy-IN, the
        // `out_subst` binding, and the exit `BlockingAssign`), and the formal-local
        // has been a real `NetKind::String` slot since the INPUT narrowing above, so
        // both ends of that assign are string handles. What actually blocked it was
        // the `array_len != 1` rejection in that arm: a scalar `string` net is
        // recorded with `array_len: 0` (netdecl.rs — a string has no packed extent to
        // record), so a perfectly ordinary `string a;` actual read as "an unpacked
        // array" and was rejected by a check aimed at whole-array actuals.
        //
        // The decision therefore moved to where the ACTUAL is known, because the
        // formal alone cannot decide it: `t_out(a)` with `string a` is expressible,
        // `t_out(a[3:0])` and `t_out(w)` with `logic [31:0] w` are not, and the old
        // gate refused all three with the same sentence. See the three narrowed
        // rejections in the Output|Inout arm below for each surviving reason.
        let Some(eff_args) = self.fill_default_args(tname.as_str(), &task.ports, args) else {
            return;
        };

        // Bind formals via formal-WIDTH local nets for OUTPUT/INOUT (IEEE 1800
        // §13.5.1 / §13.5.3 copy-in/copy-out). The old direct aliasing (formal ==
        // caller net) discarded the formal's declared width/sign, leaked every
        // intermediate write to the caller (visible to concurrent logic + the VCD),
        // corrupted a caller net passed as BOTH an input and an output, lost the
        // static-storage value across calls, and never copied out a default for an
        // unassigned output — a cluster of silent-wrongs. Each output/inout formal
        // now gets a real local of the formal's declared type (per-call-site storage
        // that, for a STATIC task, persists across calls at this site). INOUT copies
        // the caller value IN at entry (truncated to the formal width); OUTPUT starts
        // at the type default (X for 4-state, 0 for 2-state — the net's `init`); a
        // SINGLE copy-out at exit resizes the final value (sign/zero-extended per the
        // formal's signedness) onto the caller net. INPUTs keep the caller-scope
        // substitution. No output/inout formals ⇒ byte-identical to before.
        let subst_base = self.subst.len();
        let out_base = self.out_subst.len();
        // §13.4.1: a non-automatic task's formals are a SINGLE static instance. The
        // inline path only ever handles static tasks (automatic/recursive diverted to
        // the frame path above), so allocate ONE formal-local net per formal at the
        // FIRST call site and reuse it at every later site (keyed by task name) — the
        // value then persists across calls (e.g. an unwritten output retains its prior
        // value, an inout accumulates). The net `init` (X for 4-state, 0 for 2-state)
        // is the first-call default. 2-state locals are registered for X/Z→0 coercion.
        let locals: Vec<u32> = match self.task_arg_locals.get(&tname) {
            Some(c) => c.clone(),
            None => {
                let mut v = Vec::with_capacity(task.ports.len());
                for p in &task.ports {
                    let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
                    let (w, msb, lsb, signed) =
                        self.range_to_dims(kind, p.range.as_ref(), p.signed);
                    let local = self.nets.len() as u32;
                    let lname = format!("__taskarg_{}_{}_{}", tname, p.name.name, local);
                    self.add_net(
                        &lname,
                        ir::NetVar {
                            // `frame_local_net_kind`, not `map_net_kind_or_wire`: a
                            // `string` formal needs a real heap-backed `NetKind::String`
                            // slot here. The FRAME path can leave it a 1-bit Wire because
                            // it has a `FuncMeta` and the engine reads the `str_params`
                            // mask to know the slot holds a handle — an inlined task has
                            // no `FuncMeta`, so nothing would carry that fact and the
                            // copy-in truncated the actual to one bit. Which is why this
                            // used to loud-reject and tell the reader to write
                            // `automatic`. Every non-string kind maps identically.
                            kind: frame_local_net_kind(kind),
                            width: w,
                            msb,
                            lsb,
                            signed,
                            array_len: 1,
                            dir: ir::PortDir::Internal,
                            init: default_init(kind, w),
                        },
                    );
                    if net_kind_is_two_state(kind) {
                        self.intro_kind.insert(local, kind);
                    }
                    v.push(local);
                }
                self.task_arg_locals.insert(tname.clone(), v.clone());
                v
            }
        };
        // (caller lvalue, local_net), in arg order — copied out at task exit.
        let mut copy_out: Vec<(ir::Lvalue, u32)> = Vec::new();
        // V2A: (formal name → caller DynArray NetId) read-only aliases, pushed onto
        // `dyn_subst` around the body lowering below (mirrors the function inline path).
        let mut dyn_binds: Vec<(String, u32)> = Vec::new();
        for (i, (p, a)) in task.ports.iter().zip(eff_args.iter().copied()).enumerate() {
            let local = locals[i];
            match p.dir {
                ast::PortDir::Input => {
                    // V2A: an `input` dyn-array formal is pass-by-VALUE (IEEE §13.5.1).
                    // A task body has full STATEMENTS, so it (or a callee) can mutate the
                    // underlying array WHILE reading `b` — a bare read-only alias to the
                    // caller's handle would then leak the mutation into `b` (silent-wrong).
                    // So SNAPSHOT: allocate a fresh DynArray temp, deep-copy the caller's
                    // array into it at entry (`handle_copy_stmts`), and alias `b` to the
                    // COPY via `dyn_subst`. (The R11 FUNCTION path needs no snapshot — it
                    // loud-rejects statement bodies, so a function can't mutate mid-body,
                    // making its direct alias safe.) A non-bare / mismatched actual is loud.
                    if self.is_input_dyn_array_formal(p) {
                        match self.dyn_array_actual_net(a, p) {
                            Some(caller_net) => {
                                let snap = self.alloc_dyn_snapshot(caller_net);
                                let sid = self.push_stmt(ir::Stmt::SysTask {
                                    which: ir::SysTaskId::Display,
                                    fmt: None,
                                    args: Vec::new(),
                                });
                                self.handle_copy_stmts.insert(sid, (snap, caller_net));
                                b.push_stmt_id(sid);
                                dyn_binds.push((p.name.name.clone(), snap));
                            }
                            None => self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "task `{tname}`: dynamic-array formal `{}` needs a bare \
                                     matching dynamic-array actual",
                                    p.name.name
                                ),
                            ),
                        }
                        continue;
                    }
                    // Copy the actual IN to the formal-width local (a SNAPSHOT,
                    // truncated to the formal width); bind the formal to a read of that
                    // local — so body reads see the formal width (§13.5.3), not the
                    // actual's, and a later change to the actual does not leak in.
                    // §11.6: the actual is in the formal's width context, so a fill
                    // grows to it (non-fill ⇒ byte-identical via lower_expr).
                    let fw = self.nets.get(local as usize).map(|n| n.width).unwrap_or(32);
                    let actual_eid = self.lower_ctx_or_plain(a, fw); // caller-scope read (pre-bind)
                    let cin = self.push_stmt(ir::Stmt::BlockingAssign {
                        lhs: whole_net_lvalue(local),
                        rhs: actual_eid,
                    });
                    b.push_stmt_id(cin);
                    let rd = self.push_expr(ir::Expr::Signal {
                        net: local,
                        word: None,
                    });
                    self.subst.push((p.name.name.clone(), rd));
                }
                ast::PortDir::Output | ast::PortDir::Inout => {
                    // Resolve the copy-OUT target lvalue in CALLER scope, BEFORE the
                    // formal name is bound, so its base resolves the caller net (not
                    // the just-bound local). An `inout` also copies the actual's value
                    // IN at entry. The actual may be: a simple net (incl. a nested
                    // outer formal routed via out_subst), or a part/bit/indexed select
                    // or array element (§13.5.3 — any variable lvalue).
                    let is_inout = matches!(p.dir, ast::PortDir::Inout);
                    // R6: is the FORMAL declared `string`? The copy-in/copy-out pair
                    // moves a heap HANDLE for such a formal and a packed VALUE for
                    // every other one, so the two domains must agree end to end —
                    // which only this arm can check, because the actual lives here.
                    let formal_is_string = matches!(p.net_or_var, Some(ast::NetVarKind::String));
                    let out_lval: ir::Lvalue = match &a.kind {
                        ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                            let caller_net = self
                                .out_subst_lookup(&path.segments[0].name)
                                .unwrap_or_else(|| self.resolve_net(path));
                            // R6: the two domains must match. A `string` formal bound to
                            // a packed net (or the reverse) would copy a heap handle into
                            // a bit vector — iverilog renders `"made"` as its 32-bit code
                            // 1835099237 and verilator drops the write entirely (measured
                            // 2026-08-26 on `task t(output string s); s="made";` called
                            // with a `logic [31:0]` actual), so the two oracles SPLIT and
                            // this stays loud on both sides of the mismatch.
                            let caller_is_string = self.is_string_net(caller_net);
                            if formal_is_string != caller_is_string {
                                let (fk, ak) = if formal_is_string {
                                    ("`string`", "a packed net")
                                } else {
                                    ("packed", "a `string` net")
                                };
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "task `{tname}`: {fk} {} formal `{}` is bound to \
                                         {ak} (`{}`) — a string and a packed vector are \
                                         different domains here, and the two oracles \
                                         disagree about what such a copy-out should write",
                                        tf_dir_word(p),
                                        p.name.name,
                                        path.segments[0].name
                                    ),
                                );
                                // Bind the formal anyway so the body's own reads/writes
                                // resolve to the local: without it every mention of the
                                // formal raises a second, misleading E3010 "undeclared
                                // net" (measured — the packed-formal/string-actual cell
                                // printed exactly that cascade before this slice).
                                self.out_subst.push((p.name.name.clone(), local));
                                continue;
                            }
                            // A whole unpacked array can't bind to a scalar formal —
                            // reject rather than silently copy out word 0.
                            // R6: a scalar `string` net carries `array_len: 0` (netdecl
                            // records no packed extent for one), so it read as an array
                            // to this check — that, not any missing copy-out machinery,
                            // is what forced the old whole-port `string` gate. The
                            // domains are already proven equal above, so exempt the
                            // matched-string pair and leave every other actual's test
                            // byte-identical.
                            if !caller_is_string
                                && self
                                    .nets
                                    .get(caller_net as usize)
                                    .map(|n| n.array_len != 1)
                                    .unwrap_or(false)
                            {
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "task `{tname}` output/inout arg must be a simple net (v1)"
                                    ),
                                );
                                self.out_subst.push((p.name.name.clone(), local));
                                continue;
                            }
                            // A2a: the copy-out WRITES the actual.
                            self.deny_readonly_write(caller_net, "connect an output/inout to");
                            if is_inout {
                                let rd = self.push_expr(ir::Expr::Signal {
                                    net: caller_net,
                                    word: None,
                                });
                                let cin = self.push_stmt(ir::Stmt::BlockingAssign {
                                    lhs: whole_net_lvalue(local),
                                    rhs: rd,
                                });
                                b.push_stmt_id(cin);
                            }
                            whole_net_lvalue(caller_net)
                        }
                        ast::ExprKind::PartSelect { .. }
                        | ast::ExprKind::BitSelect { .. }
                        | ast::ExprKind::IndexedPart { .. } => {
                            // R6: a `string` formal's copy-in/copy-out moves a whole heap
                            // handle, which no select can name. On a scalar `string` net
                            // that is `is_non_bit_addressable_target` by construction (a
                            // string has no bit-addressable storage — iverilog agrees, it
                            // rejects `t_out(a[3:0])` with "Cannot part select assign to a
                            // string" and traps at run time on `t_out(a[1])`). On a fixed
                            // string ARRAY a const-index element `t_out(names[0])` IS a
                            // whole string net and both oracles run it — but the copy-IN
                            // on this arm reads the actual as a packed value of the
                            // formal's width (0 for a string), so accepting it would need
                            // its own handle-read path. Measured and left loud rather than
                            // half-built; the message names which of the two it is.
                            if formal_is_string {
                                let scalar_str = self
                                    .actual_root_net(a)
                                    .is_some_and(|n| self.is_non_bit_addressable_target(n));
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "task `{tname}`: `string` {} formal `{}` cannot be \
                                         bound to a select — {}",
                                        tf_dir_word(p),
                                        p.name.name,
                                        if scalar_str {
                                            "a `string` variable is a byte sequence with no \
                                             bit-addressable storage, so a partial copy-out \
                                             into one has no representation (iverilog \
                                             rejects the same code)"
                                        } else {
                                            "only a bare `string` variable can receive the \
                                             whole handle a `string` formal copies out"
                                        }
                                    ),
                                );
                                self.out_subst.push((p.name.name.clone(), local));
                                continue;
                            }
                            // A select of a FRAME-LOCAL (automatic local) cannot be a
                            // copy-out target the engine can route — loud-reject.
                            if self
                                .actual_root_net(a)
                                .map(|n| self.net_is_frame_local(n))
                                .unwrap_or(false)
                            {
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "task `{tname}` output/inout arg cannot be a select of an automatic (frame-local) variable"
                                    ),
                                );
                                continue;
                            }
                            let Some(lv_ast) = expr_to_lvalue(a) else {
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "task `{tname}` output/inout arg must be a simple net or select (v1)"
                                    ),
                                );
                                continue;
                            };
                            if is_inout {
                                // Read the actual select IN (formal-width context).
                                let fw =
                                    self.nets.get(local as usize).map(|n| n.width).unwrap_or(32);
                                let rd = self.lower_ctx_or_plain(a, fw);
                                let cin = self.push_stmt(ir::Stmt::BlockingAssign {
                                    lhs: whole_net_lvalue(local),
                                    rhs: rd,
                                });
                                b.push_stmt_id(cin);
                            }
                            self.lower_lvalue(&lv_ast)
                        }
                        _ => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "task `{tname}` output/inout arg must be a simple net or select (v1)"
                                ),
                            );
                            // R6: bind the formal so the body's own mentions of it do not
                            // raise a second, misleading E3010 on top of this one.
                            self.out_subst.push((p.name.name.clone(), local));
                            continue;
                        }
                    };
                    self.out_subst.push((p.name.name.clone(), local));
                    copy_out.push((out_lval, local));
                }
            }
        }

        // INLINE the body into the current process via normal stmt lowering. A
        // `return;` in the task jumps to a continuation block scoped to THIS task
        // (cur_return saved/restored so a nested return never escapes to the caller's
        // function exit). Gated on `body_has_return` for byte-identical IR when absent.
        //
        // §13.4.1: a static (non-automatic) task's BODY-LOCAL declarations
        // (top-level `body_decls` and any `begin … end` block-locals) have STATIC
        // storage — ONE instance per task, RETAINED across calls. They are hoisted
        // into nets under a scope keyed by the TASK NAME (`$itask$<name>$L`),
        // SHARED by every call site, and their declaration initializers run ONCE
        // (at the first call), NOT on every entry — so `task t; int c=0; c=c+1;`
        // called 3× prints 1,2,3, not 1,1,1 (an inline task is always static;
        // automatic/recursive tasks divert to the per-call frame path). A task with
        // NO body-locals takes the exact prior path (no scope, byte-identical IR).
        // V2A: activate the input dyn-array aliases for the body's read paths
        // (`b[i]`/`b.size()` resolve `b` to the caller net via `dyn_subst_lookup`),
        // popped after the body below. Empty for a task with no dyn-array formal
        // (byte-identical).
        let n_dyn = dyn_binds.len();
        self.dyn_subst.extend(dyn_binds);
        let mut tlocals = task.body_decls.clone();
        collect_block_local_decls(&task.body, &mut tlocals);
        self.inline_stack.push(tname.clone());
        // Gap B: body-local enum labels → constants under the CALLER prefix, bounded
        // to this inlining. The `$itask$<name>$L` locals scope (when present) is
        // transparent in `walk_scopes_key`, so a label at `caller.LABEL` is still
        // found from inside it; restored after so it does not leak past the call.
        let (saved_labels, saved_meta) = self.push_body_enum_labels(&task.body_enums, &task.body);
        // §4.5.426: `%m` inside a task body is `module.task` (IEEE §21.2.1), not the
        // caller's block chain — the inlined body lowers under `[task]`.
        let saved_scope = std::mem::replace(&mut self.block_scope, vec![tname.to_string()]);
        // §4.5.435: rooted at the DECLARING instance, not the caller's generate scope.
        let decl_root = self.display_of(&self.inst_prefix);
        let saved_root = std::mem::replace(&mut self.block_scope_root, Some(decl_root));
        if tlocals.is_empty() {
            self.inline_task_body(b, &task.body);
        } else {
            let scope = format!("$itask${tname}$L");
            self.with_scope(&scope, |s| {
                s.hoist_inline_task_locals(b, &tlocals);
                s.inline_task_body(b, &task.body);
            });
        }
        self.block_scope = saved_scope;
        self.block_scope_root = saved_root;
        self.restore_params(saved_labels);
        self.restore_param_meta(saved_meta);
        self.inline_stack.pop();

        // Copy-OUT each output/inout formal to its caller net AFTER the body. A
        // single write per formal (no intermediate caller-net glitch); the
        // assignment resizes (sign/zero-extends per the formal's signedness) from
        // the formal width to the caller width. For a `return`-bearing body this
        // sits in the convergence exit block, so it runs on every return path.
        for (caller_lval, local) in &copy_out {
            let rd = self.push_expr(ir::Expr::Signal {
                net: *local,
                word: None,
            });
            let cout = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: caller_lval.clone(),
                rhs: rd,
            });
            b.push_stmt_id(cout);
        }

        // pop our frames so sibling/outer code is unaffected.
        self.subst.truncate(subst_base);
        self.out_subst.truncate(out_base);
        self.dyn_subst.truncate(self.dyn_subst.len() - n_dyn); // V2A: pop dyn-array aliases
    }

    /// Lower an inline-task body with the `return`-exit-block gating (a `return;`
    /// jumps to a fresh convergence block scoped to THIS task; cur_return is
    /// saved/restored so a nested return never escapes to the caller's exit).
    pub(crate) fn inline_task_body(&mut self, b: &mut ProcessBuilder, body: &ast::Stmt) {
        if body_has_return(body) {
            let saved_ret = self.cur_return.take();
            let exit = b.new_block();
            self.cur_return = Some((None, exit));
            self.lower_stmt(b, body);
            b.goto(exit);
            b.start_block(exit);
            self.cur_return = saved_ret;
        } else {
            self.lower_stmt(b, body);
        }
    }

    /// Reserve an inline-task's body-local declarations as nets under the current
    /// (unique per-call) scope and run their declaration initializers at entry. A
    /// name already bound in this scope is skipped (a block-local shadowing a
    /// formal is illegal SV; this guards it harmlessly). 2-state locals register
    /// for X/Z→0 coercion, mirroring the formal-local path.
    pub(crate) fn hoist_inline_task_locals(
        &mut self,
        b: &mut ProcessBuilder,
        decls: &[ast::NetVarDecl],
    ) {
        // The scope is shared per-task (static retention), so a SECOND call finds
        // every local already bound: it allocates nothing and re-runs no
        // initializer. `first_call` is true only when this call allocated a local
        // — i.e. the FIRST inline of this task — gating the one-time init below.
        let mut first_call = false;
        for d in decls {
            for decl in &d.names {
                let key = self.fq(&decl.name.name);
                if self.symbols.contains_key(&key) {
                    continue;
                }
                // An UNPACKED-ARRAY body-local (`int arr [0:1];`) gets real element
                // storage, mirroring a module-level array (`array_len` + the
                // addressing sidecars below) — a static-lifetime local for a
                // non-automatic task, so it persists across calls.
                let dim_extents = self.array_dim_extents(&decl.unpacked);
                let array_len = dim_extents
                    .iter()
                    .fold(1u32, |acc, &(_, n)| acc.saturating_mul(n.max(1)));
                if (array_len as u64) > MAX_ARRAY_LEN {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "unpacked-array local `{}` has {} elements (cap {MAX_ARRAY_LEN})",
                            decl.name.name, array_len
                        ),
                    );
                    continue;
                }
                first_call = true;
                // §7.4.2 / §4.5.359: the STATIC task's body-local, the twin of the
                // framed one in `frames_reserve`. Opt-in and record are one unit.
                let odd_bound = self.declared_odd_bound(d.range.as_ref()).is_some();
                let (w, msb, lsb, signed) =
                    self.range_to_dims_opt(d.kind, d.range.as_ref(), d.signed, odd_bound);
                let odd_net = self.nets.len() as u32;
                if odd_bound {
                    self.record_declared_bounds_for(odd_net, d.range.as_ref());
                }
                self.add_net(
                    &decl.name.name,
                    ir::NetVar {
                        // R22: `frame_local_net_kind`, NOT `map_net_kind_or_wire` — a body-local
                        // `string` is WRITTEN by the body, so it needs a heap-backed
                        // `NetKind::String` slot; `map_net_kind_or_wire` has no String arm and
                        // dropped it to `_ => Wire`. This is the THIRD collector for the same
                        // concept (module scope, frame body-locals, inline/static task
                        // body-locals) and it was the one that never got the String arm, so a
                        // `string` local was correct in a `task automatic` and broken in the
                        // otherwise-identical static `task`. Two failure modes came out of the
                        // one Wire: a plain `s = "hi"` was loud (E3018 procedural assignment to
                        // a net), while `$fgets(s, fd)` — whose destination write does not go
                        // through the lvalue check that raises E3018 — was SILENT, returning 0
                        // and leaving `s` untouched at exit 0.
                        kind: frame_local_net_kind(d.kind),
                        width: w,
                        msb,
                        lsb,
                        signed,
                        array_len,
                        dir: ir::PortDir::Internal,
                        init: default_init(d.kind, w),
                    },
                );
                let Some(&id) = self.symbols.get(&key) else {
                    continue;
                };
                if net_kind_is_two_state(d.kind) {
                    self.intro_kind.insert(id, d.kind);
                }
                // Element-addressing sidecars (only for an actual unpacked array),
                // mirroring the module-level decl path so `arr[i]` resolves.
                if !decl.unpacked.is_empty() {
                    if dim_extents.len() >= 2 || dim_extents.iter().any(|&(lo, _)| lo != 0) {
                        self.array_dims.insert(id, dim_extents);
                    }
                    let desc: Vec<bool> = decl
                        .unpacked
                        .iter()
                        .map(|dm| match dm {
                            ast::Dim::Range(r) => {
                                let m = self.const_range_bound_fold(&r.msb);
                                let l = self.const_range_bound_fold(&r.lsb);
                                matches!((m, l), (Some(m), Some(l)) if m > l)
                            }
                            _ => false,
                        })
                        .collect();
                    if desc.iter().any(|&x| x) {
                        self.array_dim_desc.insert(id, desc);
                    }
                    self.record_dim_desc(id, d.kind, d.range.as_ref(), &d.packed, &decl.unpacked);
                    self.unpacked_array_nets.insert(id);
                }
            }
        }
        // §13.4.1/§6.21: a static local's initializer runs ONCE (before time 0),
        // not on each call — so emit the inits only at the first call site.
        if first_call {
            self.emit_frame_local_inits(b, decls);
        }
    }
}

/// R6: the direction word to print for a tf-port, as the USER spelled it.
///
/// `ref` and `const ref` both desugar to `PortDir::Inout` in the parser (a
/// copy-in/copy-out approximation of pass-by-reference), so `p.dir` alone made a
/// diagnostic about `task t(ref string s)` say "inout formal" — a word that
/// appears nowhere in the source, sending the reader looking for a direction they
/// never wrote. `TfDirSpelling` carries the original keyword for exactly this.
pub(crate) fn tf_dir_word(p: &ast::TfPort) -> &'static str {
    match (p.dir_spelling, p.dir) {
        (ast::TfDirSpelling::Ref, _) => "ref",
        (ast::TfDirSpelling::ConstRef, _) => "const ref",
        (_, ast::PortDir::Output) => "output",
        (_, ast::PortDir::Inout) => "inout",
        (_, ast::PortDir::Input) => "input",
    }
}
