//! split part of `state` (mechanical move).

use super::*;

impl<'a> SimState<'a> {
    /// Build a read-only `EvalCtx` over `&self` holding ZERO frame borrow
    /// (mirrors the scheduler's `eval`/`truthy` ctor). An `EvalCtx` can live
    /// across a nested `Expr::Call` because it touches the frame arena only
    /// transitively through `read_net`, which clones-and-releases. `$time`/
    /// `$realtime` inside a frame body run on the ambient `cur_time_mult` (a
    /// frame func is rejected at elaborate if it reads them — B1 cut).
    pub(crate) fn mk_eval_ctx(&self) -> crate::eval::EvalCtx<'_, SimState<'a>> {
        crate::eval::EvalCtx {
            ir: self.ir,
            nets: self,
            now: self.now,
            wt: &self.wt,
            time_mult: self.cur_time_mult,
            rng: &self.rng,
            plusargs: &self.plusargs,
        }
    }

    /// `mk_eval_ctx` against an ALTERNATE net store — the tier-3 seam, with
    /// exactly one field changed. Every other field still comes from `self`, so
    /// `$time`, the width table, the RNG and the plusargs cannot differ between
    /// the two backends by construction.
    pub(crate) fn mk_eval_ctx_with<'n, N: crate::eval::NetReader + ?Sized>(
        &'n self,
        nets: &'n N,
    ) -> crate::eval::EvalCtx<'n, N> {
        crate::eval::EvalCtx {
            ir: self.ir,
            nets,
            now: self.now,
            wt: &self.wt,
            time_mult: self.cur_time_mult,
            rng: &self.rng,
            plusargs: &self.plusargs,
        }
    }

    /// Evaluate an expression to a self-width Value against the CURRENT state
    /// (module nets, or the active frame window if one is installed). Identical to
    /// `Scheduler::eval` — both build an `EvalCtx` over this `SimState` — so the
    /// `&SimState` formatter (`format_args_str`) renders byte-for-byte the same from
    /// a process body (executor) and from inside a frame body (N1: `$sformatf` args
    /// that read frame-local formals resolve through `read_net`'s frame routing).
    pub(crate) fn eval_expr(&self, eid: u32) -> Value {
        self.mk_eval_ctx().eval(eid)
    }

    /// `eval_expr` against an ALTERNATE net store, sharing every cold field
    /// (`ir`, `now`, widths, time multiplier, RNG, plusargs) with this state.
    ///
    /// The tier-3 seam. `builtins` renders through `&SimState`, so a tier-3
    /// kernel — whose nets live in a `NetArena`, not in `SimState.nets` — cannot
    /// call `$display` without either duplicating the format engine or reading a
    /// stale store. This is the third option: the formatter keeps its one
    /// implementation and takes the reader as a parameter.
    ///
    /// The engine's own calls pass `self`, which is byte-identical to
    /// `eval_expr` by construction — same fields, same order, same reader.
    ///
    /// GENERIC, not `&dyn NetReader`, and the difference is not stylistic. With
    /// a trait object the engine's own `$display` path would have moved from a
    /// monomorphised `EvalCtx<SimState>` to a virtual one — and `EvalCtx.nets` is
    /// called several times per NET ACCESS (`is_assoc_str`, `is_assoc`,
    /// `read_net` on the leaf arm alone), not once per `eval_expr`. That would
    /// have made every tier-1 and tier-2 format argument pay for tier-3's seam.
    /// Generic, both readers monomorphise and the engine's path is the code it
    /// was before.
    pub(crate) fn eval_expr_with<N: crate::eval::NetReader + ?Sized>(
        &self,
        nets: &N,
        eid: u32,
    ) -> Value {
        crate::eval::EvalCtx {
            ir: self.ir,
            nets,
            now: self.now,
            wt: &self.wt,
            time_mult: self.cur_time_mult,
            rng: &self.rng,
            plusargs: &self.plusargs,
        }
        .eval(eid)
    }

    /// N1: evaluate a frame-body `BlockingAssign` RHS. `$sformatf(fmt, …)` is rendered
    /// through the SHARED formatter (`format_args_str`) — the generic `eval_sysfunc`
    /// arm cannot see the format string (it would dump the raw args), so a
    /// `s = $sformatf(...)` in a subroutine body needs this intercept. The frame
    /// window is active, so any format arg that reads a frame-local formal resolves
    /// through `read_net`. Any other RHS evaluates normally in the assignment-width
    /// context.
    ///
    /// ⚠️ **This paragraph used to say the intercept was GATED on a `NetKind::String`
    /// lhs**, on the grounds that a `$sformatf` into a NUMERIC target is an illegal
    /// implicit cast iverilog rejects. The premise is true and the conclusion was
    /// not: vita ACCEPTS that assignment at module scope and renders it correctly, so
    /// the gate did not decline the shape — it gave the same statement two different
    /// answers depending on which body it sat in, the numeric one silently holding
    /// its own arguments' bytes instead of the rendered text. The gate is gone; the conversion to the destination's width lives at
    /// the write funnel (`coerce_str_to_packed`), where the module path has always
    /// had it.
    pub(crate) fn frame_rhs_value(&self, lhs: &Lvalue, rhs: u32) -> Value {
        // R19-X2 SILENT-WRONG (measured against iverilog 13, exit 0, no diagnostic):
        // the file-read family's real work is a statement-level effect that only the
        // PROCESS executor performs (`StmtEffect::Fgets` / `Fread` / `Scanf` / …). Here
        // the same `SysFunc` goes through the pure `eval` path, whose arm for these ids
        // returns X and touches nothing — so `rc = $fgets(line, fd);` inside a
        // `function automatic` yielded `rc=0` and an EMPTY string where iverilog reads
        // the line. That is exactly the `.rsp` / CAVP walker shape round-19 §3.1 has
        // just made reachable, so it is latched as a FATAL rather than left silent.
        if self.rhs_is_sysread(rhs) {
            self.fatal_frame_sysread(rhs);
            return Value::xs(self.lvalue_width(lhs).max(1), true);
        }
        // `$sformatf` renders here, for ANY destination — the intercept is keyed
        // on the RHS, not on the lhs's net kind.
        //
        // ⚠️ It used to be gated on `lhs.kind == String`, and that gate was a
        // silent-wrong: `reg [63:0] p; p = $sformatf("v=%0d", x);` inside a
        // function body fell through to the plain `eval` path, whose arm for
        // `Sformatf` drops the FORMAT STRING and concatenates the remaining args as
        // packed chars — so the destination came back holding the raw argument
        // bytes (printable and plausible: `$sformatf("v=%0d", 32'h41424344)` gave
        // `ABCD`), not blank, at exit 0 — while the identical statement at module
        // scope renders correctly
        // (`compute_effect` → `k_sformatf`, which never asked about the lhs). Two
        // paths for one statement, differing only in which body it sits in.
        //
        // What comes back is an `is_str` `Value` at the TEXT's width; the
        // destination's width is applied at the write funnel
        // (`coerce_str_to_packed`), which is where the module path applies it too.
        //
        // ⚠️ An earlier version of this comment said the packing was done here by
        // `resize_keep_sign` "exactly as `apply_effect` packs `k_sformatf`'s", and
        // BOTH halves were false: the module funnel resizes before storing, and
        // `resize_keep_sign` returns an `is_str` value UNCHANGED. Rendering without
        // converting left a `reg [63:0]` holding a 24-bit value — right text, wrong
        // width — which is what the round-2 review of this very fix measured.
        if let Some(sim_ir::Expr::SysFunc {
            which: sim_ir::SysFuncId::Sformatf,
            args,
        }) = self.ir.exprs.get(rhs as usize)
        {
            let text = crate::builtins::format_args_str(
                self,
                args.first().copied(),
                args.get(1..).unwrap_or(&[]),
                None,
            );
            return Value::from_str_bytes(text.as_bytes());
        }
        let lw = self.lvalue_width(lhs);
        let sw = self.wt.get(rhs);
        self.mk_eval_ctx()
            .eval_ctx(rhs, lw.max(sw.width), sw.signed)
    }

    /// Read frame slot `slot` of function `func`: the top AUTOMATIC window, or
    /// the shared STATIC slab. Borrow → clone → drop-at-return (never held
    /// across a nested eval — §borrowDiscipline rule 1).
    pub(crate) fn frame_slot_read(&self, func: u32, automatic: bool, slot: u32) -> Value {
        if automatic {
            // Match the top WindowSlot; a `Shared(h)` dereferences the arena. Each borrow
            // is scoped to the single index-clone (never held across an eval —
            // §borrowDiscipline rule 1). `Owned` is byte-identical to the pre-arena path.
            match self
                .frame_stack
                .borrow()
                .last()
                .expect("frame read: no active call window")
            {
                crate::state::WindowSlot::Owned(w) => w[slot as usize].clone(),
                crate::state::WindowSlot::Shared(h) => {
                    // Stage-3 refcount airtightness: an arena READ must never touch a slot with
                    // no live reference (a use-after-free would otherwise read stale data).
                    debug_assert!(
                        self.frame_window_rc.borrow()[*h as usize] > 0,
                        "frame_slot_read: arena access on a freed shared window (h={h})"
                    );
                    self.frame_windows.borrow()[*h as usize]
                        .as_ref()
                        .expect("frame read: live shared window")[slot as usize]
                        .clone()
                }
            }
        } else {
            self.static_store
                .borrow()
                .get(&func)
                .expect("static read: no storage slab")[slot as usize]
                .clone()
        }
    }

    /// One packed-ASCII byte of a frame slot, WITHOUT cloning the slot's `Value`.
    ///
    /// `frame_slot_read` clones — correct, and ruinous for `.getc()`. A frame `string`
    /// slot holds the whole string as an 8-bits-per-character `Value`, so cloning it per
    /// character made a per-character loop O(len^2): the reporter's `hex2bytes` over
    /// 32,000 chars cost 6.90 s. This borrows the slot for exactly the one shift-and-mask.
    ///
    /// Mirrors `frame_slot_read`'s routing arm for arm (automatic window / shared arena /
    /// static slab) so the two cannot disagree about WHICH slot they read.
    pub(crate) fn frame_slot_byte_at(
        &self,
        func: u32,
        automatic: bool,
        slot: u32,
        i: usize,
    ) -> Option<u8> {
        let pick = |v: &Value| crate::eval::packed_byte_at(v, i);
        if automatic {
            match self
                .frame_stack
                .borrow()
                .last()
                .expect("frame read: no active call window")
            {
                crate::state::WindowSlot::Owned(w) => pick(&w[slot as usize]),
                crate::state::WindowSlot::Shared(h) => {
                    debug_assert!(
                        self.frame_window_rc.borrow()[*h as usize] > 0,
                        "frame_slot_byte_at: arena access on a freed shared window (h={h})"
                    );
                    pick(
                        &self.frame_windows.borrow()[*h as usize]
                            .as_ref()
                            .expect("frame read: live shared window")[slot as usize],
                    )
                }
            }
        } else {
            pick(
                &self
                    .static_store
                    .borrow()
                    .get(&func)
                    .expect("static read: no storage slab")[slot as usize],
            )
        }
    }

    /// Store `v` into frame slot `slot` (arg binding). `v` is an already-owned
    /// Value (computed before any borrow); the `borrow_mut` is scoped to the
    /// single index-store — NO eval inside (§borrowDiscipline rule 3).
    pub(crate) fn frame_slot_write(&self, func: u32, automatic: bool, slot: u32, v: Value) {
        // A 2-state frame slot (byte/int/shortint/longint/bit) can never hold X/Z
        // (IEEE §6.11.3). Frame slot writes bypass `write_chunk`, so the coercion
        // it applies (val &= !unk; unk = 0) must be repeated here — for the arg
        // copy-IN, body-local assignments, and the return slot alike.
        let v = self.coerce_two_state_frame(func, slot, v);
        if automatic {
            // `v` is already owned (computed before any borrow). Route by the top WindowSlot;
            // a `Shared(h)` writes into the arena. No eval inside either arm (§borrowDiscipline
            // rule 3), and `frame_stack`/`frame_windows` are DISTINCT RefCells, so holding both
            // is aliasing-free. `Owned` is byte-identical to the pre-arena path.
            let mut fs = self.frame_stack.borrow_mut();
            match fs.last_mut().expect("arg bind: no active call window") {
                crate::state::WindowSlot::Owned(w) => w[slot as usize] = v,
                crate::state::WindowSlot::Shared(h) => {
                    let h = *h as usize;
                    // Stage-3 refcount airtightness: an arena WRITE must never touch a slot
                    // with no live reference (would corrupt a freed/recycled window).
                    debug_assert!(
                        self.frame_window_rc.borrow()[h] > 0,
                        "frame_slot_write: arena access on a freed shared window (h={h})"
                    );
                    self.frame_windows.borrow_mut()[h]
                        .as_mut()
                        .expect("arg bind: live shared window")[slot as usize] = v;
                }
            }
        } else {
            let mut g = self.static_store.borrow_mut();
            g.get_mut(&func).expect("arg bind: no storage slab")[slot as usize] = v;
        }
    }

    /// Coerce X/Z bits of `v` to 0 when the frame slot `(func, slot)` is a 2-state
    /// net (registered in `two_state`). The slot's flat net id is
    /// `func_table[func].base_net + slot`. A non-2-state slot returns `v` unchanged.
    pub(crate) fn coerce_two_state_frame(&self, func: u32, slot: u32, mut v: Value) -> Value {
        let Some(net) = self
            .func_table
            .get(func as usize)
            .map(|m| (m.base_net + slot) as usize)
        else {
            return v;
        };
        if net < self.two_state.len() && self.two_state[net] && v.unk.iter().any(|&u| u != 0) {
            for k in 0..v.unk.len() {
                v.val[k] &= !v.unk[k]; // X (val0/unk1) & Z (val1/unk1) → 0
                v.unk[k] = 0;
            }
        }
        v
    }

    /// Store the fully-evaluated `v` into a whole-net frame-local lvalue. The
    /// value is resized to the slot net's declared width/sign in a LOCAL first,
    /// THEN a scoped `borrow_mut` does the index-store with no eval inside
    /// (§borrowDiscipline rule 2). Part-select / array / module-net lvalues are
    /// rejected at ELABORATE, so the engine only ever sees a whole-net chunk;
    /// the `debug_assert` is a release-stripped backstop.
    pub(crate) fn frame_write_lvalue(&self, lhs: &Lvalue, v: Value) {
        // §6.16, HERE and not only in `frame_or_class_write` — this is the funnel
        // every FRAME-SLOT destination passes through, and the other one is not.
        //
        // ⚠️ It was installed one level up first, and that missed a whole lane:
        // `write_lvalue_general`'s frame-local branch (`init_diag.rs`) calls THIS
        // function directly, and frame locals are excluded from the fast write path,
        // so every frame-local write issued by `run_process` takes it. That is the
        // lane a `task automatic` uses the moment its body leaves the `&self`
        // subset — a single `$display` is enough. The variable is the LANE, not the
        // executor: measured against the tree that had the intercept fixed and the
        // coercion only in `frame_or_class_write`, the same task WITHOUT a
        // `$display` was right and WITH one was wrong. (Against `main` BOTH are
        // wrong, for the earlier reason — so re-measuring this sentence there reads
        // as false unless the tree is stated, which is why it is stated.) The branch
        // had briefly made function and task disagree about one statement, which is
        // the defect class this slice exists to remove.
        //
        // Idempotent: `coerce_str_to_packed` returns immediately unless the value
        // is still `is_str`, so the `frame_or_class_write` call (still needed — its
        // class-FIELD arm returns before this function) costs nothing on arrival.
        let v = self.coerce_str_to_packed(lhs, v);
        debug_assert_eq!(lhs.chunks.len(), 1, "frame lvalue is a single chunk");
        let c = &lhs.chunks[0];
        let net = c.net as usize;
        debug_assert!(
            self.frame_local[net],
            "frame write targets a frame-local net"
        );
        // §4.5.178: a heap-backed DYNAMIC-array write (`b[i]=v` / a whole-handle store to a
        // dyn-array `input` formal or a frame-local dyn array) reaching this `&self` frame
        // executor (`run_frame_call` = functions, `run_task` = subset tasks) CANNOT be
        // performed — the heap store (`write_lvalue`→`dyn_write`) is `&mut`, so the write
        // would land in the unused scalar frame slot while READS come from the heap: the
        // write silently vanishes. `function f(input int b[]); b[0]=9; return b[0];` called
        // as `r = f(a)` returned the PRE-write value (1, not 9) — a pre-existing §4.5.177
        // silent-wrong (the snapshot soundness argument tacitly assumed a read-only body).
        // Loud instead (correct-or-loud).
        // A frame-local STRING is `dyn_is_handle` too but is slab-stored (§4.5.167) and DOES
        // write correctly below, so it is EXCLUDED. Empty `dyn_is_handle` ⇒ never taken.
        if self.dyn_is_handle.get(net).copied().unwrap_or(false)
            && self.ir.nets[net].kind != NetKind::String
        {
            // §4.5.194: an ELEMENT write to a frame-local dynamic array — a `new[]`-
            // allocated LOCAL (`loc[i]=v`, V5), a snapshotted `input` formal's local copy
            // (pass-by-value, IEEE §13.5.1), or an `output` formal (V2B). `dyn_heap` is
            // interior-mutable now, so resolve the index/offset EXACTLY as the module path
            // (`resolve_lvalue_offsets`: an X/Z or out-of-i32 index → the `OOR_DROP` 2^30
            // sentinel so the write DROPS) and do the heap store. A WHOLE-handle store (no
            // `word`) is not a blocking-assign element write — keep it loud (handle copy /
            // `new[]` alloc is a separate mechanism, not reached here).
            if c.word.is_some() {
                const OOR_DROP: u32 = 1 << 30;
                let idx = |e: u32| -> u32 {
                    let sw = self.wt.get(e);
                    let ev = self.mk_eval_ctx().eval_ctx(e, sw.width, sw.signed);
                    if ev.has_xz() {
                        return OOR_DROP;
                    }
                    match ev.to_i128_signed() {
                        Some(i) if (i32::MIN as i128..=i32::MAX as i128).contains(&i) => {
                            i as i32 as u32
                        }
                        _ => OOR_DROP,
                    }
                };
                let raw_off = c.offset.map(idx).unwrap_or(0);
                let raw_word = c.word.map(idx).unwrap_or(0);
                self.dyn_write(c, raw_off, raw_word, &v);
                return;
            }
            self.fatal_frame_heap_write();
            return;
        }
        // R23 §3.1/§4: an UNROUTED net here means a write this `&self` executor cannot
        // perform reached it anyway — the copy-out destination is a module/instance net, not
        // a frame slot. The `.expect()` that used to sit here aborted the process with
        // `frame lvalue net is routed` at `frame_eval.rs:236`: rc=101, `errors=` never
        // printed, and a vita-internal file:line where the user needed their own.
        //
        // The shape that produced it (a bare call statement inside a frame body whose output
        // actual is a module net) is now correct-support — `compute_suspendable_tasks` reads
        // a `Terminator::Call`'s copy-out destinations and routes such a caller to the `&mut`
        // process executor. This stays as the correct-or-loud floor for anything that still
        // reaches it: a fatal on the same channel as the other two frame-executor limits, so
        // the run ends as `FinishReason::Error` with a diagnostic instead of an abort.
        let Some((fidx, slot)) = self.frame_route[net] else {
            self.fatal_frame_unrouted_write(net);
            return;
        };
        let auto = self.frame_slot_auto[net];
        let nv = &self.ir.nets[net];
        let net_w = nv.width.max(1);
        // A WHOLE-net write: resize to the slot's declared width/sign and store.
        if c.offset.is_none() && c.word.is_none() && c.width.is_none() {
            // N1: a frame-local `string` slot (an OUTPUT/INOUT string formal) holds the
            // heap-string VALUE, not a width-resized bit-vector — materialise an is_str
            // Value from the evaluated bytes (mirrors the string input-formal copy-in
            // at `run_task` and the module `dyn_write` string path). A width resize here
            // would truncate the string to the 1-bit slot width (silently emptying it).
            let val = if nv.kind == NetKind::String {
                Value::from_str_bytes(&v.to_str_bytes())
            } else {
                v.resize_keep_sign(net_w, nv.signed)
            };
            // B4: route by this slot's EFFECTIVE lifetime (window vs static slab).
            self.frame_slot_write(fidx, auto, slot, val);
            return;
        }
        // EXT2-H: a bit/part-select write to a frame-local SCALAR net (`r[7:0]=x`,
        // `r[i]=b`) — read-modify-write the slot. (An ARRAY-element `c.word` write is
        // rejected at elaborate: frame locals are scalar.) The offset (const OR a
        // dynamic index) is evaluated FIRST with no frame borrow held; the (lsb,
        // width) computation mirrors `write_chunk` (the module-net path), and any bit
        // outside `[0, net_w)` is dropped (IEEE part-select OOB semantics).
        // Resolve the (possibly dynamic) offset with the SAME OOR_DROP semantics as
        // the module path (`resolve_lvalue_offsets`): an X/Z index or one outside the
        // signed i32 range → a huge sentinel (2^30) so every selected bit lands out of
        // range and the write is DROPPED (iverilog parity), NOT written at bit 0.
        // Signed-aware: an unsigned 0xFFFFFFFF is the huge 4294967295 (drop), not a
        // wrapped −1 (which would partial-write).
        const OOR_DROP: u32 = 1 << 30;
        let raw_off = c
            .offset
            .map(|e| {
                let sw = self.wt.get(e);
                let v = self.mk_eval_ctx().eval_ctx(e, sw.width, sw.signed);
                if v.has_xz() {
                    OOR_DROP
                } else {
                    match v.to_i128_signed() {
                        Some(i) if (i32::MIN as i128..=i32::MAX as i128).contains(&i) => {
                            i as i32 as u32
                        }
                        _ => OOR_DROP,
                    }
                }
            })
            .unwrap_or(0);
        let off_i = raw_off as i32 as i64;
        let fold = |eid: u32| crate::width::const_u32_of_expr(self.ir, eid);
        let (lsb, width) = match c.kind {
            SelKind::Bit => (off_i, 1u32),
            SelKind::PartConst | SelKind::PartIdxUp => {
                (off_i, c.width.and_then(fold).unwrap_or(net_w))
            }
            SelKind::PartIdxDown => {
                let w = c.width.and_then(fold).unwrap_or(net_w);
                (off_i - w as i64 + 1, w)
            }
        };
        let piece = v.resize_keep_sign(width.max(1), false);
        let mut cur = self.frame_slot_read(fidx, auto, slot);
        for k in 0..width {
            let bp = lsb + k as i64;
            if bp >= 0 && (bp as u32) < net_w {
                let (bv, bu) = piece.get_vu(k);
                cur.set_vu(bp as u32, bv, bu);
            }
        }
        self.frame_slot_write(fidx, auto, slot, cur);
    }

    /// N7: store a frame-body blocking-assign value, routing a class FIELD write
    /// (`this.f = v` / `obj.f = v`) to the heap (`class_field_write`) and every
    /// other (whole frame-local net) write to `frame_write_lvalue`. `&self` —
    /// both targets are interior-mutable (the `RefCell` heap / the frame arena).
    pub(crate) fn frame_or_class_write(&self, lhs: &Lvalue, v: Value) {
        let v = self.coerce_str_to_packed(lhs, v);
        if lhs.chunks.len() == 1 {
            let c = &lhs.chunks[0];
            if self.class_is_handle[c.net as usize] && c.word.is_some() {
                let field = c
                    .word
                    .and_then(|w| crate::width::const_u32_of_expr(self.ir, w))
                    .unwrap_or(0);
                self.class_field_write(c, field, &v);
                return;
            }
        }
        self.frame_write_lvalue(lhs, v);
    }

    /// IEEE §6.16: a STRING VALUE written to a PACKED destination is right-aligned
    /// in the destination's width — left-truncated when longer, zero-padded when
    /// shorter.
    ///
    /// Called from TWO places, because the frame destinations do not share one
    /// entry: `frame_write_lvalue` (every frame SLOT, including the
    /// `write_lvalue_general` lane a lifted task takes) and `frame_or_class_write`
    /// (whose class-FIELD arm returns before that function). An earlier version of
    /// this doc claimed the second was "the one entry both their destinations pass
    /// through"; it is not, and believing it left every lifted `task automatic`
    /// unconverted.
    ///
    /// ⚠️ **The module funnel does it with `Value::resize`, and `resize_keep_sign`
    /// is NOT a substitute.** `write_lvalue` resizes to the destination width
    /// before storing (`init_diag.rs`) and then stores BITS, dropping the `Value`
    /// wrapper; `resize_keep_sign` deliberately returns an `is_str` value
    /// UNCHANGED — its
    /// own doc says why ("a string's width is its DYNAMIC length … the write
    /// funnel owns §6.16 conversion"). The frame funnel stores a whole `Value`
    /// into a slot and only had the latter, so it performed no conversion at all:
    /// a `reg [63:0]` frame local assigned `$sformatf("v=%0d", 7)` held a 24-BIT
    /// value, and `~p` / `p >> 64` / a compare against a padded literal all read
    /// wrong at exit 0, while the identical statement at module scope was right.
    /// Both adversarial lenses of S3a round 2 converged on it; the `$sformatf`
    /// intercept above had fixed the TEXT and left the WIDTH.
    ///
    /// The oracle is external after all, through the legal spelling of the same
    /// rule: `reg [15:0] p = {"ab","cde"};` is `de` (25701) in iverilog 13 —
    /// left-truncation, which is what `resize` does and what the module path
    /// already produced.
    ///
    /// TWO destinations keep the byte string: a `string` net (whose slot holds a
    /// heap string, not a bit vector) and a string ELEMENT of a dynamic array.
    /// Resizing either would truncate the text to the handle's width.
    ///
    /// The two halves are NOT equal, and this note has been wrong about which three
    /// times — so it now records only what a reachability probe measured.
    ///
    /// * `NetKind::String` is REACHABLE and load-bearing. It fires on a frame string
    ///   local and on an `output string` formal, and `lvalue_width` of a String net
    ///   is 1, so removing it would empty every one of them.
    /// * `dyn_str_elem` is **DEAD CODE**. A `panic!` planted in that arm fired ZERO
    ///   times across twelve designs on two backends — `string s[3]` with `foreach`,
    ///   a `&self` function with `string s[2]`, a `&self` task with
    ///   `string d[] = new[2]`, a runtime index — while the sibling arm fired on the
    ///   same runs. It cannot fire because a string-element write never reaches this
    ///   function: `write_lvalue_general`'s frame-local branch excludes dyn handles
    ///   whose kind is not `String`, and its own `dyn_str_elem` branch hands the
    ///   value to `write_chunk` first. Kept as the conservative answer for the day
    ///   that routing changes; it is not covered, and no test can cover it.
    ///
    /// (The two earlier versions of this note claimed the opposite in each
    ///   direction, and one credited a `string da[]` element the pre-slice binary
    ///   was said to have truncated — re-measured, PRE renders that one intact; what
    ///   PRE got wrong there was the `$sformatf` source, i.e. the intercept.)
    ///
    /// PRECONDITION worth stating: the carve-out reads `chunks.first()` while the
    /// resize uses the whole lvalue's width. A MULTI-CHUNK (concat) lvalue would
    /// make those two disagree — elaborate refuses one in a frame body (E3009
    /// "concatenation-target assignment"), and `frame_write_lvalue` debug-asserts
    /// a single chunk.
    ///
    /// ⚠️ **An earlier version of this note called `resize` → bare `is_str = false`
    /// an EQUIVALENT mutation, "measured".** It was not equivalent and it was not
    /// measured in the direction that mattered: that mutation additionally clears
    /// the flag in the equal-width case, which is the case `resize` misses — so its
    /// survival in the round-2 mutation run was evidence of the DEFECT, not of
    /// redundancy. Both adversarial lenses of round 3 found it independently. Both
    /// halves are done now: the resize states the width at one place with the
    /// LVALUE's own width (the shape `write_lvalue_general` has), and the clear
    /// makes that statement unconditional.
    ///
    /// Every downstream destination branch then resizes to the same width, so the
    /// double resize is idempotent: `frame_write_lvalue`'s whole-net arm via
    /// `resize_keep_sign` (NOT `frame_slot_write`, which only coerces 2-state —
    /// an earlier version credited the wrong function), `class_field_write`
    /// likewise on the FIELD's width (the chunk carries it, not the 32-bit
    /// handle's), `coerce_dyn_elem` via `resize`, and the bit/part-select branch
    /// which slices explicitly.
    fn coerce_str_to_packed(&self, lhs: &Lvalue, v: Value) -> Value {
        if !v.is_str {
            return v;
        }
        let Some(c) = lhs.chunks.first() else {
            return v;
        };
        let n = c.net as usize;
        if self.ir.nets.get(n).map(|x| x.kind) == Some(NetKind::String)
            || self.dyn_str_elem.get(n).copied().unwrap_or(false)
        {
            return v;
        }
        // `Value::resize` is the rule and it is stated ONCE, there: a resize is a
        // conversion to a packed width, so it drops `is_str` on every path
        // including its equal-width early return.
        //
        // ⚠️ That early return did NOT drop it until the round-5 review, and this
        // function carried an explicit clear to compensate. Two spellings of one
        // rule hid each other: with both present, neither could be killed by a
        // mutation, and the module lanes that resize WITHOUT coming through here
        // (`class_field_write`, `coerce_dyn_elem`, `assoc_write` store a whole
        // `Value`) stayed broken. The compensating clear is gone; the primitive
        // owns it.
        v.resize(self.lvalue_width(lhs))
    }

    /// IEEE §13.4.3 + §6.16: size one ACTUAL to its FORMAL's type, for the three
    /// frame-entry bindings (`run_frame_call`, `enter_task_frame`, `exit_arm_frame`).
    ///
    /// A `string` formal keeps the byte string: its slot is a 1-bit `Wire` that
    /// holds a heap value, and the `str_params` mask is the only thing that says
    /// so (width and kind cannot). Every OTHER formal is an ordinary packed
    /// destination, so a string VALUE is right-aligned in the formal's width and
    /// stops being a string — the same rule `coerce_str_to_packed` applies on the
    /// write side.
    ///
    /// ⚠️ `resize_keep_sign` alone does NOT do it, and that is why this exists:
    /// it returns an `is_str` value unchanged, so a `string` actual passed to a
    /// packed formal arrived at its full byte width and unsigned. Measured, before
    /// this: `string s = "abcde"; f(s)` with `input reg [15:0] p` gave the formal
    /// `abcde` / 40 bits where the module twin `m = s` gives `de` / 16 bits. The
    /// flag is cleared BEFORE the resize so the sign gets stamped too.
    pub(crate) fn bind_formal(&self, callee: u32, slot: u32, base: u32, v: Value) -> Value {
        if self.formal_is_string(callee, slot as usize) {
            return Value::from_str_bytes(&v.to_str_bytes());
        }
        let nv = &self.ir.nets[(base + slot) as usize];
        let mut src = v;
        src.is_str = false;
        src.resize_keep_sign(nv.width.max(1), nv.signed)
    }

    /// §4.5.175: is `rhs` an associative/dynamic/queue ITERATION method
    /// (`a.first/next/last/prev(k)`) — the lowered `foreach` walk over a dynamic
    /// array / queue / associative array? Such a step WRITES the iteration key as a
    /// side effect. The process executor does that via `Scheduler::assoc_iter_step`
    /// (`&mut write_lvalue`); the synchronous frame executors (`run_frame_call` /
    /// `run_task`, `&self`) route it through [`Self::frame_assoc_iter`] instead
    /// (§4.5.176), which writes a FRAME-LOCAL key via the interior-mutable window.
    pub(crate) fn rhs_is_assoc_iter(&self, rhs: u32) -> bool {
        matches!(
            self.ir.exprs.get(rhs as usize),
            Some(sim_ir::Expr::SysFunc {
                which: sim_ir::SysFuncId::AssocFirst
                    | sim_ir::SysFuncId::AssocNext
                    | sim_ir::SysFuncId::AssocLast
                    | sim_ir::SysFuncId::AssocPrev,
                ..
            })
        )
    }

    /// §4.5.176: the PURE-COMPUTE half of one assoc/dyn/queue iteration step
    /// (`a.first/next/last/prev(k)`). Reads only (`&self`) — the handle heap and the
    /// current key — and returns `(Some((key net, located-key value)) | None, status)`
    /// where status is 1 (found+fits) / 0 (none/exhausted/x-z key) / −1 (found but
    /// truncated, hand-IEEE §7.9.4). The CALLER performs the actual key write, so the
    /// SAME compute drives both the `&mut` process path (`write_lvalue`) and the `&self`
    /// frame path (`frame_write_lvalue`). A `None` key write leaves the key var
    /// unchanged (§7.9.4). Mirrors the read/compute of `Scheduler::assoc_iter_step`.
    pub(crate) fn assoc_iter_compute(&self, rhs: u32) -> (Option<(u32, Value)>, i32) {
        use std::ops::Bound;
        let Some(sim_ir::Expr::SysFunc { which, args }) = self.ir.exprs.get(rhs as usize) else {
            return (None, 0); // defensive: hand-built IR only (the rhs probe matched)
        };
        let which = *which;
        let net_of = |a: Option<&u32>| {
            a.and_then(|&e| match self.ir.exprs.get(e as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            })
        };
        let (Some(hnet), Some(knet)) = (net_of(args.first()), net_of(args.get(1))) else {
            return (None, 0); // malformed args: degrade, never panic
        };
        let (kw, ks) = self
            .ir
            .nets
            .get(knet as usize)
            .map(|nv| (nv.width.max(1), nv.signed))
            .unwrap_or((32, true));
        use sim_ir::SysFuncId as F;
        let needs_cur = matches!(which, F::AssocNext | F::AssocPrev);
        let cur_val = if needs_cur {
            let v = self.read_net(knet, None);
            if v.has_xz() {
                self.dyn_warn_once_at(hnet, "assoc iteration key variable is X/Z (status 0)");
                return (None, 0);
            }
            Some(v)
        } else {
            None
        };
        enum Hit {
            Int(i64),
            Str(Vec<u8>),
        }
        let hit: Option<Hit> = match self
            .dyn_heap
            .borrow()
            .get(hnet as usize)
            .and_then(|o| o.as_ref())
        {
            Some(crate::state::DynObj::Assoc { map }) => {
                let cur = cur_val.as_ref().map(|v| {
                    let raw = v.val.first().copied().unwrap_or(0);
                    if kw >= 64 {
                        raw as i64
                    } else {
                        let m = (1u64 << kw) - 1;
                        let r = raw & m;
                        if ks && (r >> (kw - 1)) & 1 == 1 {
                            (r | !m) as i64
                        } else {
                            r as i64
                        }
                    }
                });
                match which {
                    F::AssocFirst => map.keys().next().copied(),
                    F::AssocLast => map.keys().next_back().copied(),
                    F::AssocNext => map
                        .range((Bound::Excluded(cur.unwrap_or(0)), Bound::Unbounded))
                        .next()
                        .map(|(k, _)| *k),
                    _ => map
                        .range((Bound::Unbounded, Bound::Excluded(cur.unwrap_or(0))))
                        .next_back()
                        .map(|(k, _)| *k),
                }
                .map(Hit::Int)
            }
            Some(crate::state::DynObj::AssocStr { map }) => {
                let cur = cur_val.as_ref().map(crate::eval::value_str_bytes);
                match which {
                    F::AssocFirst => map.keys().next().cloned(),
                    F::AssocLast => map.keys().next_back().cloned(),
                    F::AssocNext => map
                        .range((
                            Bound::Excluded(cur.clone().unwrap_or_default()),
                            Bound::Unbounded,
                        ))
                        .next()
                        .map(|(k, _)| k.clone()),
                    _ => map
                        .range::<Vec<u8>, _>((
                            Bound::Unbounded,
                            Bound::Excluded(&cur.unwrap_or_default()),
                        ))
                        .next_back()
                        .map(|(k, _)| k.clone()),
                }
                .map(Hit::Str)
            }
            // Dense walk on dyn/queue (a missing entry IS the empty object).
            other => {
                let len = other.map(|o| o.len() as u64).unwrap_or(0);
                let cur = cur_val.as_ref().and_then(|v| v.to_u64());
                let dense = match which {
                    F::AssocFirst => (len > 0).then_some(0),
                    F::AssocLast => len.checked_sub(1),
                    F::AssocNext => cur.and_then(|c| c.checked_add(1)).filter(|&n| n < len),
                    _ => cur.and_then(|c| c.checked_sub(1)).filter(|&p| p < len),
                };
                dense.map(|d| Hit::Int(d as i64))
            }
        };
        let Some(hit) = hit else {
            return (None, 0); // none/empty/exhausted: key var UNCHANGED (§7.9.4)
        };
        let (kval, fits) = match hit {
            Hit::Int(k) => {
                let fits = if kw >= 64 {
                    true
                } else if ks {
                    let m = (1u64 << kw) - 1;
                    let t = (k as u64) & m;
                    let back = if (t >> (kw - 1)) & 1 == 1 {
                        (t | !m) as i64
                    } else {
                        t as i64
                    };
                    back == k
                } else {
                    k >= 0 && (k as u64) >> kw.min(63) == 0
                };
                let mut v = Value::zeros(kw, ks);
                let sign_fill = if k < 0 { u64::MAX } else { 0 };
                for (i, w) in v.val.iter_mut().enumerate() {
                    *w = if i == 0 { k as u64 } else { sign_fill };
                }
                v.mask_top();
                (v, fits)
            }
            Hit::Str(bytes) => {
                let fits = (bytes.len() as u64) * 8 <= kw as u64;
                let mut v = Value::zeros(kw, ks);
                for (i, b) in bytes.iter().rev().enumerate() {
                    for bit in 0..8u32 {
                        let idx = i as u32 * 8 + bit;
                        if idx < kw {
                            v.set_vu(idx, ((b >> bit) & 1) as u64, 0);
                        }
                    }
                }
                (v, fits)
            }
        };
        if !fits {
            self.dyn_warn_once_at(
                hnet,
                "assoc iteration key does not fit the index variable (truncated, status -1)",
            );
        }
        (Some((knet, kval)), if fits { 1 } else { -1 })
    }

    /// §4.5.176: execute one assoc/dyn/queue `foreach` step INSIDE a `&self` synchronous
    /// frame body (`run_frame_call` function / `run_task` subset task). Computes the step
    /// via [`Self::assoc_iter_compute`], writes the located key through the interior-
    /// mutable frame window (`frame_write_lvalue`), and writes the status to the step's
    /// `__st` lhs. The `foreach` desugar's key is always a BODY-LOCAL, so the key net is
    /// frame-local; a direct `st = aa.first(module_net)` in a function would need a `&mut`
    /// module-net write we cannot do here → those fall back to the fatal (correct-or-loud).
    pub(crate) fn frame_assoc_iter(&self, lhs: &Lvalue, rhs: u32) {
        let (key_write, status) = self.assoc_iter_compute(rhs);
        if let Some((knet, kval)) = key_write {
            // The key write must land in the frame window (this executor is `&self`); a
            // module-net key can only be written by the `&mut` process path.
            if !self
                .frame_local
                .get(knet as usize)
                .copied()
                .unwrap_or(false)
            {
                self.fatal_frame_assoc_iter();
                return;
            }
            let klv = sim_ir::Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net: knet,
                    word: None,
                    offset: None,
                    width: None,
                    kind: sim_ir::SelKind::Bit,
                }],
            };
            self.frame_or_class_write(&klv, kval);
        }
        // Write the int status to `__st` (mirror `Scheduler::k_assoc_iter`: 32-bit signed,
        // resized to the lhs / rhs self-width context).
        let lw = self.lvalue_width(lhs);
        let sw = self.wt.get(rhs);
        let mut sv = Value::zeros(32, true);
        sv.val[0] = (status as u32) as u64;
        let sv = sv.resize_keep_sign(lw.max(sw.width), sw.signed);
        self.frame_or_class_write(lhs, sv);
    }

    /// §4.5.176 fallback: a `foreach`/iteration step whose key is NOT frame-local (a
    /// direct `st = aa.first(module_net)` in a function body) needs a `&mut` module-net
    /// write this `&self` executor cannot do — fatal-loud (reuses the `call_fatal`
    /// channel → `FinishReason::Error`). Correct-or-loud: a fatal, never a silent 0.
    /// §4.5.178: latch a fatal for a heap dynamic-array WRITE attempted in a synchronous
    /// `&self` frame executor (see the guard at the top of `frame_write_lvalue`). Such a
    /// write cannot reach the `&mut` heap store, so it would silently vanish. Mirrors
    /// `fatal_frame_assoc_iter`'s latch (fires once; the scheduler converts the latched
    /// `call_fatal` to `FinishReason::Error`).
    pub(crate) fn fatal_frame_heap_write(&self) {
        if !self.call_fatal.get() {
            self.call_fatal.set(true);
            self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Fatal,
                code: MsgCode::RunFatal,
                message: "writing an element of (or a whole store to) a dynamic-array `input` \
                          formal is unsupported inside a synchronous function / subset-task body \
                          — the formal is a pass-by-value copy on the heap and this `&self` \
                          frame executor cannot mutate the heap (the write would silently \
                          vanish). Read the formal without mutating it, or restructure to a \
                          process / suspendable task."
                    .to_string(),
                location: None,
                context: Vec::new(),
                sim_time: Some(TimeStamp { ticks: self.now }),
            }));
        }
    }

    /// R23 §3.1/§4: latch a fatal for a frame write whose destination net is NOT routed to
    /// a frame slot — i.e. a module/instance net reached the synchronous `&self` executor,
    /// which has no way to write the flat store or to raise the dirty channel.
    ///
    /// This replaces a bare `.expect("frame lvalue net is routed")`, which aborted the whole
    /// process at rc=101 naming a vita source line. The message names the net so the user
    /// can find it, states the condition in the terms of their own source, and gives the two
    /// restructurings that work — because a diagnostic whose only actionable content is an
    /// internal file:line is the failure the report filed this under.
    pub(crate) fn fatal_frame_unrouted_write(&self, net: usize) {
        if !self.call_fatal.get() {
            self.call_fatal.set(true);
            let name = self
                .net_names
                .get(net)
                .filter(|n| !n.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("net #{net}"));
            self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Fatal,
                code: MsgCode::RunFatal,
                message: format!(
                    "a subroutine running on the synchronous frame executor tried to write \
                     `{name}`, which is not one of its frame-local variables — a module / \
                     instance net can only be written from the process executor, which has \
                     the dirty channel that makes the change visible. Move the write into a \
                     `task` (a task body's out-of-frame write routes there automatically), or \
                     into the calling process."
                ),
                location: None,
                context: Vec::new(),
                sim_time: Some(TimeStamp { ticks: self.now }),
            }));
        }
    }

    /// R19-X2: is `rhs` a system function whose value comes with a write this executor
    /// cannot perform? Named for the reason, not for a list of ids.
    ///
    /// R22: the id list WAS the file-read family alone, and the six it left out were
    /// silently wrong here — `$fopen`, `$value$plusargs`, a seeded `$random`/`$dist_*`
    /// and `$cast` returned 0 (or left their destination at its default) at exit 0 with
    /// no diagnostic at all, which is strictly worse than the loud `$fgets` next to them.
    /// It now defers to [`sim_ir::rhs_frame_executor_cannot_perform`] — the canonical
    /// answer to exactly this question, shared with the elaborate-side routing decision —
    /// so the two can never drift apart again.
    ///
    /// Two members of the statement-effect family are deliberately outside that predicate,
    /// both because a MORE precise gate already owns them:
    /// * `Sformatf` — this executor has a working intercept for it (`frame_rhs_value`'s
    ///   `NetKind::String` arm), measured correct.
    /// * the assoc-iteration steps (`first`/`next`/`last`/`prev`) — whether THIS executor
    ///   can run one depends on the key: a body-local key works, and only a non-local key
    ///   cannot. `fatal_frame_assoc_iter` tests exactly that and says so; a blanket answer
    ///   here would both regress the working local-key case and replace a diagnostic that
    ///   names the fix with one that does not.
    pub(crate) fn rhs_is_sysread(&self, rhs: u32) -> bool {
        sim_ir::rhs_frame_executor_cannot_perform(self.ir.exprs.as_slice(), rhs)
    }

    /// R19-X2: latch the fatal for [`Self::rhs_is_sysread`], the same `call_fatal`
    /// channel `fatal_frame_heap_write` uses (fires once; the scheduler converts it to
    /// `FinishReason::Error`).
    ///
    /// Deliberately a RUNTIME fatal, not an elaborate-time reject: a `task automatic`
    /// with no output formals is lowered BOTH as a frame body and inline, and the inline
    /// copy is the one its callers run — reading the file correctly. An elaborate gate on
    /// the frame body would have loud-rejected that working design (measured while
    /// building this). Firing where the frame copy actually executes rejects exactly the
    /// calls that would have got nothing.
    pub(crate) fn fatal_frame_sysread(&self, rhs: u32) {
        if self.call_fatal.get() {
            return;
        }
        let which = match self.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { which, .. }) => match which {
                sim_ir::SysFuncId::Fgets => "$fgets",
                sim_ir::SysFuncId::Fread => "$fread",
                sim_ir::SysFuncId::Fscanf => "$fscanf",
                sim_ir::SysFuncId::Sscanf => "$sscanf",
                sim_ir::SysFuncId::Fgetc => "$fgetc",
                sim_ir::SysFuncId::Feof => "$feof",
                sim_ir::SysFuncId::Ungetc => "$ungetc",
                sim_ir::SysFuncId::Fopen => "$fopen",
                sim_ir::SysFuncId::ValuePlusargs => "$value$plusargs",
                sim_ir::SysFuncId::Random => "a seeded `$random`",
                sim_ir::SysFuncId::Cast => "$cast",
                sim_ir::SysFuncId::QPopBack | sim_ir::SysFuncId::QPopFront => "a queue pop",
                _ => "a seeded `$dist_*`",
            },
            _ => "a side-effecting system function",
        };
        self.call_fatal.set(true);
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Fatal,
            code: MsgCode::RunFatal,
            // R22 §3.1: the old advice was stale in BOTH directions. It told the user to
            // avoid `automatic` and output/inout formals, but those were never the
            // discriminator — an `automatic` task with an output formal is exactly what
            // now works, and dropping the lifetime keyword (its suggested fix) made the
            // failure SILENT instead of loud. It also promised a task "vita can inline"
            // would work, which for a `string` destination it did not. What is actually
            // left is a handful of positions vita cannot route to the statement executor,
            // so the message names those and nothing else. Measured against a matrix of
            // every statement-effect system function in every subroutine shape.
            message: format!(
                "`{which}` does its work as a statement-level effect, which the \
                 synchronous `&self` frame executor cannot perform — the call would \
                 return 0 and leave its destination untouched, so the run stops here \
                 rather than continuing on that value. This is one of the few positions \
                 vita cannot route to the statement executor: a class-method body, a \
                 CONTINUOUSLY re-evaluated expression (`assign`, `force`, a `wait` \
                 condition), or an intra-assignment delay (`x = #1 f(...)`). It DOES work \
                 in a module process, and in a task or function called from a statement \
                 — with or without `automatic`, with or without output formals. Call it \
                 there, assign the result to a variable, and use that variable here."
            ),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.now }),
        }));
    }

    pub(crate) fn fatal_frame_assoc_iter(&self) {
        if !self.call_fatal.get() {
            self.call_fatal.set(true);
            self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Fatal,
                code: MsgCode::RunFatal,
                message: "an associative-array iteration (`first/next/last/prev`) whose key \
                          variable is not a local of the function/task is unsupported inside a \
                          synchronous frame body — declare the key as a body-local, or iterate \
                          in a process / suspendable task."
                    .to_string(),
                location: None,
                context: Vec::new(),
                sim_time: Some(TimeStamp { ticks: self.now }),
            }));
        }
    }

    /// r18 (F2): render a SEVERITY task (`$info`/`$warning`/`$error`/`$fatal`, lowered as
    /// a `Display` + a `severities` sidecar) reached inside a synchronous `&self` frame
    /// function / subset-task body. The message goes to the diagnostic stream (the sink is
    /// interior-mutable) byte-for-byte the `emit_severity_message` text; the effect is
    /// applied through interior mutability — `$error` latches `had_error` (exit code),
    /// `$fatal` latches `call_fatal` (the scheduler converts it to an error finish, the
    /// SAME channel `fatal_frame_heap_write` uses). Admitting these in `classify_frame_body`
    /// makes a `unique`/`priority case` — whose no-match arm is a synthetic `$warning` —
    /// usable in a frame body (and any explicit severity call there). A severity in a
    /// SUSPENDABLE task runs on the `run_process` path and is handled by `dispatch`, not
    /// here.
    pub(crate) fn frame_emit_severity(
        &self,
        sev: crate::SeverityKind,
        fmt: Option<u32>,
        args: &[u32],
    ) {
        use crate::SeverityKind as K;
        let (severity, code) = match sev {
            K::Fatal => (Severity::Fatal, MsgCode::RunFatal),
            K::Error => (Severity::Error, MsgCode::RunUserError),
            K::Warning => (Severity::Warning, MsgCode::RunUserWarning),
            K::Info => (Severity::Info, MsgCode::RunUserInfo),
        };
        let mut message = crate::builtins::format_args_str(self, fmt, args, None);
        if message.is_empty() {
            message = code.title().to_string();
        }
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity,
            code,
            message,
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.now }),
        }));
        match sev {
            K::Fatal => self.call_fatal.set(true),
            K::Error => self.had_error.set(true),
            K::Warning | K::Info => {}
        }
    }

    /// B1 frame-call evaluator. Runs user function `func`'s lowered body (in the
    /// GLOBAL `ir.blocks` arena from `FuncDef.entry`) against a per-invocation
    /// frame, returning its return-var Value resized to the declared return
    /// width/sign. `&self` (read path) + interior-mutable frame arena; the body
    /// BB loop is iterative, native recursion occurs ONLY on a nested
    /// `Expr::Call` (heap-bounded by the window stack, capped at
    /// `MAX_CALL_DEPTH`). `None` ⇒ a corrupt/empty sidecar (the eval arm
    /// X-poisons) — but `build_func_routing` validates the table, so a populated
    /// table reaching here is well-formed.
    pub(crate) fn run_frame_call(&self, func: u32, args: &[Value]) -> Option<Value> {
        use sim_ir::{Stmt, Terminator};
        if self.func_table.is_empty() {
            return None; // non-frame Call → the eval arm X-poisons
        }
        let m = self.func_table[func as usize];
        let (rw, rsig) = (m.ret_width.max(1), m.ret_signed);

        // ── runaway-recursion guard (heap depth, NOT host stack) ──
        let d = self.call_depth.get();
        if d >= MAX_CALL_DEPTH {
            if !self.call_fatal.get() {
                self.call_fatal.set(true);
                self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Fatal,
                    code: MsgCode::RunFatal,
                    message: format!(
                        "frame-call recursion exceeded the depth limit ({MAX_CALL_DEPTH})"
                    ),
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(TimeStamp { ticks: self.now }),
                }));
            }
            // Finish THIS in-flight eval cleanly with all-X; the scheduler will
            // convert the latched `call_fatal` to FinishReason::Error.
            return Some(Value::xs(rw, rsig));
        }
        self.call_depth.set(d + 1);
        let _g = DepthGuard(&self.call_depth); // decrements on EVERY exit
                                               // N1: expose this subroutine (FuncId `func`) to `%m` rendered inside its body.
        self.frame_scope.borrow_mut().push(func);
        let _sg = FrameScopeGuard(&self.frame_scope);

        let fd = self.ir.funcs[func as usize];
        debug_assert!(
            !fd.is_task,
            "tasks are rejected at elaborate (B2 frame-call)"
        );
        debug_assert!(
            self.ir.nets[(m.base_net + m.return_slot) as usize].width == m.ret_width
                && self.ir.nets[(m.base_net + m.return_slot) as usize].signed == m.ret_signed,
            "return-var net width/sign must equal the declared ret_width/ret_signed"
        );
        let base = m.base_net;
        let nloc = m.locals_len;
        let np = fd.n_params;
        // B4: per-func storage needs (window for automatic slots, slab for static).
        let has_auto = self.func_has_auto[func as usize];
        let has_static = self.func_has_static[func as usize];

        // ── FRAME SETUP: build the fresh window in a LOCAL, then install it.
        //    The per-call WINDOW holds automatic slots (push/pop); the persistent
        //    STATIC slab (X-init ONCE, never reset) holds static slots. A func with
        //    no lifetime overrides uses exactly one of them (byte-identical to B1). ──
        // Each fresh frame slot defaults to its NET's declared init — X for 4-state
        // (reg/logic/integer), 0 for 2-state (bit/byte/int/shortint/longint) per IEEE
        // §6.4. (Using `Value::xs` unconditionally mis-defaulted 2-state locals to X,
        // silently corrupting reads/branches of an unassigned 2-state local.)
        let fresh: Vec<Value> = (0..nloc)
            .map(|s| {
                let nv = &self.ir.nets[(base + s) as usize];
                // N1: a frame-local `string` slot (an output/inout string formal or a
                // string return var) defaults to the EMPTY string, not a width-1
                // non-is_str value — the latter renders as a stray space and makes
                // `s==""` / `s.len()==0` FALSE on an unwritten path (a silent-wrong
                // caught by adversarial review; string-return t03 already matched
                // iverilog because it happened to write, output formals did not).
                if nv.kind == NetKind::String {
                    Value::from_str_bytes(&[])
                } else {
                    Value::from_packed(&nv.init, nv.width.max(1), nv.signed)
                }
            })
            .collect();
        // A pure FUNCTION never contains a Case-B fork (a fork in a function body is
        // loud), so its window is always `Owned` (byte-identical to the pre-arena path).
        match (has_auto, has_static) {
            (true, true) => {
                self.frame_stack
                    .borrow_mut()
                    .push(WindowSlot::Owned(fresh.clone()));
                self.static_store.borrow_mut().entry(func).or_insert(fresh);
            }
            (true, false) => self.frame_stack.borrow_mut().push(WindowSlot::Owned(fresh)),
            (false, _) => {
                self.static_store.borrow_mut().entry(func).or_insert(fresh);
            }
        }

        // ── BIND ARGS into the formal slots (resize to the formal's width). ──
        for i in 0..np {
            let nv = &self.ir.nets[(base + i) as usize];
            // A MISSING actual keeps its old value (`Value::xs`) rather than going
            // through `bind_formal`: for a `string` formal that helper would turn
            // the X into `""`, and nothing proves a call can supply fewer actuals
            // than formals (elaborate checks arity). An unreachable behaviour change
            // is still a behaviour change no test names.
            let v = match args.get(i as usize) {
                Some(a) => self.bind_formal(func, i, base, a.clone()),
                None => Value::xs(nv.width.max(1), nv.signed),
            };
            self.frame_slot_write(func, self.frame_slot_auto[(base + i) as usize], i, v);
        }

        // V5 (§4.5.194) / T1-9: open the function's own dyn LOCAL for THIS activation,
        // stashing whatever the outer one held. Formals `[0, np)` are caller-snapshotted
        // (§4.5.177), so `first_slot = np` excludes them (their slot is legitimately live
        // at entry and is not this activation's to take).
        // The stash is a LOCAL — a pure function body cannot suspend, so entry and
        // exit here are straight-line and cannot interleave with another activation.
        let dyn_stash = self.frame_dyn_enter_from(func, np);

        // ── BB LOOP over the GLOBAL func arena from `fd.entry`. Process bodies
        //    live in a SEPARATE `Process.body` space and are never touched. ──
        let mut cur = fd.entry;
        loop {
            debug_assert!(
                (cur as usize) < self.ir.blocks.len(),
                "frame CFG target in range (rebase complete)"
            );
            let blk = &self.ir.blocks[cur as usize];
            for &sid in &blk.stmts {
                match &self.ir.stmts[sid as usize] {
                    Stmt::BlockingAssign { lhs, rhs } => {
                        // §4.5.176: a dyn/queue/assoc `foreach` step (`__st = a.first/next(k)`)
                        // WRITES its key as a side effect. This `&self` function executor runs
                        // it through the frame-aware path (writes the frame-local key via the
                        // interior-mutable window) instead of a plain eval that would stall at 0.
                        if self.rhs_is_assoc_iter(*rhs) {
                            self.frame_assoc_iter(lhs, *rhs);
                            continue;
                        }
                        // OWNED Value FIRST — its nested Calls may recurse into
                        // run_frame_call, fine: THIS frame holds NO live borrow now.
                        // N1: `$sformatf` renders through the shared formatter here.
                        let v = self.frame_rhs_value(lhs, *rhs);
                        // THEN store (borrow scoped to the index-store only). N7: a
                        // class field write routes to the heap, not a frame slot.
                        self.frame_or_class_write(lhs, v);
                    }
                    // V5 (§4.5.194): a frame-local dyn `new[]` / `delete()` — heap ops that
                    // the interior-mutable `dyn_heap` now lets this `&self` executor perform.
                    Stmt::SysTask {
                        which: sim_ir::SysTaskId::DynNew,
                        args,
                        ..
                    } => self.frame_dyn_new(args),
                    Stmt::SysTask {
                        which: sim_ir::SysTaskId::DynDelete,
                        args,
                        ..
                    } => {
                        if let Some(&a0) = args.first() {
                            if let sim_ir::Expr::Signal { net, .. } = &self.ir.exprs[a0 as usize] {
                                self.dyn_heap.borrow_mut()[*net as usize].take();
                            }
                        }
                    }
                    // Family C (§4.5.194+r17): a dyn-array-formal SNAPSHOT marker for a
                    // `x = f(arr)` call inside THIS function body — deep-copy the caller
                    // array (src) into the callee formal's heap slot (dst), the exact op
                    // the module-process executor runs (builtins §7.10 marker path). The
                    // interior-mutable `dyn_heap` makes this a `&self`-safe heap op. Only
                    // handle-copy markers match (a real `$display` — absent from a func
                    // body per the B1 cut — is not in `handle_copy_stmts`).
                    Stmt::SysTask {
                        which: sim_ir::SysTaskId::Display,
                        ..
                    } if self.handle_copy_stmts.contains_key(&sid) => {
                        if let Some(&(dst, src)) = self.handle_copy_stmts.get(&sid) {
                            self.frame_dyn_copy_out(src, dst);
                            self.enforce_queue_bound(dst);
                        }
                    }
                    // r18 (F2): a SEVERITY task (`$info`/`$warning`/`$error`/`$fatal`, a
                    // `Display` with a `severities` sidecar) admitted by `classify_frame_body`.
                    // Route to the diag stream — MUST precede the plain Display|Write arm,
                    // which would otherwise print it to stdout as a `$display`.
                    Stmt::SysTask {
                        which: sim_ir::SysTaskId::Display,
                        fmt,
                        args,
                    } if self.severities.contains_key(&sid) => {
                        if let Some(&sev) = self.severities.get(&sid) {
                            self.frame_emit_severity(sev, *fmt, args);
                        }
                    }
                    // Family D (r17): a genuine `$display`/`$write` in this subset function
                    // body (admitted by `classify_frame_body` via `frame_print_stmts`; a
                    // timeformat/marker Display was rejected there and never reaches here, a
                    // severity Display was consumed by the arm above, and a dyn-formal marker
                    // by the handle-copy arm). Render through the same `&self`-safe formatter
                    // the module process uses and emit as RtlOutput — the sink is
                    // interior-mutable, so this is byte-identical to the module-process print.
                    Stmt::SysTask { which, fmt, args }
                        if matches!(
                            which,
                            sim_ir::SysTaskId::Display | sim_ir::SysTaskId::Write
                        ) =>
                    {
                        let radix = self.radixes.get(&sid).copied();
                        let mut s = crate::builtins::format_args_str(self, *fmt, args, radix);
                        if matches!(which, sim_ir::SysTaskId::Display) {
                            s.push('\n');
                        }
                        self.sink.emit(LogEvent::RtlOutput(diag::RtlText {
                            text: s,
                            sim_time: None,
                        }));
                    }
                    // Other SysTask / NBA / delay / event in a func body are rejected at
                    // ELABORATE (B1 cut) → never reach here.
                    _ => {}
                }
            }
            match &blk.term {
                Terminator::Goto { target } => cur = *target,
                Terminator::Branch {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    let taken = self.mk_eval_ctx().truthy(*cond); // X/Z cond → else
                    cur = if taken { *then_bb } else { *else_bb };
                }
                Terminator::Return => break,
                // Delay/Wait/Fork/Call are illegal in a pure func body (rejected
                // at elaborate); break defensively (rebase keeps targets valid).
                _ => break,
            }
        }

        // V5 (§4.5.194) / T1-9: close this activation's dyn LOCALS, restoring whatever the
        // outer activation held so a RECURSIVE call finds its own array again (formals
        // `[0, np)` excluded — they are caller-managed snapshots, §4.5.177).
        self.frame_dyn_exit(dyn_stash);

        // ── READ the return slot (clone + release), resize to declared width. ──
        let ret_auto = self.frame_slot_auto[(base + m.return_slot) as usize];
        let rv = self
            .frame_slot_read(func, ret_auto, m.return_slot)
            .resize_keep_sign(rw, rsig);
        if has_auto {
            self.frame_stack.borrow_mut().pop(); // static: leave the slab (persistence)
        }
        Some(rv) // _g drops here → call_depth decremented
    }
}
