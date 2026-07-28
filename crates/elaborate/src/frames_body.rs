//! frame body lowering — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// Emit a statement that evaluates `call` (for its side effects) and discards
    /// the result. Inside a METHOD body the target is the frame-local `$discard`
    /// slot (the write must land in-frame); in a process body a fresh module net.
    /// R4: lower a `return <expr>` value. `return $sformatf(...)` (a string return,
    /// IEEE §6.16) builds the `Sformatf` SysFunc directly — the SAME IR the direct-rhs
    /// blocking-assign special (`sformatf_special`) produces — so the string return slot
    /// stores a rendered string (the generic `lower_expr` would reject `$sformatf`
    /// outside a blocking-assign rhs). Any other value takes the normal width-context
    /// path, byte-identical to before.
    pub(crate) fn lower_return_rhs(&mut self, val: &ast::Expr, ctx_w: u32) -> u32 {
        if let ast::ExprKind::SysCall { name, args } = &val.kind {
            if name.name == "$sformatf"
                && matches!(
                    args.first().map(|a| &a.kind),
                    Some(ast::ExprKind::StrLit { .. })
                )
            {
                let arg_ids: Vec<u32> = args.iter().map(|a| self.lower_expr(a)).collect();
                return self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::Sformatf,
                    args: arg_ids,
                });
            }
        }
        // §4.5.252: a string `return` renders through `format_args_str` too, so a
        // `$sformatf` inside the returned expression (`return {s, $sformatf(…)}`) may
        // lower as a plain node rather than staying loud.
        let saved_ok = std::mem::replace(&mut self.sformatf_expr_ok, true);
        let id = if ctx_w == 0 {
            self.lower_expr(val)
        } else {
            self.lower_ctx_or_plain(val, ctx_w)
        };
        self.sformatf_expr_ok = saved_ok;
        id
    }

    /// Reserve + lower every frame function of the CURRENT module instance. Runs
    /// at step 6.5: RESERVE all (sorted) so a call to a not-yet-lowered frame func
    /// resolves (breaks self + mutual recursion), then lower each body. No-op when
    /// the module declares no automatic/recursive functions.
    pub(crate) fn lower_frame_funcs(&mut self) {
        let frame_set = self.build_frame_set();
        let task_set = self.build_task_frame_set(); // B2
                                                    // R5-B: record which framed functions have an output/inout formal so their
                                                    // calls route to the copy-out path (`emit_frame_func_out_call`) + the hoist.
        self.inout_func_names.clear();
        // §4.5.179: record which FRAMED functions have an `input` dyn-array formal, so a
        // BURIED call to one is hoisted to a `__t = f(a)` temp (re-triggering §4.5.177's
        // marker). Only `frame_set` members — an R2-inlinable (straight-line) dyn-formal
        // function is inline-aliased, never framed, so it is (correctly) excluded here.
        self.dyn_formal_func_names.clear();
        for name in &frame_set {
            if let Some(f) = self.func_table.get(name) {
                if f.ports
                    .iter()
                    .any(|p| !matches!(p.dir, ast::PortDir::Input))
                {
                    self.inout_func_names.insert(name.clone());
                }
                if f.ports.iter().any(|p| self.is_input_dyn_array_formal(p)) {
                    self.dyn_formal_func_names.insert(name.clone());
                }
            }
        }
        if frame_set.is_empty() && task_set.is_empty() {
            return;
        }
        // RESERVE (sorted name order = deterministic net/FuncId allocation).
        for name in &frame_set {
            let func = self
                .func_table
                .get(name)
                .expect("frame func in table")
                .clone();
            self.reserve_frame_func(name, &func);
        }
        // B2: reserve frame TASKS (after functions; sorted within tasks).
        for name in &task_set {
            let task = self
                .task_table
                .get(name)
                .expect("frame task in table")
                .clone();
            self.reserve_frame_task(name, &task);
        }
        // LOWER each function body (sorted) — every frame_idx is now reserved.
        for name in &frame_set {
            let func = self
                .func_table
                .get(name)
                .expect("frame func in table")
                .clone();
            let fid = self.frame_idx[name];
            self.lower_frame_func_body(name, &func, fid);
        }
        // B2: lower each frame TASK body (sorted) — task_frame_idx all reserved,
        // so self + mutual task recursion resolves.
        for name in &task_set {
            let task = self
                .task_table
                .get(name)
                .expect("frame task in table")
                .clone();
            let fid = self.task_frame_idx[name];
            self.lower_frame_task_body(name, &task, fid);
        }
        self.resolve_frame_task_rejects();
    }

    /// Lower a frame function's body into the GLOBAL `func_blocks` arena: build a
    /// process-local CFG (reusing `lower_stmt`), append it with every `Goto`/
    /// `Branch` target rebased by `+base`, set `FuncDef.entry`, then validate.
    pub(crate) fn lower_frame_func_body(&mut self, name: &str, func: &ast::FunctionDef, fid: u32) {
        // §4.5.250: a FRAME FUNCTION body is validated by `classify_frame_body`, which
        // rejects a whole write to any net outside the frame — and the engine backs that
        // up (exempting a scratch net there panics `frame lvalue net is routed`). So the
        // `$sformatf` hoist, whose temp IS a module net, must not run here: it would
        // replace one honest loud with a different, more confusing one. Frame TASK bodies
        // take a different executor path and are unaffected.
        let saved_ffl = self.frame_fn_lowering;
        self.frame_fn_lowering = true;
        let scope_seg = format!("$func${name}");
        // return-kw: a frame function's `return [expr]` assigns the func-named return
        // var (base_net + return_slot) and jumps to a single exit block; the body also
        // falls through to it. Mirrors class-method lowering — IR-0 (exit BB + Goto/
        // Return are existing shapes, no schema change). Gated on `body_has_return` so
        // a return-free frame function (plain automatic/recursive) keeps its exact
        // prior block structure (byte-identical golden output).
        let has_ret = body_has_return(&func.body);
        let retvar = {
            let m = self.func_metas[fid as usize];
            m.base_net + m.return_slot
        };
        let saved_ret = self.cur_return.take();
        let saved_frame = std::mem::replace(&mut self.in_frame_body, true);
        // v7 P2-C: record `string`-declared formals so a `string` relational compare in
        // the body routes through `StrCmp` (a frame formal is a scoped net, not in
        // `subst`, so `expr_is_string_ast` cannot otherwise see it).
        let fs_base = self.formal_str.len();
        for p in &func.ports {
            let is_str = matches!(
                p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                ast::NetVarKind::String
            );
            self.formal_str.push((p.name.name.clone(), is_str));
        }
        // A FUNCTION cannot be self-disabled (IEEE / iverilog "cannot disable
        // functions") — pass `false` so `disable <funcname>` falls to the loud
        // reject. A `disable <inner named block>` (break/continue) still lowers
        // via `lower_stmt` either way.
        let (body, entry) = self.with_scope(&scope_seg, |s| {
            // Round-9 PKG2: inject same-package constants (enum labels +
            // localparams) BEFORE the body-local labels so a body-local label
            // can still shadow a same-name package const (innermost body decl
            // wins). Gated on the frame key being `pkg::name` — module frame
            // keys are plain identifiers with no `::`, so ordinary frames inject
            // nothing and stay byte-identical.
            let (saved_pkg_p, saved_pkg_m) = match name.split_once("::") {
                Some((pkg, _)) => s.push_pkg_consts_scoped(pkg, func),
                None => (Vec::new(), Vec::new()),
            };
            // Gap B: body-local enum labels → constants under `$func$<name>` (this
            // scope), visible to the body, restored before the scope closes.
            let (saved_labels, saved_meta) = s.push_body_enum_labels(&func.body_enums, &func.body);
            let mut b = ProcessBuilder::new();
            // §13.4.4 / §4.5.189: TOP-LEVEL body_decls run at frame entry (once per
            // activation); NESTED block-local inits run at their OWN block entry (the
            // `Block` arm of `lower_stmt`, gated on `in_frame_body`) so a decl-init
            // inside a LOOP re-initializes each iteration (IEEE automatic lifetime,
            // §6.21). Frame-entry emission was a single-entry-only approximation.
            s.emit_frame_local_inits(&mut b, &func.body_decls);
            if has_ret {
                let exit = b.new_block();
                s.cur_return = Some((Some(retvar), exit));
                s.lower_frame_body_stmt(&mut b, &func.body, &func.name.name, false);
                b.goto(exit);
                b.start_block(exit);
            } else {
                s.lower_frame_body_stmt(&mut b, &func.body, &func.name.name, false);
            }
            let out = b.finish();
            s.restore_params(saved_labels);
            s.restore_param_meta(saved_meta);
            // Round-9 PKG2: unwind the injected package consts (meta first, then
            // params — reverse of injection). Empty for module frames.
            for (k, prev) in saved_pkg_m.into_iter().rev() {
                match prev {
                    Some(m) => {
                        s.param_meta.insert(k, m);
                    }
                    None => {
                        s.param_meta.remove(&k);
                    }
                }
            }
            s.restore_params(saved_pkg_p);
            out
        });
        self.cur_return = saved_ret;
        self.in_frame_body = saved_frame;
        self.formal_str.truncate(fs_base);
        // Capture the block base AFTER the body closure: lowering the body may itself
        // have appended blocks to `func_blocks` (round-7: a package-scoped call inside
        // this body reserves + lowers ITS frame on demand, pushing the callee's blocks
        // first). Capturing before the closure would rebase this function's blocks over
        // the callee's, corrupting the CFG. For an ordinary frame function nothing is
        // pushed during the closure, so `base` is unchanged → byte-identical.
        let base = self.func_blocks.len() as u32;
        for mut blk in body {
            rebase_terminator(&mut blk.term, base);
            self.func_blocks.push(blk);
        }
        self.funcs[fid as usize].entry = base + entry;
        let m = self.func_metas[fid as usize];
        let entry_bb = self.funcs[fid as usize].entry;
        self.validate_frame_body(name, entry_bb, m.base_net, m.locals_len, false);
        self.frame_fn_lowering = saved_ffl;
    }

    /// B2: lower a frame TASK body into the global `func_blocks` arena — like
    /// `lower_frame_func_body`, but a NESTED task call (`inline_task` divert)
    /// registers into `pending_task_calls` (keyed by the process-LOCAL block),
    /// which is rebased by `+base` into `task_calls_func` on append.
    pub(crate) fn lower_frame_task_body(&mut self, name: &str, task: &ast::TaskDef, fid: u32) {
        let scope_seg = format!("$func${name}");
        let saved_ftl = self.frame_task_lowering;
        let saved_pending = std::mem::take(&mut self.pending_task_calls);
        let saved_fork_modes = std::mem::take(&mut self.pending_fork_modes);
        let saved_hier_pending = std::mem::take(&mut self.pending_hier_task_calls);
        self.frame_task_lowering = true;
        // v7 P2-C (mirror of lower_frame_func_body): record `string`-declared task
        // formals so a `string` relational compare in the body routes through
        // `StrCmp` (a frame formal is a scoped net, not in `subst`, so
        // `expr_is_string_ast` cannot otherwise see it).
        let fs_base = self.formal_str.len();
        for p in &task.ports {
            let is_str = matches!(
                p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                ast::NetVarKind::String
            );
            self.formal_str.push((p.name.name.clone(), is_str));
        }
        let self_disable = stmt_disables_name(&task.body, name);
        // return-kw (void): a `return;` in a frame task jumps to a single exit block.
        // Gated on `body_has_return` for byte-identical IR when absent.
        let has_ret = body_has_return(&task.body);
        let saved_ret = self.cur_return.take();
        let saved_frame = std::mem::replace(&mut self.in_frame_body, true);
        let (body, entry) = self.with_scope(&scope_seg, |s| {
            // Gap B: body-local enum labels → constants under `$func$<name>` (this
            // scope), mirroring `lower_frame_func_body` — so a task body enum's
            // labels resolve inside the body without leaking to the module.
            let (saved_labels, saved_meta) = s.push_body_enum_labels(&task.body_enums, &task.body);
            let mut b = ProcessBuilder::new();
            // §13.4.4 / §4.5.189: TOP-LEVEL body_decls run at frame entry (once per
            // activation). NESTED block-local inits run at their OWN block entry (the
            // `Block` arm of `lower_stmt`, gated on `in_frame_body`) so a decl-init
            // inside a LOOP re-initializes each iteration — the IEEE automatic-lifetime
            // rule (§6.21). Emitting them at frame entry was a single-entry-only
            // approximation that silently ran a loop-body init exactly ONCE (an X that
            // never re-inits ⇒ silent-wrong for `for(..) begin int t = f(k); .. end`).
            s.emit_frame_local_inits(&mut b, &task.body_decls);
            if has_ret {
                let exit = b.new_block();
                s.cur_return = Some((None, exit));
                s.lower_frame_body_stmt(&mut b, &task.body, name, self_disable);
                b.goto(exit);
                b.start_block(exit);
            } else {
                s.lower_frame_body_stmt(&mut b, &task.body, name, self_disable);
            }
            let out = b.finish();
            s.restore_params(saved_labels);
            s.restore_param_meta(saved_meta);
            out
        });
        self.cur_return = saved_ret;
        self.in_frame_body = saved_frame;
        self.formal_str.truncate(fs_base);
        let pending = std::mem::replace(&mut self.pending_task_calls, saved_pending);
        let fork_modes_pending = std::mem::replace(&mut self.pending_fork_modes, saved_fork_modes);
        let hier_pending = std::mem::replace(&mut self.pending_hier_task_calls, saved_hier_pending);
        self.frame_task_lowering = saved_ftl;
        // Capture the block base AFTER the body closure (round-7): a `pkg::f()` inside
        // the task body reserves+lowers its frame on demand, appending blocks during the
        // closure; capturing before would rebase this task's blocks (and its nested
        // task-call keys) over the callee's. Unchanged for an ordinary task (nothing
        // appended during the closure) → byte-identical. (Mirrors `lower_frame_func_body`.)
        let base = self.func_blocks.len() as u32;
        for mut blk in body {
            rebase_terminator(&mut blk.term, base);
            self.func_blocks.push(blk);
        }
        self.funcs[fid as usize].entry = base + entry;
        // rebase the nested task-call sites' (process-local) block keys to global.
        for (local_block, info) in pending {
            self.task_calls_func.insert(base + local_block, info);
        }
        // Stage-1 fork-in-frame: rebase this body's fork join-modes (process-local
        // join_bb → global func_blocks id) and key them under the frame sentinel — a
        // task-body fork's mode is process-independent. Empty ⇒ no fork → byte-identical.
        for (local_join_bb, mode) in fork_modes_pending {
            self.fork_modes
                .insert((FRAME_FORK_KEY, base + local_join_bb), mode);
        }
        // §4.5.208: rebase the frame-body hier enables' (process-local) blocks to global
        // `func_blocks` ids, mark this task `has_hier_call` (→ forced suspendable in both
        // `compute_suspendable_tasks` computes), and hand them to the finish-phase resolver.
        if !hier_pending.is_empty() {
            self.func_metas[fid as usize].has_hier_call = true;
            for mut d in hier_pending {
                d.func_block = Some(base + d.call_block);
                d.call_block += base;
                self.deferred_hier_task_calls.push(d);
            }
        }
        let m = self.func_metas[fid as usize];
        // Round-14 V3/V4: DEFER the frame-TASK reject decision to `resolve_frame_task_rejects`
        // (run after every task is lowered) — a leaf non-suspending task is lifted for the
        // engine's suspendable-frame path, so its reject can only be decided once the full
        // transitive suspendable set is known.
        let unsafe_repeat = Self::ast_has_repeat_with_timing(&task.body);
        self.frame_task_pending.push((
            fid,
            name.to_string(),
            m.base_net,
            m.locals_len,
            unsafe_repeat,
        ));
    }

    /// B3: lower a frame function/task body, registering a self-disable target
    /// (`disable <name>` = early return) ONLY when the body contains one — then
    /// the name resolves to a convergence exit block all paths flow into. Without
    /// a self-disable the body lowers exactly as before (byte-identical CFG).
    /// Run a frame function/task's body-local declaration initializers (`int x =
    /// 10;`) at frame ENTRY — they were previously dropped, leaving the local at
    /// its X/0 default (a §13.4.4 silent-wrong). Emitted in declaration order
    /// (use-before-init reads the default, per IEEE). vita's frame locals reset per
    /// call, so per-call initialization is the consistent semantics.
    pub(crate) fn emit_frame_local_inits(
        &mut self,
        b: &mut ProcessBuilder,
        body_decls: &[ast::NetVarDecl],
    ) {
        for d in body_decls {
            for decl in &d.names {
                // T1-7: a FIXED frame-local string array is a heap handle with no length
                // of its own, so it is PRE-SIZED here, at frame entry — the module-scope
                // twin gets its `new[n]` from the t0 var-init flush, which a frame local
                // never reaches. Emitted before the initializer below so a decl with both
                // sizes first. `lowering_decl_init` exempts it from the `new[]`-on-fixed
                // reject the same way the module-scope pre-size is exempted.
                if let Some(n) = self
                    .lookup_net_scoped(&decl.name.name)
                    .and_then(|net| self.fixed_string_dyn_geom(net))
                    .map(|g| g.elem_count())
                {
                    let span = decl.name.span;
                    let pre = ast::Stmt::Blocking {
                        lhs: ast::Lvalue::Ident(ast::HierPath {
                            segments: vec![decl.name.clone()],
                            span,
                        }),
                        delay: None,
                        event: None,
                        rhs: ast::Expr {
                            kind: ast::ExprKind::New {
                                size: Box::new(ast::Expr {
                                    kind: ast::ExprKind::IntLit {
                                        kind: ast::IntLitKind::Decimal,
                                        raw: n.to_string(),
                                    },
                                    span,
                                }),
                                src: None,
                            },
                            span,
                        },
                        span,
                    };
                    let saved = self.lowering_decl_init;
                    self.lowering_decl_init = true;
                    self.lower_stmt(b, &pre);
                    self.lowering_decl_init = saved;
                }
                let Some(init) = &decl.init else { continue };
                let stmt = ast::Stmt::Blocking {
                    lhs: ast::Lvalue::Ident(ast::HierPath {
                        segments: vec![decl.name.clone()],
                        span: decl.name.span,
                    }),
                    delay: None,
                    event: None,
                    rhs: init.clone(),
                    span: decl.span,
                };
                // R16-X1: this IS a declaration initializer, so it carries the same
                // exemption the module-scope t0 var-init flush gives its own
                // (`var_init.rs`). Without it a frame-local `string f[3] = '{…}` was
                // rejected by the `new[]`-on-fixed guard — the `'{…}` expansion emits
                // `f = new[3]` to establish the fixed length, and the guard read that
                // synthesized allocation as a user resize.
                //
                // A user-written `new[…]` initializer is excluded BY MEASUREMENT, not by
                // assumption: exempting it too made `string f[3] = new[5];` inside a task
                // print an empty element at exit 0, where 6b6b8ef rejected it loudly (and
                // iverilog aborts on it). The module-scope declaration gate that catches
                // that shape does not cover the frame path, so the discriminator has to
                // live here. Only an allocation the LOWERING synthesizes — never one the
                // user wrote — earns the exemption.
                let user_alloc = matches!(init.kind, ast::ExprKind::New { .. });
                let saved = self.lowering_decl_init;
                self.lowering_decl_init = !user_alloc;
                self.lower_stmt(b, &stmt);
                self.lowering_decl_init = saved;
            }
        }
    }

    /// r18 (family D): emit ONLY the per-entry automatic-with-init block-locals' initializers
    /// at BLOCK ENTRY, for a MODULE-process block (`compute_per_entry_block_locals` recorded
    /// them under `block_lo` = the block's `span.lo`). A STATIC block-local keeps its
    /// once-at-t0 init (via the synthesized var-init `initial`) and is NOT emitted here — so
    /// only the automatic locals re-initialize on each block entry (§6.21). A block with no
    /// per-entry locals (the common case) is a no-op. The frame path uses
    /// `emit_frame_local_inits` (all locals) instead.
    pub(crate) fn emit_per_entry_block_inits(
        &mut self,
        b: &mut ProcessBuilder,
        body_decls: &[ast::NetVarDecl],
        block_lo: u32,
    ) {
        let Some(names) = self.per_entry_block_locals.get(&block_lo).cloned() else {
            return;
        };
        for d in body_decls {
            for decl in &d.names {
                let Some(init) = &decl.init else { continue };
                if !names.contains(&decl.name.name) {
                    continue;
                }
                // BL2/BL3 (round-19): a DYNAMIC-STORAGE per-entry local re-inits via its
                // decl-init EXPANSION — a raw whole-handle `x = '{…}'` assign is loud ("no
                // whole-value surface"). Resolve the handle; a `'{…}`/`{…}` pattern init
                // becomes `d = new[N]; d[i] = e;` (dyn array — `new[N]` self-resets) or a
                // `q.push_back(e)` sequence (queue — CLEARED first, since push_back appends).
                // A plain SCALAR per-entry local (family D — `dyn_handle` is None) falls
                // through to the raw `x = init` blocking below, unchanged.
                if let Some((_, kind)) = self.dyn_handle(&decl.name.name) {
                    if let Some(elems) = dyn_pattern_elems(init) {
                        if kind == ir::NetKind::Queue {
                            // Reset the queue before re-pushing so a loop / repeated block
                            // entry does not accumulate (a dyn array's `new[N]` needs no clear).
                            let clear = ast::Stmt::UserTaskCall {
                                name: ast::HierPath {
                                    segments: vec![
                                        decl.name.clone(),
                                        ast::Ident {
                                            name: "delete".to_string(),
                                            span: decl.name.span,
                                        },
                                    ],
                                    span: decl.name.span,
                                },
                                args: vec![],
                                span: decl.span,
                            };
                            self.lower_stmt(b, &clear);
                        }
                        let stmts = self.dyn_decl_init_stmts(&decl.name, kind, elems);
                        for st in &stmts {
                            self.lower_stmt(b, st);
                        }
                        continue;
                    }
                }
                let stmt = ast::Stmt::Blocking {
                    lhs: ast::Lvalue::Ident(ast::HierPath {
                        segments: vec![decl.name.clone()],
                        span: decl.name.span,
                    }),
                    delay: None,
                    event: None,
                    rhs: init.clone(),
                    span: decl.span,
                };
                self.lower_stmt(b, &stmt);
            }
        }
    }

    pub(crate) fn lower_frame_body_stmt(
        &mut self,
        b: &mut ProcessBuilder,
        body: &ast::Stmt,
        name: &str,
        self_disable: bool,
    ) {
        if !self_disable {
            self.lower_stmt(b, body);
            return;
        }
        // A `disable <name>` (and every normal exit) jumps to this block, which
        // Returns — the single-frame unwind. `disable_fork_floor` is already 0 at
        // step 6.5 (no fork/process context), so the name is visible to the lookup.
        let exit = b.new_block();
        self.disable_stack.push((name.to_string(), exit));
        self.lower_stmt(b, body);
        b.goto(exit);
        b.start_block(exit);
        self.disable_stack.pop();
    }
}
