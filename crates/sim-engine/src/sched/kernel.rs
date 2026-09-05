//! split part of `sched` (mechanical move).

use super::*;

/// [P7b] `Scheduler` is the interpreter's implementation of the body↔kernel ABI:
/// each method forwards to the inherent method of the same purpose (the `k_*` prefix
/// keeps the trait surface distinct from the inherent one, so there is no shadowing).
/// The statement executor (`exec::compute_effect`/`apply_effect`) drives the
/// interpreter through exactly this surface, so the existing suite already exercises
/// the seam byte-identically — and a Stage-C compiled body will call the same methods.
impl Kernel for Scheduler<'_, '_> {
    #[cfg(feature = "jit")]
    fn k_nets(&self) -> &dyn crate::eval::NetReader {
        self.st
    }
    fn k_builtin_prof(&self) -> Option<&crate::profile::BuiltinProfile> {
        self.st.builtin_prof.as_deref()
    }
    fn k_eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value {
        self.eval_for_lvalue(lhs, rhs)
    }
    fn k_eval_native(&self, prog: &crate::native_eval::NativeProg) -> Value {
        self.eval_native(prog)
    }
    fn k_resolve_lvalue_offsets(&self, lhs: &Lvalue) -> Offsets {
        self.resolve_lvalue_offsets(lhs)
    }
    fn k_force(&mut self, lhs: &Lvalue, value: Value, rhs: u32, sid: u32) {
        // The registry rule is `SimState`'s (slice #2 split it out so tier-3 can
        // run the SAME rule against its own store). The multiplier is snapshot
        // at registration so a `$time` in the RHS keeps rendering with the right
        // scale on later re-evals (C7 lesson) — `force_epilogue` reads it.
        if !self.st.force_prologue(lhs, rhs, sid) {
            return;
        }
        self.st.force_write(lhs, value);
        self.st.force_epilogue(lhs, rhs, sid);
    }
    fn k_release(&mut self, lhs: &Lvalue, sid: u32) {
        let resumed = self.st.release_prologue(lhs, sid);
        if let Some((alv, arhs, amult)) = resumed {
            let saved = self.st.cur_time_mult;
            self.st.cur_time_mult = amult;
            let v = self.eval_for_lvalue(&alv, arhs);
            self.st.force_write(&alv, v);
            self.st.cur_time_mult = saved;
            self.st.release_epilogue(&alv, arhs, amult);
        }
        self.redirty_drivers_of(lhs.chunks[0].net);
    }
    fn k_write_lvalue(&mut self, lhs: &Lvalue, value: Value, offsets: &Offsets) {
        self.st.write_lvalue(lhs, value, offsets);
    }
    fn k_schedule_nba(&mut self, lhs: &Lvalue, value: Value) {
        self.schedule_nba(lhs, value);
    }
    fn k_schedule_nba_scalar(&mut self, lhs: &Lvalue, value: Value) {
        self.schedule_nba_scalar(lhs, value);
    }
    fn k_write_scalar(&mut self, lhs: &Lvalue, net: u32, value: Value) {
        self.st.write_scalar(lhs, net, value);
    }
    fn k_delay_ticks(&self, eid: u32) -> u64 {
        self.delay_ticks(eid)
    }
    fn k_schedule_nba_at(&mut self, lhs: &Lvalue, value: Value, ticks: u64) {
        self.schedule_nba_at(lhs, value, ticks);
    }
    fn k_dispatch_systask(
        &mut self,
        which: sim_ir::SysTaskId,
        fmt: Option<u32>,
        args: &[u32],
        sid: u32,
    ) -> crate::builtins::Ctl {
        crate::builtins::dispatch(self, which, fmt, args, sid)
    }

    fn k_exec_fork(
        &mut self,
        act: u32,
        children: &[u32],
        join: u32,
        resume_bb: u32,
    ) -> Option<u32> {
        // The engine's own queue. `run_process` has its own `Fork` arm and does
        // not go through the shared walk — this exists so the body DIFFERENTIAL
        // can drive that walk with `K = Scheduler` on a forking design without
        // reaching the trait's `unreachable!`.
        self.exec_fork(act, children, join, resume_bb)
    }

