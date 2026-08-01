//! split part of `state` (mechanical move).

use super::*;

impl<'a> SimState<'a> {
    pub fn new(
        ir: &'a SimIr,
        out: Box<dyn Write + 'a>,
        sink: &'a dyn LogSink,
        timescale_unit: String,
        vcd_date: String,
        vcd_path_override: Option<String>,
    ) -> Self {
        let nets = ir
            .nets
            .iter()
            .map(|nv| {
                let alen = nv.array_len.max(1);
                let total = (nv.width as usize) * (alen as usize);
                let init = expand_init(&nv.init, nv.width, alen, total);
                NetSlot {
                    cur: init.clone(),
                    prev: init,
                    width: nv.width,
                    array_len: alen,
                    signed: nv.signed,
                    is_real: nv.kind == NetKind::Real,
                    vcd_id: None,
                    vcd_word_ids: Vec::new(),
                }
            })
            .collect();
        // Built with an EMPTY func table here; `simulate()` rebuilds with the real
        // `func_table` once it is installed (a from-`new()` SimState keeps today's
        // behavior since no `Expr::Call` is reachable without a frame func).
        let wt = crate::width::WidthTable::build(ir, &crate::FuncTable::new());
        let nnets = ir.nets.len();
        // GLITCH: a net is an edge target if it appears in a STATIC edge
        // sensitivity (`@(posedge clk …)` on an `always`) OR in a PROCEDURAL
        // edge wait (`@(posedge x)` inside a body → `Terminator::Wait` carrying
        // `WaitCause::Edge`). Both are compile-time-fixed net ids, so one scan
        // at construction yields the complete set; `slot_edge` is then only
        // maintained/consulted for these nets (every other net byte-identical).
        let mut is_edge_target = vec![false; nnets];
        let mark_edge = |net: u32, set: &mut Vec<bool>| {
            if (net as usize) < nnets {
                set[net as usize] = true;
            }
        };
        for p in &ir.processes {
            // Static edge sensitivity (`always @(posedge clk …)`).
            if p.sensitivity.kind == sim_ir::SensKind::Edge {
                for et in &p.sensitivity.edges {
                    mark_edge(et.net, &mut is_edge_target);
                }
            }
            // Procedural edge wait (`@(posedge x)` in this process's body — its
            // blocks are PROCESS-LOCAL in `body`, NOT in the global `ir.blocks`
            // arena, which holds only func/task bodies).
            for blk in &p.body {
                if let sim_ir::Terminator::Wait {
                    cond: sim_ir::WaitCause::Edge { net, .. },
                    ..
                } = &blk.term
                {
                    mark_edge(*net, &mut is_edge_target);
                }
            }
        }
        // Func/task bodies (global arena) may also carry an `@(edge …)` wait.
        for blk in &ir.blocks {
            if let sim_ir::Terminator::Wait {
                cond: sim_ir::WaitCause::Edge { net, .. },
                ..
            } = &blk.term
            {
                mark_edge(*net, &mut is_edge_target);
            }
        }
        SimState {
            ir,
            now: 0,
            nets,
            dirty: Vec::new(),
            dirty_flag: vec![false; nnets],
            ca_of_net: Vec::new(),
            ca_dirty: Vec::new(),
            ca_dirty_flag: Vec::new(),
            forced: vec![false; nnets],
            two_state: vec![false; nnets],
            wired_and_nets: std::collections::BTreeSet::new(),
            wired_or_nets: std::collections::BTreeSet::new(),
            slot_edge: vec![0u8; nnets],
            is_edge_target,
            blocking_writer: None,
            last_blocking_writer: vec![u32::MAX; nnets],
            active_forces: std::collections::BTreeMap::new(),
            force_net_to_forces: std::collections::BTreeMap::new(),
            force_always_reeval: std::collections::BTreeSet::new(),
            latent_assigns: std::collections::BTreeMap::new(),
            dyn_heap: std::cell::RefCell::new((0..nnets).map(|_| None).collect()),
            dyn_warned: std::cell::RefCell::new(std::collections::BTreeSet::new()),
            dyn_is_handle: ir
                .nets
                .iter()
                .map(|nv| {
                    matches!(
                        nv.kind,
                        NetKind::DynArray
                            | NetKind::Queue
                            | NetKind::Assoc
                            | NetKind::AssocStr
                            | NetKind::String
                    )
                })
                .collect(),
            dyn_str_elem: vec![false; nnets],
            native_ineligible: None,
            wt,
            vcd: None,
            vcd_path: None,
            dump_pending_path: None,
            vcd_path_override,
            fst_target: None,
            dumping: false,
            timescale_unit,
            vcd_date,
            net_names: Vec::new(),
            probed: Vec::new(),
            probe_prev: Vec::new(),
            trace_lines: Vec::new(),
            net_dims: crate::NetDimsTable::new(),
            net_decl_ranges: crate::NetDeclRangeTable::new(),
            file_directed_stmts: std::collections::BTreeSet::new(),
            dump_filter: None,
            dump_multi_warned: false,
            rng: RngCells::default(),
            plusargs: Vec::new(),
            final_procs: std::collections::BTreeSet::new(),
            clocking_inputs: Vec::new(),
            clocking_commit: std::collections::BTreeMap::new(),
            clocking_outputs: std::collections::BTreeMap::new(),
            ca_delays: std::collections::BTreeMap::new(),
            preponed_buf: std::collections::BTreeMap::new(),
            assert_fire: std::collections::BTreeSet::new(),
            assert_ctl: std::collections::BTreeMap::new(),
            assert_disabled: false,
            defer_marks: crate::DeferMarkTable::new(),
            defer_acts: crate::DeferActTable::new(),
            files: std::collections::BTreeMap::new(),
            mcd_files: std::collections::BTreeMap::new(),
            next_fd: 3,
            bad_fd_warned: std::collections::BTreeSet::new(),
            read_state: std::collections::BTreeMap::new(),
            readable_fds: std::collections::BTreeSet::new(),
            proc_multipliers: Vec::new(),
            init_procs: Vec::new(),
            proc_prec_mults: Vec::new(),
            severities: crate::SeverityTable::new(),
            timeformat_stmts: std::collections::BTreeSet::new(),
            stage_stmts: std::collections::BTreeSet::new(),
            stage_lines: Vec::new(),
            stage_idx: 0,
            stage_enabled: false,
            handle_copy_stmts: std::collections::BTreeMap::new(),
            queue_slice_stmts: std::collections::BTreeSet::new(),
            timeformat: None,
            global_prec_exp: -9,
            radixes: crate::RadixTable::new(),
            assign_ranks: crate::AssignRankTable::new(),
            queue_bounds: crate::QueueBoundTable::new(),
            proc_scopes: Vec::new(),
            cur_scope: "top".to_string(),
            frame_scope: std::cell::RefCell::new(Vec::new()),
            func_names: Vec::new(),
            threads: 1,
            backend: crate::Backend::Interpreter,
            vm_cache: (0..ir.processes.len())
                .map(|_| crate::backend::VmSlot::Unchecked)
                .collect(),
            cur_time_mult: 1,
            cur_prec_mult: 1,
            out,
            finished: false,
            had_error: Cell::new(false),
            had_fatal: false,
            max_class_objs: 1_000_000,
            sink,
            run_range_count: Cell::new(0),
            postponed: Postponed::default(),
            func_table: crate::FuncTable::new(),
            frame_local: vec![false; nnets],
            frame_route: vec![None; nnets],
            frame_stack: std::cell::RefCell::new(Vec::new()),
            frame_windows: std::cell::RefCell::new(Vec::new()),
            frame_window_free: std::cell::RefCell::new(Vec::new()),
            frame_window_rc: std::cell::RefCell::new(Vec::new()),
            static_store: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            call_depth: Cell::new(0),
            call_fatal: Cell::new(false),
            task_calls_proc: crate::TaskCallProc::new(),
            suspendable_tasks: std::collections::BTreeSet::new(),
            task_calls_func: crate::TaskCallFunc::new(),
            frame_slot_auto: vec![false; nnets],
            func_has_auto: Vec::new(),
            func_contains_shared_fork: Vec::new(),
            func_has_static: Vec::new(),
            func_has_dyn_local: Vec::new(),
            array_item_scratch: std::cell::RefCell::new(None),
            class_heap: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            class_obj_next: Cell::new(1),
            class_layouts: Vec::new(),
            class_rand: Vec::new(),
            class_constraints: Vec::new(),
            class_dist: Vec::new(),
            class_randc: Vec::new(),
            randomize_with: Vec::new(),
            randc_state: std::collections::HashMap::new(),
            // A fixed nonzero start so the first draw is well-defined and the
            // sequence is reproducible on every OS (dist_uniform substitutes 0
            // anyway, but pinning it keeps the contract explicit).
            randomize_seed: Cell::new(1),
            class_is_handle: vec![false; nnets],
            class_new_sites: std::collections::BTreeMap::new(),
            class_vtable: Vec::new(),
            class_calls: Vec::new(),
            class_null_warned: std::cell::RefCell::new(std::collections::BTreeSet::new()),
            fmt_cache: std::cell::RefCell::new(vec![None; ir.consts.len()]),
        }
    }

