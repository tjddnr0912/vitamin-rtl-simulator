//! split part of `builtins` (mechanical move).

use super::*;

/// Split a postponed print's args when the call site is `$fmonitor`/`$fstrobe`.
///
/// Those share the frozen `Monitor`/`Strobe` ids, so the ONLY thing that marks
/// `args[0]` as a descriptor is `file_directed_stmts` (written by elaborate from the
/// dollar-name). The fd is evaluated HERE, at registration, for the same reason
/// `time_mult` and `scope` are captured here: by the time the postponed flush renders,
/// the fd variable may have been reassigned or the file closed.
///
/// Returns `(None, args)` unchanged for every plain `$monitor`/`$strobe`.
fn split_file_directed<'a>(
    sched: &Scheduler,
    sid: u32,
    args: &'a [u32],
) -> (Option<u32>, &'a [u32]) {
    if !sched.st.file_directed_stmts.contains(&sid) {
        return (None, args);
    }
    let fd = args
        .first()
        .map(|&a| sched.eval(a))
        .filter(|v| !v.has_xz())
        .and_then(|v| v.to_u64())
        .map(|v| v as u32);
    // An unusable descriptor still consumes `args[0]` — printing it as a value would be
    // worse than the `bad_fd_warn` the flush emits.
    (Some(fd.unwrap_or(u32::MAX)), args.get(1..).unwrap_or(&[]))
}

/// Where a FUNNEL-OUTSIDE task write goes (A1-iii).
///
/// Three task ids write a net from inside their own dispatch rather than through
/// the statement's lvalue: `$sformat` renders into its destination, `$readmem*`
/// fills a memory element by element, and the `$cast` TASK form writes `dst`.
/// Every one of them called `sched.st.write_lvalue` — the ENGINE's nets, the one
/// store a native run never writes — and the only thing that kept that from being
/// a live silent-wrong was the `stmt_effect` reject row, since `systask_refusal`
/// lets all three through.
///
/// The split is OPT-IN in the §4.5.314 sense: [`TaskWrites::Direct`] is literally
/// the call these sites made before, so the engine path is unchanged by
/// construction, and only tier-3 asks to collect.
///
/// ⚠️ A HEAP destination needs none of this — `write_lvalue` routes a heap-kind
/// net to `dyn_heap` by net id, and that object is shared. `$s.itoa(v)` and a
/// `string`-destination `$sformat` were therefore already correct; what was wrong
/// is a FLAT destination, which is where the arena and `SimState` differ.
pub(crate) enum TaskWrites<'a> {
    /// The engine: straight through `SimState`, exactly as before.
    Direct,
    /// Tier-3: collect, because only the caller holds the funnel that reaches the
    /// store it owns. Drained by `NativeKernel::k_dispatch_systask`.
    Collect(&'a mut Vec<(sim_ir::Lvalue, Value, crate::exec::Offsets)>),
}

impl TaskWrites<'_> {
    /// The ONE place a funnel-outside task write is spelled.
    pub(crate) fn put(
        &mut self,
        sched: &mut Scheduler,
        lv: sim_ir::Lvalue,
        v: Value,
        off: crate::exec::Offsets,
    ) {
        match self {
            // `write_lvalue` returns "changed"; no site here consulted it.
            TaskWrites::Direct => {
                sched.st.write_lvalue(&lv, v, &off);
            }
            TaskWrites::Collect(buf) => buf.push((lv, v, off)),
        }
    }
}

pub(crate) fn dispatch(
    sched: &mut Scheduler,
    which: SysTaskId,
    fmt: Option<u32>,
    args: &[u32],
    sid: u32,
) -> Ctl {
    dispatch_with(
        sched,
        None::<&crate::SimState>,
        &mut TaskWrites::Direct,
        which,
        fmt,
        args,
        sid,
    )
}

