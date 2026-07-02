//! $-task handlers (inlined for v1; HOOK: extract to hdl-builtins post-v1).
//! Handles $dumpfile/$dumpvars/$dumpoff/$dumpon/$dumpall → vcd-writer,
//! $display/$write/$monitor/$strobe formatting → stdout sink, $finish/$stop.

use std::io::Write;

use sim_ir::SysTaskId;
use vcd_writer::{IdCode, ScopeType};

use crate::eval::NetReader;
use crate::sched::Scheduler;
use crate::state::{vcd_var_type, FmtCapture, MonitorState, SimState};
use crate::value::Value;

/// Control-flow signal back to the executor.
pub(crate) enum Ctl {
    Continue,
    Finish,
    Stop,
    /// Runtime `$fatal` (RunFatal): abort the run with `ExitClass::Fatal`.
    Fatal,
}

pub(crate) fn dispatch(
    sched: &mut Scheduler,
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
        return run_severity(sched, sev, fmt, args);
    }
    // §21.3.2: a `$timeformat` call is a no-op `Display` whose StmtId is in
    // `timeformat_stmts` (the assert_ctl pattern) — update the live `%t` format
    // state instead of printing. Args are evaluated HERE, at execution time
    // (runtime-variable args are legal; iverilog-pinned).
    if sched.st.timeformat_stmts.contains(&sid) {
        return run_timeformat(sched, args);
    }
    // §7.10 whole-handle copy `dst = src`: a no-op Display whose StmtId maps to
    // (dst, src) — DEEP-clone the src heap object (VALUE semantics: later
    // writes to either side never show through; iverilog-pinned for dyn/queue,
    // hand-IEEE for assoc). A never-touched src slot (None) copies as empty.
    if let Some(&(dst, src)) = sched.st.handle_copy_stmts.get(&sid) {
        let obj = sched
            .st
            .dyn_heap
            .get(src as usize)
            .and_then(|o| o.as_ref().cloned());
        if let Some(slot) = sched.st.dyn_heap.get_mut(dst as usize) {
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
        return run_queue_slice(sched, args);
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
            let nv = args.get(1).map(|&a| sched.eval(a));
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
            let (w, signed) = sched
                .st
                .ir
                .nets
                .get(net as usize)
                .map(|nv| (nv.width.max(1), nv.signed))
                .unwrap_or((1, false));
            // IEEE 1800-2017 §7.5.2: each `new[]` element takes its type's
            // default — 0 for 2-state element types (int/bit/byte/shortint/
            // longint), X for 4-state. The per-net `two_state` flag carries the
            // element type's 2-state-ness; honoring it here keeps dyn arrays
            // consistent with scalar/fixed-unpacked/assoc defaults.
            let elem_default = if sched
                .st
                .two_state
                .get(net as usize)
                .copied()
                .unwrap_or(false)
            {
                Value::zeros(w, signed)
            } else {
                Value::xs(w, signed)
            };
            let mut elems = vec![elem_default; n];
            // copy form `new[n](src)`: prefix-copy from the src handle.
            if let Some(src_net) = dyn_handle_net(sched, args.get(2)) {
                if let Some(crate::state::DynObj::DynArray { elems: src }) = sched
                    .st
                    .dyn_heap
                    .get(src_net as usize)
                    .and_then(|o| o.as_ref())
                {
                    for (dst, s) in elems.iter_mut().zip(src.iter()) {
                        *dst = s.clone();
                    }
                }
            }
            sched.st.dyn_heap[net as usize] = Some(crate::state::DynObj::DynArray { elems });
            Ctl::Continue
        }
        SysTaskId::DynDelete => {
            if let Some(net) = dyn_handle_net(sched, args.first()) {
                sched.st.dyn_heap[net as usize].take(); // absent entry IS the empty object
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
            if let Some(obj) = sched
                .st
                .dyn_heap
                .get_mut(net as usize)
                .and_then(|o| o.as_mut())
            {
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
            let v = match args.get(1) {
                Some(&a) => {
                    let sw = sched.st.wt.get(a);
                    sched.eval_ctx_top(a, w.max(sw.width), sw.signed).resize(w)
                }
                None => Value::xs(w, false),
            };
            // Cap BEFORE taking the entry borrow (the warn needs `&mut sched`).
            // No silent caps (P2-6 class): a runaway push loop is a runtime
            // OOM hazard — warn (once per net) and DROP the push.
            let len = sched
                .st
                .dyn_heap
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
            let entry = sched.st.dyn_entry(net, || crate::state::DynObj::Queue {
                elems: std::collections::VecDeque::new(),
            });
            if let crate::state::DynObj::Queue { elems } = entry {
                if which == SysTaskId::QPushFront {
                    elems.push_front(v);
                } else {
                    elems.push_back(v);
                }
            }
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
            let idx = args.get(1).and_then(|&a| sched.eval(a).to_u64());
            let len = sched
                .st
                .dyn_heap
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
                // Element cast = the push recipe (§5.5 assignment semantics).
                let v = match args.get(2) {
                    Some(&a) => {
                        let sw = sched.st.wt.get(a);
                        sched.eval_ctx_top(a, w.max(sw.width), sw.signed).resize(w)
                    }
                    None => Value::xs(w, false),
                };
                let entry = sched.st.dyn_entry(net, || crate::state::DynObj::Queue {
                    elems: std::collections::VecDeque::new(),
                });
                if let crate::state::DynObj::Queue { elems } = entry {
                    elems.insert(idx.unwrap_or(0) as usize, v);
                }
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
                    match args.get(1).and_then(|&k| sched.assoc_str_key_of(k)) {
                        None => dyn_warn_once(sched, net, "assoc delete key is X/Z (ignored)"),
                        Some(k) => {
                            if let Some(crate::state::DynObj::AssocStr { map }) = sched
                                .st
                                .dyn_heap
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
                match args.get(1).and_then(|&k| sched.assoc_key_of(k)) {
                    None => dyn_warn_once(sched, net, "assoc delete key is X/Z (ignored)"),
                    Some(k) => {
                        if let Some(crate::state::DynObj::Assoc { map }) = sched
                            .st
                            .dyn_heap
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
            let mut s = format_args_str(sched, fmt, args, radix);
            s.push('\n');
            write_out(sched.st, &s);
            Ctl::Continue
        }
        SysTaskId::Write => {
            let s = format_args_str(sched, fmt, args, radix);
            write_out(sched.st, &s);
            Ctl::Continue
        }
        // $strobe: REGISTER a postponed capture (does NOT print now). It is
        // rendered with settled end-of-timestep values at `flush_postponed`,
        // then cleared (one-shot per call). Multiple strobes in one step print
        // in call order (FIFO push).
        SysTaskId::Strobe => {
            let time_mult = sched.st.cur_time_mult;
            sched.st.postponed.strobes.push(FmtCapture {
                fmt,
                args: args.to_vec(),
                time_mult,
                radix,
                scope: sched.st.cur_scope.clone(),
            });
            Ctl::Continue
        }
        // $monitor: REPLACE the global singleton (IEEE: at most one active
        // monitor in the whole sim). `last_vals = None` forces an establishment
        // print at the next postponed flush of THIS timestep, seeding the
        // baseline value list.
        SysTaskId::Monitor => {
            let time_mult = sched.st.cur_time_mult;
            sched.st.postponed.monitor = Some(MonitorState {
                cap: FmtCapture {
                    fmt,
                    args: args.to_vec(),
                    time_mult,
                    radix,
                    scope: sched.st.cur_scope.clone(),
                },
                last_vals: None,
            });
            // v9 rank 6: (re-)establishing a monitor does NOT touch the global
            // enable flag — a standing `$monitoroff` persists across re-`$monitor`
            // (the establishment line still prints, see the flush). So this does
            // NOT reset `monitor_disabled`.
            Ctl::Continue
        }
        SysTaskId::Finish => Ctl::Finish,
        SysTaskId::Stop => Ctl::Stop,
        SysTaskId::DumpFile => {
            let name = arg_string(sched, args.first().copied());
            sched.st.dump_pending_path = Some(name);
            Ctl::Continue
        }
        SysTaskId::DumpVars => {
            dumpvars(sched.st, args);
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
                .map(|&a| sched.eval(a))
                .filter(|v| !v.has_xz())
                .and_then(|v| v.to_u64())
                .map(|v| v as u32);
            let mut text = format_args_str(sched, fmt, args.get(1..).unwrap_or(&[]), radix);
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
            readmem(sched, args, matches!(which, SysTaskId::ReadmemH));
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
            let i = args.get(1).and_then(|&a| sched.eval(a).to_u64());
            let c = args.get(2).and_then(|&a| sched.eval(a).to_u64());
            if let (Some(net), Some(i), Some(c)) = (net, i, c) {
                let c = (c & 0xff) as u8;
                if c != 0 {
                    if let Some(crate::state::DynObj::Str { bytes }) = sched
                        .st
                        .dyn_heap
                        .get_mut(net as usize)
                        .and_then(|o| o.as_mut())
                    {
                        if let Some(slot) = bytes.get_mut(i as usize) {
                            *slot = c;
                        }
                    }
                }
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
                    .map(|&a| sched.eval(a))
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
            let text = format_args_str(sched, fmt, args.get(1..).unwrap_or(&[]), radix);
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
                sched.st.write_lvalue(&lv, v, &off);
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
            Ctl::Continue
        }
        // v9 rank 6: $cast TASK form `$cast(dst, src);` — write resized src into
        // dst (no status). The func form `ok = $cast(...)` is a direct-rhs
        // intercept (k_cast). Hand-IEEE §6.24.2 (iverilog 13.0 rejects $cast):
        // an integral cast always succeeds in this class-free subset.
        SysTaskId::Cast => {
            cast_task(sched, args);
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
fn class_randomize(sched: &mut Scheduler, args: &[u32]) {
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
fn inline_with_call(sched: &Scheduler, args: &[u32]) -> Option<crate::RandWithCall> {
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

/// Run the randomize() draw; returns whether it SUCCEEDED (§18.11). A null/X
/// handle fails (0); a class with no rand fields succeeds trivially (1); the
/// rejection sampler succeeds when it finds a satisfying assignment within the
/// cap, else fails (fields left unchanged).
fn class_randomize_run(sched: &mut Scheduler, args: &[u32]) -> bool {
    let Some(&handle_e) = args.first() else {
        return false;
    };
    let hv = sched.eval_ctx_top(handle_e, 32, false);
    if hv.unk.iter().any(|&u| u != 0) {
        return false; // X/Z handle → fail
    }
    let id = hv.val.first().copied().unwrap_or(0) as u32;
    if id == 0 {
        return false; // null handle → fail
    }
    let class_id = match sched.st.class_heap.borrow().get(&id) {
        Some(o) => o.class_id,
        None => return false,
    };
    let Some(mut rand) = sched.st.class_rand.get(class_id as usize).cloned() else {
        return true; // no rand table → nothing to randomize → success
    };
    if rand.is_empty() {
        return true; // no rand fields → trivially succeeds
    }
    let mut preds: Vec<Vec<sim_ir::COp>> = sched
        .st
        .class_constraints
        .get(class_id as usize)
        .cloned()
        .unwrap_or_default();
    // N7-REST B-CRV final: inline `randomize() with {…}` (IEEE §18.7) — the with-id
    // is a Const arg. Its per-call domain overrides INTERSECT the class `[lo,hi]`
    // (an empty intersection ⇒ infeasible ⇒ fail BEFORE drawing); its predicates
    // are ANDed with the class predicates. Absent ⇒ byte-identical to B2.
    if let Some((domains, extra)) = inline_with_call(sched, args) {
        for (fi, ilo, ihi) in domains {
            if let Some(b) = rand.iter_mut().find(|r| r.0 == fi) {
                b.3 = b.3.max(ilo);
                b.4 = b.4.min(ihi);
                b.5 = true; // constrained ⇒ draw within the narrowed [lo,hi]
                if b.3 > b.4 {
                    return false; // inline ∩ class is empty → randomize fails (§18.11)
                }
            }
        }
        preds.extend(extra);
    }
    let mut seed = sched.st.randomize_seed.get();
    // Rejection sampling (B2): draw every rand field from its `class_rand` domain,
    // keep the candidate only if every predicate holds. Draw order + the per-field
    // draw are IDENTICAL to B1, so a design with no general predicate accepts on the
    // FIRST try — byte-identical to B1. SOFT constraints (§18.5.14) are tried first
    // (phase 1 = hard+soft); if that is infeasible, the soft predicates are dropped
    // and only the hard ones retried (phase 2). The seed stream is consumed
    // deterministically → reproducible + 3-OS byte-identical.
    let dist: Vec<crate::DistField> = sched
        .st
        .class_dist
        .get(class_id as usize)
        .cloned()
        .unwrap_or_default();
    let randc: Vec<crate::RandcField> = sched
        .st
        .class_randc
        .get(class_id as usize)
        .cloned()
        .unwrap_or_default();
    // `randc` fields (cyclic, B2 v1 = unconstrained) are drawn SEPARATELY via a
    // per-instance permutation (a value visited once per cycle), not through the
    // rejection loop. The non-randc fields go through `try_solve`.
    let mut randc_updates: Vec<(u32, Value)> = Vec::new();
    for &(field_idx, lo, hi) in &randc {
        let (w, s) = rand
            .iter()
            .find(|r| r.0 == field_idx)
            .map(|r| (r.1, r.2))
            .unwrap_or((32, false));
        let v = draw_randc(
            &mut sched.st.randc_state,
            &mut seed,
            (id, field_idx),
            lo,
            hi,
            w,
            s,
        );
        randc_updates.push((field_idx, v));
    }
    let rand_rej: Vec<crate::RandBound> = rand
        .iter()
        .filter(|r| !randc.iter().any(|rc| rc.0 == r.0))
        .cloned()
        .collect();
    let has_soft = preds
        .iter()
        .any(|p| matches!(p.first(), Some(sim_ir::COp::SoftMarker)));
    let mut accepted = try_solve(&rand_rej, &preds, &dist, &mut seed, false);
    if accepted.is_none() && has_soft {
        accepted = try_solve(&rand_rej, &preds, &dist, &mut seed, true);
    }
    sched.st.randomize_seed.set(seed);
    match accepted {
        Some(updates) => {
            let mut heap = sched.st.class_heap.borrow_mut();
            if let Some(obj) = heap.get_mut(&id) {
                for (idx, v) in updates.into_iter().chain(randc_updates) {
                    if let Some(slot) = obj.fields.get_mut(idx as usize) {
                        *slot = v;
                    }
                }
            }
            true
        }
        None => false, // cap exhausted / unsatisfiable → fields unchanged, fail
    }
}

/// Draw the next value of a `randc` field: a random permutation of `[lo,hi]` per
/// (object, field) is consumed one value at a time, reshuffled when exhausted
/// (seeded Fisher-Yates → deterministic + reproducible).
fn draw_randc(
    state: &mut std::collections::HashMap<(u32, u32), (Vec<i64>, usize)>,
    seed: &mut u32,
    key: (u32, u32),
    lo: i64,
    hi: i64,
    width: u32,
    signed: bool,
) -> Value {
    let entry = state.entry(key).or_insert((Vec::new(), 0));
    if entry.0.is_empty() || entry.1 >= entry.0.len() {
        let n = (hi - lo + 1).max(1) as usize;
        let mut perm: Vec<i64> = (0..n as i64).map(|i| lo + i).collect();
        for i in (1..n).rev() {
            let j = crate::rng::dist_uniform(seed, 0, i as i32) as usize;
            perm.swap(i, j);
        }
        entry.0 = perm;
        entry.1 = 0;
    }
    let v = entry.0[entry.1];
    entry.1 += 1;
    value_from_bits(v as u64, width, signed)
}

/// One rejection-sampling pass: draw every field, accept the first candidate that
/// satisfies all predicates (skipping SOFT predicates when `drop_soft`). Returns
/// the accepted draws, or None if the cap is exhausted. The per-field draw matches
/// B1 exactly so a constraint-free design is byte-identical.
fn try_solve(
    rand: &[crate::RandBound],
    preds: &[Vec<sim_ir::COp>],
    dist: &[crate::DistField],
    seed: &mut u32,
    drop_soft: bool,
) -> Option<Vec<(u32, Value)>> {
    const MAX_TRIES: u32 = 10_000;
    for _ in 0..MAX_TRIES {
        let mut cand: std::collections::HashMap<u32, i64> = Default::default();
        let mut draws: Vec<(u32, Value)> = Vec::with_capacity(rand.len());
        for &(field_idx, width, signed, lo, hi, constrained) in rand {
            let v = if let Some((_, entries)) = dist.iter().find(|(fi, _)| *fi == field_idx) {
                // `dist`: weighted-sample from the distribution (§18.5.4).
                draw_dist(seed, entries, width, signed)
            } else if constrained {
                draw_in_range(seed, lo, hi, width, signed)
            } else {
                random_full_width(seed, width, signed)
            };
            cand.insert(field_idx, value_to_i64(&v, width, signed));
            draws.push((field_idx, v));
        }
        let ok = preds.iter().all(|p| {
            if drop_soft && matches!(p.first(), Some(sim_ir::COp::SoftMarker)) {
                return true; // soft predicate dropped this phase
            }
            eval_pred(p, &cand)
        });
        if ok {
            return Some(draws);
        }
    }
    None
}

/// Weighted-sample a `dist` field: pick an entry with probability proportional to
/// its total weight, then a uniform value within that entry's `[lo,hi]`. Seeded /
/// deterministic. An empty / all-zero-weight distribution draws 0.
fn draw_dist(seed: &mut u32, entries: &[(i64, i64, i64)], width: u32, signed: bool) -> Value {
    let total: i64 = entries.iter().map(|e| e.2).sum();
    if total <= 0 {
        return value_from_bits(0, width, signed);
    }
    let r = if total - 1 <= i32::MAX as i64 {
        crate::rng::dist_uniform(seed, 0, (total - 1) as i32) as i64
    } else {
        (draw_u64(seed) % total as u64) as i64
    };
    let mut acc = 0i64;
    for &(lo, hi, w) in entries {
        acc += w;
        if r < acc {
            let v = draw_i64_range(seed, lo, hi);
            return value_from_bits(v as u64, width, signed);
        }
    }
    value_from_bits(entries[0].0 as u64, width, signed)
}

/// A uniform i64 in `[lo, hi]` (matches `draw_in_range`'s value lane: ≤i32 fast
/// path, else i128-span modulo). `lo == hi` ⇒ `lo` with no draw.
fn draw_i64_range(seed: &mut u32, lo: i64, hi: i64) -> i64 {
    if lo >= hi {
        return lo;
    }
    if lo >= i32::MIN as i64 && hi <= i32::MAX as i64 {
        crate::rng::dist_uniform(seed, lo as i32, hi as i32) as i64
    } else {
        let span = (hi as i128 - lo as i128 + 1) as u128;
        lo + (draw_u64(seed) as u128 % span) as i64
    }
}

/// Extract the signed i64 value of a drawn field for constraint-predicate eval
/// (the draw is always 2-state). Sign-extends a signed field narrower than 64;
/// a field ≥64 bits uses its low 64 bits (B2 evaluates predicates in i64).
fn value_to_i64(v: &Value, width: u32, signed: bool) -> i64 {
    let bits = v.val.first().copied().unwrap_or(0);
    if signed && width < 64 {
        let shift = 64 - width;
        ((bits << shift) as i64) >> shift
    } else {
        bits as i64
    }
}

/// Evaluate a postfix constraint predicate against the candidate field values.
fn eval_pred(prog: &[sim_ir::COp], cand: &std::collections::HashMap<u32, i64>) -> bool {
    let mut stack: Vec<i64> = Vec::with_capacity(8);
    for op in prog {
        match op {
            sim_ir::COp::Field(idx) => stack.push(cand.get(idx).copied().unwrap_or(0)),
            sim_ir::COp::Const(v) => stack.push(*v),
            sim_ir::COp::Not => {
                let a = stack.pop().unwrap_or(0);
                stack.push((a == 0) as i64);
            }
            sim_ir::COp::SoftMarker => {} // tag only — no stack effect
            sim_ir::COp::Bin(b) => {
                let r = stack.pop().unwrap_or(0);
                let l = stack.pop().unwrap_or(0);
                stack.push(apply_cbin(*b, l, r));
            }
        }
    }
    stack.pop().map(|v| v != 0).unwrap_or(false)
}

fn apply_cbin(op: sim_ir::CBinOp, l: i64, r: i64) -> i64 {
    use sim_ir::CBinOp as C;
    match op {
        C::Add => l.wrapping_add(r),
        C::Sub => l.wrapping_sub(r),
        C::Mul => l.wrapping_mul(r),
        C::Div => {
            if r == 0 {
                0
            } else {
                l.wrapping_div(r)
            }
        }
        C::Mod => {
            if r == 0 {
                0
            } else {
                l.wrapping_rem(r)
            }
        }
        C::Lt => (l < r) as i64,
        C::Le => (l <= r) as i64,
        C::Gt => (l > r) as i64,
        C::Ge => (l >= r) as i64,
        C::Eq => (l == r) as i64,
        C::Ne => (l != r) as i64,
        C::And => ((l != 0) && (r != 0)) as i64,
        C::Or => ((l != 0) || (r != 0)) as i64,
    }
}

/// Draw a uniform value in the inclusive `[lo, hi]` constraint range, honoring the
/// bound at ANY width: bounds that fit i32 use the iverilog-pinned `dist_uniform`
/// (preserving the verified ≤32-bit draws); wider i64 bounds use a 64-bit draw
/// reduced modulo the (u128) span. The result is masked to the field width.
fn draw_in_range(seed: &mut u32, lo: i64, hi: i64, width: u32, signed: bool) -> Value {
    if lo > hi {
        return value_from_bits(0, width, signed); // empty range (elaborate rejected)
    }
    if lo >= i32::MIN as i64 && hi <= i32::MAX as i64 {
        let drawn = crate::rng::dist_uniform(seed, lo as i32, hi as i32);
        return value_from_bits(drawn as i64 as u64, width, signed);
    }
    // Wide/large bounds: a 64-bit draw mapped into the span. The span fits u128 even
    // for [i64::MIN, i64::MAX]; for power-of-two spans the modulo is exactly uniform.
    let span = (hi as i128 - lo as i128 + 1) as u128;
    let r = draw_u64(seed) as u128;
    let v = lo as i128 + (r % span) as i128;
    value_from_bits(v as i64 as u64, width, signed)
}

/// A 64-bit seeded draw assembled from two `dist_uniform` half-word draws (shares
/// the one deterministic `randomize()` stream).
fn draw_u64(seed: &mut u32) -> u64 {
    let hi = crate::rng::dist_uniform(seed, i32::MIN, i32::MAX) as u32 as u64;
    let lo = crate::rng::dist_uniform(seed, i32::MIN, i32::MAX) as u32 as u64;
    (hi << 32) | lo
}

/// A `width`-bit `Value` holding the low bits of `bits` (two's-complement when
/// signed); used for a ranged rand draw.
fn value_from_bits(bits: u64, width: u32, signed: bool) -> Value {
    // The drawn value lives in the LOW word. For a field wider than 64 bits, the
    // high words must be SIGN-filled for a negative signed draw (a `[-100,-50]`
    // constraint on a `rand bit signed [127:0]`); a plain single-word pack would
    // zero-pad and silently store a huge positive out-of-range value.
    let n = (width.max(1) as usize).div_ceil(64);
    let fill = if signed && (bits as i64) < 0 {
        u64::MAX
    } else {
        0
    };
    let mut val = vec![fill; n];
    val[0] = bits;
    Value::from_packed(
        &sim_ir::BitPacked {
            val,
            unk: vec![0; n],
        },
        width.max(1),
        signed,
    )
}

/// A full-width seeded random `Value` (an UNCONSTRAINED rand field), one 64-bit
/// `draw_u64` per word, masked to `width`.
fn random_full_width(seed: &mut u32, width: u32, signed: bool) -> Value {
    let nwords = (width as usize).div_ceil(64).max(1);
    let words: Vec<u64> = (0..nwords).map(|_| draw_u64(seed)).collect();
    Value::from_packed(
        &sim_ir::BitPacked {
            val: words,
            unk: vec![0; nwords],
        },
        width.max(1),
        signed,
    )
}

fn cast_task(sched: &mut Scheduler, args: &[u32]) {
    if args.len() != 2 {
        return;
    }
    let dst = match sched.st.ir.exprs.get(args[0] as usize) {
        Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
        _ => None,
    };
    if let Some(net) = dst {
        let lv = sim_ir::Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let v = sched.eval_for_lvalue(&lv, args[1]); // context-size src to dst width
        let off = sched.resolve_lvalue_offsets(&lv);
        sched.st.write_lvalue(&lv, v, &off);
    }
}

/// v7 `$readmemb/h(file, mem[, start[, finish]])` — iverilog-pinned (t11–14):
/// default fill = LOWEST declared index ascending (1364-2005), `@addr` is hex
/// in BOTH variants and lives in the DECLARED index domain, unwritten
/// elements keep their value, token shortfall warns only for directive-free
/// files, and every problem is W4023 + continue (exit parity with iverilog).
fn readmem(sched: &mut Scheduler, args: &[u32], hex: bool) {
    let warn = |sched: &mut Scheduler, msg: String| {
        sched
            .st
            .sink
            .emit(diag::LogEvent::Diagnostic(diag::Diagnostic {
                severity: diag::Severity::Warning,
                code: diag::MsgCode::RunReadmem,
                message: msg,
                location: None,
                context: Vec::new(),
                sim_time: Some(diag::TimeStamp {
                    ticks: sched.st.now,
                }),
            }));
    };
    let Some(&a0) = args.first() else { return };
    let name = match sched.st.ir.exprs.get(a0 as usize) {
        Some(sim_ir::Expr::Const { val }) => const_string(sched.st.ir, *val),
        _ => return,
    };
    let net = match args.get(1).and_then(|&a| sched.st.ir.exprs.get(a as usize)) {
        Some(sim_ir::Expr::Signal { net, word: None }) => *net,
        _ => {
            warn(sched, "$readmem target is not a memory".to_string());
            return;
        }
    };
    let (alen, w) = {
        let nv = &sched.st.ir.nets[net as usize];
        (nv.array_len.max(1) as u64, nv.width.max(1))
    };
    // declared base = min index of dim 0 (sparse table; absent ⇒ 0-based).
    // Multi-dim memories use flat word-offset addressing from that base.
    let base = sched
        .st
        .net_dims
        .get(&net)
        .and_then(|d| d.first())
        .map(|&(lo, hi)| lo.min(hi) as u64)
        .unwrap_or(0);
    let Ok(text) = std::fs::read_to_string(&name) else {
        warn(
            sched,
            format!("$readmem: unable to open '{name}' for reading"),
        );
        return;
    };
    // strip // line and /* */ block comments.
    let mut cleaned = String::with_capacity(text.len());
    let mut rest = text.as_str();
    'outer: while !rest.is_empty() {
        let line_c = rest.find("//");
        let block_c = rest.find("/*");
        match (line_c, block_c) {
            (Some(l), b) if b.is_none_or(|b| l < b) => {
                cleaned.push_str(&rest[..l]);
                match rest[l..].find('\n') {
                    Some(nl) => rest = &rest[l + nl..],
                    None => break 'outer,
                }
            }
            (_, Some(bs)) => {
                cleaned.push_str(&rest[..bs]);
                match rest[bs..].find("*/") {
                    Some(be) => rest = &rest[bs + be + 2..],
                    None => break 'outer,
                }
            }
            _ => {
                cleaned.push_str(rest);
                break 'outer;
            }
        }
    }
    // range window (declared-index domain). Default: full array ascending.
    let r_start = args.get(2).and_then(|&a| sched.eval(a).to_u64());
    let r_finish = args.get(3).and_then(|&a| sched.eval(a).to_u64());
    let (start, finish) = match (r_start, r_finish) {
        (Some(s), Some(f)) => (s, f),
        (Some(s), None) => (s, base + alen - 1),
        _ => (base, base + alen - 1),
    };
    let step: i64 = if start <= finish { 1 } else { -1 };
    let (win_lo, win_hi) = (start.min(finish), start.max(finish));
    let window = win_hi - win_lo + 1;

    let mut addr = start as i64;
    let mut wrote: u64 = 0;
    let mut had_at = false;
    for tok in cleaned.split_whitespace() {
        if let Some(a) = tok.strip_prefix('@') {
            had_at = true;
            match u64::from_str_radix(a, 16) {
                Ok(v) => addr = v as i64,
                Err(_) => warn(sched, format!("$readmem: bad address token '@{a}'")),
            }
            continue;
        }
        let a = addr as u64;
        if addr < 0 || a < win_lo || a > win_hi || a < base || a - base >= alen {
            warn(
                sched,
                format!("$readmem('{name}'): address {addr} outside the load range; stopped"),
            );
            return;
        }
        let val = parse_mem_token(tok, w, hex);
        let word = (a - base) as u32;
        // funnel write: the dummy `word: Some(0)` ExprId is never evaluated —
        // `write_chunk` takes the resolved word from the offsets pair.
        let lv = sim_ir::Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: Some(0),
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let off = crate::exec::Offsets::Inline {
            buf: [(0, word), (0, 0)],
            len: 1,
        };
        sched.st.write_lvalue(&lv, val, &off);
        wrote += 1;
        addr += step;
    }
    if !had_at && wrote < window {
        warn(
            sched,
            format!(
                "$readmem('{name}'): not enough words for the range \
                 [{start}:{finish}] ({wrote} of {window}); rest unchanged"
            ),
        );
    }
}

/// v9 `$writememb/h(file, mem[, start[, finish]])` — the write-side mirror of
/// `readmem`, iverilog-pinned: the FIRST line is ALWAYS the literal
/// `// 0x00000000` header (it never reflects the base/start); each element is
/// one line (every line incl the last ends '\n'); the optional (start[,finish])
/// is an inclusive declared-index window, descending when finish < start; an
/// out-of-range start/finish is non-fatal (a warning, the file is NOT created,
/// the sim continues). Element values come from the engine word-read path; hex
/// uses per-nibble X/Z compression, bin is per-bit uncompressed.
fn writemem(sched: &mut Scheduler, args: &[u32], hex: bool) {
    let warn = |sched: &mut Scheduler, msg: String| {
        sched
            .st
            .sink
            .emit(diag::LogEvent::Diagnostic(diag::Diagnostic {
                severity: diag::Severity::Warning,
                code: diag::MsgCode::RunReadmem,
                message: msg,
                location: None,
                context: Vec::new(),
                sim_time: Some(diag::TimeStamp {
                    ticks: sched.st.now,
                }),
            }));
    };
    let Some(&a0) = args.first() else { return };
    let name = match sched.st.ir.exprs.get(a0 as usize) {
        Some(sim_ir::Expr::Const { val }) => const_string(sched.st.ir, *val),
        _ => return,
    };
    let net = match args.get(1).and_then(|&a| sched.st.ir.exprs.get(a as usize)) {
        Some(sim_ir::Expr::Signal { net, word: None }) => *net,
        _ => {
            warn(sched, "$writemem target is not a memory".to_string());
            return;
        }
    };
    let (alen, w) = {
        let nv = &sched.st.ir.nets[net as usize];
        (nv.array_len.max(1) as u64, nv.width.max(1))
    };
    // declared base = min index of dim 0 (sparse table; absent ⇒ 0-based).
    let base = sched
        .st
        .net_dims
        .get(&net)
        .and_then(|d| d.first())
        .map(|&(lo, hi)| lo.min(hi) as u64)
        .unwrap_or(0);
    let last = base + alen - 1;
    // range window (declared-index domain). Default: full array ascending.
    let r_start = args.get(2).and_then(|&a| sched.eval(a).to_u64());
    let r_finish = args.get(3).and_then(|&a| sched.eval(a).to_u64());
    let (start, finish) = match (r_start, r_finish) {
        (Some(s), Some(f)) => (s, f),
        (Some(s), None) => (s, last),
        _ => (base, last),
    };
    // OOB start/finish is non-fatal AND the file is NOT created (iverilog
    // validates before opening — file never appears). Mirror its report text.
    for (label, idx) in [("Start", start), ("Finish", finish)] {
        if idx < base || idx > last {
            warn(
                sched,
                format!(
                    "$writemem('{name}'): {label} address {idx} is out of bounds \
                     for the memory [{base}:{last}]; file not written"
                ),
            );
            return;
        }
    }
    let step: i64 = if start <= finish { 1 } else { -1 };
    let mut body = String::from("// 0x00000000\n");
    let mut addr = start as i64;
    loop {
        let word = (addr as u64 - base) as u32;
        let v = sched.st.read_net(net, Some(word));
        if hex {
            fmt_writemem_hex(&v, w, &mut body);
        } else {
            fmt_writemem_bin(&v, w, &mut body);
        }
        body.push('\n');
        if addr as u64 == finish {
            break;
        }
        addr += step;
    }
    if let Err(e) = std::fs::write(&name, body) {
        warn(
            sched,
            format!("$writemem: unable to open '{name}' for writing: {e}"),
        );
    }
}

/// One memory element → a `$writememh` hex field: ceil(w/4) lowercase digits,
/// MSB-first, with iverilog's per-nibble X/Z compression. The compression
/// examines ONLY the REAL bits of each nibble — a partial top nibble's phantom
/// zero-pad bit does NOT participate (iverilog-pinned: an all-x 3-bit top
/// nibble renders 'x', not 'X'). Rules: clean ⇒ hex digit; all-x ⇒ 'x';
/// all-z ⇒ 'z'; any x mixed in ⇒ 'X' (X dominates Z); else z mixed ⇒ 'Z'.
fn fmt_writemem_hex(v: &Value, w: u32, out: &mut String) {
    let ndig = w.div_ceil(4);
    for nib in (0..ndig).rev() {
        let (mut xc, mut zc, mut nbits, mut val) = (0u32, 0u32, 0u32, 0u32);
        for k in 0..4 {
            let bit = nib * 4 + k;
            if bit >= w {
                continue; // phantom pad bit — excluded from value AND compression
            }
            nbits += 1;
            let (bv, bu) = v.get_vu(bit);
            if bu != 0 {
                if bv != 0 {
                    zc += 1;
                } else {
                    xc += 1;
                }
            } else if bv != 0 {
                val |= 1 << k;
            }
        }
        let ch = if xc == 0 && zc == 0 {
            std::char::from_digit(val, 16).unwrap()
        } else if xc == nbits {
            'x'
        } else if zc == nbits {
            'z'
        } else if xc > 0 {
            'X'
        } else {
            'Z'
        };
        out.push(ch);
    }
}

/// One memory element → a `$writememb` binary field: exactly `w` per-bit chars,
/// MSB-first, NO compression (0/1/x/z lowercase).
fn fmt_writemem_bin(v: &Value, w: u32, out: &mut String) {
    for bit in (0..w).rev() {
        let (bv, bu) = v.get_vu(bit);
        out.push(match (bv != 0, bu != 0) {
            (false, false) => '0',
            (true, false) => '1',
            (false, true) => 'x',
            (true, true) => 'z',
        });
    }
}

/// One memory-file token → a `Value` of element width `w` (right-aligned,
/// high bits zero; surplus digits truncate on the left). Hex digits are 4
/// bits, binary 1; `x`/`z` poison their digit's bits; `_` is ignored.
fn parse_mem_token(tok: &str, w: u32, hex: bool) -> Value {
    let bits_per = if hex { 4u32 } else { 1 };
    // per-bit (val, unk) MSB-first
    let mut bits: Vec<(bool, bool)> = Vec::with_capacity(tok.len() * bits_per as usize);
    for ch in tok.chars() {
        match ch {
            '_' => {}
            'x' | 'X' => bits.extend(std::iter::repeat_n((false, true), bits_per as usize)),
            'z' | 'Z' | '?' => bits.extend(std::iter::repeat_n((true, true), bits_per as usize)),
            // a non-digit stray char skips (comment-residue defensiveness)
            c => {
                if let Some(d) = c.to_digit(if hex { 16 } else { 2 }) {
                    for k in (0..bits_per).rev() {
                        bits.push(((d >> k) & 1 != 0, false));
                    }
                }
            }
        }
    }
    let mut v = Value::zeros(w, false);
    // place LSB-first from the token's tail; bits beyond w truncate (left).
    for (i, &(bv, bu)) in bits.iter().rev().enumerate() {
        if (i as u32) >= w {
            break;
        }
        let word = i / 64;
        let sh = i % 64;
        if bv {
            v.val[word] |= 1u64 << sh;
        }
        if bu {
            v.unk[word] |= 1u64 << sh;
        }
    }
    v.mask_top();
    v
}

/// v7: route `text` to a descriptor — fd form (bit 31) hits one file; MCD
/// form broadcasts to every set channel bit (bit 0 = stdout). A bad/closed
/// fd warns once (W4022) and drops the write, iverilog parity.
fn file_write(sched: &mut Scheduler, fd: u32, text: &str) {
    use std::io::Write as _;
    if fd & 0x8000_0000 != 0 {
        // §21.3.4 pre-opened descriptors. STDOUT (0x8000_0001) routes through
        // the SAME deterministic sink as `$display`/MCD-bit-0, so it interleaves
        // in statement order (iverilog-pinned) inside the golden stdout stream.
        // STDERR (0x8000_0002) goes to the process stderr like iverilog. A write
        // to the read-only STDIN (0x8000_0000) falls through to the files-map
        // miss → W4022 warn + drop (iverilog drops it SILENTLY; the warn is
        // strictly more diagnostic, output bytes identical).
        match fd {
            0x8000_0001 => {
                write_out(sched.st, text);
                return;
            }
            0x8000_0002 => {
                let _ = std::io::stderr().write_all(text.as_bytes());
                return;
            }
            _ => {}
        }
        match sched.st.files.get_mut(&fd) {
            Some(f) => {
                let _ = f.write_all(text.as_bytes());
            }
            None => bad_fd_warn(sched, fd),
        }
        return;
    }
    // MCD broadcast.
    if fd == 0 {
        bad_fd_warn(sched, fd);
        return;
    }
    if fd & 1 != 0 {
        write_out(sched.st, text);
    }
    for bit in 1..31u32 {
        if fd & (1 << bit) != 0 {
            match sched.st.mcd_files.get_mut(&bit) {
                Some(f) => {
                    let _ = f.write_all(text.as_bytes());
                }
                None => bad_fd_warn(sched, fd),
            }
        }
    }
}

/// Read one byte from an fd-form descriptor for the v9 SYS-READ family,
/// honoring the `$ungetc` pushback stack and tracking lazy EOF. Returns
/// `Some(byte)` or `None` at EOF / bad-fd / write-only-fd. Only fd-form
/// descriptors (bit 31 set) opened with read capability (a mode containing
/// 'r' or '+') are readable — MCD channels are write-only broadcast masks, and
/// a plain "w"/"a" descriptor is write-only. A genuinely bad/closed fd warns
/// once (W4022); a valid-but-write-only fd returns `None` WITHOUT a warning and
/// WITHOUT latching EOF (iverilog parity — `$fgetc`=-1 yet `$feof`=0). The lazy
/// EOF flag is set only by a FAILED read on a READABLE fd (a read returning
/// zero bytes), matching iverilog's `$feof` timing.
pub(crate) fn file_read_byte(sched: &mut Scheduler, fd: u32) -> Option<u8> {
    // a pushed-back byte ($ungetc) is served before the underlying stream
    // (LIFO — the top of the pushback stack). Only readable fds ever carry a
    // pushback (k_ungetc rejects write-only/bad fds), so this is safe first.
    if let Some(s) = sched.st.read_state.get_mut(&fd) {
        if let Some(b) = s.pushback.pop() {
            return Some(b);
        }
    }
    // §21.3.4: reads on the pre-opened STDOUT/STDERR behave like any write-only
    // fd — `$fgetc`=-1 with NO warning and NO EOF latch (iverilog-pinned:
    // fgetc=-1, $feof stays 0). STDIN (0x8000_0000) is DELIBERATELY excluded
    // from this early return: reading it is a deferred feature (a stdin-driven
    // sim breaks byte-determinism), so it falls through to the files-map miss
    // → W4022 warn + -1 (iverilog reads stdin).
    if fd == 0x8000_0001 || fd == 0x8000_0002 {
        return None;
    }
    if fd & 0x8000_0000 == 0 || !sched.st.files.contains_key(&fd) {
        bad_fd_warn(sched, fd);
        return None;
    }
    if !sched.st.readable_fds.contains(&fd) {
        // a valid but write-only ("w"/"a") fd: reads fail WITHOUT a warning and
        // WITHOUT latching EOF (iverilog: $fgetc=-1, $feof stays 0).
        return None;
    }
    let file = sched
        .st
        .files
        .get_mut(&fd)
        .expect("readable fd is in files");
    let mut buf = [0u8; 1];
    match std::io::Read::read(file, &mut buf) {
        Ok(1) => Some(buf[0]),
        // EOF (0 bytes) or a read error sets the lazy EOF flag.
        _ => {
            sched.st.read_state.entry(fd).or_default().eof = true;
            None
        }
    }
}

/// §7.10.1 queue-slice executor: args = [dst_handle, src_handle, a, b].
/// Clamp semantics (hand-IEEE, no oracle — Icarus parses `q[a:b]` but ignores
/// the bounds): a<0 clamps to 0, b>size-1 clamps to size-1; a>b, a≥size, b<0,
/// or an x/z bound ⇒ the empty queue (x/z also warns once, the dyn pattern).
/// The result replaces dst wholesale (value semantics) and then the bounded-
/// queue post-op runs (mirrors the whole-copy path).
fn run_queue_slice(sched: &mut Scheduler, args: &[u32]) -> Ctl {
    let (Some(dst), Some(src)) = (
        dyn_handle_net(sched, args.first()),
        dyn_handle_net(sched, args.get(1)),
    ) else {
        return Ctl::Continue; // elaborate never emits a malformed marker
    };
    let bound = |sched: &mut Scheduler, i: usize| -> Option<i64> {
        let v = sched.eval(*args.get(i)?);
        if v.has_xz() {
            return None;
        }
        Some(
            v.to_i128_signed()
                .unwrap_or(0)
                .clamp(i64::MIN as i128, i64::MAX as i128) as i64,
        )
    };
    let a = bound(sched, 2);
    let b = bound(sched, 3);
    let elems: std::collections::VecDeque<crate::value::Value> = match (a, b) {
        (Some(a), Some(b)) => {
            let n = match sched.st.dyn_heap.get(src as usize).and_then(|o| o.as_ref()) {
                Some(crate::state::DynObj::Queue { elems }) => elems.len() as i64,
                _ => 0,
            };
            let lo = a.max(0);
            let hi = b.min(n - 1);
            if lo > hi {
                std::collections::VecDeque::new()
            } else {
                match sched.st.dyn_heap.get(src as usize).and_then(|o| o.as_ref()) {
                    Some(crate::state::DynObj::Queue { elems }) => elems
                        .iter()
                        .skip(lo as usize)
                        .take((hi - lo + 1) as usize)
                        .cloned()
                        .collect(),
                    _ => std::collections::VecDeque::new(),
                }
            }
        }
        _ => {
            dyn_warn_once(sched, src, "queue slice bound is X/Z; result is empty");
            std::collections::VecDeque::new()
        }
    };
    if let Some(slot) = sched.st.dyn_heap.get_mut(dst as usize) {
        *slot = Some(crate::state::DynObj::Queue { elems });
    }
    sched.st.enforce_queue_bound(dst);
    Ctl::Continue
}

/// `$fclose` on a pre-opened descriptor: W4022-class warn (once per fd, the
/// same latch), descriptor STAYS open — iverilog parity ("could not close
/// file descriptor STDOUT (0x80000001)"; a later write still prints).
/// LATCH NOTE: STDIN shares the once-latch with its write/read W4022 — a
/// design that first writes/reads STDIN and then `$fclose`s it gets only the
/// first warning (accepted: same fd, same "ignored" outcome, bytes identical).
fn preopened_close_warn(sched: &mut Scheduler, fd: u32) {
    if !sched.st.bad_fd_warned.insert(fd) {
        return;
    }
    let name = match fd {
        0x8000_0000 => "STDIN",
        0x8000_0001 => "STDOUT",
        _ => "STDERR",
    };
    sched
        .st
        .sink
        .emit(diag::LogEvent::Diagnostic(diag::Diagnostic {
            severity: diag::Severity::Warning,
            code: diag::MsgCode::RunBadFd,
            message: format!(
                "$fclose cannot close the pre-opened {name} descriptor 0x{fd:08x} (ignored)"
            ),
            location: None,
            context: Vec::new(),
            sim_time: Some(diag::TimeStamp {
                ticks: sched.st.now,
            }),
        }));
}

/// W4022 once-per-descriptor (the dyn W4020 latch pattern).
pub(crate) fn bad_fd_warn(sched: &mut Scheduler, fd: u32) {
    if !sched.st.bad_fd_warned.insert(fd) {
        return;
    }
    sched
        .st
        .sink
        .emit(diag::LogEvent::Diagnostic(diag::Diagnostic {
            severity: diag::Severity::Warning,
            code: diag::MsgCode::RunBadFd,
            message: format!("file operation on invalid/closed descriptor 0x{fd:08x} ignored"),
            location: None,
            context: Vec::new(),
            sim_time: Some(diag::TimeStamp {
                ticks: sched.st.now,
            }),
        }));
}

pub(crate) fn write_out(st: &mut SimState, text: &str) {
    let _ = st.out.write_all(text.as_bytes());
}

/// P1-1: execute a severity task (doc-13 §Severity). The user message renders
/// through the SAME `format_args_str` engine as `$display` (so `%0d`/defaults
/// behave identically) but is emitted as a `LogEvent::Diagnostic` — stderr in
/// production, never the stdout stream. Empty message ⇒ the code's title.
/// `$fatal` aborts (implicit `$finish`, `ExitClass::Fatal`); `$error` flags
/// `HadErrors` and continues; `$warning`/`$info` only print.
fn run_severity(
    sched: &mut Scheduler,
    sev: crate::SeverityKind,
    fmt: Option<u32>,
    args: &[u32],
) -> Ctl {
    let message = format_args_str(sched, fmt, args, None);
    emit_severity_message(sched, sev, message)
}

/// Emit an already-rendered severity message to the diagnostic stream and apply
/// its control/exit-class effect. Split out of `run_severity` so a §16.4
/// deferred assert can render its text at REACH and emit it at maturation
/// (the args are sampled at reach per §16.4.3, not re-evaluated here).
pub(crate) fn emit_severity_message(
    sched: &mut Scheduler,
    sev: crate::SeverityKind,
    mut message: String,
) -> Ctl {
    use crate::SeverityKind as K;
    use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
    let (severity, code) = match sev {
        K::Fatal => (Severity::Fatal, MsgCode::RunFatal),
        K::Error => (Severity::Error, MsgCode::RunUserError),
        K::Warning => (Severity::Warning, MsgCode::RunUserWarning),
        K::Info => (Severity::Info, MsgCode::RunUserInfo),
    };
    if message.is_empty() {
        message = code.title().to_string();
    }
    sched.st.sink.emit(LogEvent::Diagnostic(Diagnostic {
        severity,
        code,
        message,
        location: None,
        context: Vec::new(),
        sim_time: Some(TimeStamp {
            ticks: sched.st.now,
        }),
    }));
    match sev {
        K::Fatal => Ctl::Fatal,
        K::Error => {
            sched.st.had_error = true;
            Ctl::Continue
        }
        K::Warning | K::Info => Ctl::Continue,
    }
}

// ── $dumpvars: declare all nets, header, initial dump ──────────────────────

fn dumpvars(st: &mut SimState, args: &[u32]) {
    // ⑤b: the FIRST call opens the VCD and fixes the filter; the header
    // cannot be rewritten, so later calls warn once (W4021) and no-op
    // (the LRM's accumulate-across-calls model is a v1 cut).
    if st.vcd.is_some() {
        if !st.dump_multi_warned {
            st.dump_multi_warned = true;
            use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
            st.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Warning,
                code: MsgCode::RunDumpMulti,
                message: "extra $dumpvars call ignored (v1: the first call wins)".to_string(),
                location: None,
                context: Vec::new(),
                sim_time: Some(TimeStamp { ticks: st.now }),
            }));
        }
        return;
    }
    st.dump_filter = dump_filter_from_args(st, args);
    let path = st
        .vcd_path_override
        .clone()
        .or_else(|| st.dump_pending_path.clone())
        .unwrap_or_else(|| "dump.vcd".to_string());

    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            // P2-1: the main artifact must not vanish silently — warn (with the
            // path + OS error) and keep simulating without a waveform.
            use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
            st.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Warning,
                code: MsgCode::RunVcdOpenFail,
                message: format!("cannot open VCD dump file '{path}': {e}"),
                location: None,
                context: Vec::new(),
                sim_time: Some(TimeStamp { ticks: st.now }),
            }));
            return;
        }
    };
    // P3-3/T0b: buffer the VCD sink (raw `File` was ~1 write syscall per record).
    // `finalize_vcd` flushes explicitly, so buffering never changes the bytes.
    // P4-T1: with `--threads ≥2` the buffered chunks go to a dedicated writer
    // thread (order-preserving bounded FIFO) — byte-identical, wall-clock only.
    let sink: crate::state::VcdSink = if st.threads >= 2 {
        Box::new(std::io::BufWriter::with_capacity(
            64 * 1024,
            crate::vcd_thread::ThreadedWriter::spawn(file),
        ))
    } else {
        Box::new(std::io::BufWriter::with_capacity(64 * 1024, file))
    };
    st.open_vcd(sink);

    let date = st.vcd_date.clone();
    let unit = st.timescale_unit.clone();
    let mut ids: Vec<Option<IdCode>> = vec![None; st.ir.nets.len()];
    let mut word_ids: Vec<Vec<Option<IdCode>>> = vec![Vec::new(); st.ir.nets.len()];
    let st_dims = st.net_dims.clone();
    let dump_filter = st.dump_filter.clone();
    // B1 frame-call: frame-local nets are REAL ir.nets entries (for width/
    // metadata) but live in the call frame arena, never the flat store — they
    // have no VCD surface and must not be declared/dumped. Captured here (like
    // `st_dims`) so the borrow block below need not re-borrow `st`. Empty
    // func_table ⇒ all-false ⇒ byte-identical (no net is skipped).
    let frame_local = st.frame_local.clone();
    // Hierarchical naming when the elaborate side table is present (one FQ name per
    // net); otherwise the legacy flat `top` scope + synthetic `n{i}`.
    let use_names = st.net_names.len() == st.ir.nets.len();
    {
        let nets = &st.ir.nets;
        let names = &st.net_names;
        let w = st.vcd.as_mut().unwrap();
        let _ = w.write_preamble(&date, &unit);
        if use_names {
            // Split each FQ name into (scope segments, leaf). Emit a correctly nested
            // $scope/$upscope tree by visiting nets in scope-sorted order and pushing
            // / popping as the scope prefix changes (classic sorted-leaf tree walk).
            let mut order: Vec<usize> = (0..nets.len()).collect();
            let segs: Vec<Vec<&str>> = names.iter().map(|s| s.split('.').collect()).collect();
            // sort by scope path (all but the leaf); stable → vars keep net order
            // within a scope.
            order.sort_by(|&a, &b| segs[a][..segs[a].len() - 1].cmp(&segs[b][..segs[b].len() - 1]));
            let mut cur: Vec<&str> = Vec::new();
            for &i in &order {
                let scope = &segs[i][..segs[i].len() - 1];
                let leaf = *segs[i].last().unwrap();
                // pop to the common prefix
                let common = cur
                    .iter()
                    .zip(scope.iter())
                    .take_while(|(a, b)| a == b)
                    .count();
                while cur.len() > common {
                    let _ = w.pop_scope();
                    cur.pop();
                }
                // push the remaining scope segments
                while cur.len() < scope.len() {
                    let seg = scope[cur.len()];
                    let _ = w.push_scope(ScopeType::Module, seg);
                    cur.push(seg);
                }
                // B1: frame-local nets have no VCD surface (see capture above).
                if frame_local[i] {
                    continue;
                }
                // v5 (C): dyn handles have no $var form (variable length) —
                // never declared, so no initial dump and no change records.
                if matches!(
                    nets[i].kind,
                    sim_ir::NetKind::DynArray
                        | sim_ir::NetKind::Queue
                        | sim_ir::NetKind::Assoc
                        | sim_ir::NetKind::AssocStr
                        | sim_ir::NetKind::String
                ) {
                    continue;
                }
                // ⑤b: outside the $dumpvars depth/scope/net selection → no var.
                if dump_filter
                    .as_ref()
                    .is_some_and(|f| !f.contains(&(i as u32)))
                {
                    continue;
                }
                let vt = vcd_var_type(nets[i].kind);
                if nets[i].array_len > 1 {
                    // Phase-1.x ⑤: one $var PER ELEMENT (`mem[4]`, `g[1][2]`),
                    // declared indices from the dims sidecar (absent ⇒ 1-D
                    // 0-based). v1 only ever declared/dumped word 0.
                    let dims = st_dims
                        .get(&(i as u32))
                        .cloned()
                        .unwrap_or_else(|| vec![(0, nets[i].array_len)]);
                    let mut wv = Vec::with_capacity(nets[i].array_len as usize);
                    for word in 0..nets[i].array_len {
                        let name = elem_name(leaf, &dims, word);
                        wv.push(w.declare_var(vt, nets[i].width.max(1), &name).ok());
                    }
                    word_ids[i] = wv;
                } else if let Ok(id) = w.declare_var(vt, nets[i].width.max(1), leaf) {
                    ids[i] = Some(id);
                }
            }
            while !cur.is_empty() {
                let _ = w.pop_scope();
                cur.pop();
            }
        } else {
            let _ = w.push_scope(ScopeType::Module, "top");
            for (i, nv) in nets.iter().enumerate() {
                if frame_local[i] {
                    continue; // B1: frame-local nets have no VCD surface
                }
                if matches!(
                    nv.kind,
                    sim_ir::NetKind::DynArray
                        | sim_ir::NetKind::Queue
                        | sim_ir::NetKind::Assoc
                        | sim_ir::NetKind::AssocStr
                        | sim_ir::NetKind::String
                ) {
                    continue; // dyn handles: no $var form (see above)
                }
                if dump_filter
                    .as_ref()
                    .is_some_and(|f| !f.contains(&(i as u32)))
                {
                    continue; // ⑤b: outside the $dumpvars selection
                }
                let vt = vcd_var_type(nv.kind);
                let name = format!("n{i}");
                if nv.array_len > 1 {
                    let dims = st_dims
                        .get(&(i as u32))
                        .cloned()
                        .unwrap_or_else(|| vec![(0, nv.array_len)]);
                    let mut wv = Vec::with_capacity(nv.array_len as usize);
                    for word in 0..nv.array_len {
                        let ename = elem_name(&name, &dims, word);
                        wv.push(w.declare_var(vt, nv.width.max(1), &ename).ok());
                    }
                    word_ids[i] = wv;
                } else if let Ok(id) = w.declare_var(vt, nv.width.max(1), &name) {
                    ids[i] = Some(id);
                }
            }
            let _ = w.pop_scope();
        }
        let _ = w.write_header();
    }
    for (i, id) in ids.iter().enumerate() {
        st.nets[i].vcd_id = *id;
    }
    for (i, wv) in word_ids.into_iter().enumerate() {
        st.nets[i].vcd_word_ids = wv;
    }

    // initial dump of every declared var (arrays: one entry per element).
    let snap = full_snapshot(st);
    {
        let w = st.vcd.as_mut().unwrap();
        let _ = w.dump_initial(snap.iter().map(|(id, b, wd)| (*id, b, *wd)));
        let _ = w.set_time(st.now);
    }
    st.dumping = true;
    st.vcd_path = Some(path);
}

fn dump_on(st: &mut SimState) {
    st.dumping = true;
    let snap = full_snapshot(st);
    let now = st.now;
    if let Some(w) = st.vcd.as_mut() {
        let _ = w.set_time(now);
        let _ = w.dump_on(snap.iter().map(|(id, b, wd)| (*id, b, *wd)));
    }
}

fn dump_all(st: &mut SimState) {
    let snap = full_snapshot(st);
    let now = st.now;
    if let Some(w) = st.vcd.as_mut() {
        let _ = w.set_time(now);
        let _ = w.dump_all(snap.iter().map(|(id, b, wd)| (*id, b, *wd)));
    }
}

/// ⑤b: build the dump filter from `$dumpvars` args. `None` ⇒ everything
/// (bare call, or level-only). Net args (`Signal{net}`) select that net;
/// scope-string args (the elaborate `fq\x01raw` encoding) select nets whose
/// hierarchical name sits within LEVEL segments below the scope (level 0 =
/// unlimited; level N = N levels — iverilog-pinned: `$dumpvars(1, top)` is
/// top's OWN vars only). Scope args resolve against `net_names`; with no
/// name table they cannot match and are ignored.
fn dump_filter_from_args(st: &SimState, args: &[u32]) -> Option<std::collections::BTreeSet<u32>> {
    let mut level: Option<u64> = None;
    let mut net_targets: Vec<u32> = Vec::new();
    let mut scopes: Vec<Vec<String>> = Vec::new(); // candidate list per arg
    for &a in args {
        match &st.ir.exprs[a as usize] {
            sim_ir::Expr::Signal { net, word: None } => net_targets.push(*net),
            sim_ir::Expr::Const { val } => {
                let cv = &st.ir.consts[*val as usize];
                if cv.repr == sim_ir::ConstRepr::StrUtf8 {
                    let enc = const_string(st.ir, *val);
                    scopes.push(enc.split('\u{0001}').map(str::to_string).collect());
                } else if level.is_none() {
                    level = Some(cv.bits.val.first().copied().unwrap_or(0));
                }
            }
            _ => {}
        }
    }
    if net_targets.is_empty() && scopes.is_empty() {
        return None;
    }
    let lvl = level.unwrap_or(0);
    let mut set: std::collections::BTreeSet<u32> = net_targets.into_iter().collect();
    let scope_count = scopes.len();
    if !scopes.is_empty() && st.net_names.len() == st.ir.nets.len() {
        for cands in &scopes {
            // First candidate that matches ANY net wins (fq form, then raw).
            let chosen = cands.iter().find(|c| {
                st.net_names.iter().any(|n| {
                    n.strip_prefix(c.as_str())
                        .is_some_and(|r| r.starts_with('.'))
                })
            });
            let Some(scope) = chosen else { continue };
            let depth_of = |s: &str| s.split('.').count() as u64;
            let base = depth_of(scope);
            for (i, name) in st.net_names.iter().enumerate() {
                let within = name
                    .strip_prefix(scope.as_str())
                    .is_some_and(|r| r.starts_with('.'));
                if !within {
                    continue;
                }
                let extra = depth_of(name) - base;
                if lvl == 0 || extra <= lvl {
                    set.insert(i as u32);
                }
            }
        }
    }
    // An UNRESOLVED scope arg (no name table — the legacy n{i} path — or a
    // path matching nothing) degrades to the historical dump-everything
    // rather than an empty waveform.
    if set.is_empty() && scope_count > 0 {
        return None;
    }
    Some(set)
}

fn full_snapshot(st: &SimState) -> Vec<(IdCode, sim_ir::BitPacked, u32)> {
    let mut out = Vec::new();
    for slot in &st.nets {
        if !slot.vcd_word_ids.is_empty() {
            for (word, id) in slot.vcd_word_ids.iter().enumerate() {
                if let Some(id) = id {
                    out.push((
                        *id,
                        nth_word(&slot.cur, slot.width, word as u32),
                        slot.width,
                    ));
                }
            }
        } else if let Some(id) = slot.vcd_id {
            out.push((id, word0(&slot.cur, slot.width), slot.width));
        }
    }
    out
}

/// Extract array word `k` (`width` bits) from a packed net store.
fn nth_word(store: &sim_ir::BitPacked, width: u32, word: u32) -> sim_ir::BitPacked {
    let base = word * width;
    let mut v = Value::zeros(width.max(1), false);
    v.width = width;
    for i in 0..width {
        let bit = base + i;
        let w = (bit / 64) as usize;
        let s = bit % 64;
        let bv = store.val.get(w).map_or(0, |x| (x >> s) & 1);
        let bu = store.unk.get(w).map_or(0, |x| (x >> s) & 1);
        v.set_vu(i, bv, bu);
    }
    v.into_bitpacked(width)
}

/// Per-element VCD var name: row-major word → declared indices (`lo + digit`
/// per dim, e.g. word 5 of `[0:1][0:2]` ⇒ `leaf[1][2]`).
fn elem_name(leaf: &str, dims: &[(u32, u32)], word: u32) -> String {
    let mut digits = vec![0u32; dims.len()];
    let mut rem = u64::from(word);
    for k in (0..dims.len()).rev() {
        let size = u64::from(dims[k].1.max(1));
        digits[k] = (rem % size) as u32;
        rem /= size;
    }
    let mut s = String::from(leaf);
    for (k, &(lo, _)) in dims.iter().enumerate() {
        s.push('[');
        s.push_str(&(lo + digits[k]).to_string());
        s.push(']');
    }
    s
}

/// Extract array-word-0 (`width` bits) from a packed net store.
fn word0(store: &sim_ir::BitPacked, width: u32) -> sim_ir::BitPacked {
    let mut v = Value::zeros(width.max(1), false);
    v.width = width;
    for i in 0..width {
        let w = (i / 64) as usize;
        let s = i % 64;
        let bv = store.val.get(w).map_or(0, |x| (x >> s) & 1);
        let bu = store.unk.get(w).map_or(0, |x| (x >> s) & 1);
        v.set_vu(i, bv, bu);
    }
    v.into_bitpacked(width)
}

// ── argument / const string helpers ────────────────────────────────────────

/// Read a string from a $dumpfile/$display arg ExprId → Const{StrUtf8} → bytes.
fn arg_string(sched: &Scheduler, eid: Option<u32>) -> String {
    let Some(eid) = eid else { return String::new() };
    if let sim_ir::Expr::Const { val } = &sched.st.ir.exprs[eid as usize] {
        return const_string(sched.st.ir, *val);
    }
    // non-const arg: render its value as decimal (best-effort)
    fmt_dec(&sched.eval(eid))
}

/// Resolve an ExprId that is a `Const{val}` into its const string (format str).
fn expr_const_string(st: &SimState, eid: u32) -> String {
    if let sim_ir::Expr::Const { val } = &st.ir.exprs[eid as usize] {
        st.fmt_const_string(*val) // FMT-CACHE: memoized by ConstId
    } else {
        String::new()
    }
}

/// Decode a `ConstVal` (StrUtf8 → text; numeric → packed bytes).
pub(crate) fn const_string(ir: &sim_ir::SimIr, cid: u32) -> String {
    let c = &ir.consts[cid as usize];
    let nbytes = ((c.width + 7) / 8) as usize;
    let mut bytes = Vec::with_capacity(nbytes);
    // StrUtf8 packs in IEEE §5.9 order (v6): the FIRST character is the MOST
    // significant byte — read the value top byte down to recover source order.
    for b in (0..nbytes).rev() {
        let bit = (b as u32) * 8;
        let w = (bit / 64) as usize;
        let s = bit % 64;
        let byte = if s <= 56 {
            (c.bits.val.get(w).copied().unwrap_or(0) >> s) as u8
        } else {
            let lo = c.bits.val.get(w).copied().unwrap_or(0) >> s;
            let hi = c.bits.val.get(w + 1).copied().unwrap_or(0) << (64 - s);
            (lo | hi) as u8
        };
        bytes.push(byte);
    }
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

// ── $display format engine (4-state aware) ─────────────────────────────────

pub(crate) fn format_args_str(
    sched: &Scheduler,
    fmt: Option<u32>,
    args: &[u32],
    radix: Option<u8>,
) -> String {
    let mut out = String::new();
    let mut argi = 0usize;
    if let Some(fmt_eid) = fmt {
        // FROZEN IR: `SysTask.fmt` is an ExprId pointing to a `Const{val}` whose
        // `val` is the format-string ConstId (verified against elaborate).
        let template = expr_const_string(sched.st, fmt_eid);
        render_template(sched, &template, args, &mut argi, &mut out);
    }
    // IEEE 1364-2005 §17.1 (P0-8): any argument NOT consumed by a format
    // string prints sequentially — a string-literal arg is itself a format
    // segment (its `%` specs consume the args that follow it); every other
    // arg prints in the default radix (a padded `%d` field; `%g` for a real).
    // Previously everything after the leading format string was silently
    // dropped, and a bare string arg printed as a packed-ASCII decimal.
    while argi < args.len() {
        let e = args[argi];
        argi += 1;
        if let Some(text) = str_const_of_expr(sched.st, e) {
            render_template(sched, &text, args, &mut argi, &mut out);
        } else {
            push_default_radix(&sched.eval(e), &mut out, radix);
        }
    }
    out
}

/// The argument ExprId IFF it is a string-literal constant (`ConstRepr::StrUtf8`).
fn str_const_of_expr(st: &SimState, eid: u32) -> Option<String> {
    if let sim_ir::Expr::Const { val } = &st.ir.exprs[eid as usize] {
        if st.ir.consts[*val as usize].repr == sim_ir::ConstRepr::StrUtf8 {
            return Some(st.fmt_const_string(*val)); // FMT-CACHE
        }
    }
    None
}

/// Default-radix rendering of an argument with no format spec: a padded `%d`
/// field (`%g` for a real) — or, under a b/o/h task variant (P1-5), the padded
/// `%b`/`%o`/`%h` form (same `fmt_radix` the explicit specs use; iverilog joins
/// these fields with no separator).
fn push_default_radix(v: &Value, out: &mut String, radix: Option<u8>) {
    if v.is_real {
        out.push_str(&fmt_real(v, 'g', None, None));
        return;
    }
    match radix {
        Some(2) => out.push_str(&fmt_radix(v, 1, false, None)),
        Some(8) => out.push_str(&fmt_radix(v, 3, false, None)),
        Some(16) => out.push_str(&fmt_radix(v, 4, false, None)),
        _ => {
            let s = fmt_dec(v);
            let fw = dec_field_width(v.width, v.signed);
            if s.len() < fw {
                out.push_str(&" ".repeat(fw - s.len()));
            }
            out.push_str(&s);
        }
    }
}

/// iverilog-style `%v` strength form: per-bit St0/St1/StX/HiZ, MSB-first, joined
/// by `_` (live pin: `4'b10xz` → "St1_St0_StX_HiZ"). vitamin has no strength model,
/// so a driven bit takes the conventional STRONG (St) prefix and z is HiZ — an
/// approximation that matches iverilog for register / strong-net designs. A 1-bit
/// value yields a single field (e.g. "St1"), unchanged from the old behavior.
fn strength_form(v: &Value) -> String {
    let w = v.width.max(1);
    (0..w)
        .rev()
        .map(|bi| match v.get_vu(bi) {
            (0, 0) => "St0",
            (1, 0) => "St1",
            (1, 1) => "HiZ",
            _ => "StX",
        })
        .collect::<Vec<_>>()
        .join("_")
}

fn render_template(
    sched: &Scheduler,
    template: &str,
    args: &[u32],
    argi: &mut usize,
    out: &mut String,
) {
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // optional width/flags: %0d, %5h, %8.2f …  (v1 records `0` for integer
        // specs; width/precision are threaded into the real `%f/%e/%g` formatters).
        let mut min_zero = false;
        let mut width_digits = String::new();
        while let Some(&d) = chars.peek() {
            if d == '0' && width_digits.is_empty() {
                min_zero = true;
                width_digits.push('0');
                chars.next();
            } else if d.is_ascii_digit() {
                width_digits.push(d);
                chars.next();
            } else {
                break;
            }
        }
        let mut prec_digits = String::new();
        let mut has_prec = false;
        if chars.peek() == Some(&'.') {
            has_prec = true;
            chars.next();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    prec_digits.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
        }
        let field_width: Option<usize> = width_digits
            .trim_start_matches('0')
            .parse::<usize>()
            .ok()
            .or_else(|| {
                if width_digits.chars().all(|c| c == '0') && !width_digits.is_empty() {
                    Some(0)
                } else {
                    None
                }
            });
        let precision: Option<usize> = if has_prec {
            Some(prec_digits.parse::<usize>().unwrap_or(0))
        } else {
            None
        };
        let spec = chars.next().unwrap_or('%');
        match spec {
            '%' => out.push('%'),
            // P2-11: hierarchical scope of the EXECUTING process (was: always
            // the literal "top"). Strobe/monitor renders restore the
            // REGISTERING process's scope first (FmtCapture.scope).
            'm' | 'M' => out.push_str(&sched.st.cur_scope),
            't' | 'T' => {
                // `%T` aliases `%t` (consumes one time arg). Full §21.3.2 semantics:
                // the arg (a time in the DISPLAYING module's unit) is rescaled to the
                // `$timeformat` units (default: the global precision) and justified
                // in the min field width (default 20; an explicit `%Nt`/`%0t`
                // OVERRIDES it — iverilog-pinned).
                let v = next_arg(sched, args, argi);
                out.push_str(&fmt_time_spec(sched, &v, field_width, min_zero));
            }
            'd' | 'D' => {
                let v = next_arg(sched, args, argi);
                // IEEE 1364 %d: right-justify in a field width. `%0d` ⇒ minimal;
                // `%Nd` ⇒ width N; bare `%d` ⇒ the operand's default decimal width
                // (digit count of its max value). An X/Z prints as a right-justified
                // `x`/`z` in that field, like a numeric value.
                let s = fmt_dec(&v);
                // `%0d` (bare leading zero, no width) = minimal; `%0Nd` = zero-pad
                // to N (sign-aware: "-42"→"-00042"); `%Nd` = space-pad to N; bare
                // `%d` = the operand's default decimal field width (iverilog-pinned).
                let fw = match (min_zero, field_width) {
                    (true, Some(0)) => 0,
                    (_, Some(n)) => n,
                    (_, None) => dec_field_width(v.width, v.signed),
                };
                if s.len() < fw {
                    let pad = fw - s.len();
                    if min_zero {
                        // zero-pad AFTER any leading sign: "-42" → "-00042".
                        if let Some(rest) = s.strip_prefix('-') {
                            out.push('-');
                            out.push_str(&"0".repeat(pad));
                            out.push_str(rest);
                        } else {
                            out.push_str(&"0".repeat(pad));
                            out.push_str(&s);
                        }
                    } else {
                        out.push_str(&" ".repeat(pad));
                        out.push_str(&s);
                    }
                } else {
                    out.push_str(&s);
                }
            }
            'h' | 'H' | 'x' | 'X' => {
                let v = next_arg(sched, args, argi);
                out.push_str(&fmt_radix(&v, 4, min_zero, field_width));
            }
            'o' | 'O' => {
                let v = next_arg(sched, args, argi);
                out.push_str(&fmt_radix(&v, 3, min_zero, field_width));
            }
            'b' | 'B' => {
                let v = next_arg(sched, args, argi);
                out.push_str(&fmt_radix(&v, 1, min_zero, field_width));
            }
            'f' | 'F' | 'g' | 'G' | 'e' | 'E' => {
                let v = next_arg(sched, args, argi);
                let s = fmt_real(&v, spec, field_width, precision);
                // `%E`/`%G` uppercase the exponent letter and non-finite labels
                // (iverilog: `%E` → "1.5E+20", `%G` → "1E-05", `%E` of inf → "INF").
                // `%F` is identical to `%f` for ALL values including inf/nan — iverilog
                // outputs lowercase "inf"/"nan" for `%F` (only `%E`/`%G` uppercase them).
                if spec == 'E' || spec == 'G' {
                    // ASCII-only output (digits/`.`/`-`/`+`/`e`/inf/nan) — uppercase
                    // affects only `e`→`E` and inf/nan→INF/NAN.
                    out.push_str(&s.to_ascii_uppercase());
                } else {
                    out.push_str(&s);
                }
            }
            'c' | 'C' => {
                let v = next_arg(sched, args, argi);
                out.push(char_of(&v));
            }
            's' | 'S' => {
                let e = args.get(*argi).copied();
                *argi += 1;
                // Build the content string, then right-justify it in an explicit
                // field width (a MINIMUM — a longer string overflows, it is never
                // truncated). The content for an explicit-width `%Ns`/`%0s` on a
                // packed reg is its leading-NUL-stripped form; a bare `%s` keeps the
                // full reg-width form (NUL → space). A string literal / string-domain
                // value renders its exact text either way. (iverilog-pinned: 64-bit
                // "hello" → `%s` "   hello", `%2s` "hello", `%10s` "     hello".)
                let content = match e {
                    // string LITERAL: decoded text (the classic fmt-arg path).
                    Some(eid)
                        if matches!(
                            sched.st.ir.exprs.get(eid as usize),
                            Some(sim_ir::Expr::Const { .. })
                        ) =>
                    {
                        arg_string(sched, Some(eid))
                    }
                    Some(eid) => {
                        let v = sched.eval(eid);
                        if v.is_str {
                            // v7 P2-C: a STRING-domain value renders its EXACT bytes.
                            String::from_utf8_lossy(&v.to_str_bytes()).into_owned()
                        } else if field_width.is_some() {
                            // `%0s` / `%Ns` strip leading NUL padding (all-NUL → "").
                            fmt_packed_chars_min(&v)
                        } else {
                            // bare `%s` pads to the reg width (NUL → space).
                            fmt_packed_chars(&v)
                        }
                    }
                    None => String::new(),
                };
                let clen = content.chars().count();
                match field_width {
                    Some(n) if clen < n => {
                        out.push_str(&" ".repeat(n - clen));
                        out.push_str(&content);
                    }
                    _ => out.push_str(&content),
                }
            }
            // P0-8③: the remaining IEEE specs CONSUME their argument — leaving
            // them unconsumed shifted every later spec onto the wrong arg.
            'v' | 'V' => {
                let v = next_arg(sched, args, argi);
                out.push_str(&strength_form(&v));
            }
            // binary-dump specs: consume; vitamin emits no text for them (v1 —
            // the IEEE form writes raw bytes, useless in a text log).
            'u' | 'U' | 'z' | 'Z' => {
                let _ = next_arg(sched, args, argi);
            }
            // `%p` (SV assignment pattern): minimal-width value form (v1).
            'p' | 'P' => {
                let v = next_arg(sched, args, argi);
                out.push_str(&fmt_dec(&v));
            }
            other => {
                out.push('%');
                out.push(other);
            }
        }
    }
}

/// `%s` on a packed value: width/8 chars MSB-first, NUL bytes render as
/// spaces (iverilog live pin, probe t7).
pub(crate) fn fmt_packed_chars(v: &Value) -> String {
    let nbytes = (v.width as usize).div_ceil(8).max(1);
    let mut s = String::with_capacity(nbytes);
    for bi in (0..nbytes).rev() {
        let byte = packed_byte(v, bi);
        s.push(if byte == 0 { ' ' } else { byte as char });
    }
    s
}

/// Byte `bi` of a packed value (LSB byte = 0). Unknown (x/z) bits are masked OFF
/// before the byte is read — so an x byte (val=0) AND a z byte (val=1,unk=1) both
/// read as `0` (a NUL → rendered as a space), matching iverilog. vita's z-bit
/// encoding is val=1, which would otherwise leak `0xFF`.
fn packed_byte(v: &Value, bi: usize) -> u8 {
    let bit = bi * 8;
    let w = bit / 64;
    let known = v.val.get(w).copied().unwrap_or(0) & !v.unk.get(w).copied().unwrap_or(0);
    (known >> (bit % 64)) as u8
}

/// `%0s` on a packed value: like [`fmt_packed_chars`] but the LEADING NUL bytes
/// (the high zero-byte padding of a string in a wider reg) are dropped rather than
/// rendered as spaces. Once the first non-NUL byte is seen, every later byte is
/// emitted (an embedded or trailing NUL still becomes a space). An all-NUL value
/// yields the empty string. iverilog-pinned: 64-bit "hello" → "hello"; "hi\0\0" →
/// "hi  "; "\0h\0i" → "h i"; all-NUL → "".
pub(crate) fn fmt_packed_chars_min(v: &Value) -> String {
    let nbytes = (v.width as usize).div_ceil(8).max(1);
    let mut s = String::with_capacity(nbytes);
    let mut started = false;
    for bi in (0..nbytes).rev() {
        let byte = packed_byte(v, bi); // x/z masked → 0, so leading x/z also strips
        if !started {
            if byte == 0 {
                continue; // skip leading NUL (and x/z) padding
            }
            started = true;
        }
        s.push(if byte == 0 { ' ' } else { byte as char });
    }
    s
}

fn next_arg(sched: &Scheduler, args: &[u32], argi: &mut usize) -> Value {
    let e = args.get(*argi).copied();
    *argi += 1;
    e.map(|x| sched.eval(x)).unwrap_or_else(Value::x1)
}

/// IEEE %d default field width = decimal digit count of an `n`-bit operand's max
/// value (`2^n − 1`): 1-bit→1, 8-bit→3, 32-bit→10. Computed exactly up to 128 bits,
/// then via `n·log10(2)` (a column-alignment hint; exactness beyond 128 is moot).
fn dec_field_width(n: u32, signed: bool) -> usize {
    if n == 0 {
        return 1;
    }
    if signed && n > 1 {
        // A signed `%d` field holds a sign char plus the digits of the most-negative
        // magnitude 2^(n-1) (iverilog-pinned: 8-bit → "-128" = 4, 32-bit →
        // "-2147483648" = 11). This is NOT simply unsigned_width + 1 — for some
        // widths the two coincide (10-bit: signed "-512" and unsigned 1023 are both
        // 4 wide), so the magnitude must be computed directly. A 1-bit signed value
        // is the exception: iverilog gives it field width 1 (NOT 2), so n==1 falls
        // through to the unsigned branch below (the lone `-1` overflows the 1-col
        // field, as in iverilog).
        if n <= 128 {
            let mag: u128 = 1u128 << (n - 1);
            1 + mag.to_string().len()
        } else {
            // wide: digits(2^(n-1)) ≈ (n-1)*log10(2)+1, plus the sign.
            2 + ((n - 1) as f64 * std::f64::consts::LOG10_2) as usize
        }
    } else if n <= 128 {
        let maxv: u128 = if n == 128 {
            u128::MAX
        } else {
            (1u128 << n) - 1
        };
        maxv.to_string().len()
    } else {
        (n as f64 * std::f64::consts::LOG10_2) as usize + 1
    }
}

/// IEEE 1800 §21.2.1.2 letter for a bit range `[lo,hi)` that contains ≥1 unknown
/// bit. The letter is LOWERCASE only when the group is UNIFORM — no known bit AND
/// a single unknown kind (entirely x → `x`, entirely z → `z`). ANY mixing —
/// known+unknown, or x+z together — is UPPERCASE (`X` if any x, else `Z`). x takes
/// precedence over z. `None` when the group is fully known. (iverilog-pinned: e.g.
/// `8'bxxxxzzzz` prints `X`, not `x`.)
fn unknown_group_char(v: &Value, lo: u32, hi: u32) -> Option<char> {
    let (mut has_known, mut has_x, mut has_z) = (false, false, false);
    for i in lo..hi {
        let (val, unk) = v.get_vu(i);
        if unk == 1 {
            if val == 0 {
                has_x = true; // x = (val0, unk1); z = (val1, unk1)
            } else {
                has_z = true;
            }
        } else {
            has_known = true;
        }
    }
    if !has_x && !has_z {
        return None;
    }
    let uppercase = has_known || (has_x && has_z);
    Some(match (uppercase, has_x) {
        (false, true) => 'x',  // uniform all-x, no known
        (false, false) => 'z', // uniform all-z, no known
        (true, true) => 'X',   // mixed (known and/or x+z), some x
        (true, false) => 'Z',  // mixed, all unknowns are z
    })
}

/// %d: decimal. A value with any X/Z renders one §21.2.1.2 letter for the whole
/// field (`x`/`z` if entirely unknown, `X`/`Z` if partially). A real ROUNDS
/// half-away (saturating to i64 extremes; NaN → 0).
fn fmt_dec(v: &Value) -> String {
    if v.is_real {
        let x = v.to_f64().unwrap_or(0.0);
        // round half-away; large |x| SATURATES to i64::MAX/MIN; NaN.round() as i64 == 0.
        return format!("{}", x.round() as i64);
    }
    if let Some(c) = unknown_group_char(v, 0, v.width) {
        return c.to_string();
    }
    // Exact decimal at ANY width (Phase-1.x ⑥): a wide signed value renders
    // sign + two's-complement magnitude; unsigned long-divides by 10^19.
    // (%d used to render signed >64 as unsigned and TRUNCATE past 128 bits.)
    let n = crate::value::nwords(v.width).max(1);
    let mut words: Vec<u64> = (0..n).map(|k| v.val.get(k).copied().unwrap_or(0)).collect();
    let neg = v.signed && v.width >= 1 && v.get_vu(v.width - 1).0 == 1;
    if neg {
        words = crate::eval::mw_mask(crate::eval::mw_neg(&words), v.width);
    }
    let s = crate::eval::mw_decimal(&words);
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

/// `%t` (IEEE 1800 §21.3.2): render a time VALUE (expressed in the displaying
/// module's time unit) per the live `$timeformat` state. iverilog-pinned:
/// - display value = arg × M (the module unit in ticks), decimal-shifted by
///   `units_exp − global_prec_exp` places (default units = the global precision,
///   so a 1ns/1ps module shows ps: `%0t` of `$time`=5 → "5000");
/// - INTEGER args are exact decimal-string math and TRUNCATE the fraction to
///   `prec` digits (9995ns @ −6/prec 1 → "9.9", never rounds); REAL args go
///   through f64 and round like `%f` (5.556 @ prec 2 → "5.56");
/// - an unknown-bearing integer keeps its collapsed x/X/z/Z char while the net
///   shift only APPENDS zeros ("X000", "X.00 ns") and falls back to numeric with
///   the unknown bits cleared once the shift cuts into the digits ("0.00us");
/// - the suffix appends verbatim; the result is right-justified in the explicit
///   `%Nt`/`%0t` width if given, else the min field width (default 20); a
///   NEGATIVE min width LEFT-justifies in |width|.
fn fmt_time_spec(
    sched: &Scheduler,
    v: &Value,
    explicit_width: Option<usize>,
    min_zero: bool,
) -> String {
    let (units_exp, prec, suffix, minw): (i32, usize, &str, i64) = match &sched.st.timeformat {
        Some(tf) => (
            tf.units_exp,
            tf.prec as usize,
            tf.suffix.as_str(),
            tf.minw as i64,
        ),
        None => (sched.st.global_prec_exp as i32, 0, "", 20),
    };
    let k = units_exp as i64 - sched.st.global_prec_exp as i64;
    let m = sched.st.cur_time_mult.max(1);
    // M is 10^(unit_exp − global_prec_exp) by the timescale-wiring contract, so
    // ilog10 recovers the exponent and the integer path stays exact string math.
    let mz = m.ilog10() as i64;
    debug_assert_eq!(10u64.checked_pow(mz as u32), Some(m));
    let shift = mz - k; // net decimal shift: ≥0 appends zeros, <0 cuts into digits
    let body = if v.is_real {
        // `$realtime`-style value: scale in f64, then %f-round to `prec` digits
        // (mirrors fmt_real's −0.0 normalization and non-finite C spelling).
        let x = v.to_f64().unwrap_or(0.0) * (m as f64) / 10f64.powi(k as i32);
        let x = if x == 0.0 { 0.0 } else { x };
        if x.is_finite() {
            format!("{x:.prec$}")
        } else {
            nonfinite_c(x)
        }
    } else if let Some(c) = unknown_group_char(v, 0, v.width) {
        if shift >= 0 {
            // Collapsed unknown char + appended zeros + zero-filled fraction.
            let frac = if prec > 0 {
                format!(".{}", "0".repeat(prec))
            } else {
                String::new()
            };
            format!("{c}{}{frac}", "0".repeat(shift as usize))
        } else {
            // Scale-down goes numeric with unknown bits cleared (iverilog live).
            let mut cleared = v.clone();
            for i in 0..cleared.val.len() {
                let u = cleared.unk[i];
                cleared.val[i] &= !u;
                cleared.unk[i] = 0;
            }
            time_digits_shift(&fmt_dec(&cleared), shift, prec)
        }
    } else {
        time_digits_shift(&fmt_dec(v), shift, prec)
    };
    let full = format!("{body}{suffix}");
    let width: i64 = explicit_width.map(|w| w as i64).unwrap_or(minw);
    let n = full.chars().count() as i64;
    if width >= 0 {
        if n < width {
            // `%0Nt`: zero-fill; zeros go before any sign char (iverilog-pinned:
            // `%08t` of -1000 → "000-1000", not sign-aware like `%0Nd`).
            let fill = if min_zero { "0" } else { " " };
            return format!("{}{full}", fill.repeat((width - n) as usize));
        }
    } else if n < -width {
        // Negative min width ⇒ left-justify in |width| (iverilog-pinned).
        return format!("{full}{}", " ".repeat((-width - n) as usize));
    }
    full
}

/// Shift the decimal point of an exact decimal string (optionally `-`-signed) by
/// `shift` places (≥0 appends zeros; <0 inserts a point) and TRUNCATE/zero-fill
/// the fraction to `prec` digits — the iverilog `%t` integer behavior (it never
/// rounds: 9.995 @ prec 1 renders "9.9").
fn time_digits_shift(digits: &str, shift: i64, prec: usize) -> String {
    let (sign, mag) = match digits.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", digits),
    };
    let body = if shift >= 0 {
        // A zero value stays "0" — appending scale zeros would render "0000"
        // (iverilog-pinned: `%t` of $time=0 in a 1ns/1ps module is "0").
        let int_part = if mag == "0" {
            mag.to_string()
        } else {
            format!("{mag}{}", "0".repeat(shift as usize))
        };
        if prec > 0 {
            format!("{int_part}.{}", "0".repeat(prec))
        } else {
            int_part
        }
    } else {
        let p = (-shift) as usize;
        // Left-pad so at least one integer digit remains ("2" @ p=3 → "0002").
        let padded = if mag.len() <= p {
            format!("{}{mag}", "0".repeat(p + 1 - mag.len()))
        } else {
            mag.to_string()
        };
        let cut = padded.len() - p;
        if prec == 0 {
            padded[..cut].to_string()
        } else {
            let frac: String = padded[cut..]
                .chars()
                .chain(std::iter::repeat('0'))
                .take(prec)
                .collect();
            format!("{}.{frac}", &padded[..cut])
        }
    };
    format!("{sign}{body}")
}

/// `$timeformat` executor (§21.3.2): 0 args resets the defaults; 4 args set
/// (units, precision, suffix, min width). Elaborate guarantees the 0/4 arity
/// (1–3 args are a compile-time error, iverilog parity). Values are evaluated
/// at EXECUTION time (runtime-variable args are legal). A negative precision
/// clamps to 0. Units and min width are taken arithmetically like iverilog, but
/// each caps at a hostile-input bound (|units−precision| ≤ 64 decimal places,
/// |min width| ≤ 4096, precision ≤ 64 digits — the WIDE-ARITH-CAP pattern):
/// `$timeformat(-2_000_000_000, …)` would otherwise make `%t` allocate a
/// multi-GiB zero string. The caps sit far above every physical unit (the IEEE
/// table spans 2…−15) so no real testbench can observe them.
/// The suffix is %s-coerced NOW: a string-domain value keeps its exact bytes; a
/// packed value renders its NUL-stripped ASCII form (8'h6E → "n").
fn run_timeformat(sched: &mut Scheduler, args: &[u32]) -> Ctl {
    if args.is_empty() {
        sched.st.timeformat = None;
        return Ctl::Continue;
    }
    fn int_arg(sched: &Scheduler, args: &[u32], i: usize) -> i64 {
        args.get(i)
            .map(|&e| {
                sched
                    .eval(e)
                    .to_i128_signed()
                    .unwrap_or(0)
                    .clamp(i64::MIN as i128, i64::MAX as i128) as i64
            })
            .unwrap_or(0)
    }
    let gp = sched.st.global_prec_exp as i64;
    let units_exp = int_arg(sched, args, 0).clamp(gp - 64, gp + 64) as i32;
    let prec = int_arg(sched, args, 1).clamp(0, 64) as u32;
    let suffix = match args.get(2).copied() {
        // String LITERAL: exact text. Every OTHER expr — including a NUMERIC
        // literal — goes through the same value path (`fmt_packed_chars_min`),
        // so a literal 8'h6E and a reg holding 8'h6E render the identical "n"
        // (soundness review Q7: the old const_string route stripped trailing
        // NULs while the value route spaces them — value-dependent divergence).
        Some(eid) => match str_const_of_expr(sched.st, eid) {
            Some(text) => text,
            None => {
                let v = sched.eval(eid);
                if v.is_str {
                    String::from_utf8_lossy(&v.to_str_bytes()).into_owned()
                } else {
                    fmt_packed_chars_min(&v)
                }
            }
        },
        None => String::new(),
    };
    let minw = int_arg(sched, args, 3).clamp(-4096, 4096) as i32;
    sched.st.timeformat = Some(crate::state::TfState {
        units_exp,
        prec,
        suffix,
        minw,
    });
    Ctl::Continue
}

/// C/iverilog spelling of a non-finite f64: lowercase `nan` (any sign — iverilog
/// never prints `-nan`) and `inf`/`-inf`. Rust's Display gives `NaN`, so every
/// real formatter must coerce here for `$display` parity (a silent-wrong
/// otherwise — caught by the N6 real-math domain-error oracle).
fn nonfinite_c(x: f64) -> String {
    if x.is_nan() {
        "nan".to_string()
    } else if x < 0.0 {
        "-inf".to_string()
    } else {
        "inf".to_string()
    }
}

/// `%f`/`%e`/`%g` of a real Value (the arg may be an integer promoted to real).
/// `width`/`prec` are the optional field-width / precision modifiers (`%8.2f`).
fn fmt_real(v: &Value, spec: char, width: Option<usize>, prec: Option<usize>) -> String {
    // Normalize IEEE-754 negative zero to +0.0 so every real spec displays it as
    // a plain "0" (the %g path / VCD `fmt_g` already do). iverilog prints a
    // constant/literal -0.0 as "0"/"0.000000"; matching it here keeps %f/%e/%g
    // internally consistent and was a $display silent-wrong (Rust's "{:.6}" of
    // -0.0 emits "-0.000000"). NOTE: a -0.0 deliberately CONSTRUCTED at runtime
    // (e.g. `-5.0*0.0`) also normalizes here — iverilog keeps that one's sign
    // ("-0"); that single corner is an accepted, documented divergence (negative
    // zero display is implementation-defined; IEEE 1800 does not pin it).
    let x = v.to_f64().unwrap_or(0.0);
    let x = if x == 0.0 { 0.0 } else { x };
    let body = if !x.is_finite() {
        // nan / inf / -inf — spelled the C way for every spec (incl. %g, which
        // fmt_g would also lowercase; routing here keeps all specs consistent).
        nonfinite_c(x)
    } else {
        match spec {
            'f' | 'F' => format!("{:.*}", prec.unwrap_or(6), x), // default 6 fractional digits
            'e' | 'E' => fmt_real_e(x, prec),
            'g' | 'G' => format_g(x, prec),
            _ => format!("{x}"),
        }
    };
    if let Some(w) = width {
        if body.len() < w {
            return format!("{}{}", " ".repeat(w - body.len()), body);
        }
    }
    body
}

/// %e → C/printf/LRM form: `prec` mantissa fraction digits (default 6), signed
/// exponent zero-padded to AT LEAST 2 digits (`1.500000e+03`). Non-finite is
/// handled by the caller (`fmt_real` short-circuits via `nonfinite_c`); this
/// guard defends direct callers.
fn fmt_real_e(x: f64, prec: Option<usize>) -> String {
    if !x.is_finite() {
        return nonfinite_c(x); // inf / -inf / nan
    }
    let p = prec.unwrap_or(6);
    let s = format!("{x:.p$e}"); // e.g. "1.500000e3" or "1.234500e-5"
    let (mant, exp) = s.split_once('e').expect("rust {:e} always emits 'e'");
    let (sign, digits) = match exp.strip_prefix('-') {
        Some(d) => ('-', d),
        None => ('+', exp),
    };
    let padded = if digits.len() < 2 {
        format!("{digits:0>2}")
    } else {
        digits.to_string()
    };
    format!("{mant}e{sign}{padded}")
}

/// %g: shortest of %e/%f with trailing zeros stripped, per C/LRM. `prec` is the
/// total significant-digit precision P (default 6). REALG-DEDUP: delegates to the
/// single shared `vcd_writer::fmt_g` so the `$display %g` and VCD `%g` formatters
/// can never drift apart.
fn format_g(x: f64, prec: Option<usize>) -> String {
    vcd_writer::fmt_g(x, prec.unwrap_or(6).max(1) as i32)
}

/// %h/%o/%b: group bits per digit (1=bin,3=oct,4=hex), MSB-first; a group with
/// any X → 'x', any Z (no X) → 'z'.
fn fmt_radix(v: &Value, bits_per_digit: u32, min_zero: bool, field_width: Option<usize>) -> String {
    if v.width == 0 {
        return "0".to_string();
    }
    let ndig = (v.width + bits_per_digit - 1) / bits_per_digit;
    let mut s = String::new();
    for d in (0..ndig).rev() {
        let base = d * bits_per_digit;
        let mut val = 0u32;
        let mut has_x = false;
        let mut has_z = false;
        let mut has_known = false;
        for k in 0..bits_per_digit {
            let bi = base + k;
            if bi >= v.width {
                continue;
            }
            let (b, u) = v.get_vu(bi);
            match (b, u) {
                (_, 0) => {
                    has_known = true;
                    if b == 1 {
                        val |= 1 << k;
                    }
                }
                (0, 1) => has_x = true,
                (1, 1) => has_z = true,
                _ => {}
            }
        }
        // §21.2.1.2 per-digit: lowercase x/z only when the digit is UNIFORM (no
        // known bit AND a single unknown kind); any mixing (known+unknown, or x+z)
        // is uppercase X/Z. x takes precedence over z.
        s.push(if has_x || has_z {
            let uppercase = has_known || (has_x && has_z);
            match (uppercase, has_x) {
                (false, true) => 'x',
                (false, false) => 'z',
                (true, true) => 'X',
                (true, false) => 'Z',
            }
        } else {
            std::char::from_digit(val, 16).unwrap()
        });
    }
    // Width/flag handling (iverilog-pinned):
    //   `%h`   → full vector width (leading zeros retained)
    //   `%0h`  → minimum width: strip leading zeros (keep ≥1 digit)
    //   `%0Nh` → zero-pad to N digits
    //   `%Nh`  → space-pad to N digits (over the full-width form)
    let base = if min_zero && field_width == Some(0) {
        let trimmed = s.trim_start_matches('0');
        if trimmed.is_empty() {
            "0".to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        s
    };
    match field_width {
        Some(w) if base.len() < w => {
            let pad = if min_zero { '0' } else { ' ' };
            let mut p: String = std::iter::repeat(pad).take(w - base.len()).collect();
            p.push_str(&base);
            p
        }
        _ => base,
    }
}

fn char_of(v: &Value) -> char {
    // IEEE %c: the LOW 8 bits regardless of value width — a wide value with
    // high bits set must not degrade to NUL under the strict no-truncation
    // `to_u64`. X/Z keeps the old None→0 policy.
    if v.has_xz() {
        return '\0';
    }
    let byte = (v.val.first().copied().unwrap_or(0) & 0xFF) as u8;
    byte as char
}

/// v5 (C): resolve a dyn-method HANDLE argument (the ExprId of the handle's
/// whole-net `Signal`) to its NetId. Anything else → None (defensive no-op).
fn dyn_handle_net(sched: &Scheduler, arg: Option<&u32>) -> Option<u32> {
    let &eid = arg?;
    match sched.st.ir.exprs.get(eid as usize) {
        Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
        _ => None,
    }
}

/// ⓑ-breadth (v16): total order over array elements for `sort`/`rsort`. CLEAN
/// (no x/z) values compare by signed/unsigned numeric value; x/z values sort
/// AFTER all clean values, among themselves by a deterministic raw-bit order so
/// the sort is stable across runs/OSes and never panics (IEEE leaves the x/z
/// ordering implementation-defined).
///
/// `signed` is the array's DECLARED element type (§6.11.1), NOT each element's
/// stored provenance: a `32'h80000000` pushed into a signed `int q[$]` must sort
/// as a negative. We therefore force the comparison-domain sign onto a clone (the
/// stored `Value.signed` reflects how the element literal was written, which is
/// irrelevant to the array's order) rather than calling `to_i128_signed` on the
/// raw element, whose own `self.signed` gate would otherwise leak the literal's
/// provenance.
fn arr_cmp(a: &Value, b: &Value, signed: bool) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // Sign-extend at the element width regardless of the element's stored flag.
    let signed_key = |v: &Value| -> i128 {
        let mut t = v.clone();
        t.signed = true;
        t.to_i128_signed().unwrap_or(0)
    };
    match (a.has_xz(), b.has_xz()) {
        (false, false) => {
            if signed {
                signed_key(a).cmp(&signed_key(b))
            } else {
                a.to_u128().unwrap_or(0).cmp(&b.to_u128().unwrap_or(0))
            }
        }
        (false, true) => Ordering::Less,
        (true, false) => Ordering::Greater,
        (true, true) => (&*a.unk, &*a.val).cmp(&(&*b.unk, &*b.val)),
    }
}

/// Apply an in-place ordering method to a contiguous element slice.
fn apply_order(slice: &mut [Value], which: SysTaskId, signed: bool) {
    match which {
        SysTaskId::ArrSort => slice.sort_by(|a, b| arr_cmp(a, b, signed)),
        SysTaskId::ArrRsort => slice.sort_by(|a, b| arr_cmp(b, a, signed)),
        _ => slice.reverse(),
    }
}

/// ⓑ-breadth (v17): execute a queue-returning locator method (IEEE §7.12.1).
/// `args = [dst, src, kind_const, with_pred?]`. The source is snapshotted up
/// front so `dst == src` aliasing (`q = q.unique();`) is safe. Result elements
/// are cast to the dst element type before storage.
fn arr_locator(sched: &mut Scheduler, args: &[u32]) {
    let Some(dst_net) = dyn_handle_net(sched, args.first()) else {
        return;
    };
    let Some(src_net) = dyn_handle_net(sched, args.get(1)) else {
        return;
    };
    let code = args
        .get(2)
        .and_then(|&e| match sched.st.ir.exprs.get(e as usize) {
            Some(sim_ir::Expr::Const { val }) => sched
                .st
                .ir
                .consts
                .get(*val as usize)
                .and_then(|c| c.bits.val.first().copied()),
            _ => None,
        })
        .unwrap_or(0);
    let pred = args.get(3).copied();
    let signed = sched
        .st
        .ir
        .nets
        .get(src_net as usize)
        .map(|nv| nv.signed)
        .unwrap_or(true);
    let (dw, dsigned) = sched
        .st
        .ir
        .nets
        .get(dst_net as usize)
        .map(|nv| (nv.width.max(1), nv.signed))
        .unwrap_or((32, true));
    let elems = sched.st.dyn_values(src_net).unwrap_or_default();
    let idx_val = |i: usize| {
        let mut v = Value::zeros(32, true);
        v.val[0] = (i as u64).min(i32::MAX as u64);
        v
    };
    let mut result: Vec<Value> = Vec::new();
    match code {
        0 => {
            if let Some(m) = elems.iter().min_by(|a, b| arr_cmp(a, b, signed)) {
                result.push(m.clone());
            }
        }
        1 => {
            if let Some(m) = elems.iter().max_by(|a, b| arr_cmp(a, b, signed)) {
                result.push(m.clone());
            }
        }
        2 => {
            for e in &elems {
                if !result.iter().any(|r| r == e) {
                    result.push(e.clone());
                }
            }
        }
        9 => {
            let mut seen: Vec<Value> = Vec::new();
            for (i, e) in elems.iter().enumerate() {
                if !seen.iter().any(|r| r == e) {
                    seen.push(e.clone());
                    result.push(idx_val(i));
                }
            }
        }
        // find family — predicate-driven (`with` clause)
        _ => {
            if let Some(pred) = pred {
                let saved = sched.st.swap_array_item(None);
                let mut matches: Vec<(usize, Value)> = Vec::new();
                for (i, e) in elems.iter().enumerate() {
                    sched.st.swap_array_item(Some((e.clone(), i as u64)));
                    if sched.truthy(pred) {
                        matches.push((i, e.clone()));
                    }
                }
                sched.st.swap_array_item(saved);
                match code {
                    3 => result.extend(matches.iter().map(|(_, e)| e.clone())),
                    4 => result.extend(matches.iter().map(|(i, _)| idx_val(*i))),
                    5 => {
                        if let Some((_, e)) = matches.first() {
                            result.push(e.clone());
                        }
                    }
                    6 => {
                        if let Some((_, e)) = matches.last() {
                            result.push(e.clone());
                        }
                    }
                    7 => {
                        if let Some((i, _)) = matches.first() {
                            result.push(idx_val(*i));
                        }
                    }
                    8 => {
                        if let Some((i, _)) = matches.last() {
                            result.push(idx_val(*i));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let elems: std::collections::VecDeque<Value> = result
        .into_iter()
        .map(|v| v.resize_keep_sign(dw, dsigned))
        .collect();
    sched.st.dyn_heap[dst_net as usize] = Some(crate::state::DynObj::Queue { elems });
}

/// One W-RUN-DYN-DEGRADE per handle net (latched in `dyn_warned`) — a degraded
/// dyn op inside a loop must not spam the diagnostic stream.
fn dyn_warn_once(sched: &mut Scheduler, net: u32, msg: &str) {
    sched.st.dyn_warn_once_at(net, msg);
}
