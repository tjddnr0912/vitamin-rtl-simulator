//! split part of `builtins` (mechanical move).

use super::*;

/// §7.10.1 queue-slice executor: args = [dst_handle, src_handle, a, b].
/// Clamp semantics (hand-IEEE, no oracle — Icarus parses `q[a:b]` but ignores
/// the bounds): a<0 clamps to 0, b>size-1 clamps to size-1; a>b, a≥size, b<0,
/// or an x/z bound ⇒ the empty queue (x/z also warns once, the dyn pattern).
/// The result replaces dst wholesale (value semantics) and then the bounded-
/// queue post-op runs (mirrors the whole-copy path).
pub(crate) fn run_queue_slice(sched: &mut Scheduler, args: &[u32]) -> Ctl {
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
            let n = match sched
                .st
                .dyn_heap
                .borrow()
                .get(src as usize)
                .and_then(|o| o.as_ref())
            {
                Some(crate::state::DynObj::Queue { elems }) => elems.len() as i64,
                _ => 0,
            };
            let lo = a.max(0);
            let hi = b.min(n - 1);
            if lo > hi {
                std::collections::VecDeque::new()
            } else {
                match sched
                    .st
                    .dyn_heap
                    .borrow()
                    .get(src as usize)
                    .and_then(|o| o.as_ref())
                {
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
    if let Some(slot) = sched.st.dyn_heap.borrow_mut().get_mut(dst as usize) {
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
pub(crate) fn preopened_close_warn(sched: &mut Scheduler, fd: u32) {
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

/// P1-1: execute a severity task (doc-13 §Severity), optionally against an
/// ALTERNATE net store.
///
/// The user message renders through the SAME `format_args_str` engine as
/// `$display` (so `%0d`/defaults behave identically) but is emitted as a
/// `LogEvent::Diagnostic` — stderr in production, never the stdout stream.
/// Empty message ⇒ the code's title. `$fatal` aborts (implicit `$finish`,
/// `ExitClass::Fatal`); `$error` flags `HadErrors` and continues;
/// `$warning`/`$info` only print.
///
/// `nets = None` is the scheduler's own state. This IS a render site and it is
/// not in `dispatch.rs`, which is why the first enumeration of them missed it.
pub(crate) fn run_severity_with<N: crate::eval::NetReader + ?Sized>(
    sched: &mut Scheduler,
    nets: Option<&N>,
    sev: crate::SeverityKind,
    fmt: Option<u32>,
    args: &[u32],
) -> Ctl {
    let message = render_task_args(sched, nets, fmt, args, None);
    emit_severity_message(sched, sev, message)
}

/// Emit an already-rendered severity message to the diagnostic stream and apply
/// its control/exit-class effect. Split out of `run_severity_with` so a §16.4
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
            sched.st.had_error.set(true);
            Ctl::Continue
        }
        K::Warning | K::Info => Ctl::Continue,
    }
}

// ── $dumpvars: declare all nets, header, initial dump ──────────────────────

/// `dumpvars` against an ALTERNATE net store (tier-3 seam, S1d-4d-2). Only the
/// t0 value snapshot differs; everything else reads IR and metadata.
pub(crate) fn dumpvars_with<N: crate::eval::NetReader + ?Sized>(
    st: &mut SimState,
    nets: Option<&N>,
    args: &[u32],
) {
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
    // A2b-prereq: package-level variables (reserved `$pkg$…` scope) have no
    // VCD surface in v1 — iverilog parity for the bare dump (it declares no
    // package vars either), and an EXPLICIT `$dumpvars(…, pkg_var)` selection
    // is warned once, never silently ignored (iverilog asserts/crashes here).
    if st.net_names.len() == st.ir.nets.len() {
        if let Some(f) = &st.dump_filter {
            if f.iter().any(|&n| {
                st.net_names
                    .get(n as usize)
                    .is_some_and(|s| s.starts_with("$pkg$"))
            }) {
                use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
                st.sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Warning,
                    code: MsgCode::RunVcdPkgVarSkip,
                    message: "a package variable has no VCD surface (v1): it is \
                              excluded from the dump"
                        .to_string(),
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(TimeStamp { ticks: st.now }),
                }));
            }
        }
    }
    let path = st
        .vcd_path_override
        .clone()
        .or_else(|| st.dump_pending_path.clone())
        .unwrap_or_else(|| "dump.vcd".to_string());

    // G2 FST breadth: a `.fst` dump target is produced by transcoding the VCD at
    // finalize. Write the VCD to a temp sidecar now; `simulate` transcodes it to
    // the real `.fst` path and removes the sidecar once the writer has flushed.
    let is_fst = path.to_ascii_lowercase().ends_with(".fst");
    if is_fst {
        st.fst_target = Some(path.clone());
    }
    let write_path = if is_fst {
        format!("{path}.vcdtmp")
    } else {
        path.clone()
    };

    let file = match std::fs::File::create(&write_path) {
        Ok(f) => f,
        Err(e) => {
            // P2-1: the main artifact must not vanish silently — warn (with the
            // path + OS error) and keep simulating without a waveform.
            use diag::{Diagnostic, LogEvent, MsgCode, Severity, TimeStamp};
            st.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Warning,
                code: MsgCode::RunVcdOpenFail,
                message: format!("cannot open VCD dump file '{write_path}': {e}"),
                location: None,
                context: Vec::new(),
                sim_time: Some(TimeStamp { ticks: st.now }),
            }));
            st.fst_target = None;
            return;
        }
    };
    // P3-3/T0b: buffer the VCD sink (raw `File` was ~1 write syscall per record).
    // `finalize_vcd` flushes explicitly, so buffering never changes the bytes.
    // P4-T1: with `--threads ≥2` the buffered chunks go to a dedicated writer
    // thread (order-preserving bounded FIFO) — byte-identical, wall-clock only.
    // An FST target keeps the sidecar VCD single-threaded so the file is fully
    // flushed and closed (no writer thread to join) before `simulate` transcodes.
    let sink: crate::state::VcdSink = if st.threads >= 2 && !is_fst {
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
    // Declared ranges for nets stored with a NORMALIZED range (a negative low bound);
    // empty for almost every design, so the `$var` lines below are unchanged.
    let st_decl_ranges = st.net_decl_ranges.clone();
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
            // A2b-prereq: package variables live under the reserved `$pkg$<pkg>`
            // scope and have no VCD surface in v1 (iverilog parity). Dropped
            // BEFORE the scope-tree walk so no empty `$pkg$…` $scope is emitted.
            // No pre-existing design has such names → byte-identical.
            order.retain(|&i| !names[i].starts_with("$pkg$"));
            // Internal throwaway nets (`$ia_tmp$<n>` — an intra-assignment-delay
            // capture or a discarded sys-call sink like a bare `$random(seed);` /
            // `$sscanf(...);`) are implementation detail, not user signals; iverilog
            // emits no such net. Drop them from the VCD (the leaf carries the `$ia_tmp$`
            // sigil — the FQ name is `<scope>.$ia_tmp$<n>`). A synthetic temp is NEVER
            // an escaped identifier, so a name containing a backslash is a real user
            // signal (e.g. `\x.$ia_tmp$0`, whose embedded `.` the leaf split would
            // otherwise mistake for a scope separator) — keep it, unfiltered.
            order.retain(|&i| {
                names[i].contains('\\')
                    || !names[i]
                        .rsplit('.')
                        .next()
                        .unwrap_or(&names[i])
                        .starts_with("$ia_tmp$")
            });
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
                        let r = vcd_var_reference_decl(
                            vt,
                            &name,
                            nets[i].width,
                            nets[i].msb,
                            nets[i].lsb,
                            st_decl_ranges.get(&(i as u32)).copied(),
                        );
                        wv.push(w.declare_var(vt, nets[i].width.max(1), &r).ok());
                    }
                    word_ids[i] = wv;
                } else {
                    let r = vcd_var_reference_decl(
                        vt,
                        leaf,
                        nets[i].width,
                        nets[i].msb,
                        nets[i].lsb,
                        st_decl_ranges.get(&(i as u32)).copied(),
                    );
                    if let Ok(id) = w.declare_var(vt, nets[i].width.max(1), &r) {
                        ids[i] = Some(id);
                    }
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
                        let r = vcd_var_reference_decl(
                            vt,
                            &ename,
                            nv.width,
                            nv.msb,
                            nv.lsb,
                            st_decl_ranges.get(&(i as u32)).copied(),
                        );
                        wv.push(w.declare_var(vt, nv.width.max(1), &r).ok());
                    }
                    word_ids[i] = wv;
                } else {
                    let r = vcd_var_reference_decl(
                        vt,
                        &name,
                        nv.width,
                        nv.msb,
                        nv.lsb,
                        st_decl_ranges.get(&(i as u32)).copied(),
                    );
                    if let Ok(id) = w.declare_var(vt, nv.width.max(1), &r) {
                        ids[i] = Some(id);
                    }
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
    let snap = full_snapshot_with(st, nets);
    {
        let w = st.vcd.as_mut().unwrap();
        let _ = w.dump_initial(snap.iter().map(|(id, b, wd)| (*id, b, *wd)));
        let _ = w.set_time(st.now);
    }
    st.dumping = true;
    st.vcd_path = Some(path);
}