/// `dispatch` against an ALTERNATE net store (tier-3, S1d-4b-2).
///
/// `nets = None` is the engine's own state and every arm below then reduces to
/// the call it made before, so the engine path is unchanged by construction.
/// `Some(&arena)` is tier-3: the format engine reads net VALUES from the arena
/// while everything else — the file table, the output sink, `$time`, the RNG,
/// the format cache, the assertion-control side tables — comes from `sched`,
/// which is where those live for both backends.
///
/// ⚠️ SCOPE. Only the RENDER path takes the alternate store. Arms that read the
/// store directly rather than through the formatter (`$dumpvars`'s full
/// snapshot, `$writemem*`'s memory read, `$dumpfile`/`$fclose`/`$dumplimit`'s
/// arguments, `$timeformat`'s non-literal arguments) still read `sched.st`, and
/// tier-3 designs reaching them would render from the wrong store. They are not
/// silently accepted: `native::design_eligibility` does not refuse them today,
/// so `k_dispatch_systask` REFUSES the task ids that reach them rather than
/// dispatching — see its arm list.
pub(crate) fn dispatch_with<N: crate::eval::NetReader + ?Sized>(
    sched: &mut Scheduler,
    nets: Option<&N>,
    out: &mut TaskWrites<'_>,
    which: SysTaskId,
    fmt: Option<u32>,
    args: &[u32],
    sid: u32,
) -> Ctl {
    // SVA-REST assertion control. A `$assertoff`/`$asserton`/`$assertkill` site is a
    // no-op `Display` whose StmtId is in `assert_ctl`: flip the global enable instead
    // of printing. A gated assertion FIRE (`assert_fire`) is SUPPRESSED while disabled
    // (no diag, no exit-class bump). Both checked before the deferred/severity paths.
    if let Some(&kind) = sched.st.assert_ctl.get(&sid) {
        // 0 = off, 1 = on, 2 = kill (kill = off; the gate prevents fires while
        // disabled — in-flight pipeline regs persist but cannot report).
        sched.st.assert_disabled = kind != 1;
        return Ctl::Continue;
    }
    if sched.st.assert_disabled && sched.st.assert_fire.contains(&sid) {
        return Ctl::Continue;
    }
    // §16.4 DEFERRED immediate assertion: a flush MARKER (cancel prior pending
    // report) or a deferred ACTION (enqueue for Observed/Reactive maturation) is
    // intercepted here and does NOT fire inline. Bypassed while the engine is
    // maturing a captured action (then it re-dispatches for real, below).
    if sched.try_defer(which, fmt, args, sid) {
        return Ctl::Continue;
    }
    // P1-1: `$fatal`/`$error`/`$warning`/`$info` lower as `Display` plus an
    // out-of-band severity entry keyed by StmtId — intercept BEFORE the normal
    // stdout print so the text reaches the DIAGNOSTIC stream only (doc-13).
    if let Some(sev) = sched.st.severities.get(&sid).copied() {
        return crate::builtins::run_severity_with(sched, nets, sev, fmt, args);
    }
    // §21.3.2: a `$timeformat` call is a no-op `Display` whose StmtId is in
    // `timeformat_stmts` (the assert_ctl pattern) — update the live `%t` format
    // state instead of printing. Args are evaluated HERE, at execution time
    // (runtime-variable args are legal; iverilog-pinned).
    if sched.st.timeformat_stmts.contains(&sid) {
        return crate::builtins::run_timeformat_with(sched, nets, args);
    }
    // OBS-3: a `$vita_stage("label", vals…)` call is a no-op `Display` whose StmtId is
    // in `stage_stmts` — NEVER print; instead (under `+STAGE_TRACE`) append a
    // `stage.jsonl` line. Args are evaluated HERE at execution time.
    if sched.st.stage_stmts.contains(&sid) {
        return run_vita_stage(sched, args);
    }
    // §7.10 whole-handle copy `dst = src`: a no-op Display whose StmtId maps to
    // (dst, src) — DEEP-clone the src heap object (VALUE semantics: later
    // writes to either side never show through; iverilog-pinned for dyn/queue,
    // hand-IEEE for assoc). A never-touched src slot (None) copies as empty.
    if let Some(&(dst, src)) = sched.st.handle_copy_stmts.get(&sid) {
        let obj = sched
            .st
            .dyn_heap
            .borrow()
            .get(src as usize)
            .and_then(|o| o.as_ref().cloned());
        if let Some(slot) = sched.st.dyn_heap.borrow_mut().get_mut(dst as usize) {
            *slot = obj;
        }
        // §7.10.2: a whole-assign into a BOUNDED queue truncates to the bound
        // (+W4020), exactly like the push/insert post-op — without this the
        // clone silently overfills a `[$:N]` dst (R1 both-lens converged
        // finding; a no-op for unbounded queues and non-queue kinds).
        sched.st.enforce_queue_bound(dst);
        return Ctl::Continue;
    }
    // §7.10.1 queue slice `dst = src[a:b]`: args = [dst, src, a, b]. Bounds are
    // runtime i64; partial out-of-range CLAMPS, a reversed range / fully-out /
    // x-z bound yields the EMPTY queue (hand-IEEE — Icarus mis-executes this
    // form). The subrange is cloned BEFORE the dst slot is written, so a
    // self-slice `q = q[a:b]` is safe.
    if sched.st.queue_slice_stmts.contains(&sid) {
        return run_queue_slice(sched, nets, args);
    }
    // P1-5: the b/o/h variants change the default radix of unformatted args.
    let radix = sched.st.radixes.get(&sid).copied();
    match which {
        // v5 (C)-③: dyn-array object methods. args[0] is always the HANDLE's
        // Signal expr (elaborate contract); a malformed handle is a defensive
        // no-op, never a panic.
        SysTaskId::DynNew => {
            let Some(net) = dyn_handle_net(sched, args.first()) else {
                return Ctl::Continue;
            };
            // `new[]` is dyn-array syntax: acting on a queue/assoc handle
            // would put a kind-mismatched object in the heap — defensive
            // warn+ignore (elaborate never emits it).
            if sched.st.ir.nets.get(net as usize).map(|nv| nv.kind)
                != Some(sim_ir::NetKind::DynArray)
            {
                dyn_warn_once(sched, net, "new[] on a non-dynamic-array handle (ignored)");
                return Ctl::Continue;
            }
            // n: X/Z degrades to EMPTY + warn-once; an explicit 0 is
            // legal-silent (IEEE §7.5.1). Cap at the static array cap class —
            // a huge n is a t-runtime OOM hazard exactly like P2-6.
            let nv = args
                .get(1)
                .map(|&a| crate::builtins::eval_task_arg(sched, nets, a));
            let n = match nv {
                Some(v) if v.has_xz() => {
                    dyn_warn_once(sched, net, "new[] size is X/Z; array degraded to empty");
                    0
                }
                Some(v) => {
                    // Same cap class as elaborate's MAX_ARRAY_LEN (P2-6): a
                    // runtime OOM is as silent-deadly as the t0 one. NO silent
                    // caps — a clamped n warns (once per net).
                    let raw = v.to_u64().unwrap_or(0);
                    if raw > crate::state::MAX_DYN_ELEMS as u64 {
                        dyn_warn_once(
                            sched,
                            net,
                            "new[] size exceeds the element cap (1<<24); clamped",
                        );
                    }
                    raw.min(crate::state::MAX_DYN_ELEMS as u64) as usize
                }
                None => 0,
            };
            // §4.5.194: the alloc core (elem default + `new[n](src)` copy + heap store)
            // is shared with the `&self` frame executor path (`frame_dyn_new`).
            let src_net = dyn_handle_net(sched, args.get(2));
            sched.st.alloc_dyn_array(net, n, src_net);
            Ctl::Continue
        }
        SysTaskId::DynDelete => {
            if let Some(net) = dyn_handle_net(sched, args.first()) {
                sched.st.dyn_heap.borrow_mut()[net as usize].take(); // absent entry IS the empty object
            }
            Ctl::Continue
        }
        // ⓑ-breadth (v16): array ordering methods — in-place mutators on an
        // ORDERED collection. A missing heap entry IS the empty array (no-op).
        SysTaskId::ArrSort | SysTaskId::ArrRsort | SysTaskId::ArrReverse => {
            let Some(net) = dyn_handle_net(sched, args.first()) else {
                return Ctl::Continue;
            };
            let signed = sched
                .st
                .ir
                .nets
                .get(net as usize)
                .map(|nv| nv.signed)
                .unwrap_or(true);
            let mut bad_kind = false;
            {
                let mut heap = sched.st.dyn_heap.borrow_mut();
                if let Some(obj) = heap.get_mut(net as usize).and_then(|o| o.as_mut()) {
                    match obj {
                        crate::state::DynObj::DynArray { elems } => {
                            apply_order(elems.as_mut_slice(), which, signed)
                        }
                        crate::state::DynObj::Queue { elems } => {
                            apply_order(elems.make_contiguous(), which, signed)
                        }
                        _ => bad_kind = true,
                    }
                }
            }
            if bad_kind {
                dyn_warn_once(
                    sched,
                    net,
                    "ordering method on a non-ordered handle (ignored)",
                );
            }
            Ctl::Continue
        }
        // ⓑ-breadth (v17): locator methods returning a queue (min/max/unique/
        // find*). args = [dst, src, kind_const, with_pred?]. Snapshot the source,
        // compute the result vector, write the dst handle as a fresh queue.
        SysTaskId::ArrLocator => {
            arr_locator(sched, args);
            Ctl::Continue
        }
        // N7-REST: `obj.randomize()` — draw the receiver's rand fields per the folded
        // constraint bounds and write them into the heap object. Deterministic
        // (seeded `dist_uniform`); a null/X handle is a no-op.
        SysTaskId::ClassRandomize => {
            class_randomize(sched, args);
            Ctl::Continue
        }
        // v5 (C)-④: queue pushes. args = [handle, value]; the value is CAST
        // to the element type with assignment semantics (§5.5: evaluate at
        // max(element, self) width with the SOURCE's signedness, then truncate
        // — `push_back(300)` into a byte queue stores 44; iverilog live).
        SysTaskId::QPushBack | SysTaskId::QPushFront => {
            let Some(net) = dyn_handle_net(sched, args.first()) else {
                return Ctl::Continue;
            };
            let Some((w, kind)) = sched
                .st
                .ir
                .nets
                .get(net as usize)
                .map(|nv| (nv.width.max(1), nv.kind))
            else {
                return Ctl::Continue;
            };
            if kind != sim_ir::NetKind::Queue {
                dyn_warn_once(sched, net, "queue push on a non-queue handle (ignored)");
                return Ctl::Continue;
            }
            // `coerce_dyn_elem`, not a bare `.resize(w)` — a string element must keep
            // its byte string (see the funnel's doc; a private resize here is what made
            // `string q[$]` read back empty).
            let v = match args.get(1) {
                Some(&a) => {
                    let sw = sched.st.wt.get(a);
                    let raw = crate::builtins::eval_task_arg_ctx(
                        sched,
                        nets,
                        a,
                        w.max(sw.width),
                        sw.signed,
                    );
                    sched.st.coerce_dyn_elem(net, &raw, w)
                }
                None => Value::xs(w, false),
            };
            // Cap BEFORE taking the entry borrow (the warn needs `&mut sched`).
            // No silent caps (P2-6 class): a runaway push loop is a runtime
            // OOM hazard — warn (once per net) and DROP the push.
            let len = sched
                .st
                .dyn_heap
                .borrow()
                .get(net as usize)
                .and_then(|o| o.as_ref())
                .map(|o| o.len())
                .unwrap_or(0);
            if len >= crate::state::MAX_DYN_ELEMS {
                dyn_warn_once(
                    sched,
                    net,
                    "queue exceeds the element cap (1<<24); push dropped",
                );
                return Ctl::Continue;
            }
            // A missing entry IS the empty queue (lazy, like every dyn object).
            sched.st.with_dyn_entry(
                net,
                || crate::state::DynObj::Queue {
                    elems: std::collections::VecDeque::new(),
                },
                |obj| {
                    if let crate::state::DynObj::Queue { elems } = obj {
                        if which == SysTaskId::QPushFront {
                            elems.push_front(v);
                        } else {
                            elems.push_back(v);
                        }
                    }
                },
            );
            sched.st.enforce_queue_bound(net); // v6 ③ (no-op when unbounded)
            Ctl::Continue
        }
        // v6: queue `.insert(i, v)` / `.delete(i)` — iverilog live (2026-06-11):
        // insert shifts right, `insert(size, v)` APPENDS, OOB/X index = warn +
        // no-op; delete(i) erases one, OOB/X = warn + skip.
        SysTaskId::QInsert | SysTaskId::QDeleteIdx => {
            let Some(net) = dyn_handle_net(sched, args.first()) else {
                return Ctl::Continue;
            };
            let Some((w, kind)) = sched
                .st
                .ir
                .nets
                .get(net as usize)
                .map(|nv| (nv.width.max(1), nv.kind))
            else {
                return Ctl::Continue;
            };
            if kind != sim_ir::NetKind::Queue {
                dyn_warn_once(
                    sched,
                    net,
                    "queue insert/delete on a non-queue handle (ignored)",
                );
                return Ctl::Continue;
            }
            // The index: X/Z (or beyond-u64 wide) → invalid; a NEGATIVE int
            // evaluates to a huge unsigned here and lands in the same OOB arm
            // (warn + no-op) — identical surface either way.
            let idx = args
                .get(1)
                .and_then(|&a| crate::builtins::eval_task_arg(sched, nets, a).to_u64());
            let len = sched
                .st
                .dyn_heap
                .borrow()
                .get(net as usize)
                .and_then(|o| o.as_ref())
                .map(|o| o.len())
                .unwrap_or(0);
            if which == SysTaskId::QInsert {
                let ok = matches!(idx, Some(i) if i <= len as u64);
                if !ok {
                    dyn_warn_once(
                        sched,
                        net,
                        "queue insert index out of range or X (not inserted)",
                    );
                    return Ctl::Continue;
                }
                if len >= crate::state::MAX_DYN_ELEMS {
                    dyn_warn_once(
                        sched,
                        net,
                        "queue exceeds the element cap (1<<24); insert dropped",
                    );
                    return Ctl::Continue;
                }
                // Element cast = the push recipe (§5.5 assignment semantics), through
                // the same funnel so a string element is not resized away.
                let v = match args.get(2) {
                    Some(&a) => {
                        let sw = sched.st.wt.get(a);
                        let raw = crate::builtins::eval_task_arg_ctx(
                            sched,
                            nets,
                            a,
                            w.max(sw.width),
                            sw.signed,
                        );
                        sched.st.coerce_dyn_elem(net, &raw, w)
                    }
                    None => Value::xs(w, false),
                };
                sched.st.with_dyn_entry(
                    net,
                    || crate::state::DynObj::Queue {
                        elems: std::collections::VecDeque::new(),
                    },
                    |obj| {
                        if let crate::state::DynObj::Queue { elems } = obj {
                            elems.insert(idx.unwrap_or(0) as usize, v);
                        }
                    },
                );
                sched.st.enforce_queue_bound(net); // v6 ③ (no-op when unbounded)
            } else {
                let ok = matches!(idx, Some(i) if i < len as u64);
                if !ok {
                    dyn_warn_once(sched, net, "queue delete index out of range or X (skipped)");
                    return Ctl::Continue;
                }
                if let Some(crate::state::DynObj::Queue { elems }) = sched
                    .st
                    .dyn_heap
                    .borrow_mut()
                    .get_mut(net as usize)
                    .and_then(|o| o.as_mut())
                {
                    elems.remove(idx.unwrap_or(0) as usize);
                }
            }
            Ctl::Continue
        }
        // v5 ⑤: `a.delete(k)` — args = [handle, key]. A MISSING key is a
        // SILENT no-op (IEEE §7.9); an X/Z key warns (invalid index, §7.8.6);
        // a non-assoc handle warns (hand-built IR only — ⑥ type-checks).
        SysTaskId::AssocDeleteKey => {
            if let Some(net) = dyn_handle_net(sched, args.first()) {
                let kind = sched.st.ir.nets.get(net as usize).map(|nv| nv.kind);
                // v6: the string-keyed twin shares the SysTask — dispatch on
                // the handle's key domain.
                if kind == Some(sim_ir::NetKind::AssocStr) {
                    match args
                        .get(1)
                        .and_then(|&k| crate::builtins::assoc_str_key_arg(sched, nets, k))
                    {
                        None => dyn_warn_once(sched, net, "assoc delete key is X/Z (ignored)"),
                        Some(k) => {
                            if let Some(crate::state::DynObj::AssocStr { map }) = sched
                                .st
                                .dyn_heap
                                .borrow_mut()
                                .get_mut(net as usize)
                                .and_then(|o| o.as_mut())
                            {
                                map.remove(&k);
                            }
                        }
                    }
                    return Ctl::Continue;
                }
                if kind != Some(sim_ir::NetKind::Assoc) {
                    dyn_warn_once(sched, net, "assoc delete on a non-assoc handle (ignored)");
                    return Ctl::Continue;
                }
                match args
                    .get(1)
                    .and_then(|&k| crate::builtins::assoc_key_arg(sched, nets, k))
                {
                    None => dyn_warn_once(sched, net, "assoc delete key is X/Z (ignored)"),
                    Some(k) => {
                        if let Some(crate::state::DynObj::Assoc { map }) = sched
                            .st
                            .dyn_heap
                            .borrow_mut()
                            .get_mut(net as usize)
                            .and_then(|o| o.as_mut())
                        {
                            map.remove(&k);
                        }
                    }
                }
            }
            Ctl::Continue
        }
        SysTaskId::Display => {
            let mut s = crate::builtins::render_task_args(sched, nets, fmt, args, radix);
            s.push('\n');
            write_out(sched.st, &s);
            Ctl::Continue
        }
        SysTaskId::Write => {
            let s = crate::builtins::render_task_args(sched, nets, fmt, args, radix);
            write_out(sched.st, &s);
            Ctl::Continue
        }
        // $strobe: REGISTER a postponed capture (does NOT print now). It is
        // rendered with settled end-of-timestep values at `flush_postponed`,
        // then cleared (one-shot per call). Multiple strobes in one step print
        // in call order (FIFO push).
        SysTaskId::Strobe => {
            let time_mult = sched.st.cur_time_mult;
            // `$fstrobe`: `args[0]` is the descriptor (see `file_directed_stmts`), so it
            // is consumed here and the remaining args are the value list.
            let (fd, args) = split_file_directed(sched, sid, args);
            sched.st.postponed.strobes.push(FmtCapture {
                fmt,
                args: args.to_vec(),
                time_mult,
                radix,
                scope: sched.st.cur_scope.clone(),
                fd,
            });
            Ctl::Continue
        }
        // $monitor: REPLACE the global singleton (IEEE: at most one active
        // monitor in the whole sim). `last_vals = None` forces an establishment
        // print at the next postponed flush of THIS timestep, seeding the
        // baseline value list.
        SysTaskId::Monitor => {
            let time_mult = sched.st.cur_time_mult;
            let (fd, args) = split_file_directed(sched, sid, args);
            let ms = MonitorState {
                cap: FmtCapture {
                    fmt,
                    args: args.to_vec(),
                    time_mult,
                    radix,
                    scope: sched.st.cur_scope.clone(),
                    fd,
                },
                last_vals: None,
            };
            // One monitor per DESTINATION: `$fmonitor` replaces the monitor for its own
            // descriptor and leaves a standing `$monitor` (stdout) alone.
            match fd {
                Some(d) => {
                    sched.st.postponed.file_monitors.insert(d, ms);
                }
                None => sched.st.postponed.monitor = Some(ms),
            }
            // v9 rank 6: (re-)establishing a monitor does NOT touch the global
            // enable flag — a standing `$monitoroff` persists across re-`$monitor`
            // (the establishment line still prints, see the flush). So this does
            // NOT reset `monitor_disabled`.
            Ctl::Continue
        }
        SysTaskId::Finish => Ctl::Finish,
        SysTaskId::Stop => Ctl::Stop,
        SysTaskId::DumpFile => {
            let name = arg_string_with(sched.st, nets, args.first().copied());
            sched.st.dump_pending_path = Some(name);
            Ctl::Continue
        }
        SysTaskId::DumpVars => {
            // `nets` is already threaded into this function, so the tier-3 path
            // needs no arm of its own. It briefly had one, in `k_dispatch_systask`,
            // and that was a twin: any future change here would have been
            // silently skipped on the native backend.
            dumpvars_with(sched.st, nets, args);
            Ctl::Continue
        }
        SysTaskId::DumpOff => {
            if let Some(w) = sched.st.vcd.as_mut() {
                let _ = w.set_time(sched.st.now);
                let _ = w.dump_off();
            }
            sched.st.dumping = false;
            Ctl::Continue
        }
        SysTaskId::DumpOn => {
            dump_on(sched.st);
            Ctl::Continue
        }
        SysTaskId::DumpAll => {
            dump_all(sched.st);
            Ctl::Continue
        }
        SysTaskId::DumpFlush => {
            // IEEE §21.7.2.4: push buffered VCD bytes to the OS now (crash-safe
            // checkpoints for long runs). Errors surface at finalize (W4019).
            if let Some(w) = sched.st.vcd.as_mut() {
                let _ = w.flush();
            }
            Ctl::Continue
        }
        SysTaskId::DumpLimit => {
            // IEEE §21.7.2.5: byte budget; the writer emits a one-time
            // `$comment Dump limit reached $end` and drops further records.
            // X/Z or missing size → no-op (no budget installed).
            let size = args
                .first()
                .and_then(|&a| sched.eval(a).to_u64())
                .unwrap_or(0);
            if size > 0 {
                if let Some(w) = sched.st.vcd.as_mut() {
                    w.set_limit(size);
                }
            }
            Ctl::Continue
        }
        // v7 file I/O. args[0] = descriptor; fmt/args render like $display.
        SysTaskId::Fdisplay | SysTaskId::Fwrite => {
            let fd = args
                .first()
                .map(|&a| crate::builtins::eval_task_arg(sched, nets, a))
                .filter(|v| !v.has_xz())
                .and_then(|v| v.to_u64())
                .map(|v| v as u32);
            let mut text = crate::builtins::render_task_args(
                sched,
                nets,
                fmt,
                args.get(1..).unwrap_or(&[]),
                radix,
            );
            if matches!(which, SysTaskId::Fdisplay) {
                text.push('\n');
            }
            match fd {
                Some(fd) => file_write(sched, fd, &text),
                None => bad_fd_warn(sched, u32::MAX),
            }
            Ctl::Continue
        }
        SysTaskId::Fclose => {
            let fd = args
                .first()
                .map(|&a| sched.eval(a))
                .filter(|v| !v.has_xz())
                .and_then(|v| v.to_u64())
                .map(|v| v as u32);
            match fd {
                // §21.3.4: the pre-opened STDIN/STDOUT/STDERR cannot be closed —
                // warn + no-op, the descriptor STAYS usable (iverilog-pinned:
                // "could not close file descriptor STDOUT", later writes print).
                Some(fd) if (0x8000_0000..=0x8000_0002).contains(&fd) => {
                    preopened_close_warn(sched, fd);
                }
                // fd form: drop the File (flush+close on Drop).
                Some(fd) if fd & 0x8000_0000 != 0 => {
                    if sched.st.files.remove(&fd).is_none() {
                        bad_fd_warn(sched, fd);
                    }
                    // FD-RECLAIM: drop this fd's auxiliary bookkeeping so a long
                    // open/close cycle doesn't leak read_state/readable_fds, and
                    // the bad-fd latch resets. fd numbers stay monotonic, so this
                    // is pure hygiene (no observable output change).
                    sched.st.read_state.remove(&fd);
                    sched.st.readable_fds.remove(&fd);
                    sched.st.bad_fd_warned.remove(&fd);
                }
                // MCD form: close every set channel bit (bit 0 = stdout, kept).
                Some(mcd) => {
                    for bit in 1..31u32 {
                        if mcd & (1 << bit) != 0 {
                            sched.st.mcd_files.remove(&bit);
                        }
                    }
                }
                None => bad_fd_warn(sched, u32::MAX),
            }
            Ctl::Continue
        }
        SysTaskId::ReadmemB | SysTaskId::ReadmemH => {
            readmem(sched, nets, out, args, matches!(which, SysTaskId::ReadmemH));
            Ctl::Continue
        }
        // v7 P2-C `s.putc(i, c)` — the one string MUTATOR (in-place byte
        // write; OOB index or a NUL byte = silent no-op, IEEE §6.16.3).
        SysTaskId::StrPutC => {
            let net = args
                .first()
                .and_then(|&a| match sched.st.ir.exprs.get(a as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                });
            let i = args
                .get(1)
                .and_then(|&a| crate::builtins::eval_task_arg(sched, nets, a).to_u64());
            let c = args
                .get(2)
                .and_then(|&a| crate::builtins::eval_task_arg(sched, nets, a).to_u64());
            if let (Some(net), Some(i), Some(c)) = (net, i, c) {
                // R23: `str_putc` routes by where this string's bytes live. Writing
                // `dyn_heap[net]` here unconditionally missed a FRAME-LOCAL `string`,
                // whose bytes are slab-stored in the frame slot — `s[0] = 65` inside a
                // `task automatic` silently did nothing at exit 0.
                sched.st.str_putc(net, i, (c & 0xff) as u8);
            }
            Ctl::Continue
        }
        // ⓑ-breadth (v18): number→string conversions (IEEE §6.16.14-17). Render
        // the value argument in the requested base (minimal form, no leading
        // zeros; itoa signed-decimal, the rest the unsigned bit pattern) and
        // OVERWRITE the string handle.
        SysTaskId::StrItoa | SysTaskId::StrHextoa | SysTaskId::StrOcttoa | SysTaskId::StrBintoa => {
            let net = args
                .first()
                .and_then(|&a| match sched.st.ir.exprs.get(a as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                });
            if let Some(net) = net {
                let v = args
                    .get(1)
                    .map(|&a| crate::builtins::eval_task_arg(sched, nets, a))
                    .unwrap_or(Value::xs(32, true));
                let text = match which {
                    SysTaskId::StrItoa => {
                        let sv = if v.signed {
                            v.to_i128_signed().unwrap_or(0)
                        } else {
                            v.to_u128().unwrap_or(0) as i128
                        };
                        format!("{sv}")
                    }
                    SysTaskId::StrHextoa => format!("{:x}", v.to_u128().unwrap_or(0)),
                    SysTaskId::StrOcttoa => format!("{:o}", v.to_u128().unwrap_or(0)),
                    _ => format!("{:b}", v.to_u128().unwrap_or(0)),
                };
                let sv = Value::from_str_bytes(text.as_bytes());
                let lv = sim_ir::Lvalue {
                    chunks: vec![sim_ir::LvalChunk {
                        net,
                        word: None,
                        offset: None,
                        width: None,
                        kind: sim_ir::SelKind::Bit,
                    }],
                };
                let off = sched.resolve_lvalue_offsets(&lv);
                sched.st.write_lvalue(&lv, sv, &off);
            }
            Ctl::Continue
        }
        // v7 P2-C `$sformat(dest, fmt, args…)` — renders through the SAME
        // format engine and writes dest (string net = byte store; packed =
        // the normal funnel with §6.16 conversion).
        SysTaskId::Sformat => {
            let text = crate::builtins::render_task_args(
                sched,
                nets,
                fmt,
                args.get(1..).unwrap_or(&[]),
                radix,
            );
            let dest = args
                .first()
                .and_then(|&a| match sched.st.ir.exprs.get(a as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                });
            if let Some(net) = dest {
                let v = Value::from_str_bytes(text.as_bytes());
                let lv = sim_ir::Lvalue {
                    chunks: vec![sim_ir::LvalChunk {
                        net,
                        word: None,
                        offset: None,
                        width: None,
                        kind: sim_ir::SelKind::Bit,
                    }],
                };
                let off = sched.resolve_lvalue_offsets(&lv);
                out.put(sched, lv, v, off);
            }
            Ctl::Continue
        }
        // v9 shape-bump placeholders: elaborate maps no task NAME to these yet
        // (they orphan-exist in the enum until Medium-bundle ranks 5-6 wire the
        // name→id mapping AND the engine semantics together), so this arm is
        // dead. A defensive no-op keeps the bump provably inert.
        // v9 (Medium-bundle rank 5): the write-side mirror of $readmem*.
        SysTaskId::WritememB | SysTaskId::WritememH => {
            writemem(sched, args, matches!(which, SysTaskId::WritememH));
            Ctl::Continue
        }
        // v9 rank 6: $monitoroff disables change-triggered reprints. The flag is
        // GLOBAL (sim-wide, not per-monitor) so it works even before any $monitor
        // and survives re-`$monitor` (IEEE 1364-2005 §17.1).
        SysTaskId::MonitorOff => {
            sched.st.postponed.monitor_disabled = true;
            Ctl::Continue
        }
        // v9 rank 6: $monitoron re-enables AND forces a reprint of the current
        // values at the next postponed flush by clearing the baseline (None ⇒
        // "establishment" ⇒ print regardless of change), independent of whether a
        // monitor is currently established.
        SysTaskId::MonitorOn => {
            sched.st.postponed.monitor_disabled = false;
            if let Some(m) = sched.st.postponed.monitor.as_mut() {
                m.last_vals = None;
            }
            // The flag is sim-wide, so the forced reprint covers every destination.
            for m in sched.st.postponed.file_monitors.values_mut() {
                m.last_vals = None;
            }
            Ctl::Continue
        }
        // v9 rank 6: $cast TASK form `$cast(dst, src);` — write resized src into
        // dst (no status). The func form `ok = $cast(...)` is a direct-rhs
        // intercept (k_cast). Hand-IEEE §6.24.2 (iverilog 13.0 rejects $cast):
        // an integral cast always succeeds in this class-free subset.
        SysTaskId::Cast => {
            cast_task(sched, nets, out, args);
            Ctl::Continue
        }
    }
}

