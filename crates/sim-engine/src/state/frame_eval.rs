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

    /// Evaluate an expression to a self-width Value against the CURRENT state
    /// (module nets, or the active frame window if one is installed). Identical to
    /// `Scheduler::eval` — both build an `EvalCtx` over this `SimState` — so the
    /// `&SimState` formatter (`format_args_str`) renders byte-for-byte the same from
    /// a process body (executor) and from inside a frame body (N1: `$sformatf` args
    /// that read frame-local formals resolve through `read_net`'s frame routing).
    pub(crate) fn eval_expr(&self, eid: u32) -> Value {
        self.mk_eval_ctx().eval(eid)
    }

    /// N1: evaluate a frame-body `BlockingAssign` RHS. `$sformatf(fmt, …)` written to
    /// a `string` target is rendered through the SHARED formatter (`format_args_str`) —
    /// the generic `eval_sysfunc` arm cannot see the format string (it would dump the
    /// raw args), so a `s = $sformatf(...)` in a subroutine body (an OUTPUT string
    /// formal, a string return) needs this intercept. The frame window is active, so any
    /// format arg that reads a frame-local formal resolves through `read_net`. The
    /// intercept is GATED on a `NetKind::String` lhs: a `$sformatf` assigned to a
    /// NUMERIC target is an illegal implicit string→int cast (iverilog rejects it), so
    /// vita leaves it on the pre-existing generic path rather than changing its value.
    /// Any other RHS evaluates normally in the assignment-width context.
    pub(crate) fn frame_rhs_value(&self, lhs: &Lvalue, rhs: u32) -> Value {
        let lhs_is_string = lhs
            .chunks
            .first()
            .and_then(|c| self.ir.nets.get(c.net as usize))
            .map(|n| n.kind == NetKind::String)
            .unwrap_or(false);
        if lhs_is_string {
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
        let (fidx, slot) = self.frame_route[net].expect("frame lvalue net is routed");
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
            let v = args
                .get(i as usize)
                .cloned()
                .unwrap_or_else(|| Value::xs(nv.width.max(1), nv.signed))
                .resize_keep_sign(nv.width.max(1), nv.signed);
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