    /// FMT-CACHE: decode a format/string const, memoized by ConstId. The
    /// expensive bit-unpack + UTF-8 validation (`const_string`) runs once per
    /// const; later calls clone the cached text. Byte-identical to calling
    /// `const_string(self.ir, cid)` directly.
    pub(crate) fn fmt_const_string(&self, cid: u32) -> String {
        let mut cache = self.fmt_cache.borrow_mut();
        let slot = &mut cache[cid as usize];
        if slot.is_none() {
            *slot = Some(crate::builtins::const_string(self.ir, cid).into_boxed_str());
        }
        slot.as_deref().unwrap().to_string()
    }

    /// B1: derive the per-net frame-local routing tables from `func_table` (the
    /// `dyn_is_handle` build pattern). VALIDATES the hand-built/elaborated sidecar
    /// (index alignment + in-range slots) and latches a loud fatal rather than
    /// risking an out-of-bounds panic on a malformed table. No-op for an EMPTY
    /// table (every existing design byte-identical).
    pub fn build_func_routing(&mut self) {
        let nnets = self.ir.nets.len();
        self.frame_local = vec![false; nnets];
        self.frame_route = vec![None; nnets];
        // B4: per-net EFFECTIVE-automatic flag + per-func storage needs.
        self.frame_slot_auto = vec![false; nnets];
        self.func_has_auto = vec![false; self.func_table.len()];
        self.func_has_static = vec![false; self.func_table.len()];
        self.func_has_dyn_local = vec![false; self.func_table.len()];
        // Stage-2 fork-in-frame: threaded verbatim from the elaborate `FuncMeta` sidecar
        // (same as `has_hier_call`), so a staged velab→vrun carries it too.
        self.func_contains_shared_fork = self
            .func_table
            .iter()
            .map(|m| m.contains_shared_fork)
            .collect();
        if self.func_table.is_empty() {
            return;
        }
        if self.func_table.len() != self.ir.funcs.len() {
            self.fatal_run("frame-call func_table length != ir.funcs length");
            return;
        }
        for (fi, m) in self.func_table.iter().enumerate() {
            // The return-slot check applies only to FUNCTIONS (a task has no
            // func-named return var; `return_slot` is unused/0 for `is_task`).
            let is_task = self.ir.funcs.get(fi).map(|f| f.is_task).unwrap_or(false);
            let return_bad = !is_task && m.locals_len > 0 && m.return_slot >= m.locals_len;
            if return_bad || (m.base_net as usize) + (m.locals_len as usize) > nnets {
                self.fatal_run("frame-call FuncMeta slot range out of bounds");
                return;
            }
            for slot in 0..m.locals_len {
                let net = (m.base_net + slot) as usize;
                self.frame_local[net] = true;
                self.frame_route[net] = Some((fi as u32, slot));
                // B4: a slot is EFFECTIVE-automatic iff its override bit is set OR
                // the function/task default is automatic (bit 0 for slots ≥ 64).
                let overridden = slot < 64 && (m.auto_override >> slot) & 1 == 1;
                let auto = overridden || m.is_automatic;
                self.frame_slot_auto[net] = auto;
                if auto {
                    self.func_has_auto[fi] = true;
                } else {
                    self.func_has_static[fi] = true;
                }
                // V5: a frame-local DYN-ARRAY net (its heap lives in `dyn_heap[net]`)
                // needs the per-activation reentry guard + free-at-exit. Only DynArray
                // (NOT a frame-local `String`, which is `dyn_is_handle` but slab-stored).
                if self.ir.nets[net].kind == NetKind::DynArray {
                    self.func_has_dyn_local[fi] = true;
                }
            }
        }
    }