/// v9 rank 6: the `$cast(dst, src);` TASK form — assign the resized `src` into
/// the whole-net `dst` ref arg (the func-form mirror, minus the status return).
/// N7-REST randomize() body: resolve the receiver object, draw each rand field per
/// its folded `[lo, hi]` bound (`dist_uniform` when `ranged`, else a full-width
/// seeded draw), and write the values back into the heap object. A null/X handle is
/// a no-op (IEEE §18.6 — randomize on a null handle is illegal; here it is benign).
pub(crate) fn class_randomize(sched: &mut Scheduler, args: &[u32]) {
    let success = class_randomize_run(sched, args);
    // IEEE 1800 §18.11: randomize() returns 1 on success, 0 on failure. When the
    // call captured a result (`r = obj.randomize()`), elaborate passes the result
    // status net as args[1]; write the verdict there (was hardcoded to 1, so a
    // failed/unsatisfiable/null randomize silently reported success).
    if let Some(&status_e) = args.get(1) {
        let net = match sched.st.ir.exprs.get(status_e as usize) {
            Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
            _ => None,
        };
        if let Some(net) = net {
            let lv = sim_ir::Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net,
                    word: None,
                    offset: None,
                    width: None,
                    kind: sim_ir::SelKind::Bit,
                }],
            };
            let v = Value::from_packed(
                &sim_ir::BitPacked {
                    val: vec![success as u64],
                    unk: vec![0],
                },
                32,
                false,
            );
            let off = sched.resolve_lvalue_offsets(&lv);
            sched.st.write_lvalue(&lv, v, &off);
        }
    }
}

/// N7-REST B-CRV final: resolve the inline-`with` per-call constraints for a
/// `ClassRandomize`. `args[0]` is the handle (Signal) and an optional status is a
/// Signal too; the with-id is the lone `Const` arg. None ⇒ a plain randomize().
pub(crate) fn inline_with_call(sched: &Scheduler, args: &[u32]) -> Option<crate::RandWithCall> {
    for &e in args.get(1..)? {
        if let Some(sim_ir::Expr::Const { val }) = sched.st.ir.exprs.get(e as usize) {
            let idx = sched
                .st
                .ir
                .consts
                .get(*val as usize)
                .and_then(|c| c.bits.val.first().copied())
                .unwrap_or(0) as usize;
            return sched.st.randomize_with.get(idx).cloned();
        }
    }
    None
}
