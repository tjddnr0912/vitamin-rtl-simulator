//! lower_stmt — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    // ── the recursive statement-lowering heart ─────────────────────
    /// CONTRACT: on entry `b.cur` is open; on exit `b.cur` is open and is the
    /// "continue point" (where control flows next). Every form upholds this.
    pub(crate) fn lower_stmt(&mut self, b: &mut ProcessBuilder, s: &ast::Stmt) {
        // R5-B: hoist an inout/output-function call out of a once-evaluated expression
        // to a temp (emitting its copy-out `Terminator::Call` first), so the statement
        // below lowers as a plain read of the temp. Gated on `inout_func_names`, so a
        // design with no such function skips this entirely and is byte-identical.
        if !self.inout_func_names.is_empty() || !self.dyn_formal_func_names.is_empty() {
            if let Some(rewritten) = self.hoist_stmt_top(b, s) {
                return self.lower_stmt(b, &rewritten);
            }
        }
        match s {
            // ── STRAIGHT-LINE (stay in the same block) ──────────────
            ast::Stmt::Blocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } => {
                // ⓑ-breadth (§25.9): the `vif = inst;` binding of a virtual
                // interface is resolved to a member alias at net-elaboration time —
                // skip it here (the handle is not a real net to assign).
                if let ast::Lvalue::Ident(p) = lhs {
                    if p.segments.len() == 1
                        && self.vif_handles.contains(&self.fq(&p.segments[0].name))
                    {
                        return;
                    }
                }
                // §6.16.3: a string element write `s[i] = …` is detected FIRST —
                // ahead of the event/delay early-return and the whole RHS-special
                // chain below, every one of which would otherwise route through
                // `lower_lvalue` and emit a silent packed BIT-write into the
                // materialized vector. The plain form routes to the `.putc(i, c)`
                // byte primitive; intra-assignment event/delay control (whose byte
                // write would have to land after the wait) is honest-loud — iverilog
                // supports it, but a clean reject beats the silent wrong path.
                if self.is_string_index_lvalue(lhs) {
                    if event.is_some() || delay.is_some() {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "event/delay control on a string element write `s[i]` \
                             is unsupported (use a plain blocking assignment)",
                        );
                        return;
                    }
                    self.string_index_write_putc(b, lhs, rhs);
                    return;
                }
                // Intra-assignment EVENT control `= [repeat(n)] @(ev) rhs` (IEEE
                // §9.4.5): capture-now / wait / write. Handled FIRST — the rhs is a
                // plain captured value (a special form like `new`/`pop` combined with
                // event control falls through lower_expr to its own loud diagnostic).
                if let Some(ie) = event {
                    self.lower_intra_event_assign(b, lhs, ie, rhs, *span);
                    return;
                }
                // N7: `h = new` / `h = new(args)` — class allocation + ctor.
                if self.class_blocking_special(b, lhs, delay.as_ref(), rhs) {
                    return;
                }
                // v5 ⑥: `d = new[n]` / `x = q.pop_*()` special forms.
                if self.dyn_blocking_special(b, lhs, delay.as_ref(), rhs) {
                    return;
                }
                // Phase-1.x ②: unpacked-array assignment (whole / slice).
                if self.array_assign_special(b, lhs, delay.as_ref(), rhs, false) {
                    return;
                }
                // v7: `x = $random(seed)` — the seeded draw writes the seed
                // back, statement-level intercept (the only legal placement).
                if self.random_seeded_special(b, Some(lhs), delay.as_ref(), rhs) {
                    return;
                }
                // v9 rank 6: `x = $dist_uniform(seed, start, end)` — seeded family.
                if self.dist_uniform_special(b, Some(lhs), delay.as_ref(), rhs) {
                    return;
                }
                // v9 rank 6: `ok = $cast(dst, src)` (func form) — writes dst ref.
                if self.cast_special(b, lhs, delay.as_ref(), rhs) {
                    return;
                }
                // v7: `ok = $value$plusargs(fmt, var)` — same family.
                if self.value_plusargs_special(b, Some(lhs), delay.as_ref(), rhs) {
                    return;
                }
                // v7: `fd = $fopen(name[, mode])` — same family.
                if self.fopen_special(b, lhs, delay.as_ref(), rhs) {
                    return;
                }
                // v7 P2-C: `s = $sformatf(fmt, args…)` — same family.
                if self.sformatf_special(b, lhs, delay.as_ref(), rhs) {
                    return;
                }
                // ⓑ-breadth: `s = {a, b, …}` string concat → $sformatf desugar.
                if self.string_concat_special(b, lhs, delay.as_ref(), rhs) {
                    return;
                }
                // v9 SYS-READ: `c = $fgetc(fd)` / `e = $feof(fd)` /
                // `r = $ungetc(c, fd)` — each advances/mutates the fd read
                // state, statement-level intercept (the only legal placement).
                if self.file_read_int_special(b, Some(lhs), delay.as_ref(), rhs) {
                    return;
                }
                // v9 SYS-READ: `n = $fgets(str, fd)` — writes the str dest +
                // returns the byte count, the $value$plusargs family.
                if self.fgets_special(b, Some(lhs), delay.as_ref(), rhs) {
                    return;
                }
                // v9 SYS-READ: `rc = $fread(target, fd[, start[, count]])` —
                // binary read into a reg/memory, same family.
                if self.fread_special(b, Some(lhs), delay.as_ref(), rhs) {
                    return;
                }
                // v9 SYS-READ: `n = $fscanf(fd, fmt, args...)` /
                // `n = $sscanf(str, fmt, args...)` — the scanf parser writes the
                // matched ref args + returns the count, same family.
                if self.scanf_special(b, Some(lhs), delay.as_ref(), rhs) {
                    return;
                }
                // §4.5.177: a direct-rhs `x = f(arr)` to a FRAMED function with an `input`
                // dyn-array formal, at module-process level — emit the `handle_copy`
                // snapshot marker(s) that fill the callee formal's heap slot, and BLESS the
                // call so `emit_frame_call` (during `lower_expr` below) binds the formal.
                // Every other context returns false here → the call stays loud in
                // `emit_frame_call` (no marker ⇒ no support). The flag is cleared right
                // after the rhs lowers so it never leaks to a later statement.
                let dyn_blessed = self.emit_frame_dyn_formal_markers(b, delay.as_ref(), rhs);
                // N7: reject forging a handle from an integral / leaking a handle
                // to an integral (closes the use-after-free hole).
                self.check_handle_assign(lhs, rhs);
                let rhs_id = self.lower_expr(rhs);
                if dyn_blessed {
                    self.dyn_formal_call_ok = false;
                }
                let lv = self.lower_lvalue(lhs);
                self.check_lvalue_kind(&lv, true); // P1-9 (E3018): no proc write to a net
                                                   // §5.7.1: context-determined fill literal → lvalue width.
                let rhs_id = self.resize_fill_rhs(rhs, rhs_id, &lv);
                if let Some(d) = delay {
                    // IEEE §9.2.2 intra-assignment delay: the RHS evaluates NOW,
                    // the process suspends `d`, THEN the write happens. Lower as
                    //   tmp = rhs;  #d;  lhs = tmp;
                    // with a synthetic tmp sized EXACTLY to the lvalue width so
                    // the rhs eval context (max(lhs_w, self_w)) is unchanged —
                    // a wider tmp would alter div/shift operand truncation.
                    let w = self.ir_lvalue_width(&lv);
                    let tmp = self.fresh_ia_tmp(w);
                    let cap = self.push_stmt(ir::Stmt::BlockingAssign {
                        lhs: whole_net_lvalue(tmp),
                        rhs: rhs_id,
                    });
                    b.push_stmt_id(cap);
                    let (amount, region) = self.lower_delay(d);
                    let resume = b.new_block();
                    b.end_block_with(ir::Terminator::Delay {
                        amount,
                        region,
                        resume: resume.raw(),
                    });
                    b.start_block(resume);
                    let tmp_read = self.push_expr(ir::Expr::Signal {
                        net: tmp,
                        word: None,
                    });
                    let wr = self.push_stmt(ir::Stmt::BlockingAssign {
                        lhs: lv,
                        rhs: tmp_read,
                    });
                    b.push_stmt_id(wr);
                } else {
                    let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                        lhs: lv,
                        rhs: rhs_id,
                    });
                    b.push_stmt_id(sid);
                }
            }
            ast::Stmt::NonBlocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } => {
                // §6.16.3: a string element write `s[i]` has no defined non-blocking
                // form (iverilog 13.0 aborts on `s[i] <= c`). Detected FIRST — ahead
                // of the event early-return — so the intra-event variant `s[i] <=
                // @(ev) c` is caught too. The packed bit-write the normal path would
                // emit is a silent-wrong (one bit of the materialized vector, not the
                // character byte) — honest-loud.
                if self.is_string_index_lvalue(lhs) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "non-blocking write to a string element `s[i] <= c` is \
                         unsupported (use a blocking assignment)",
                    );
                    return;
                }
                // N1: non-blocking intra-assignment EVENT control
                // (`a <= [repeat(n)] @(ev) rhs`). Capture-now / fork-join_none /
                // NBA-write desugar — the process does NOT block.
                if let Some(ie) = event {
                    self.lower_intra_event_nba(b, lhs, ie, rhs, *span);
                    return;
                }
                // `a <= #d rhs` (v5 increment A): a VALUE-CARRYING transport
                // delay — RHS and any LHS index are sampled at execution time,
                // the update joins the NBA region of t+d, and overlapping
                // activations each deliver their own capture. `d` is an ExprId
                // evaluated at execution (v4 runtime-delay model, scaled by
                // the process's timescale multiplier); d == 0 degenerates to
                // the plain same-tick NBA path (statement order preserved).
                // Phase-1.x ②: unpacked-array assignment (whole / slice).
                if self.array_assign_special(b, lhs, delay.as_ref(), rhs, true) {
                    return;
                }
                self.check_handle_assign(lhs, rhs); // N7 handle type gate
                let rhs_id = self.lower_expr(rhs);
                let lv = self.lower_lvalue(lhs);
                self.check_lvalue_kind(&lv, true); // P1-9 (E3018)
                                                   // §5.7.1: context-determined fill literal → lvalue width.
                let rhs_id = self.resize_fill_rhs(rhs, rhs_id, &lv);
                let delay_id = delay.as_ref().map(|d| self.lower_delay(d).0);
                let sid = self.push_stmt(ir::Stmt::NonblockingAssign {
                    lhs: lv,
                    rhs: rhs_id,
                    delay: delay_id,
                });
                b.push_stmt_id(sid);
            }
            ast::Stmt::SysTaskCall { name, args, span } => {
                // v9 SYS-READ as a BARE statement (return discarded): $sscanf/
                // $fscanf/$fgets/$fread WRITE their destination args, $value$plusargs
                // writes its ref var, and $fgetc/$ungetc advance/mutate the fd — all
                // as a side-effect of evaluating the SysFunc. The `*_special` helpers
                // emit that write only from an assignment rhs; as a bare statement
                // they never ran, so the effect was silently dropped (iverilog does
                // it regardless of the return use). Route through the same helpers
                // with NO lhs (return discarded). ($feof is a pure query — a bare
                // $feof has no effect, so it is not routed here.)
                //
                // NOT in a FRAME function/task/method body (`in_frame_body`): the
                // discard uses a fresh MODULE net (`emit_discarded_call`→
                // `fresh_ia_tmp`), which is outside the frame, and `run_frame_call`
                // cannot execute the sys-read StmtEffect anyway — so leave the
                // pre-existing `lower_systask` (warn+skip) path there unchanged (a
                // bare sys-read in a subprogram body is a separate follow-on; its
                // ASSIGNMENT form is equally unsupported). A process body and an
                // INLINE task body (both in the process stream) DO execute it.
                if !self.in_frame_body
                    && matches!(
                        name.name.as_str(),
                        "$sscanf"
                            | "$fscanf"
                            | "$fgets"
                            | "$fread"
                            | "$value$plusargs"
                            | "$fgetc"
                            | "$ungetc"
                            | "$random"
                            // Of the `$dist_*` family only `$dist_uniform` is routed:
                            // its integer LCG advance AND draw match iverilog exactly
                            // for all inputs (the same verified `$random` LCG). The
                            // NON-uniform siblings have a PRE-EXISTING vita-vs-iverilog
                            // divergence — `$dist_chi_square`/`$dist_t` in the seed LCG
                            // advance itself, `$dist_normal`/`$dist_exponential`/
                            // `$dist_erlang` (and possibly `$dist_poisson`) in the draw
                            // VALUE (vendored-libm rounding, off-by-one) — present in
                            // the ASSIGNMENT form too. Routing them bare would turn
                            // base's skip into a DIFFERENT wrong value; leave them
                            // warn+skip (base==fix) until the algorithm is fixed
                            // (a separate, deep follow-on).
                            | "$dist_uniform"
                    )
                {
                    let synth = ast::Expr {
                        kind: ast::ExprKind::SysCall {
                            name: name.clone(),
                            args: args.clone(),
                        },
                        span: *span,
                    };
                    if self.scanf_special(b, None, None, &synth)
                        || self.fgets_special(b, None, None, &synth)
                        || self.fread_special(b, None, None, &synth)
                        || self.value_plusargs_special(b, None, None, &synth)
                        || self.file_read_int_special(b, None, None, &synth)
                        // Bare seeded `$random(seed);` advances the seed (rhs-based
                        // writeback). Unseeded bare `$random;` has no seed → the
                        // helper declines (args empty) → a harmless discarded draw.
                        || self.random_seeded_special(b, None, None, &synth)
                        // Bare `$dist_*(seed, …);` — the same rhs-based seed writeback
                        // (`StmtEffect::SeededDist`). §4.5.125 sibling.
                        || self.dist_uniform_special(b, None, None, &synth)
                    {
                        return;
                    }
                }
                // $sformatf-as-print-arg (§4.5.127, +§4.5.128 $sformat/$swrite):
                // a top-level `$sformatf(...)` VALUE argument to an IMMEDIATE format
                // task ($display/$write/$fdisplay/$fwrite, and the string-dest
                // $sformat/$swrite* family) is not the direct rhs of an assignment,
                // so lowering it would hit the loud guard in `lower_expr`. Hoist
                // each such arg to a fresh string temp (rendered once, before the
                // task — the immediate task renders exactly once too, and $sformatf
                // is pure) and pass the temp Ident.
                //
                // GATED on the FORMAT arg (index `fmt_idx` = 0, or 1 after the fd of
                // an `$f…` task) being a STRING LITERAL. A runtime string in the
                // format position (a `string` variable, a function result, …) needs
                // a runtime format engine vita lacks — it renders WRONG in both base
                // and fix (`$display(str_var, x)` is a pre-existing silent gap). If
                // we hoisted the value `$sformatf` there, we would strip the loud
                // reject the raw `$sformatf` was providing and EXPOSE that silent
                // mis-render (base loud → fix wrong). So a non-literal format is left
                // untouched → the raw `$sformatf` still hits the loud guard → LOUD.
                // ALSO gated on the literal having ENOUGH arg-consuming specifiers to
                // consume every value arg (`count_arg_specs(fmt) >= value args`). A
                // SURPLUS value arg (or an empty format) has no specifier — vita then
                // renders that leftover string temp NUMERICALLY (a pre-existing gap;
                // iverilog prints it as text with no warning), so hoisting it would be
                // a base-loud → fix-wrong silent-wrong. Too few specifiers → not
                // hoisted → the raw `$sformatf` stays → LOUD. (The hoist is byte-for-
                // byte transparent — a `$sfmt_tmp$` string net renders exactly like a
                // user `string` — so "no surplus" is the exact safe boundary.)
                // Only args STRICTLY AFTER the format (`i > fmt_idx`) are hoisted —
                // the fd of an `$f…` task (or the string DEST of `$sformat`/`$swrite*`)
                // at index 0 is never a valid `$sformatf`, so it stays raw → loud.
                //
                // DEFERRED tasks ($monitor/$strobe) are excluded: they re-render, so
                // hoisting would freeze the value (and iverilog itself rejects a
                // $sformatf arg there — no oracle). A NESTED $sformatf (in a concat/
                // ternary) is left untouched → still loud (a separate follow-on).
                //
                // `$sformat`/`$swrite*` write their rendered text to a string DEST at
                // index 0 (like an `$f…` task's fd) and are IMMEDIATE (rendered once at
                // execution), so their format is at index 1 and value args at i > 1.
                let fmt_idx = usize::from(
                    name.name.starts_with("$f")
                        || matches!(
                            name.name.as_str(),
                            "$sformat" | "$swrite" | "$swriteb" | "$swriteo" | "$swriteh"
                        ),
                );
                if matches!(
                    name.name.as_str(),
                    "$display" | "$displayb" | "$displayo" | "$displayh"
                        | "$write" | "$writeb" | "$writeo" | "$writeh"
                        | "$fdisplay" | "$fdisplayb" | "$fdisplayo" | "$fdisplayh"
                        | "$fwrite" | "$fwriteb" | "$fwriteo" | "$fwriteh"
                        | "$sformat" | "$swrite" | "$swriteb" | "$swriteo" | "$swriteh"
                ) && matches!(
                    args.get(fmt_idx).map(|a| &a.kind),
                    Some(ast::ExprKind::StrLit { raw })
                        if count_arg_specs(raw) >= args.len().saturating_sub(fmt_idx + 1)
                ) && args.iter().enumerate().any(|(i, a)| {
                    i > fmt_idx
                        && matches!(&a.kind, ast::ExprKind::SysCall { name, .. } if name.name == "$sformatf")
                }) {
                    let hoisted: Vec<ast::Expr> = args
                        .iter()
                        .enumerate()
                        .map(|(i, a)| {
                            if i > fmt_idx {
                                self.hoist_sformatf_arg(b, a).unwrap_or_else(|| a.clone())
                            } else {
                                a.clone()
                            }
                        })
                        .collect();
                    if let Some(sid) = self.lower_systask(name, &hoisted) {
                        b.push_stmt_id(sid);
                    }
                    return;
                }
                if let Some(sid) = self.lower_systask(name, args) {
                    b.push_stmt_id(sid);
                }
            }
            ast::Stmt::Null(_) => { /* no-op, same block */ }

            // ── SEQUENCING: begin … end ─────────────────────────────
            // begin..end: block-local decls were already hoisted to module nets in
            // the Nets phase (hoist_block_local_nets), so just lower the stmts here.
            ast::Stmt::Block {
                label,
                decls,
                stmts,
                span,
                ..
            } => {
                // Named block targeted by some `disable` in its own body:
                // allocate an exit BB so the disable lowers as a Goto (doc-17
                // lowering row). Allocation is LAZY (pre-scan) so unlabeled /
                // never-disabled blocks lower byte-identically to the old CFG.
                // (A MODULE-process block-local NON-const initializer is static-lifetime
                // — applied ONCE at t0 by the synthesized var-init `initial`, NOT on
                // each block entry — see `hoist_block_local_nets`.)
                let exit = label.as_ref().and_then(|lab| {
                    stmts
                        .iter()
                        .any(|st| stmt_disables_label(st, &lab.name))
                        .then(|| {
                            let exit = b.new_block();
                            self.disable_stack.push((lab.name.clone(), exit));
                            exit
                        })
                });
                // §4.5.189: inside a FRAME body (automatic task/function), a nested
                // block's decl-inits run at BLOCK ENTRY (here) — so a decl-init inside a
                // LOOP re-initializes every iteration (IEEE automatic lifetime, §6.21).
                // The outermost body block has empty `decls` (parser), so the top-level
                // body_decls still init once at frame entry (`emit_frame_local_inits`).
                // MODULE-process block-locals keep their static (once-at-t0) init.
                let emit_block_inits = self.in_frame_body;
                // DUP (round-5): if this block has `$blk$`-scoped locals, lower its
                // body under the SAME segment the Nets-phase hoist used so a scoped
                // local resolves to its own net; `walk_scopes_key` treats `$blk$` as
                // transparent so every non-scoped name still falls through to the
                // enclosing module net (byte-identical for unscoped blocks).
                // r18 (family D): a MODULE process (not `in_frame_body`) emits its
                // per-entry automatic-with-init block-locals here so they RE-INITIALIZE on
                // each block entry (§6.21); static block-locals keep their once-at-t0 init.
                let span_lo = span.lo;
                if self.scoped_block_locals.contains_key(&span.lo) {
                    let seg = format!("$blk${}", span.lo);
                    self.with_scope(&seg, |s| {
                        if emit_block_inits {
                            s.emit_frame_local_inits(b, decls);
                        } else {
                            s.emit_per_entry_block_inits(b, decls, span_lo);
                        }
                        for st in stmts {
                            s.lower_stmt(b, st);
                        }
                    });
                } else {
                    if emit_block_inits {
                        self.emit_frame_local_inits(b, decls);
                    } else {
                        self.emit_per_entry_block_inits(b, decls, span_lo);
                    }
                    for st in stmts {
                        self.lower_stmt(b, st);
                    }
                }
                if let Some(exit) = exit {
                    self.disable_stack.pop();
                    b.goto(exit);
                    b.start_block(exit);
                }
            }

            // ── IF / ELSE — the canonical merge pattern ─────────────
            ast::Stmt::If {
                cond,
                then_s,
                else_s,
                ..
            } => {
                let then_bb = b.new_block();
                let else_bb = b.new_block();
                let merge = b.new_block();
                // F-record-out follow-on (§4.5.215): an output/inout-formal call in a
                // short-circuit `&&`/`||` IF-condition is lowered as the SAME explicit
                // branch chain as a loop cond (`lower_shortcircuit_cond`) — the guarded
                // call's copy-out fires ONLY on the path that reaches it, never made
                // unconditional. Every other condition is byte-identical (one Branch).
                if self.cond_needs_shortcircuit_split(cond) {
                    self.lower_shortcircuit_cond(b, cond, then_bb.raw(), else_bb.raw());
                } else {
                    let cond_id = self.lower_branch_cond(b, cond);
                    b.end_block_with(ir::Terminator::Branch {
                        cond: cond_id,
                        then_bb: then_bb.raw(),
                        else_bb: else_bb.raw(),
                    });
                }
                b.start_block(then_bb);
                self.lower_stmt(b, then_s);
                b.goto(merge);
                b.start_block(else_bb);
                if let Some(e) = else_s {
                    self.lower_stmt(b, e);
                }
                b.goto(merge);
                b.start_block(merge); // continue in merge (post-condition)
            }

            // ── DEFERRED IMMEDIATE ASSERT (§16.4) — If + flush marker ─
            // `assert #0` (Observed) / `assert final` (Reactive): lowered like an
            // immediate assert (Branch over pass/fail BBs) PLUS a per-assertion
            // flush marker emitted before the Branch. The action SysTask(s) are
            // recorded out-of-band so the engine enqueues them for region
            // maturation with flush-on-re-reach instead of firing inline.
            ast::Stmt::DeferredAssert {
                region,
                cond,
                then_s,
                else_s,
                ..
            } => {
                let dr = match region {
                    ast::AssertDefer::Observed => DeferRegion::Observed,
                    ast::AssertDefer::Reactive => DeferRegion::Reactive,
                };
                // Clear any enclosing deferred context so the marker itself is not
                // recorded as an outer assert's action (nested deferreds are
                // pathological but kept correct).
                let outer = self.cur_defer.take();
                // (1) FLUSH MARKER: a suppressed no-op `$display`. Reaching it (in
                //     the Active region) cancels any prior pending report for this
                //     assertion instance + activity — flush-on-re-reach (§16.4).
                let marker = self.push_stmt(ir::Stmt::SysTask {
                    which: ir::SysTaskId::Display,
                    fmt: None,
                    args: Vec::new(),
                });
                self.defer_marks.insert(marker, dr);
                b.push_stmt_id(marker);
                // (2) Lower like an `If`; the `push_stmt` hook (keyed on
                //     `cur_defer`) records each arm's action SysTasks into
                //     `defer_acts` under this marker.
                let cond_id = self.lower_expr(cond);
                let then_bb = b.new_block();
                let else_bb = b.new_block();
                let merge = b.new_block();
                b.end_block_with(ir::Terminator::Branch {
                    cond: cond_id,
                    then_bb: then_bb.raw(),
                    else_bb: else_bb.raw(),
                });
                let n_before = self.defer_acts.len();
                self.cur_defer = Some((marker, dr));
                b.start_block(then_bb);
                self.lower_stmt(b, then_s);
                b.goto(merge);
                b.start_block(else_bb);
                self.lower_stmt(b, else_s);
                b.goto(merge);
                self.cur_defer = outer;
                b.start_block(merge);
                // (3) Neither arm produced a deferrable action ⇒ it ran inline
                //     (evaluate-when-reached): a documented hand-IEEE corner.
                if self.defer_acts.len() == n_before && !self.defer_inline_warned {
                    self.defer_inline_warned = true;
                    self.warn(
                        "a deferred assertion's action contains no $display/$error-class \
                         statement; it executes inline (evaluate-when-reached), not deferred",
                    );
                }
            }

            // ── CASE / CASEZ / CASEX — Branch chain ─────────────────
            ast::Stmt::Case {
                kind,
                scrutinee,
                items,
                ..
            } => self.lower_case(b, *kind, scrutinee, items),

            // ── #delay ──────────────────────────────────────────────
            ast::Stmt::DelayCtrl { delay, body, .. } => {
                let (amount, region) = self.lower_delay(delay);
                let resume = b.new_block();
                b.end_block_with(ir::Terminator::Delay {
                    amount,
                    region,
                    resume: resume.raw(),
                });
                b.start_block(resume);
                if let Some(body) = body {
                    self.lower_stmt(b, body);
                }
            }

            // ── @(event) ────────────────────────────────────────────
            ast::Stmt::EventCtrl { ctrl, body, .. } => {
                // G7: `@(edge sig iff cond) STMT` waits for an `edge sig` occurrence AT
                // WHICH `cond` is true (IEEE §9.4.2.3) — a GUARDED wait. It is NOT
                // `@(edge sig) if(cond) STMT` (which would unblock at the FIRST edge, so a
                // one-shot `@(posedge clk iff rdy);` fires a cycle early — silent-wrong).
                // Desugar to a wait LOOP and lower it: `@(edge sig); while(!cond) @(edge
                // sig); STMT`. (The `always @(… iff …)` proc-block form keeps the `if
                // (cond)` desugar in `desugar_event_iff`, which is correct there because
                // the always re-arms on every edge.) A multi-term `iff` already louded in
                // `event_iff_guard` (returns None → falls through, elaboration fails).
                if let Some(guard) = self.event_iff_guard(ctrl) {
                    let gspan = guard.span;
                    let stripped = match ctrl {
                        ast::Sensitivity::List(terms) => {
                            ast::Sensitivity::List(vec![ast::EventExpr {
                                iff: None,
                                ..terms[0].clone()
                            }])
                        }
                        other => other.clone(),
                    };
                    let wait = ast::Stmt::EventCtrl {
                        ctrl: stripped,
                        body: None,
                        span: gspan,
                    };
                    let not_guard = ast::Expr {
                        kind: ast::ExprKind::Unary {
                            op: ast::UnOp::LogNot,
                            operand: Box::new(guard),
                        },
                        span: gspan,
                    };
                    let mut stmts = vec![
                        wait.clone(),
                        ast::Stmt::While {
                            cond: not_guard,
                            body: Box::new(wait),
                            span: gspan,
                        },
                    ];
                    if let Some(body) = body {
                        stmts.push((**body).clone());
                    }
                    self.lower_stmt(
                        b,
                        &ast::Stmt::Block {
                            label: None,
                            decls: Vec::new(),
                            stmts,
                            span: gspan,
                        },
                    );
                    return;
                }
                // P1-4: in-body `@(*)` infers the read-set of the statement it
                // CONTROLS (IEEE 1800 §9.4.2.2) — the cause is patched after the
                // body lowers (blocks ≥ `resume` are exactly the controlled
                // statement at snapshot time; later siblings append afterwards).
                let star = matches!(ctrl, ast::Sensitivity::Star);
                let cause = if star {
                    ir::WaitCause::Level { nets: Vec::new() } // patched below
                } else {
                    self.lower_event_wait_cause(ctrl)
                };
                let wait_bb = b.cur.expect("EventCtrl with no open block");
                let resume = b.new_block();
                b.end_block_with(ir::Terminator::Wait {
                    cond: cause,
                    resume: resume.raw(),
                });
                b.start_block(resume);
                if let Some(body) = body {
                    self.lower_stmt(b, body);
                }
                if star {
                    let nets = self.comb_read_set(&b.body[resume.raw() as usize..]);
                    if nets.is_empty() {
                        self.warn("in-body @(*) reads no nets; it can never wake");
                    }
                    b.body[wait_bb.raw() as usize].term = ir::Terminator::Wait {
                        cond: ir::WaitCause::Level { nets },
                        resume: resume.raw(),
                    };
                }
            }

            // ── wait(expr) — level wait via WaitCause::Expr ─────────
            ast::Stmt::Wait { cond, body, .. } => {
                let e = self.lower_expr(cond);
                let resume = b.new_block();
                b.end_block_with(ir::Terminator::Wait {
                    cond: ir::WaitCause::Expr { expr: e },
                    resume: resume.raw(),
                });
                b.start_block(resume);
                if let Some(body) = body {
                    self.lower_stmt(b, body);
                }
            }

            // ── LOOPS (SECONDARY) ───────────────────────────────────
            ast::Stmt::Forever { body, .. } => self.lower_forever(b, body),
            ast::Stmt::While { cond, body, .. } => self.lower_while(b, cond, body),
            ast::Stmt::Repeat { count, body, .. } => self.lower_repeat(b, count, body),
            ast::Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => self.lower_for(b, init, cond, step, body),

            // disable: REAL for a lexically-enclosing named begin-block (the
            // break/continue idiom) — doc-17's lowering row "Stmt::Disable then
            // Goto": the Disable stmt keeps the diagnostic shape (engine no-op,
            // target = the exit BB it jumps to), the Goto does the work. The
            // fork floor keeps a child from jumping across its join barrier.
            // Anything else (cross-process block, task body, hierarchical path,
            // unknown label) is a LOUD error — the old warn+no-op silently kept
            // executing the "disabled" block.
            ast::Stmt::Disable { target, .. } => {
                let hit = if target.segments.len() == 1 {
                    self.disable_stack[self.disable_fork_floor..]
                        .iter()
                        .rev()
                        .find(|(n, _)| n == &target.segments[0].name)
                        .map(|(_, exit)| *exit)
                } else {
                    None
                };
                // P2-E: `disable fork` — kill the caller's descendants
                // (IEEE §9.6.3). Straight-line statement, no control flow.
                if target.segments.len() == 1 && target.segments[0].name == "fork" {
                    let sid = self.push_stmt(ir::Stmt::Disable {
                        scope_kind: ir::DisableKind::Fork,
                        target: 0,
                    });
                    b.push_stmt_id(sid);
                    return;
                }
                match hit {
                    Some(exit) => {
                        let sid = self.push_stmt(ir::Stmt::Disable {
                            scope_kind: ir::DisableKind::Scope,
                            target: exit.raw(),
                        });
                        b.push_stmt_id(sid);
                        b.goto(exit);
                        // unreachable continuation keeps the one-open-cursor
                        // contract (INV-1) for the rest of the block.
                        let dead = b.new_block();
                        b.start_block(dead);
                    }
                    None => {
                        let path = target
                            .segments
                            .iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>()
                            .join(".");
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "disable target `{path}` is not a lexically-enclosing \
                                 named block of this statement; v1 supports only the \
                                 break/continue idiom (cross-process, task, fork-crossing \
                                 and hierarchical disable are unsupported)"
                            ),
                        );
                    }
                }
            }
            ast::Stmt::Fork {
                stmts,
                join,
                decls,
                span,
                label: _,
            } => {
                // ── HARD MVP BOUNDARY: no nested fork (§6.2). A fork inside a fork
                //    child is a fatal elaborate error — NOT "warn and proceed". This
                //    keeps child identity flat (a single, non-chained tie shift) and
                //    forbids tie aliasing/overflow.
                if self.in_fork {
                    self.error_unsupported(
                        *span,
                        "nested fork is unsupported in v1 \
                         (a fork child may not itself contain a fork)",
                    );
                    // Emit a well-formed but inert block so INV-1/INV-2 still hold:
                    // seal the cursor straight to the continuation. No children, no
                    // barrier, no mode entry.
                    let cont = b.new_block();
                    b.goto(cont);
                    b.start_block(cont);
                    return;
                }

                // fork-local decls share the enclosing scope in v1 (like begin-block
                // decls); WARN-ignore them, matching Stmt::Block decl handling.
                if !decls.is_empty() {
                    self.warn(
                        "fork-local decls ignored (v1 shared-scope); \
                         declared in enclosing scope",
                    );
                }

                // INV-2: allocate EVERY named block BEFORE building the Fork
                // terminator. Allocation order is deterministic (3-OS golden
                // stability): join, resume, then each child entry in source order.
                let join_bb = b.new_block();
                let resume_bb = b.new_block();
                let child_entries: Vec<BlockId> = (0..stmts.len()).map(|_| b.new_block()).collect();

                // Record the join MODE into the side table (NOT IR), keyed by
                // (cur_proc, join_bb). The engine's lookup is total-or-fatal.
                self.record_fork_mode(*join, join_bb.raw());

                // INV-1: seal the parent block with Fork — this CLOSES the cursor.
                b.end_block_with(ir::Terminator::Fork {
                    children: child_entries.iter().map(|e| e.raw()).collect(),
                    join: join_bb.raw(),
                    resume_bb: resume_bb.raw(),
                });

                // Lower each child chain. `lower_stmt` returns with the cursor open
                // at the child's single continuation; `goto(join_bb)` seals that tail
                // so the child's LAST block hands control to the join. Empty `stmts`
                // ⇒ this loop is skipped (valid: Fork{children:[]}). `in_fork` is set
                // so any Fork lowered INSIDE a child hits the hard error above.
                let prev_in_fork = self.in_fork;
                self.in_fork = true;
                // A fork child is its own process: it may disable only blocks
                // inside its OWN body (floor), never across the join barrier.
                let prev_floor = self.disable_fork_floor;
                self.disable_fork_floor = self.disable_stack.len();
                for (child_entry, st) in child_entries.iter().zip(stmts.iter()) {
                    b.start_block(*child_entry);
                    self.lower_stmt(b, st);
                    b.goto(join_bb);
                }
                self.disable_fork_floor = prev_floor;
                self.in_fork = prev_in_fork;

                // Seal the join block: join_bb → resume_bb. IMPORTANT: this Goto is a
                // NEVER-EXECUTED sentinel. The engine intercepts a child the instant
                // its next bb equals join_bb (centralized loop-top check) and routes
                // it to on_child_complete + Step::Done BEFORE this block is fetched.
                // The parent is resumed DIRECTLY at resume_bb by the barrier. The
                // block exists only to keep the CFG well-formed (INV-2) and to give
                // join_bb a concrete, unique BlockId used as the completion sentinel.
                b.start_block(join_bb);
                b.goto(resume_bb);

                // Open resume_bb as the single continuation. Post-condition for the
                // caller: exactly one open block, at the parent's continuation point.
                b.start_block(resume_bb);
            }
            ast::Stmt::RandomizeWith {
                name,
                args,
                constraints,
                ..
            } => {
                // N7-REST B-CRV final: `obj.randomize() with {…};` as a void statement.
                if !self.try_emit_randomize(b, name, args, Some(constraints), None) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "inline `randomize() with {…}` is only valid on a class handle",
                    );
                }
            }
            ast::Stmt::UserTaskCall { name, args, .. } => {
                // N7-REST: `obj.randomize();` as a statement (discards the result).
                if self.try_emit_randomize(b, name, args, None, None) {
                    return;
                }
                // N7: a class void-method call as a statement (`obj.run(x);`) is a
                // frame-FUNCTION call whose result is discarded.
                if name.segments.len() == 2 {
                    if let Some(call) = self.try_class_method_call(name, args) {
                        self.emit_discarded_call(b, call);
                        return;
                    }
                }
                // N4: a single-segment name that is a user FUNCTION called in
                // statement position (`void'(f())`, and the bare `f();` form) —
                // evaluate the call for its side effects and DISCARD the result.
                // Routed before `inline_task` so it is not mis-reported as an
                // undeclared task.
                if name.segments.len() == 1 && self.func_table.contains_key(&name.segments[0].name)
                {
                    let call_expr = ast::Expr {
                        kind: ast::ExprKind::Call {
                            name: name.clone(),
                            args: args.to_vec(),
                        },
                        span: name.span,
                    };
                    let call = self.lower_expr(&call_expr);
                    self.emit_discarded_call(b, call);
                    return;
                }
                self.inline_task(b, name, args)
            }
            // N7: `return [expr];` — assign the enclosing method/function's return
            // var (if a value method) and jump to the body exit block.
            ast::Stmt::Return { value, .. } => match self.cur_return {
                Some((retvar, exit)) => {
                    if let Some(val) = value {
                        // §11.6: the return value is in the context of the return
                        // var's width, so a fill grows to that width (non-fill ⇒
                        // byte-identical via lower_expr).
                        let ctx_w = match retvar {
                            Some(rv) => self.nets.get(rv as usize).map(|n| n.width).unwrap_or(32),
                            None => 0,
                        };
                        let rhs = self.lower_return_rhs(val, ctx_w);
                        if let Some(rv) = retvar {
                            let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                                lhs: whole_net_lvalue(rv),
                                rhs,
                            });
                            b.push_stmt_id(sid);
                        } else {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "`return <expr>` in a void task/method (use `return;`)",
                            );
                        }
                    }
                    b.goto(exit);
                    let dead = b.new_block(); // statements after `return` are unreachable
                    b.start_block(dead);
                }
                None => self.error(
                    MsgCode::ElabUnsupported,
                    "`return` outside a function/task method body",
                ),
            },
            // P1-2: these were warn+no-op — values never changed and an `@(ev)`
            // waited forever. A hard error beats silent misbehavior (defparam
            // precedent); real semantics are a Phase-2 item.
            // `->e` (v5 batch B): the named event is a 64-bit counter reg, so
            // the trigger is `e = e + 1` — every waiter (`@(e)`, mixed lists)
            // sees a guaranteed value change. A same-slot DOUBLE trigger still
            // changes the counter (0→2) where a 1-bit toggle would be lost
            // (0→1→0). The frozen WaitCause::Named / WakeCond::NamedEvent stay
            // reserved-unused; sim-ir is untouched by this lane.
            ast::Stmt::EventTrigger { name, .. } => {
                let net = self.resolve_net(name);
                if net == POISON_NET {
                    return; // resolve_net already errored
                }
                if !self.event_nets.contains(&net) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "`->` target `{}` is not a named event",
                            name.segments
                                .iter()
                                .map(|s| s.name.as_str())
                                .collect::<Vec<_>>()
                                .join(".")
                        ),
                    );
                    return;
                }
                let sig = self.push_expr(ir::Expr::Signal { net, word: None });
                let one = self.const_u32_expr(1, 64);
                let add = self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Add,
                    lhs: sig,
                    rhs: one,
                });
                let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                    lhs: whole_net_lvalue(net),
                    rhs: add,
                });
                b.push_stmt_id(sid);
            }
            // Procedural continuous assignment (IEEE 1364 §9.3.1): reuses the
            // force machinery at a WEAKER rank — lowered as Stmt::Force /
            // Stmt::Release with the StmtId marked in the assign-rank sidecar
            // (the frozen Stmt has no Assign/Deassign variants; designs without
            // proc-assign lower byte-identically). Targets must be a WHOLE
            // VARIABLE: a net is E3018 (same check as procedural writes), a
            // bit/part-select is loud-unsupported (the force restriction).
            ast::Stmt::Assign { lhs, rhs, .. } => {
                let rhs_id = self.lower_expr(rhs);
                let lv = self.lower_lvalue(lhs);
                self.check_lvalue_kind(&lv, true);
                if !is_whole_single_net(&lv) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "procedural assign target must be a whole variable (a bit/part-select is not a legal target)",
                    );
                    return;
                }
                let sid = self.push_stmt(ir::Stmt::Force {
                    lhs: lv,
                    rhs: rhs_id,
                });
                self.assign_ranks.insert(sid);
                b.push_stmt_id(sid);
            }
            ast::Stmt::Deassign { lhs, .. } => {
                let lv = self.lower_lvalue(lhs);
                self.check_lvalue_kind(&lv, true);
                if !is_whole_single_net(&lv) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "deassign target must be a whole variable",
                    );
                    return;
                }
                let sid = self.push_stmt(ir::Stmt::Release { lhs: lv });
                self.assign_ranks.insert(sid);
                b.push_stmt_id(sid);
            }
            // force/release (IEEE 1364 §9.3.2) — WHOLE net/variable targets
            // only (bit/part-selects are illegal force targets per the LRM and
            // rejected loudly). v1 model = sample-once: the RHS is evaluated
            // at execution time (matching the iverilog oracle, which warns
            // "RHS will only be evaluated once"); full procedural-continuous
            // re-evaluation is a documented refinement.
            ast::Stmt::Force { lhs, rhs, .. } => {
                let rhs_id = self.lower_expr(rhs);
                let lv = self.lower_lvalue(lhs);
                if !is_whole_single_net(&lv) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "force target must be a whole net/variable (a bit/part-select is not a legal force target)",
                    );
                    return;
                }
                let sid = self.push_stmt(ir::Stmt::Force {
                    lhs: lv,
                    rhs: rhs_id,
                });
                b.push_stmt_id(sid);
            }
            ast::Stmt::Release { lhs, .. } => {
                let lv = self.lower_lvalue(lhs);
                if !is_whole_single_net(&lv) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "release target must be a whole net/variable",
                    );
                    return;
                }
                let sid = self.push_stmt(ir::Stmt::Release { lhs: lv });
                b.push_stmt_id(sid);
            }
            // ── wait fork — block on the implicit child barrier (v8) ────
            // No condition expr and no body: the scheduler resolves it against
            // this process's outstanding forked children (IEEE §9.6.1). Final
            // blocks already reject it via `stmt_has_timing` above.
            ast::Stmt::WaitFork { .. } => {
                let resume = b.new_block();
                b.end_block_with(ir::Terminator::Wait {
                    cond: ir::WaitCause::Fork,
                    resume: resume.raw(),
                });
                b.start_block(resume);
            }
            // v8 SVA subset: a concurrent assertion is a CONTINUOUSLY-checking
            // background process clocked by `@(clk)`, not a one-shot procedural
            // statement — collect it and emit nothing into the enclosing block.
            // It is materialized as a synthesized clocked checker after the
            // module's process loop (`materialize_sva_checkers`).
            ast::Stmt::ConcurrentAssert {
                clock,
                disable_iff,
                antecedent,
                implication_kind,
                consequent,
                consequent_clock,
                pass,
                fail,
                prop_expr,
                local_vars,
                span,
            } => {
                // Named-property INSTANCE: `assert property(NAME)` parses to an empty
                // clock + a `Sequence::Instance` antecedent. Splice the declared
                // property's REAL spec (clock/disable_iff/ante/kind/cons) here, at
                // collect time, so the single-clock gate in `materialize_sva_checkers`
                // sees the named property's clock (never the empty sentinel). The
                // call-site action block (pass/fail) is carried through.
                if let ast::Sequence::Instance { name, args, .. } = antecedent {
                    match self.prop_table.get(&name.name).cloned() {
                        Some(pd) if pd.formals.len() != args.len() => {
                            // Arity mismatch (slice A1) — includes a bare `p` instance
                            // of a parameterized property (0 args vs N formals).
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "named property `{}` expects {} formal argument(s), got {}",
                                    name.name,
                                    pd.formals.len(),
                                    args.len()
                                ),
                            );
                        }
                        Some(pd) if pd.formals.is_empty() => {
                            // Byte-identical to before slice A1 (no substitution).
                            // N2d: a `prop_expr` body carries the property's own NAME
                            // as `prop_self_name` so `synth_prop_expr` recognises the
                            // legal tail-`|=>` recursion (`… |=> NAME`).
                            self.pending_sva.push(PendingSva {
                                clock: pd.clock,
                                disable_iff: pd.disable_iff,
                                ante: pd.antecedent,
                                kind: pd.implication_kind,
                                cons: pd.consequent,
                                pass: pass.clone(),
                                fail: fail.clone(),
                                cons_clock: pd.consequent_clock,
                                prop_expr: pd.prop_expr,
                                prop_self_name: Some(name.name.clone()),
                                local_vars: pd.local_vars,
                                span: *span,
                            });
                        }
                        Some(pd) if pd.prop_expr.is_some() => {
                            // N2d: a property-level `and`/`or` tree combined with
                            // FORMAL arguments (slice A1) is out of subset — the
                            // synthesis reduction does not substitute through a tree.
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a parameterized property with property-level \
                                 `and`/`or` is unsupported in this subset",
                            );
                        }
                        Some(pd) if !pd.local_vars.is_empty() => {
                            // N2c: a property with sequence/property LOCAL VARIABLES
                            // combined with FORMAL arguments is out of subset — the
                            // formal substitution would have to thread through the
                            // capture/read sites of the data-tracking machinery. Loud.
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a parameterized property with sequence/property local \
                                 variables is unsupported in this subset",
                            );
                        }
                        Some(pd) => {
                            // Parameterized property: bind actuals → formals and
                            // substitute through the whole spliced spec (clock,
                            // disable, antecedent, consequent) before scheduling.
                            let map = sva_formal_map(&pd.formals, args);
                            self.pending_sva.push(PendingSva {
                                clock: subst_sensitivity(&pd.clock, &map),
                                disable_iff: pd.disable_iff.as_ref().map(|e| subst_expr(e, &map)),
                                ante: subst_sequence(&pd.antecedent, &map),
                                kind: pd.implication_kind,
                                cons: subst_sequence(&pd.consequent, &map),
                                pass: pass.clone(),
                                fail: fail.clone(),
                                cons_clock: pd
                                    .consequent_clock
                                    .as_ref()
                                    .map(|s| subst_sensitivity(s, &map)),
                                prop_expr: None,
                                prop_self_name: None,
                                local_vars: Vec::new(),
                                span: *span,
                            });
                        }
                        None => {
                            // Distinguish a declared-but-wrong-kind name from a
                            // genuinely unknown one (review 2026-06-16: a net or a
                            // sequence name reported a misleading "unknown property").
                            let msg = if self.seq_table.contains_key(&name.name) {
                                format!(
                                    "`{}` is a named SEQUENCE, not a property; a bare \
                                     sequence used as a property (without `|->`/`|=>`) \
                                     is unsupported in this subset",
                                    name.name
                                )
                            } else {
                                format!(
                                    "unknown property `{}` in `assert property(...)` \
                                     (declare a `property {} ; … endproperty`, or use a \
                                     clocked boolean property `@(...) {}`)",
                                    name.name, name.name, name.name
                                )
                            };
                            self.error(MsgCode::ElabUnsupported, &msg);
                        }
                    }
                } else {
                    // Inline `assert property(...)`: a `prop_expr` (N2d and/or tree)
                    // has NO self-name (anonymous ⇒ no recursion site).
                    self.pending_sva.push(PendingSva {
                        clock: clock.clone(),
                        disable_iff: disable_iff.clone(),
                        ante: antecedent.clone(),
                        kind: *implication_kind,
                        cons: consequent.clone(),
                        pass: pass.clone(),
                        fail: fail.clone(),
                        cons_clock: consequent_clock.clone(),
                        prop_expr: prop_expr.clone(),
                        prop_self_name: None,
                        local_vars: local_vars.clone(),
                        span: *span,
                    });
                }
            }
            // SVA-REST cover property: collected like a concurrent assertion and
            // materialized (counter + end-of-sim `$display`) after the module loop.
            ast::Stmt::CoverProperty {
                clock,
                disable_iff,
                seq,
                span,
            } => {
                self.pending_cover.push(PendingCover {
                    clock: clock.clone(),
                    disable_iff: disable_iff.clone(),
                    seq: seq.clone(),
                    span: *span,
                });
            }
            // Parse error is the ONE genuinely-fatal stmt: keep self.error.
            ast::Stmt::Error(_) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "cannot lower parse-error statement",
                );
            }
        }
    }
}