    /// NATIVE-TYPE GUARD: the per-net "the native expression path must not read this"
    /// flags, memoized on first use.
    ///
    /// `native_eval::ineligible_nets` supplies the IR-visible half (a positive allow-list
    /// of plain integral net kinds). This ORs in the half no net KIND can reveal: a class
    /// handle, a dyn/queue handle, a `real r[]` element handle (which re-flags `is_real`
    /// on an otherwise ordinary slot) and a `string s[]` element handle are all installed
    /// as out-of-band sidecars after construction. Hence lazy: called at first compile,
    /// which is always after every sidecar has landed.
    pub(crate) fn native_ineligible(&mut self) -> std::rc::Rc<Vec<bool>> {
        if let Some(v) = &self.native_ineligible {
            return std::rc::Rc::clone(v);
        }
        let mut v = crate::native_eval::ineligible_nets(self.ir);
        for (i, slot) in v.iter_mut().enumerate() {
            if self.nets.get(i).is_none_or(|n| n.is_real)
                || self.class_is_handle.get(i).copied().unwrap_or(false)
                || self.dyn_is_handle.get(i).copied().unwrap_or(false)
                || self.dyn_str_elem.get(i).copied().unwrap_or(false)
            {
                *slot = true;
            }
        }
        let rc = std::rc::Rc::new(v);
        self.native_ineligible = Some(std::rc::Rc::clone(&rc));
        rc
    }

