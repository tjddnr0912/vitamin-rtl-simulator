//! `NetReader` impl for `SimState` (mechanical move).

use super::*;

impl NetReader for SimState<'_> {
    /// See [`NetReader::read_scalar_words`]. Mirrors, exactly and in the same order,
    /// what `read_net(net, None)` then `resize_keep_sign(w, ctx_signed)` produce:
    /// read word 0 of the flat store, mask to the net's declared width, narrow the
    /// signedness to `slot.signed && ctx_signed`, sign-extend only when EXTENDING a
    /// signed value whose sign bit is 1 or x, then mask to `w`.
    ///
    /// Bails to `None` — and therefore to the original path — for every case that is not
    /// a plain single-word scalar. Locked by `leaf_fast_path_matches_read_net`.
    fn read_scalar_words(&self, net: u32, w: u32, ctx_signed: bool) -> Option<(u64, u64)> {
        let i = net as usize;
        if self.class_is_handle[i] || self.frame_local[i] || self.dyn_is_handle[i] {
            return None;
        }
        let slot = self.nets.get(i)?;
        // `is_str` is not a slot flag; a string net is a dyn handle, already excluded.
        if slot.is_real || slot.array_len != 1 || slot.width == 0 || slot.width > 64 || w > 64 {
            return None;
        }
        let nw = slot.width;
        let nm = crate::value::top_mask(nw);
        let mut v = slot.cur.val.first().copied().unwrap_or(0) & nm;
        let mut u = slot.cur.unk.first().copied().unwrap_or(0) & nm;
        // `resize_keep_sign`: the value's own signedness is narrowed by the context's
        // before the resize decides whether to sign-extend.
        let eff_signed = slot.signed && ctx_signed;
        if w > nw && eff_signed {
            let bit = nw - 1;
            let fv = (v >> bit) & 1;
            let fu = (u >> bit) & 1;
            if fv != 0 || fu != 0 {
                let bits = w - nw;
                let mask = if bits >= 64 {
                    u64::MAX
                } else {
                    ((1u64 << bits) - 1) << nw
                };
                v = (v & !mask) | (if fv != 0 { u64::MAX } else { 0 } & mask);
                u = (u & !mask) | (if fu != 0 { u64::MAX } else { 0 } & mask);
            }
        }
        let m = crate::value::top_mask(w);
        Some((v & m, u & m))
    }

    fn dyn_size(&self, net: u32) -> Option<u64> {
        // Only a dyn HANDLE answers; a missing heap entry IS the empty object
        // (size 0 — IEEE: a declared dynamic array/queue/assoc starts empty).
        // Assoc included: `size()` is the IEEE alias of `num()` (§7.9.1). Any
        // other net kind returns None → the eval arm X-poisons defensively.
        match self.ir.nets.get(net as usize).map(|n| n.kind) {
            Some(
                sim_ir::NetKind::DynArray
                | sim_ir::NetKind::Queue
                | sim_ir::NetKind::Assoc
                | sim_ir::NetKind::AssocStr,
            ) => Some(
                self.dyn_heap
                    .borrow()
                    .get(net as usize)
                    .and_then(|o| o.as_ref())
                    .map(|o| o.len() as u64)
                    .unwrap_or(0),
            ),
            _ => None,
        }
    }
    /// ⓑ-breadth (v15): snapshot the element VALUES of a dyn handle in
    /// deterministic order, for the array reduction/ordering/locator methods.
    /// dyn array / queue iterate insertion order; assoc iterates its BTree key
    /// order (the IEEE iteration order). A non-handle (or a string handle, which
    /// is a byte sequence, not an element array) returns None → the caller
    /// X-poisons. A missing heap entry IS the empty array (`Some(vec![])`).
    fn dyn_values(&self, net: u32) -> Option<Vec<Value>> {
        match self.ir.nets.get(net as usize).map(|n| n.kind) {
            Some(
                sim_ir::NetKind::DynArray
                | sim_ir::NetKind::Queue
                | sim_ir::NetKind::Assoc
                | sim_ir::NetKind::AssocStr,
            ) => Some(
                match self
                    .dyn_heap
                    .borrow()
                    .get(net as usize)
                    .and_then(|o| o.as_ref())
                {
                    Some(DynObj::DynArray { elems }) => elems.clone(),
                    Some(DynObj::Queue { elems }) => elems.iter().cloned().collect(),
                    Some(DynObj::Assoc { map }) => map.values().cloned().collect(),
                    Some(DynObj::AssocStr { map }) => map.values().cloned().collect(),
                    _ => Vec::new(),
                },
            ),
            _ => None,
        }
    }
    fn array_item(&self, index: bool) -> Value {
        match &*self.array_item_scratch.borrow() {
            Some((val, idx)) => {
                if index {
                    let mut v = Value::zeros(32, true);
                    v.val[0] = (*idx).min(i32::MAX as u64);
                    v
                } else {
                    val.clone()
                }
            }
            None => Value::xs(32, true),
        }
    }
    fn swap_array_item(&self, v: Option<(Value, u64)>) -> Option<(Value, u64)> {
        self.array_item_scratch.replace(v)
    }
    fn dyn_warn(&self, net: u32, msg: &str) {
        // The eval-side degradation hook (e.g. a pop outside its statement
        // intercept) — same W4020 once-per-net latch as every other lane.
        self.dyn_warn_once_at(net, msg);
    }
    fn is_assoc(&self, net: u32) -> bool {
        // Bitmap first: the static-array hot path short-circuits on one bit
        // load (the same budget `read_net` already pays), only real handles
        // pay the kind lookup.
        self.dyn_is_handle
            .get(net as usize)
            .copied()
            .unwrap_or(false)
            && matches!(
                self.ir.nets.get(net as usize).map(|n| n.kind),
                Some(sim_ir::NetKind::Assoc)
            )
    }
    fn assoc_read(&self, net: u32, key: Option<i64>) -> Value {
        let (w, signed) = self
            .ir
            .nets
            .get(net as usize)
            .map(|nv| (nv.width.max(1), nv.signed))
            .unwrap_or((1, false));
        let Some(k) = key else {
            // X/Z key = invalid index (IEEE §7.8.6): element-width X + warn.
            self.dyn_warn_once_at(net, "assoc key is X/Z (read X)");
            return Value::xs(w, signed);
        };
        match self
            .dyn_heap
            .borrow()
            .get(net as usize)
            .and_then(|o| o.as_ref())
        {
            Some(DynObj::Assoc { map }) => map.get(&k).cloned().unwrap_or_else(|| {
                self.dyn_warn_once_at(net, "assoc key not found (read X)");
                Value::xs(w, signed)
            }),
            // A missing entry IS the empty assoc — every key is "not found".
            _ => {
                self.dyn_warn_once_at(net, "assoc key not found (read X)");
                Value::xs(w, signed)
            }
        }
    }
    fn assoc_exists(&self, net: u32, key: Option<i64>) -> Option<bool> {
        if !self.is_assoc(net) {
            return None; // not an assoc handle → the eval arm X-poisons
        }
        let Some(k) = key else {
            // exists() with an X/Z key matches nothing — 0, but LOUD (the
            // same invalid-index family as read/write, policy pin).
            self.dyn_warn_once_at(net, "assoc key is X/Z (exists 0)");
            return Some(false);
        };
        Some(
            match self
                .dyn_heap
                .borrow()
                .get(net as usize)
                .and_then(|o| o.as_ref())
            {
                Some(DynObj::Assoc { map }) => map.contains_key(&k),
                _ => false,
            },
        )
    }
    fn str_byte_at(&self, net: u32, i: usize) -> Option<u8> {
        // FRAME FIRST, and WITHOUT a `NetKind::String` test. A frame string FORMAL is a
        // one-bit wire whose slot holds the materialised string `Value` (the same thing
        // `handle_str_bytes` documents), so gating on the declared kind rejected exactly
        // the case this exists for: the formal fell through to `eval`, which clones the
        // whole string on every `.getc()`. Only genuine string operands reach the string
        // primitives (elaborate routes them), which is what makes the missing kind test
        // sound here — the same argument `handle_str_bytes` already relies on.
        if self.frame_local.get(net as usize).copied().unwrap_or(false) {
            // `read_net`'s frame routing, verbatim: a frame-local DYN ARRAY (`int loc[]`)
            // is `dyn_is_handle` AND not a string, and its elements live in the heap — it
            // is not a string operand at all. Everything else frame-local, INCLUDING a
            // frame-local `string` (which is `dyn_is_handle` but slab-stored), reads from
            // the frame slot. Getting this split wrong sent frame-local strings to an
            // empty `dyn_heap` entry and `s[1]` read 0 instead of 'B'.
            if self
                .dyn_is_handle
                .get(net as usize)
                .copied()
                .unwrap_or(false)
                && self.ir.nets.get(net as usize).map(|n| n.kind) != Some(NetKind::String)
            {
                return None;
            }
            let (fidx, slot) = self.frame_route[net as usize].expect("frame-local net is routed");
            let auto = self.frame_slot_auto[net as usize];
            return Some(self.frame_slot_byte_at(fidx, auto, slot, i).unwrap_or(0));
        }
        if self.ir.nets.get(net as usize).map(|n| n.kind) != Some(NetKind::String) {
            return None;
        }
        let h = self.dyn_heap.borrow();
        let Some(crate::state::DynObj::Str { bytes }) =
            h.get(net as usize).and_then(|o| o.as_ref())
        else {
            return Some(0);
        };
        Some(bytes.get(i).copied().unwrap_or(0))
    }
    fn str_bytes(&self, net: u32) -> Option<Vec<u8>> {
        if self.ir.nets.get(net as usize).map(|n| n.kind) != Some(NetKind::String) {
            return None;
        }
        // A frame-local `string` slot's value lives in the active call window / static
        // slab (via `frame_slot_write`), NOT `dyn_heap[net]` (which stays empty). Read
        // the live slot Value and take its string bytes — mirrors `read_net`'s
        // frame-local branch — so a string method (`.len()`, `.getc()`, `.substr()`, …)
        // on a frame-local string reads the actual value, not an empty string. Without
        // this, `string s; s = $sformatf(...); ... s.len()` returned 0 (round-14 V1
        // method-on-local twin: value-use worked but method-read hit the static net).
        if self.frame_local.get(net as usize).copied().unwrap_or(false) {
            return Some(self.read_net(net, None).to_str_bytes());
        }
        Some(
            match self
                .dyn_heap
                .borrow()
                .get(net as usize)
                .and_then(|o| o.as_ref())
            {
                Some(DynObj::Str { bytes }) => bytes.clone(),
                _ => Vec::new(),
            },
        )
    }
    fn is_assoc_str(&self, net: u32) -> bool {
        self.dyn_is_handle
            .get(net as usize)
            .copied()
            .unwrap_or(false)
            && matches!(
                self.ir.nets.get(net as usize).map(|n| n.kind),
                Some(sim_ir::NetKind::AssocStr)
            )
    }
    fn assoc_str_read(&self, net: u32, key: &Option<Vec<u8>>) -> Value {
        let (w, signed) = self
            .ir
            .nets
            .get(net as usize)
            .map(|nv| (nv.width.max(1), nv.signed))
            .unwrap_or((1, false));
        let Some(k) = key else {
            self.dyn_warn_once_at(net, "assoc key is X/Z (read X)");
            return Value::xs(w, signed);
        };
        match self
            .dyn_heap
            .borrow()
            .get(net as usize)
            .and_then(|o| o.as_ref())
        {
            Some(DynObj::AssocStr { map }) => map.get(k).cloned().unwrap_or_else(|| {
                self.dyn_warn_once_at(net, "assoc key not found (read X)");
                Value::xs(w, signed)
            }),
            _ => {
                self.dyn_warn_once_at(net, "assoc key not found (read X)");
                Value::xs(w, signed)
            }
        }
    }
    fn assoc_str_exists(&self, net: u32, key: &Option<Vec<u8>>) -> Option<bool> {
        if !self.is_assoc_str(net) {
            return None;
        }
        let Some(k) = key else {
            self.dyn_warn_once_at(net, "assoc key is X/Z (exists 0)");
            return Some(false);
        };
        Some(
            match self
                .dyn_heap
                .borrow()
                .get(net as usize)
                .and_then(|o| o.as_ref())
            {
                Some(DynObj::AssocStr { map }) => map.contains_key(k),
                _ => false,
            },
        )
    }
    /// Round-9 FIO: the read-only VALUE half of `Scheduler::k_feof`, for a pure
    /// expression-context `$feof(fd)` (e.g. `while (!$feof(fd))`). MUST stay in
    /// sync with `k_feof` (sched.rs): pre-opened STDOUT/STDERR/STDIN → 0, a
    /// bad/closed fd → −1, else the fd's latched EOF flag (0/1). The only
    /// difference is the missing `bad_fd_warn` — this path is `&self`, and the
    /// direct-rhs `e = $feof(fd)` form (intercepted by `file_read_int_special`)
    /// still emits that warning via `k_feof`.
    fn fd_eof(&self, fd: u32) -> Value {
        if (0x8000_0000..=0x8000_0002).contains(&fd) {
            return Value::from_i128(0, 32, true);
        }
        if fd & 0x8000_0000 == 0 || !self.files.contains_key(&fd) {
            return Value::from_i128(-1, 32, true);
        }
        let eof = self.read_state.get(&fd).map(|s| s.eof).unwrap_or(false);
        Value::from_i128(if eof { 1 } else { 0 }, 32, true)
    }
    fn eval_call(&self, func: u32, args: &[Value]) -> Option<Value> {
        self.run_frame_call(func, args)
    }
    fn resolve_virtual_call(&self, call_eid: u32, static_fid: u32, args: &[Value]) -> u32 {
        // Only a virtual call site (sidecar `vslot = Some`) redirects. CLS-CALL-VEC:
        // O(1) ExprId index; out-of-range (non-class / smaller Vec) ⇒ None.
        let Some(&(Some(vslot), _)) = self
            .class_calls
            .get(call_eid as usize)
            .and_then(|o| o.as_ref())
        else {
            return static_fid;
        };
        // args[0] = the receiver handle's object-id; X / null → static target
        // (the body then null-derefs to X — never a vtable OOB).
        let Some(this_v) = args.first() else {
            return static_fid;
        };
        if this_v.unk.iter().any(|&u| u != 0) {
            return static_fid;
        }
        let id = this_v.val.first().copied().unwrap_or(0) as u32;
        if id == 0 {
            return static_fid;
        }
        let class_id = match self.class_heap.borrow().get(&id) {
            Some(o) => o.class_id,
            None => return static_fid,
        };
        self.class_vtable
            .get(class_id as usize)
            .and_then(|t| t.get(vslot as usize))
            .copied()
            .filter(|&f| f != u32::MAX)
            .unwrap_or(static_fid)
    }
    fn formal_width(&self, func: u32, i: usize) -> Option<(u32, bool)> {
        // The i-th formal is `base_net + i` (port order, [0..n_params)); its
        // NetVar carries the declared (width, signed). Out of range (the call
        // passes MORE actuals than the func has formals) ⇒ None → the eval arm
        // falls back to the actual's self-width.
        let m = self.func_table.get(func as usize)?;
        if (i as u32) >= m.n_params {
            return None;
        }
        let nv = &self.ir.nets[(m.base_net + i as u32) as usize];
        Some((nv.width.max(1), nv.signed))
    }
    fn formal_is_string(&self, func: u32, i: usize) -> bool {
        // A `string` formal lowers to a 1-bit Wire net (a string is a dynamic
        // handle, not a fixed width), so it is NOT distinguishable by net
        // kind/width — the elaborate-side `str_params` bitmask carries the fact.
        if i >= 64 {
            return false;
        }
        self.func_table
            .get(func as usize)
            .map(|m| (m.str_params >> i) & 1 == 1)
            .unwrap_or(false)
    }
    fn read_net(&self, net: u32, word: Option<u32>) -> Value {
        // N7: a class-handle FIELD-select read (`obj.f` / `this.f`, word = field-id)
        // goes to the heap. Checked FIRST so a method's `this` slot — which is also
        // a frame-local net — routes to the field, not the frame window. A bare
        // (word-less) handle read falls through (frame window or flat store = the id).
        if self.class_is_handle[net as usize] {
            if let Some(field) = word {
                return self.class_field_read(net, field);
            }
        }
        // B1 frame-call: a frame-local net reads from the ACTIVE call window
        // (automatic) or the shared static slab — never the flat store. One
        // bitmap load on the hot path (sibling of `dyn_is_handle`, always
        // full-length so it indexes directly). EMPTY func_table ⇒ all-false ⇒
        // byte-identical. The read clones the slot Value and releases the frame
        // borrow at the `return` BEFORE control re-enters any evaluator.
        if self.frame_local[net as usize] {
            // V5: a frame-local DYN-ARRAY (`int loc[]`) — both `frame_local` AND
            // `dyn_is_handle` — keeps its elements in the HEAP (`dyn_heap[net]` = the
            // current activation's array), so `loc[i]`/whole reads route to `dyn_read`.
            // A frame-local STRING (§4.5.167) is `dyn_is_handle` too but is SLAB-stored,
            // so it must NOT take the dyn path — it (and scalars) stay in the frame slot.
            if self.dyn_is_handle[net as usize]
                && self.ir.nets[net as usize].kind != NetKind::String
            {
                return self.dyn_read(net, word);
            }
            let (fidx, slot) = self.frame_route[net as usize].expect("frame-local net is routed");
            debug_assert!(
                word.is_none(),
                "B1 frame-local nets are scalar (no array word)"
            );
            // B4: route by this slot's EFFECTIVE lifetime (window vs static slab).
            return self.frame_slot_read(fidx, self.frame_slot_auto[net as usize], slot);
        }
        // v5 (C)-3b: a dyn HANDLE (module) never reads the flat store — its elements
        // live in the heap. One bitmap load on the hot path.
        if self.dyn_is_handle[net as usize] {
            return self.dyn_read(net, word);
        }
        let slot = &self.nets[net as usize];
        let width = slot.width;
        let w = word.unwrap_or(0);
        // Out-of-range array word reads all-X (spec E-RUN-RANGE) — NOT a clamp to the
        // last element (which would silently return a neighbour's value).
        if w >= slot.array_len {
            self.warn_run_index("array word index", w == crate::eval::WORD_UNKNOWN);
            let mut v = Value::xs(width.max(1), slot.signed);
            v.width = width;
            v.is_real = slot.is_real;
            return v;
        }
        // Read the element DIRECTLY into an inline `Value` — no transient `BitPacked`
        // Vec (the prior `net_word_packed → slice_word → BitPacked → from_packed` path
        // allocated two Vecs per read just to copy them into the inline planes). The
        // word-aligned fast path (every scalar net + 64-aligned array element) copies
        // whole store words; an unaligned base (non-64 array element) falls to bit-serial.
        let base = w * width;
        let n = nwords(width).max(1);
        let mut v = if base % 64 == 0 {
            let wbase = (base / 64) as usize;
            let mut val = Words::zeros(n);
            let mut unk = Words::zeros(n);
            for k in 0..n {
                val[k] = slot.cur.val.get(wbase + k).copied().unwrap_or(0);
                unk[k] = slot.cur.unk.get(wbase + k).copied().unwrap_or(0);
            }
            let m = top_mask(width);
            val[n - 1] &= m;
            unk[n - 1] &= m;
            Value {
                val,
                unk,
                width,
                signed: slot.signed,
                is_real: false,
                is_str: false,
            }
        } else {
            let mut tmp = Value::zeros(width.max(1), slot.signed);
            tmp.width = width;
            for i in 0..width {
                let (vv, uu) = bit_of(&slot.cur, base + i);
                tmp.set_vu(i, vv, uu);
            }
            tmp
        };
        v.is_real = slot.is_real; // flag the read-back as real (val[0] = IEEE bits)
        v
    }
}