    fn k_exec_wait_fork(&mut self, act: u32, resume_bb: u32) -> bool {
        // Same reason as `k_exec_fork` above: `run_process` has its own arm, and
        // this exists so the body differential can drive the shared walk with
        // `K = Scheduler` without reaching the trait's `unreachable!`.
        self.exec_wait_fork(act, resume_bb)
    }
    fn k_queue_pop_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::queue_pop_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_random_seeded_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::random_seeded_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_eval(&self, eid: u32) -> Value {
        self.eval(eid)
    }
    fn k_ir(&self) -> &sim_ir::SimIr {
        self.st.ir
    }
    fn k_lvalue_width(&self, lhs: &Lvalue) -> u32 {
        self.st.lvalue_width(lhs)
    }
    fn k_self_width(&self, eid: u32) -> (u32, bool) {
        let sw = self.st.wt.get(eid);
        (sw.width, sw.signed)
    }
    fn k_eval_ctx(&self, eid: u32, ctx_width: u32, ctx_signed: bool) -> Value {
        // The call `split_frame_in_binds` made before A3-i lifted its body out.
        self.eval_ctx_top(eid, ctx_width, ctx_signed)
    }
    fn k_frame_base(&self, func: u32) -> u32 {
        self.st.func_table[func as usize].base_net
    }
    fn k_task_call_site(&self, proc: u32, bb: u32) -> Option<crate::TaskCallInfo> {
        self.st.task_calls_proc.get(&(proc, bb)).cloned()
    }
    fn k_nested_call_site(&self, global_bb: u32) -> Option<crate::TaskCallInfo> {
        self.st.task_calls_func.get(&global_bb).cloned()
    }
    fn k_callee_is_driven(&self, callee: u32) -> bool {
        self.st.suspendable_tasks.contains(&callee)
    }
    fn k_enter_driven_frame(
        &mut self,
        callee: u32,
        in_vals: &[(u32, Value)],
        dyn_snaps: &[(u32, u32)],
    ) -> Vec<(u32, Option<crate::state::DynObj>)> {
        self.st.enter_driven_frame(callee, in_vals, dyn_snaps)
    }
    fn k_exit_driven_frame(
        &mut self,
        callee: u32,
        out_binds: &[(u32, Lvalue)],
        dyn_stash: Vec<(u32, Option<crate::state::DynObj>)>,
    ) -> Vec<(Lvalue, Value)> {
        self.st.exit_driven_frame(callee, out_binds, dyn_stash)
    }
    fn k_call_site_runnable(&self, proc: u32, bb: u32) -> bool {
        crate::exec::frame_call::site_runnable(
            self.st.ir,
            &self.st.suspendable_tasks,
            self.st.task_calls_proc.get(&(proc, bb)),
        )
    }
    fn k_run_subset_task(
        &mut self,
        callee: u32,
        in_vals: &[(u32, Value)],
        dyn_snaps: &[(u32, u32)],
        out_binds: &[(u32, Lvalue)],
    ) -> Vec<(Lvalue, Value)> {
        self.st
            .run_subset_task(callee, in_vals, dyn_snaps, out_binds)
    }
    fn k_file_read_byte(&mut self, fd: u32) -> Option<u8> {
        crate::builtins::file_read_byte(self, fd)
    }
    fn k_file_unget(&mut self, fd: u32, b: u8) {
        self.st.read_state.entry(fd).or_default().pushback.push(b);
    }
    fn k_read_net(&self, net: u32, word: Option<u32>) -> Value {
        self.st.read_net(net, word)
    }
    fn k_array_base(&self, net: u32) -> Option<u64> {
        crate::builtins::declared_array_base(&self.st.net_dims, net)
    }
    fn k_warn_readmem(&mut self, msg: String) {
        // V33-8: one spelling, shared with `$readmem*`/`$writemem*` — see
        // `SimState::warn_readmem` for why the three copies became one.
        self.st.warn_readmem(msg);
    }
    fn k_file_open(&mut self, name: &str, mode: Option<&str>) -> u32 {
        let open = |mode: &str| -> std::io::Result<std::fs::File> {
            let mut o = std::fs::OpenOptions::new();
            // a '+' mode (r+/w+/a+) is read-AND-write; plain w/a are write-only.
            let plus = mode.contains('+');
            match mode.trim_end_matches('b') {
                "r" | "r+" => o.read(true).write(plus),
                "a" | "a+" => o.create(true).append(true).read(plus),
                // "w"/"w+" and anything unrecognized: truncate-write (the
                // overwhelmingly common TB mode; unknown modes behave as "w").
                _ => o.create(true).write(true).truncate(true).read(plus),
            };
            o.open(name)
        };
        match mode {
            Some(m) => match open(m) {
                Ok(f) => {
                    let n = self.st.next_fd;
                    self.st.next_fd = self.st.next_fd.saturating_add(1); // FD-RECLAIM: no wrap
                    let fd = 0x8000_0000 | n;
                    self.st.files.insert(fd, f);
                    // v9 SYS-READ: a mode with 'r' or '+' is read-capable
                    // (r/r+/w+/a+); plain "w"/"a" stays write-only and absent.
                    if m.contains('r') || m.contains('+') {
                        self.st.readable_fds.insert(fd);
                    }
                    fd
                }
                Err(_) => 0, // IEEE: $fopen failure returns 0
            },
            None => match open("w") {
                Ok(f) => {
                    // MCD-RECLAIM: hand out the LOWEST channel bit not currently
                    // open (bit 0 = stdout, reserved), so a $fclose'd bit is
                    // reused (iverilog reclaims). Byte-identical to the old
                    // monotonic counter when nothing has been freed.
                    match (1..31u32).find(|b| !self.st.mcd_files.contains_key(b)) {
                        Some(bit) => {
                            self.st.mcd_files.insert(bit, f);
                            1u32 << bit
                        }
                        None => 0, // space full
                    }
                }
                Err(_) => 0,
            },
        }
    }
    fn k_file_eof(&mut self, fd: u32) -> Option<bool> {
        if fd & 0x8000_0000 == 0 || !self.st.files.contains_key(&fd) {
            crate::builtins::bad_fd_warn(self, fd);
            return None;
        }
        Some(self.st.read_state.get(&fd).map(|s| s.eof).unwrap_or(false))
    }
    fn k_file_ungetc(&mut self, fd: u32, byte: u8) -> bool {
        if fd & 0x8000_0000 == 0 || !self.st.files.contains_key(&fd) {
            crate::builtins::bad_fd_warn(self, fd);
            return false;
        }
        if !self.st.readable_fds.contains(&fd) {
            return false;
        }
        // LIFO push (iverilog retains every pushed byte); pushing clears EOF
        // (there is data to read again).
        let st = self.st.read_state.entry(fd).or_default();
        st.pushback.push(byte);
        st.eof = false;
        true
    }
    fn k_assoc_iter_cur_key(&self, rhs: u32) -> Option<u32> {
        self.st.assoc_iter_cur_key(rhs)
    }
    fn k_assoc_iter_compute(&self, rhs: u32, cur: Option<Value>) -> (Option<(u32, Value)>, i32) {
        self.st.assoc_iter_compute(rhs, cur)
    }
    fn k_random_seeded(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::random_seeded(self, rhs)
    }
    fn k_dist_seeded_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::dist_seeded_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_dist_seeded(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::dist_seeded(self, rhs)
    }
    fn k_cast_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::cast_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_cast(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::cast(self, rhs)
    }
    fn k_value_plusargs_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::value_plusargs_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_value_plusargs(&mut self, rhs: u32) -> Value {
        // Parse/match/convert are the shared half (`exec::plusargs::effect`,
        // extracted so the tier-3 kernel runs the same spelling); only the
        // write is this store's.
        let (status, write, warn) =
            crate::exec::plusargs::effect(self.st.ir, &self.st.plusargs, rhs);
        if let Some((radix, text)) = warn {
            self.st.warn_plusargs_invalid(radix, &text);
        }
        if let Some((lv, v)) = write {
            let off = self.resolve_lvalue_offsets(&lv);
            self.k_write_lvalue(&lv, v, &off);
        }
        status
    }
    fn k_fopen_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fopen_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fopen(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fopen(self, rhs)
    }
    // ── v9 SYS-READ: file-read int functions ($fgetc/$feof/$ungetc) ──
    fn k_fgetc_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fgetc_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fgetc(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fgetc(self, rhs)
    }
    fn k_feof_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::feof_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_feof(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::feof(self, rhs)
    }
    fn k_ungetc_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::ungetc_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_ungetc(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::ungetc(self, rhs)
    }
    fn k_fgets_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fgets_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fgets(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fgets(self, rhs)
    }
    fn k_fread_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fread_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fread(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fread(self, rhs)
    }
    fn k_fscanf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fscanf_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fscanf(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fscanf(self, rhs)
    }
    fn k_sscanf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::sscanf_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_sscanf(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::sscanf(self, rhs)
    }
    fn k_sformatf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::sformatf_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_rhs_is_stmt_effect_family(&self, rhs: u32) -> bool {
        sim_ir::rhs_is_stmt_effect(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_sformatf(&mut self, rhs: u32) -> Value {
        // args = [fmt string-literal Const, value args…] (elaborate contract).
        let Some(sim_ir::Expr::SysFunc { args, .. }) = self.st.ir.exprs.get(rhs as usize) else {
            return Value::from_str_bytes(&[]);
        };
        let (fmt, rest) = (args.first().copied(), args.get(1..).unwrap_or(&[]).to_vec());
        // §4.5.426: `%m` inside a named block (see `expr_scopes`).
        let saved = self
            .st
            .cur_block_scope
            .replace(self.st.expr_scopes.get(&rhs).cloned().unwrap_or_default());
        let text = crate::builtins::format_args_str(&*self.st, fmt, &rest, None);
        self.st.cur_block_scope.replace(saved);
        Value::from_str_bytes(text.as_bytes())
    }
    fn k_disable_fork(&mut self) {
        // IEEE §9.6.3: terminate every ACTIVE DESCENDANT of the calling
        // process. Transitive walk: barriers parented by the kill set spread
        // to their children. The arena is append-only and the walk is
        // index-ordered — deterministic. Stale resume entries are dropped at
        // the `run_body` choke.
        let mut kill: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        kill.insert(self.cur_aid);
        loop {
            let mut grew = false;
            for (aid, a) in self.activities.iter().enumerate() {
                if a.dead || a.reported || kill.contains(&(aid as u32)) {
                    continue;
                }
                let Some(jr) = a.join_ref else { continue };
                let parent = self.barriers[jr as usize].parent;
                if kill.contains(&parent) {
                    kill.insert(aid as u32);
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        kill.remove(&self.cur_aid); // the caller itself lives on
                                    // §16.4: a deferred report pending in a KILLED process is cancelled (the
                                    // action never matures). Drop by `(aid, gen)` of the LIVE killed
                                    // activities so a recycled slot's COMPLETED predecessor report (a
                                    // different gen under the same aid) is NOT also cancelled.
        let mut kill_keys: std::collections::BTreeSet<(u32, u32)> =
            std::collections::BTreeSet::new();
        for &aid in &kill {
            self.activities[aid as usize].dead = true;
            kill_keys.insert((aid, self.activities[aid as usize].gen));
        }
        if !self.st.postponed.deferred_observed.is_empty() {
            self.st
                .postponed
                .deferred_observed
                .retain(|&(_, aid, gen), _| !kill_keys.contains(&(aid, gen)));
        }
        if !self.st.postponed.deferred_reactive.is_empty() {
            self.st
                .postponed
                .deferred_reactive
                .retain(|&(_, aid, gen), _| !kill_keys.contains(&(aid, gen)));
        }
    }
    fn k_queue_pop(&mut self, lhs: &Lvalue, rhs: u32) -> Value {
        // `k_queue_pop_rhs` guaranteed the shape; everything below is
        // defensive against a hand-built IR — degrade, never panic.
        let Some(sim_ir::Expr::SysFunc { which, args }) = self.st.ir.exprs.get(rhs as usize) else {
            return Value::xs(1, false);
        };
        let front = matches!(which, sim_ir::SysFuncId::QPopFront);
        let net = args
            .first()
            .and_then(|&a| match self.st.ir.exprs.get(a as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            });
        let popped = match net {
            Some(n) => {
                let (w, signed) = self
                    .st
                    .ir
                    .nets
                    .get(n as usize)
                    .map(|nv| (nv.width.max(1), nv.signed))
                    .unwrap_or((1, false));
                // Scope the `borrow_mut` to the pop; the empty/non-queue warn
                // runs after it releases (§C6 — no heap guard across the warn).
                let popped_opt = {
                    let mut heap = self.st.dyn_heap.borrow_mut();
                    match heap.get_mut(n as usize).and_then(|o| o.as_mut()) {
                        Some(crate::state::DynObj::Queue { elems }) if !elems.is_empty() => {
                            let v = if front {
                                elems.pop_front()
                            } else {
                                elems.pop_back()
                            };
                            Some(v.unwrap_or_else(|| Value::xs(w, signed)))
                        }
                        _ => None,
                    }
                };
                match popped_opt {
                    Some(v) => v,
                    _ => {
                        // empty (a missing entry IS the empty queue) or a
                        // non-queue object: element-width X + warn-once
                        // (iverilog live: per-call warning + x; our once-latch
                        // is the established anti-spam policy).
                        self.st.dyn_warn_once_at(n, "pop on an empty queue (X)");
                        Value::xs(w, signed)
                    }
                }
            }
            None => Value::xs(1, false),
        };
        // Context-size EXACTLY as `k_eval_for_lvalue` sizes an rhs: width =
        // max(lhs width, pop self-width), extension driven by the pop's
        // self-signedness (= the ELEMENT's, via the width table).
        let lw = self.st.lvalue_width(lhs);
        let sw = self.st.wt.get(rhs);
        popped.resize_keep_sign(lw.max(sw.width), sw.signed)
    }
    fn k_assoc_iter_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::assoc_iter_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_assoc_iter(&mut self, lhs: &Lvalue, rhs: u32) -> Value {
        crate::exec::stmt_effect::assoc_iter(self, lhs, rhs)
    }

    // ── terminator / control surface (C1) — pure forwarders ──
    fn k_truthy(&self, eid: u32) -> bool {
        self.truthy(eid)
    }
    fn k_truthy_value(&self, v: &Value) -> bool {
        self.truthy_value(v)
    }
    fn k_rearm(&mut self, proc: u32) {
        self.rearm(proc);
    }
    fn k_enter_body(&mut self, tmpl: u32) {
        crate::exec::enter_body(self.st, tmpl as usize);
    }
    fn k_now(&self) -> u64 {
        self.st.now
    }
    fn k_delta_budget(&self) -> u64 {
        self.max_deltas
    }
    fn k_time_limit(&self) -> Option<u64> {
        self.time_limit
    }
    fn k_suspend_on(&mut self, proc: u32, block: u32, cause: &sim_ir::WaitCause) {
        self.suspend_on(proc, block, cause.clone());
    }
    fn k_schedule_resume(&mut self, proc: u32, block: u32, tick: u64, inactive: bool) {
        self.schedule_resume(proc, block, tick, inactive);
    }
    fn k_call_fatal(&self) -> bool {
        self.st.call_fatal.get()
    }
    fn k_set_cur_stmt(&self, sid: u32) {
        self.st.cur_stmt.set(sid);
    }
    fn k_drain_diags(&mut self) {
        // Nothing to drain: this store emits at the access (`warn_run_range` is
        // called from `read_net` and from the write funnel). A no-op here is the
        // honest answer, not a stub — see the trait doc.
    }
    fn k_max_deltas(&self) -> u64 {
        self.max_deltas_guard()
    }
    fn k_mark_fatal(&mut self) {
        self.mark_fatal();
    }
    fn k_class_new_site(&self, sid: u32) -> Option<u32> {
        self.st.class_new_sites.get(&sid).copied()
    }
    fn k_class_alloc(&mut self, class_id: u32) -> Value {
        let id = self.st.class_alloc(class_id);
        // CLASS-HEAP-CAP: the heap is never garbage-collected, so an unbounded
        // `new()` in a loop would grow without limit. Bound it to a loud fatal
        // (graceful $finish) instead of an OOM — this is the single allocation
        // chokepoint (`class_alloc`'s only caller).
        if self.st.class_heap.borrow().len() as u64 > self.st.max_class_objs {
            self.st.fatal_class_limit();
        }
        // The handle holds the object-id as a 32-bit unsigned integer (0 = null).
        Value::from_i128(id as i128, 32, false)
    }
}

impl Scheduler<'_, '_> {
    /// Mark every continuous assign that DRIVES `net` dirty, so the next settle
    /// re-evaluates it. The `release` half of the fix documented at
    /// `SimState::drivers_of_net`; tier-3 has the same two lines against its own
    /// worklist.
    pub(crate) fn redirty_drivers_of(&mut self, net: u32) {
        for ci in self.st.drivers_of_net(net) {
            let i = ci as usize;
            if i < self.st.ca_dirty_flag.len() && !self.st.ca_dirty_flag[i] {
                self.st.ca_dirty_flag[i] = true;
                self.st.ca_dirty.push(ci);
            }
        }
    }
}