    /// Stage-C VM dispatch (P0a): return the cached `CompiledBody` for template `tmpl`
    /// if it is codegen-able, compiling + caching on first sight; `None` ⇒ interpret.
    /// The decision is made ONCE per template (the per-fire `is_codegen_able` scan is
    /// removed). The returned `Rc` is an OWNED handle (cloned out of the cache) so the
    /// caller can pass `&mut self` as the `Kernel` to `vm_exec` without aliasing the
    /// cache — the §2.3 borrow protocol.
    pub(crate) fn vm_compiled(&mut self, tmpl: usize) -> Option<Rc<crate::backend::CompiledBody>> {
        use crate::backend::VmSlot;
        match &self.vm_cache[tmpl] {
            VmSlot::Compiled(rc) => return Some(Rc::clone(rc)),
            VmSlot::NotCodegenable => return None,
            VmSlot::Unchecked => {}
        }
        // `self.ir` is a `&SimIr` field — copy the reference out so the immutable read
        // of the IR does not borrow `self` across the `self.vm_cache` write below.
        let ir: &SimIr = self.ir;
        if !crate::backend::is_codegen_able(
            &ir.stmts,
            &ir.exprs,
            &ir.processes[tmpl].body,
            &self.class_new_sites,
        ) {
            self.vm_cache[tmpl] = VmSlot::NotCodegenable;
            return None;
        }
        let nonint = self.native_ineligible();
        let compiled = Rc::new(crate::backend::compile_body(
            &ir.stmts,
            &ir.processes[tmpl].body,
            Some((ir, &self.wt, &nonint)),
        ));
        self.vm_cache[tmpl] = VmSlot::Compiled(Rc::clone(&compiled));
        Some(compiled)
    }