pub(crate) fn dump_on(st: &mut SimState) {
    st.dumping = true;
    let snap = full_snapshot(st);
    let now = st.now;
    if let Some(w) = st.vcd.as_mut() {
        let _ = w.set_time(now);
        let _ = w.dump_on(snap.iter().map(|(id, b, wd)| (*id, b, *wd)));
    }
}

pub(crate) fn dump_all(st: &mut SimState) {
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
pub(crate) fn dump_filter_from_args(
    st: &SimState,
    args: &[u32],
) -> Option<std::collections::BTreeSet<u32>> {
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

pub(crate) fn full_snapshot(st: &SimState) -> Vec<(IdCode, sim_ir::BitPacked, u32)> {
    full_snapshot_with::<crate::state::SimState>(st, None)
}

/// `full_snapshot` against an ALTERNATE net store (tier-3 seam, S1d-4d-2).
///
/// The id tables come from `st` either way — they are static metadata
/// `$dumpvars` filled — and only the VALUES come from `nets`. `None` means "use
/// `st`'s own store", which is what every engine call site passes, so those
/// sites are byte-identical by construction (the same shape `dispatch_with` and
/// `format_args_str_with` use).
///
/// This is the ONLY store read in the whole `$dumpvars` path: the header, the
/// scope/var declarations and `dump_filter_from_args` are IR and metadata. That
/// is why the task's refusal — "`full_snapshot` walks `&st.nets` wholesale" —
/// was one function rather than a subsystem.
pub(crate) fn full_snapshot_with<N: crate::eval::NetReader + ?Sized>(
    st: &SimState,
    nets: Option<&N>,
) -> Vec<(IdCode, sim_ir::BitPacked, u32)> {
    let mut out = Vec::new();
    for (n, slot) in st.nets.iter().enumerate() {
        if !slot.vcd_word_ids.is_empty() {
            for (word, id) in slot.vcd_word_ids.iter().enumerate() {
                if let Some(id) = id {
                    let v = match nets {
                        Some(r) => bits_of(&r.read_net(n as u32, Some(word as u32)), slot.width),
                        None => nth_word(&slot.cur, slot.width, word as u32),
                    };
                    out.push((*id, v, slot.width));
                }
            }
        } else if let Some(id) = slot.vcd_id {
            let v = match nets {
                Some(r) => bits_of(&r.read_net(n as u32, None), slot.width),
                None => word0(&slot.cur, slot.width),
            };
            out.push((id, v, slot.width));
        }
    }
    out
}

/// A `Value`'s planes as the `BitPacked` the VCD writer wants, at `width`.
fn bits_of(v: &Value, width: u32) -> sim_ir::BitPacked {
    let mut out = Value::zeros(width.max(1), false);
    out.width = width;
    for i in 0..width {
        let (bv, bu) = v.get_vu(i);
        out.set_vu(i, bv, bu);
    }
    sim_ir::BitPacked {
        val: out.val.to_vec(),
        unk: out.unk.to_vec(),
    }
}

/// Extract array word `k` (`width` bits) from a packed net store.
pub(crate) fn nth_word(store: &sim_ir::BitPacked, width: u32, word: u32) -> sim_ir::BitPacked {
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
pub(crate) fn elem_name(leaf: &str, dims: &[(i64, u32)], word: u32) -> String {
    let mut digits = vec![0i64; dims.len()];
    let mut rem = u64::from(word);
    for k in (0..dims.len()).rev() {
        let size = u64::from(dims[k].1.max(1));
        digits[k] = (rem % size) as i64;
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

/// Declared base (minimum declared index) of `net`'s FIRST unpacked dim, for the
/// file-I/O tasks whose address arguments live in the DECLARED index domain
/// (`$readmem`/`$writemem`/`$fread`). Absent from the sparse table ⇒ 0-based.
///
/// `net_dims` stores `(lo, SIZE)`. Reading the second field as an upper bound —
/// `lo.min(hi)` — happened to agree whenever `lo <= size` and silently addressed
/// `reg m[10:11]` from base 2 when it did not.
///
/// `None` when the declared base is NEGATIVE (`reg m[-1:1]`): every address below is
/// `u64`, so that domain has no representation here. The caller warns rather than
/// silently reading or writing from word 0.
pub(crate) fn declared_array_base(dims: &crate::NetDimsTable, net: u32) -> Option<u64> {
    match dims.get(&net).and_then(|d| d.first()) {
        Some(&(lo, _)) => u64::try_from(lo).ok(),
        None => Some(0),
    }
}

/// Extract array-word-0 (`width` bits) from a packed net store.
pub(crate) fn word0(store: &sim_ir::BitPacked, width: u32) -> sim_ir::BitPacked {
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
pub(crate) fn arg_string(st: &SimState, eid: Option<u32>) -> String {
    arg_string_with::<crate::state::SimState>(st, None, eid)
}

/// `arg_string` against an ALTERNATE net store (tier-3 seam, S1d-4d-2 round 2).
///
/// ⚠️ This function does NOT return early for a non-`Const` argument, and a
/// comment in `native/kernel.rs` claimed it did — which is what let `$dumpfile`
/// off the refusal list with a false argument. `$dumpfile(nm)` with `nm` an
/// ordinary reg falls through to the value render below, and on a native run
/// the ENGINE's store never moves: the waveform silently landed in a file named
/// `x` instead of `42`. Same stdout, same VCD content, wrong filename.
pub(crate) fn arg_string_with<N: crate::eval::NetReader + ?Sized>(
    st: &SimState,
    nets: Option<&N>,
    eid: Option<u32>,
) -> String {
    let Some(eid) = eid else { return String::new() };
    if let sim_ir::Expr::Const { val } = &st.ir.exprs[eid as usize] {
        return const_string(st.ir, *val);
    }
    // non-const arg: render its value as decimal (best-effort)
    match nets {
        Some(r) => fmt_dec(&st.mk_eval_ctx_with(r).eval(eid)),
        None => fmt_dec(&st.eval_expr(eid)),
    }
}

/// Resolve an ExprId that is a `Const{val}` into its const string (format str).
pub(crate) fn expr_const_string(st: &SimState, eid: u32) -> String {
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

/// Render a task's args, optionally against an ALTERNATE net store.
///
/// `nets = None` means "the scheduler's own state", which is what every engine
/// call site passes and what makes those sites byte-identical: the `None` arm is
/// literally the pre-existing `format_args_str(sched.st, …)` call. An
/// `Option` rather than a plain reader because the engine CANNOT pass both — its
/// reader IS `sched.st`, and `&mut Scheduler` plus a reborrow of `sched.st` do
/// not coexist. Tier-3 passes `Some(&arena)`, whose store is genuinely a
/// different object.
pub(crate) fn render_task_args<N: crate::eval::NetReader + ?Sized>(
    sched: &Scheduler,
    nets: Option<&N>,
    fmt: Option<u32>,
    args: &[u32],
    radix: Option<u8>,
) -> String {
    match nets {
        Some(n) => format_args_str_with(sched.st, n, fmt, args, radix),
        None => format_args_str(sched.st, fmt, args, radix),
    }
}

/// Evaluate a task ARGUMENT (not a format arg) against an optional alternate
/// net store — the fd of `$fdisplay`, the units of `$timeformat`.
///
/// The reader has to reach these too, and it is easy to miss that it does not:
/// they are net reads that never pass through the format engine, so threading
/// `render_task_args` alone leaves them reading `sched.st`. Measured, that is
/// exactly what happened — `$fdisplay(fd, …)` with a NET fd read the untouched
/// engine store, got X, and DROPPED the line with a bad-descriptor warning on a
/// design the gate reports fully runnable.
pub(crate) fn eval_task_arg<N: crate::eval::NetReader + ?Sized>(
    sched: &Scheduler,
    nets: Option<&N>,
    eid: u32,
) -> crate::value::Value {
    match nets {
        Some(n) => sched.st.eval_expr_with(n, eid),
        None => sched.eval(eid),
    }
}

/// The `$display` format engine, rendering against THIS state's nets.
///
/// A literal forward to [`format_args_str_with`] passing `st` as the reader, so
/// every existing caller keeps its behaviour by construction rather than by
/// inspection — the tier-3 seam added a parameter without touching a single
/// engine call site.
pub(crate) fn format_args_str(
    st: &SimState,
    fmt: Option<u32>,
    args: &[u32],
    radix: Option<u8>,
) -> String {
    format_args_str_with(st, st, fmt, args, radix)
}

/// `format_args_str` against an ALTERNATE net store (tier-3 seam, S1d-4b).
///
/// `st` still supplies every COLD field — the IR, `now`, widths, the time
/// multiplier, the format state — and `nets` supplies the net VALUES. The split
/// exists because tier-3's nets live in a `NetArena`, not in `SimState.nets`,
/// and the alternative to a parameter here was a second format engine. A second
/// format engine is the exact shape of defect this codebase keeps finding: two
/// spellings of one rule, drifting.
///
/// The reader threads down through `render_template`/`next_arg_with`, which is
/// the whole VALUE-reading path of the `$display` family.
///
/// ⚠️ WHAT THIS DOES NOT YET DO, stated because a seam is easy to mistake for a
/// wiring: **`builtins::dispatch` still hard-codes `sched.st` as the reader** at
/// each of its four render sites, so `k_dispatch_systask` cannot call it with an
/// arena yet. `dispatch` takes `&mut Scheduler` and would need the reader
/// threaded to it as well — that is S1d-4b-2, and it is the step that turns this
/// from a capability into a call. What this slice buys is that the format engine
/// itself no longer has to be duplicated or changed to get there.
///
/// Also still `SimState`-tied (same slice): `full_snapshot` (`$dumpvars`, which
/// walks `&st.nets` wholesale), `$timeformat`'s non-literal args, `$fclose`/
/// `$fdisplay`'s fd argument, `$dumplimit`, and `$writemem*`, which reads the
/// memory itself. Those are STORE reads rather than formatter reads, so they do
/// not thread through here at all.
pub(crate) fn format_args_str_with<N: crate::eval::NetReader + ?Sized>(
    st: &SimState,
    nets: &N,
    fmt: Option<u32>,
    args: &[u32],
    radix: Option<u8>,
) -> String {
    let mut out = String::new();
    let mut argi = 0usize;
    // ORDER SEAM: this function holds BOTH the reader and (through `st`) the
    // diagnostic sink, which makes it the one place a deferred out-of-range
    // report can be emitted at the moment the engine would have emitted it —
    // after the argument reads, before the caller writes its line or its
    // severity message. `_drain` at the end of the body does it; declaring the
    // guard here keeps the early `return` paths covered too.
    struct DrainRange<'a, N: crate::eval::NetReader + ?Sized>(&'a SimState<'a>, &'a N);
    impl<N: crate::eval::NetReader + ?Sized> Drop for DrainRange<'_, N> {
        fn drop(&mut self) {
            for unknown in self.1.take_deferred_range_kinds() {
                self.0.warn_run_index("array word index", unknown);
            }
        }
    }
    let _drain = DrainRange(st, nets);
    if let Some(fmt_eid) = fmt {
        // FROZEN IR: `SysTask.fmt` is an ExprId pointing to a `Const{val}` whose
        // `val` is the format-string ConstId (verified against elaborate).
        // NOT threaded, and that is a contract not an omission: `expr_const_string`
        // and `str_const_of_expr` read `Expr::Const` and return early for anything
        // else, so they cannot touch a net. Giving them a reader would add a
        // parameter no test could ever pin — the mutation that drops it survives
        // by construction, which is indistinguishable from a real gap.
        let template = expr_const_string(st, fmt_eid);
        render_template(st, nets, &template, args, &mut argi, &mut out);
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
        if let Some(text) = str_const_of_expr(st, e) {
            render_template(st, nets, &text, args, &mut argi, &mut out);
        } else {
            let v = st.eval_expr_with(nets, e);
            if v.is_str {
                // A string-typed VALUE (a `string` variable, not just a literal)
                // is itself a format segment (IEEE 1364-2005 §17.1): its `%` specs
                // consume the args that follow, exactly like a string literal.
                // Previously a runtime string fell through to push_default_radix
                // and printed as a packed-ASCII decimal (silently wrong).
                let text = String::from_utf8_lossy(&v.to_str_bytes()).into_owned();
                render_template(st, nets, &text, args, &mut argi, &mut out);
            } else {
                push_default_radix(&v, &mut out, radix);
            }
        }
    }
    out
}

/// The argument ExprId IFF it is a string-literal constant (`ConstRepr::StrUtf8`).
pub(crate) fn str_const_of_expr(st: &SimState, eid: u32) -> Option<String> {
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
pub(crate) fn push_default_radix(v: &Value, out: &mut String, radix: Option<u8>) {
    if v.is_real {
        out.push_str(&fmt_real(v, 'g', None, None, false, false, false));
        return;
    }
    match radix {
        Some(2) => out.push_str(&fmt_radix(v, 1, false, None, false)),
        Some(8) => out.push_str(&fmt_radix(v, 3, false, None, false)),
        Some(16) => out.push_str(&fmt_radix(v, 4, false, None, false)),
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
pub(crate) fn strength_form(v: &Value) -> String {
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

/// Pad `content` to `field_width` (a MINIMUM — a longer string is never
/// truncated): right-justified by default, or left-justified (space right-pad)
/// under the `-` flag. Used by the plain-content specs `%c`/`%v`/`%m`/`%s`, which
/// iverilog justifies in an explicit field width. `None` width → content verbatim.
pub(crate) fn justify(content: &str, field_width: Option<usize>, left_just: bool) -> String {
    match field_width {
        Some(n) => {
            let clen = content.chars().count();
            if clen < n {
                let pad = " ".repeat(n - clen);
                if left_just {
                    format!("{content}{pad}")
                } else {
                    format!("{pad}{content}")
                }
            } else {
                content.to_string()
            }
        }
        None => content.to_string(),
    }
}