    /// Emit a rate-limited `E-RUN-RANGE` (VITA-E4002) runtime diagnostic for an
    /// out-of-range array word / select. The OOR is RECOVERED (read X / drop write),
    /// so the run still finishes; this only surfaces it. Capped at `CAP` per run with
    /// a final "further suppressed" note so a loop of OOR accesses can't spam.
    pub fn warn_run_range(&self, what: &str) {
        const CAP: u32 = 8;
        let n = self.run_range_count.get();
        let msg = match n.cmp(&CAP) {
            std::cmp::Ordering::Less => {
                Some(format!("{what} (out of range; read X / write ignored)"))
            }
            std::cmp::Ordering::Equal => {
                Some("further out-of-range diagnostics suppressed".to_string())
            }
            std::cmp::Ordering::Greater => None,
        };
        if let Some(message) = msg {
            self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Error,
                code: MsgCode::RunRange,
                message,
                location: None,
                context: Vec::new(),
                sim_time: Some(diag::TimeStamp { ticks: self.now }),
            }));
        }
        self.run_range_count.set(n.saturating_add(1));
    }

    /// Emit a loud `RunFatal` (F-RUN-FATAL / exit class Fatal — same code as user
    /// `$fatal`) and latch `had_fatal`/`finished`. Used for malformed engine input
    /// that must not silently mis-route (e.g. a corrupt frame `func_table`), as a
    /// hardened alternative to a raw out-of-bounds panic.
    pub fn fatal_run(&mut self, what: &str) {
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Fatal,
            code: MsgCode::RunFatal,
            message: what.to_string(),
            location: None,
            context: Vec::new(),
            sim_time: Some(diag::TimeStamp { ticks: self.now }),
        }));
        self.had_fatal = true;
        self.finished = true;
    }

    /// CLASS-HEAP-CAP: the class-object budget (`max_class_objs`) was exceeded.
    /// Emit a single loud `F-RUN-CLASS-LIMIT` and latch `had_fatal`/`finished`
    /// for a graceful `$finish` (exit class Fatal) instead of an OOM — the class
    /// heap is never garbage-collected, so an unbounded `new()` in a loop would
    /// grow without limit. Single-shot (guarded by `had_fatal`).
    pub fn fatal_class_limit(&mut self) {
        if !self.had_fatal {
            self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Fatal,
                code: MsgCode::RunClassLimit,
                message: format!(
                    "class object budget ({}) exceeded — likely an unbounded `new()` \
                     (the class heap is not garbage-collected); raise \
                     SimOpts::max_class_objs if intended",
                    self.max_class_objs
                ),
                location: None,
                context: Vec::new(),
                sim_time: Some(diag::TimeStamp { ticks: self.now }),
            }));
        }
        self.had_fatal = true;
        self.finished = true;
    }

    // ── reads ────────────────────────────────────────────────────────────

    // ── writes (single choke point) ──────────────────────────────────────

    /// Write `value` (any width) into the LHS chunks of `lhs`, MSB-first source
    /// consumption (Verilog concat-LHS). Returns true if ANY bit changed.
    ///
    /// `offsets[i]` is the already-EVALUATED bit offset of `lhs.chunks[i]` — the
    /// runtime value of a dynamic index (`a[i]`) or the const for `a[3]`; ignored
    /// for a whole-net/`None` chunk. The caller resolves these at the correct
    /// sampling moment (statement time for blocking, SAMPLE time for NBA so
    /// `a[i] <= x; i = i+1;` uses the OLD `i`, settle time for cont-assign),
    /// because this `&mut self` path has no read-only `EvalCtx`.
    pub fn write_lvalue(
        &mut self,
        lhs: &Lvalue,
        value: Value,
        offsets: &crate::exec::Offsets,
    ) -> bool {
        // ── real↔int assignment coercion (IEEE 1364 §6.2) ──
        // Only a WHOLE-NET lvalue (single Bit chunk, no offset/width) can be a
        // real destination: a real is dimensionless and never bit/part-selected
        // (§6.2 makes r[i]/r[hi:lo] illegal at elaborate). Detect the whole-net
        // case and consult NetSlot.is_real.
        let dest_is_real = lhs.chunks.len() == 1
            && matches!(lhs.chunks[0].kind, SelKind::Bit)
            && lhs.chunks[0].offset.is_none()
            && lhs.chunks[0].width.is_none()
            && self.nets[lhs.chunks[0].net as usize].is_real;

        let value = match (dest_is_real, value.is_real) {
            // real net ← real value: store verbatim (already 64 IEEE bits).
            (true, true) => value,
            // real net ← integer value (int→real CONVERT): exact for ≤53-bit.
            (true, false) => Value::from_f64(value.to_f64().unwrap_or(0.0)),
            // integer net ← real value (real→int ASSIGNMENT: ROUND half-away).
            // A real RHS only legally targets a whole scalar int net (concat-LHS
            // of a real is illegal §6.2). Round to that net's width; for the rare
            // multi-chunk case round to the total LHS width.
            (false, true) => {
                let w = if lhs.chunks.len() == 1 {
                    self.nets[lhs.chunks[0].net as usize].width
                } else {
                    lhs.chunks.iter().map(|c| self.chunk_width(c)).sum()
                };
                let signed = lhs.chunks.len() == 1 && self.nets[lhs.chunks[0].net as usize].signed;
                crate::value::real_to_int_round(value.to_f64().unwrap_or(0.0), w.max(1), signed)
            }
            // integer net ← integer value: unchanged legacy path.
            (false, false) => value,
        };

        // ── Round-14 V3/V4: frame-local write lane ──
        // A write to a frame-local net from `run_process` (executing a suspendable task
        // body) MUST route to the frame slot, symmetric with `read_net`'s frame-local read
        // branch — the flat-store path below would silently write the module net space, so
        // the output-formal / body-local slot would never be updated (o=0 silent-wrong).
        // Frame locals are single-chunk scalars; `frame_write_lvalue` handles both a whole
        // and a bit/part-select write (re-evaluating the offset in this frame's context).
        // Empty `frame_local` (no frame tasks) ⇒ never taken (byte-identical).
        // V5: a frame-local DYN-ARRAY net (`int loc[]`) is EXCLUDED — its `loc[i]=v`
        // element write must land in the heap (`write_chunk` → `dyn_write`), not the
        // unused scalar window slot; the heap holds the current activation's array. A
        // frame-local STRING is `dyn_is_handle` too but is SLAB-stored (§4.5.167), so it
        // still takes the frame-slot write (only a non-string dyn handle is excluded).
        if lhs.chunks.len() == 1
            && self
                .frame_local
                .get(lhs.chunks[0].net as usize)
                .copied()
                .unwrap_or(false)
            && (!self
                .dyn_is_handle
                .get(lhs.chunks[0].net as usize)
                .copied()
                .unwrap_or(false)
                || self.ir.nets[lhs.chunks[0].net as usize].kind == NetKind::String)
        {
            self.frame_write_lvalue(lhs, value);
            return false; // a frame slot is not a flat-store net → no dirty channel
        }

        // ── v5 ⑤: assoc-element lane (single chunk, i64 key) ──
        // The key cannot ride the u32 pairs; `resolve_lvalue_offsets` claims
        // the shape and the funnel splits here, AFTER the real coercion (an
        // assoc element gets the same real→int rounding as a dyn element).
        if let crate::exec::Offsets::AssocKey(key) = offsets {
            if let Some(c) = lhs.chunks.first() {
                self.assoc_write(c.net, *key, &value);
            }
            return false; // dyn content never enters the dirty channel
        }
        // v6: the string-keyed twin lane.
        if let crate::exec::Offsets::AssocStrKey(key) = offsets {
            if let Some(c) = lhs.chunks.first() {
                self.assoc_str_write(c.net, key, &value);
            }
            return false;
        }
        let offsets = offsets.as_slice();
        debug_assert_eq!(
            offsets.len(),
            lhs.chunks.len(),
            "one (offset,word) per chunk"
        );

        // v7 P2-C: a STRING destination takes the WHOLE source value — its
        // net-table width is 0 (dynamic), so the total-width resize below
        // would chop the value to 1 bit; §6.16 conversion is dyn_write's
        // byte-strip, never a bit resize. N3 Phase 2: a `string s[]` ELEMENT
        // (`s[i] = …`, net kind DynArray but `dyn_str_elem`) is the same — pass
        // the whole value so `dyn_write` stores the byte-string, not 1 bit.
        if let [c] = lhs.chunks.as_slice() {
            if self.ir.nets[c.net as usize].kind == NetKind::String
                || self
                    .dyn_str_elem
                    .get(c.net as usize)
                    .copied()
                    .unwrap_or(false)
            {
                let (raw_off, raw_word) = offsets.first().copied().unwrap_or((0, 0));
                return self.write_chunk(c, raw_off, raw_word, &value);
            }
        }
        // Total destination bit width = sum of chunk widths.
        let total: u32 = lhs.chunks.iter().map(|c| self.chunk_width(c)).sum();
        let src = value.resize(total.max(1));

        // Single-chunk LHS — the dominant case (a whole net, or a single bit-/part-
        // select). With one chunk, `take_lo == 0` and `cw == total`, so the sliced
        // "piece" IS `src` exactly; skip the per-bit slice entirely and hand `src`
        // straight to `write_chunk` (which word-writes the whole-net/aligned case). The
        // per-bit slice loop below is only for a multi-chunk concat LHS (`{a,b} = x`).
        if lhs.chunks.len() == 1 {
            let (raw_off, raw_word) = offsets.first().copied().unwrap_or((0, 0));
            return self.write_chunk(&lhs.chunks[0], raw_off, raw_word, &src);
        }

        let mut changed = false;
        let mut src_hi = total; // next source bit (exclusive top)
        for (idx, chunk) in lhs.chunks.iter().enumerate() {
            let cw = self.chunk_width(chunk);
            let take_lo = src_hi.saturating_sub(cw);
            // slice src[take_lo .. src_hi) → low-aligned chunk value
            let mut piece = Value::zeros(cw.max(1), false);
            piece.width = cw;
            for i in 0..cw {
                let (v, u) = src.get_vu(take_lo + i);
                piece.set_vu(i, v, u);
            }
            src_hi = take_lo;
            let (raw_off, raw_word) = offsets.get(idx).copied().unwrap_or((0, 0));
            changed |= self.write_chunk(chunk, raw_off, raw_word, &piece);
        }
        changed
    }

    /// Width (in bits) a single lvalue chunk writes.
    pub(crate) fn chunk_width(&self, c: &sim_ir::LvalChunk) -> u32 {
        match c.kind {
            // whole-net write: offset/width None.
            SelKind::Bit => {
                if c.offset.is_none() && c.width.is_none() {
                    self.nets[c.net as usize].width
                } else {
                    1
                }
            }
            SelKind::PartConst | SelKind::PartIdxUp | SelKind::PartIdxDown => {
                // `c.width` is an ExprId (frozen IR: a const-expr edge like
                // `Add(Sub(msb,lsb),1)`), NOT a literal — fold it to its value.
                c.width
                    .and_then(|eid| crate::width::const_u32_of_expr(self.ir, eid))
                    .unwrap_or_else(|| self.nets[c.net as usize].width)
            }
        }
    }

    /// Total destination bit-width of an lvalue (Σ chunk widths). Used to seed
    /// the RHS context width. Does NOT compute a sign — lhs sign never
    /// propagates (IEEE 1364-2005 assignment rule, §5.5).
    pub(crate) fn lvalue_width(&self, lhs: &Lvalue) -> u32 {
        lhs.chunks
            .iter()
            .map(|c| self.chunk_width(c))
            .sum::<u32>()
            .max(1)
    }

    /// Write a low-aligned `piece` into the destination chunk. `raw_off` is the
    /// already-EVALUATED `c.offset` (the runtime index for `a[i]`, the const for
    /// `a[3]`; ignored for a whole-net chunk). Returns changed.
    pub(crate) fn write_chunk(
        &mut self,
        c: &sim_ir::LvalChunk,
        raw_off: u32,
        raw_word: u32,
        piece: &Value,
    ) -> bool {
        let net = c.net as usize;
        // v5 (C)-3b: dyn-handle element write → heap (never the flat store).
        if self.dyn_is_handle[net] {
            return self.dyn_write(c, raw_off, raw_word, piece);
        }
        // N7: a class-handle field-select write (`obj.f = v`, `word = field-id`)
        // goes to the heap; a bare word-less write (the handle id itself —
        // ref-copy / `h = null` / `h = new` placeholder) falls through to the
        // flat store below.
        if self.class_is_handle[net] && c.word.is_some() {
            return self.class_field_write(c, raw_word, piece);
        }
        // A forced net ignores every normal driver until release (§9.3.2).
        if self.forced[net] {
            return false;
        }
        // SVPART: a 2-state variable can never hold X/Z (IEEE §6.11.3) — coerce any
        // unknown bit of the incoming value to 0 before it lands. Only fires for the
        // (new) 2-state nets, so every prior design is byte-identical.
        let coerced;
        let piece = if self.two_state[net] && piece.unk.iter().any(|&u| u != 0) {
            let mut v = piece.clone();
            for k in 0..v.unk.len() {
                v.val[k] &= !v.unk[k]; // X (val0/unk1) & Z (val1/unk1) → 0
                v.unk[k] = 0;
            }
            coerced = v;
            &coerced
        } else {
            piece
        };
        // `c.word` is an ExprId; `raw_word` is the caller-evaluated array index
        // (the runtime `k` of `mem[k] = …`). None ⇒ index 0. An out-of-range word
        // write is IGNORED (spec E-RUN-RANGE) — clamping to the last element would
        // silently corrupt a valid neighbor.
        let word = if c.word.is_some() {
            if raw_word >= self.nets[net].array_len {
                self.warn_run_range("array word index");
                return false;
            }
            raw_word
        } else {
            0
        };
        let net_w = self.nets[net].width;
        let base = word * net_w; // bit offset of this array element

        // `c.width` is a const-expr edge (part-select bounds are constant); fold
        // it. `c.offset` was evaluated by the caller (dynamic-index capable) and
        // arrives as `raw_off`, symmetric with the runtime offset eval that
        // `eval_select` already does on the READ side.
        let ir = self.ir;
        let fold = |eid: u32| crate::width::const_u32_of_expr(ir, eid);
        // P0-IPU: the low (LSB) net-bit position is SIGNED — an underflowing indexed
        // part-select (`v[-2+:4]`, `v[1-:3]`) extends below bit 0, and only the
        // in-range bits are written (the low OOB bits are dropped, NOT shifted up or
        // wrapped). Mirror the READ side (`eval_select`): keep `lsb` an `i64` and let
        // the bit loop clamp at both ends. `raw_off` arrives as the offset's 32-bit
        // 2's-complement; sign-extend it (bit positions never exceed i32 range).
        let off_i = raw_off as i32 as i64;
        let (lsb, width) = match c.kind {
            SelKind::Bit => {
                if c.offset.is_none() && c.width.is_none() {
                    (0i64, net_w) // whole net
                } else {
                    (off_i, 1)
                }
            }
            SelKind::PartConst | SelKind::PartIdxUp => {
                (off_i, c.width.and_then(fold).unwrap_or(net_w))
            }
            SelKind::PartIdxDown => {
                let w = c.width.and_then(fold).unwrap_or(net_w);
                (off_i - (w as i64) + 1, w)
            }
        };

        // GLITCH: capture scalar bit0 BEFORE the mutation so a same-slot re-write
        // (A→B→A) records its real `B→A` transition (the endpoint compare would
        // see only A==A). Gated on `is_edge_target`, so non-clock nets pay nothing.
        let track_edge = self.is_edge_target[net];
        let old_b0 = if track_edge {
            scalar_bit0(&self.nets[net].cur)
        } else {
            sim_ir::FourState::Zero
        };

        let slot = &mut self.nets[net];

        // WORD-PARALLEL fast path: a whole-element write to a 64-aligned destination
        // (every scalar whole-net assign, plus array elements whose width is a multiple
        // of 64). Copy `piece`'s words into the store with word-granular change detection
        // + a top-word mask, replacing the per-bit `set_bit` loop. Guard:
        // `array_len <= 1 || net_w % 64 == 0` guarantees the element occupies WHOLE store
        // words, so masking the top word cannot clobber a neighbouring element packed in
        // the same word. Everything else (part/bit-select, unaligned base, OOR) falls
        // through to the proven bit-serial path below — byte-identical by construction.
        if lsb == 0 && width == net_w && base % 64 == 0 && (slot.array_len <= 1 || net_w % 64 == 0)
        {
            let wbase = (base / 64) as usize;
            let nw = nwords(net_w).max(1);
            let m = top_mask(net_w);
            let mut changed = false;
            for k in 0..nw {
                let mut nv = piece.val.get(k).copied().unwrap_or(0);
                let mut nu = piece.unk.get(k).copied().unwrap_or(0);
                if k == nw - 1 {
                    nv &= m;
                    nu &= m;
                }
                let idx = wbase + k;
                if slot.cur.val.len() <= idx {
                    slot.cur.val.resize(idx + 1, 0);
                    slot.cur.unk.resize(idx + 1, 0);
                }
                if slot.cur.val[idx] != nv || slot.cur.unk[idx] != nu {
                    slot.cur.val[idx] = nv;
                    slot.cur.unk[idx] = nu;
                    changed = true;
                }
            }
            if changed {
                self.note_change(net as u32, word);
                if track_edge {
                    self.accumulate_edge(net, old_b0);
                }
            }
            return changed;
        }

        let mut changed = false;
        for i in 0..width {
            // P0-IPU: the destination net-bit is `lsb + i` in a SIGNED domain — a bit
            // below 0 (underflow) OR at/above `net_w` (overflow) is dropped cleanly
            // (matching the READ side and iverilog), so only in-range bits are written.
            let dst = lsb + i as i64;
            if dst < 0 || dst as u32 >= net_w {
                continue;
            }
            let (v, u) = piece.get_vu(i);
            if set_bit(&mut slot.cur, base + dst as u32, v, u) {
                changed = true;
            }
        }
        if changed {
            self.note_change(net as u32, word);
            if track_edge {
                self.accumulate_edge(net, old_b0);
            }
        }
        changed
    }

    /// N4 clocking: commit a whole-net SCALAR value into a holding net (the
    /// preponed sample → `cb.sig`), blocking + same-slot, marking it changed for
    /// propagation. Mirrors `write_chunk`'s word-parallel whole-net fast path
    /// (holding nets are scalar `Reg`s: `array_len == 1`, never forced/2-state).
    pub fn commit_clocking_sample(&mut self, net: u32, v: &Value) -> bool {
        let i = net as usize;
        let net_w = self.nets[i].width;
        let nw = nwords(net_w).max(1);
        let m = top_mask(net_w);
        // GLITCH: a holding net can itself be an edge target (`@(posedge cb.sig)`),
        // so this write must feed the same intra-slot edge accumulator as the
        // normal funnel — `note_change` alone RESETS `slot_edge`, so without the
        // matching `accumulate_edge` a posedge on the committed sample would be
        // silently lost (the pre-fix endpoint compare caught it via `cur != prev`).
        let track_edge = self.is_edge_target[i];
        let old_b0 = if track_edge {
            scalar_bit0(&self.nets[i].cur)
        } else {
            sim_ir::FourState::Zero
        };
        let slot = &mut self.nets[i];
        let mut changed = false;
        for k in 0..nw {
            let mut nv = v.val.get(k).copied().unwrap_or(0);
            let mut nu = v.unk.get(k).copied().unwrap_or(0);
            if k == nw - 1 {
                nv &= m;
                nu &= m;
            }
            if slot.cur.val.len() <= k {
                slot.cur.val.resize(k + 1, 0);
                slot.cur.unk.resize(k + 1, 0);
            }
            if slot.cur.val[k] != nv || slot.cur.unk[k] != nu {
                slot.cur.val[k] = nv;
                slot.cur.unk[k] = nu;
                changed = true;
            }
        }
        if changed {
            self.note_change(net, 0);
            if track_edge {
                self.accumulate_edge(i, old_b0);
            }
        }
        changed
    }

    /// N4 clocking: take the PREPONED snapshot of every clocking-input source net
    /// at the start of a time slot (called right after time advances, BEFORE any
    /// slot activity — so each source holds the value it had ENTERING the slot, the
    /// true preponed value). EMPTY `clocking_inputs` ⇒ no-op ⇒ byte-identical.
    pub(crate) fn snapshot_preponed(&mut self) {
        if self.clocking_inputs.is_empty() {
            return;
        }
        for idx in 0..self.clocking_inputs.len() {
            let src = self.clocking_inputs[idx];
            let v = self.read_net(src, None);
            self.preponed_buf.insert(src, v);
        }
    }

    /// N4 clocking: a marked commit-handler proc fired on its clocking edge —
    /// commit `preponed_buf[source] → holding` for each of its inputs (blocking,
    /// same-slot), then drive `source = holding` for each of its outputs.
    /// Returns `true` iff this proc was a clocking handler (so the scheduler
    /// skips the no-op body dispatch).
    pub(crate) fn commit_clocking(&mut self, proc: u32) -> bool {
        let is_input_handler = self.clocking_commit.contains_key(&proc);
        let is_output_handler = self.clocking_outputs.contains_key(&proc);
        if !is_input_handler && !is_output_handler {
            return false;
        }
        // INPUT phase: preponed_buf[source] → holding_net (existing).
        if let Some(pairs) = self.clocking_commit.get(&proc).cloned() {
            for (hold, src) in pairs {
                if let Some(v) = self.preponed_buf.get(&src).cloned() {
                    self.commit_clocking_sample(hold, &v);
                }
            }
        }
        // OUTPUT phase: current_value(holding_net) → source_net (new).
        // Simplified synchronous model: drive in Active region at edge detection,
        // after INPUT commits. Hand-IEEE (no Reactive region; covers typical TB use).
        // NOTE: tuple is (source_net, holding_net) — REVERSED from clocking_commit.
        if let Some(out_pairs) = self.clocking_outputs.get(&proc).cloned() {
            for (src, hold) in out_pairs {
                let v = self.read_net(hold, None);
                self.commit_clocking_sample(src, &v);
            }
        }
        true
    }
}
