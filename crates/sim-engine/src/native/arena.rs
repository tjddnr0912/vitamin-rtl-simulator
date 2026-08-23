//! S1a (doc-21 §5 S1) — **R1: the tier-3 backend owns net storage.**
//!
//! The existing flat store answers a stack of questions on EVERY access — the
//! `NetSlot` metadata load (`width`/`array_len`/`signed`/`is_real`) plus the
//! routing bitmaps in front of it (`class_is_handle` → `frame_local` →
//! `dyn_is_handle`), each a branch the answer of which never changes after
//! elaboration. doc-18 measured that flattening the LAYOUT alone (leaving the
//! questions) yields 0%; R1's bet is eliminating the questions themselves.
//! An eligible design (S0 gate: no heap kinds, no class, no fork …) makes every
//! answer static: this module resolves them ONCE at build into a dense `Slot`
//! descriptor, and no runtime access asks again.
//!
//! Layout (doc-21 §5 S1 "`(val, unk)` 인접"): one flat `u64` buffer; each net's
//! element `e` owns `2*words` consecutive words — the `val` plane then the
//! `unk` plane adjacent — at `off + e*2*words`. Elements are therefore
//! word-ALIGNED by construction (the flat store packs elements bit-contiguously
//! and pays a bit-serial fallback on unaligned bases; here that path does not
//! exist).
//!
//! S1a scope: storage + t0 init + the read path (`NetReader` impl in this
//! file). Writes beyond the test helper land with S1c (the write funnel), the
//! scheduler with S1d. The R1 stop-judgment — "넷 저장이 폭별로 안 나뉘면 중단"
//! — is answered affirmatively by `build` succeeding: every eligible net gets a
//! compile-time-fixed width/word-count slot.

use sim_ir::{NetKind, SimIr};

use crate::eval::NetReader;
use crate::value::{nwords, top_mask, Value, Words};

/// Is this net's value in `SimState::dyn_heap` rather than in an arena slot?
///
/// ONE spelling, three readers: the `heap` map this store carries, the gate row
/// that refuses a heap chunk inside a concat lvalue, and — through the map —
/// `NativeKernel::is_heap_net`. It is `_`-free on purpose: a new `NetKind` is a
/// format bump AND a forced answer here, because "which store owns it" is the
/// question every route in slice 2 turns on.
pub(crate) fn kind_is_heap(kind: NetKind) -> bool {
    match kind {
        NetKind::DynArray
        | NetKind::Queue
        | NetKind::Assoc
        | NetKind::AssocStr
        | NetKind::String => true,
        NetKind::Wire | NetKind::Reg | NetKind::Logic | NetKind::Integer | NetKind::Real => false,
    }
}

/// One net's slot descriptor, fully resolved at build time. This ONE dense
/// record replaces the per-access `NetSlot` metadata questions and the three
/// routing bitmaps — in an eligible design there is nothing else a net can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// Word index of element 0's `val` plane in `NetArena::buf`.
    pub off: u32,
    /// Words per plane (per element): `nwords(width).max(1)`.
    pub words: u32,
    /// ELEMENT width in bits (the declared packed width).
    pub width: u32,
    /// Element count (`array_len.max(1)`; 1 = scalar).
    pub elems: u32,
    pub signed: bool,
    /// SVPART: a 2-state variable (`bit`/`int`/…) — the write funnel coerces
    /// every X/Z bit to 0 before it lands (IEEE §6.11.3). Resolved at build from
    /// the `two_state_nets` sidecar, so the write path never asks a side table.
    pub two_state: bool,
    /// A6: `NetKind::Real` — the 64 stored bits are an IEEE-754 double, not an
    /// integer. The STORAGE is the ordinary word plane (that is why admitting
    /// reals needed no new store); what the flag carries is that every read must
    /// stamp `Value::is_real` and every write must go through the real↔int
    /// coercion instead of the integer path.
    ///
    /// Seeded exactly as the engine seeds `NetSlot::is_real` — from the net's
    /// KIND, which cannot change during a run.
    pub is_real: bool,
}

/// The tier-3 net store: every net of the design, in slot form.
pub struct NetArena {
    pub slots: Vec<Slot>,
    /// V1 slice 2: per net, "this net's value is NOT in this store".
    ///
    /// A heap-kind net (`string`/`queue`/`dyn_array`/`assoc`) gets a slot like
    /// any other so `slots[net]` keeps meaning net `net`, and that slot is DEAD:
    /// the value lives in `SimState::dyn_heap`, reached through
    /// `NativeKernel::read_net` / `write_routed`. This vector exists so the
    /// arena can SAY SO — every entry point below `debug_assert`s on it, which
    /// turns "audit sixteen call sites that index `slots` by net id" into "run
    /// the suite once with the gate open and read the panics". Named after
    /// §4.5.334's `panic!` probe, and a debug assert for the same reason: the
    /// release path must stay byte-identical.
    pub heap: Vec<bool>,
    /// A3-ii-a: per net, "this net is a subroutine FRAME SLOT".
    ///
    /// The exact twin of `heap` above, for the other store this arena does not
    /// own: a frame-local net's value lives in `SimState`'s activation window or
    /// static slab, reached through `NativeKernel::read_net` / `write_routed`.
    /// Its arena slot exists (so `slots[net]` keeps meaning net `net`) and is
    /// DEAD.
    ///
    /// ⚠️ It has to be HERE, beside `heap`, rather than only on `SimState`,
    /// because the consumer that needed it most does not hold the kernel:
    /// `wprog::compile` resolves a `Signal` to an arena SLOT at compile time, and
    /// a frame-local net passed every shape check it makes. Measured — the first
    /// end-to-end run of a driven frame printed `x` for every formal while the
    /// module net beside it was right, because `k_eval_for_lvalue` took the
    /// specialised path and read the dead slot. `frames_admitted`'s module-body
    /// row is why this was harmless until the walk started executing frame
    /// bodies.
    pub frame: Vec<bool>,
    /// A2-i: per net, "this net is a CLASS HANDLE".
    ///
    /// The third twin, and the one whose slot is only PARTLY dead — which is
    /// what makes it different from the two above and why it could not simply
    /// join them. A class handle's own value IS in this store: it is an ordinary
    /// `Logic`/`Integer` slot holding the object id (`0` = null), written by
    /// `h = new` / `h = null` / a ref copy. What is NOT here is the OBJECT's
    /// fields, which live in `SimState::class_heap` and are reached with the
    /// SAME net id plus a field id in the `word` position.
    ///
    /// So every consumer must ask the question as `class[net] && word.is_some()`
    /// — routing on the bitmap alone would send a bare handle read to the heap,
    /// where there is nothing to read. `SimState::write_chunk` spells the pair
    /// the same way and says so in the same words.
    ///
    /// It is on the arena rather than only on `SimState` for `wprog::compile`'s
    /// sake, exactly as `frame` is: that function resolves a `Signal` to a slot
    /// at compile time and a field select passes every shape check it makes.
    pub class: Vec<bool>,
    /// S1d-2: the dirty/edge channel, driven by the write funnel's two store
    /// points. It lives HERE, as it does on `SimState`, because those points are
    /// inside the funnel — a channel the funnel could not reach would have to be
    /// updated by its callers, and the one caller that forgot would produce
    /// correct values and a frozen design.
    pub ch: crate::native::dirty::DirtyChannel,
    /// The single flat storage buffer. Top-word bits beyond `width` are kept
    /// ZERO as an invariant (writes mask; reads may therefore copy words
    /// verbatim and only re-mask the top word, mirroring the engine's read).
    pub buf: Vec<u64>,
    /// Out-of-range array-word accesses that have not been REPORTED yet.
    ///
    /// The engine calls `SimState::warn_run_range` at the access itself; this
    /// store cannot, because the read path is `&self` through `NetReader` and
    /// the diagnostic sink lives on the scheduler, which the kernel owning this
    /// arena borrows mutably (the reverse edge would be a cycle). So the access
    /// COUNTS and the run loop reports, at the FIVE seams where `now` is
    /// unchanged since the access: every statement boundary, after a body,
    /// after the NBA apply, in each cont-assign settle pass, and at the end of
    /// `propagate`. (It was two when this note was written; each later slice
    /// that made the arena readable from a new place added one, and `drain_vcd`
    /// now rides all five.)
    ///
    /// ⚠️ This is a real correctness surface, not bookkeeping. `warn_run_range`
    /// emits a `Severity::Error` diagnostic, which the CLI's own sink counts into
    /// the process exit code — NOT via `had_error`, which only the `$error` family
    /// sets. Without it a design whose write pointer walks past a memory
    /// runs `--backend native` to a PASS verdict and the default backend to a
    /// FAIL — measured on an ordinary FIFO, by both adversarial reviews of
    /// S1d-4c-2c independently.
    ///
    /// The 8-per-run cap and its "further suppressed" note are NOT duplicated
    /// here: draining calls the engine's own function, so the cap counter is the
    /// engine's one and the messages are byte-identical by construction.
    /// Deferred range diagnostics, IN SOURCE ORDER — one entry per out-of-range
    /// access, `true` when the index was UNKNOWN (x/z).
    ///
    /// ⚠️ This was two counters, one per kind. Counters cannot carry ORDER, and the
    /// split into E4002/W4029 made order observable: `$display("%h %h", mem[xi],
    /// mem[oi])` with `xi` unknown and `oi` a known out-of-range emitted W4029 then
    /// E4002 on interp and bytecode, and the reverse on native. The drain is not
    /// decoration — it is the order — so the record has to be ordered too.
    pub pending_range: std::cell::RefCell<Vec<(u32, bool)>>,
}

impl NetArena {
    /// Build the arena for an ELIGIBLE design: slot layout + t0 initial values
    /// (the same broadcast rule as the engine's `expand_init` — elaborate emits
    /// ONE width-wide init plane; an array replicates it per element).
    ///
    /// `opts` supplies the per-net STATIC properties the write funnel would
    /// otherwise have to ask a side table for on every write (today:
    /// `two_state_nets`) — resolving them into the slot is the same R1 move as
    /// the geometry.
    ///
    /// `Err(reason)` names the first net kind this storage cannot own. The S0
    /// design gate already rejects designs carrying heap kinds, so the heap
    /// arms are defense in depth; `Real` is a genuine narrowing (an f64 slot is
    /// an S2 width class — recorded, and the S1d wiring must fold this into the
    /// runtime gate so eligibility-set ≡ executor-set).
    pub fn build(ir: &SimIr, opts: &crate::SimOpts) -> Result<NetArena, &'static str> {
        Self::buildable(ir, opts)?;
        let mut slots = Vec::with_capacity(ir.nets.len());
        let mut off: u64 = 0;
        for (n, nv) in ir.nets.iter().enumerate() {
            let words = nwords(nv.width.max(1)).max(1) as u32;
            let elems = nv.array_len.max(1);
            let slot = Slot {
                off: u32::try_from(off).map_err(|_| "arena exceeds u32 words")?,
                words,
                width: nv.width,
                elems,
                signed: nv.signed,
                two_state: opts.two_state_nets.contains(&(n as u32)),
                is_real: nv.kind == NetKind::Real,
            };
            slots.push(slot);
            off += u64::from(words) * 2 * u64::from(elems);
        }
        let total = usize::try_from(off).map_err(|_| "arena exceeds usize")?;
        let mut arena = NetArena {
            slots,
            heap: ir.nets.iter().map(|nv| kind_is_heap(nv.kind)).collect(),
            // Every net inside some func's window `[base, base+len)`. Built from
            // the SAME sidecar `SimState::build_func_routing` uses, so the two
            // tables cannot disagree about which nets are frame slots.
            frame: {
                let mut f = vec![false; ir.nets.len()];
                for m in opts.func_table.iter() {
                    let lo = m.base_net as usize;
                    let hi = lo.saturating_add(m.locals_len as usize).min(f.len());
                    for slot in f.iter_mut().take(hi).skip(lo) {
                        *slot = true;
                    }
                }
                f
            },
            // A2-i: from the SAME sidecar `simulate` fills `SimState::
            // class_is_handle` from, for the same reason `frame` shares
            // `func_table` — two tables built from two sources cannot be shown
            // to agree about which nets are handles.
            class: {
                let mut c = vec![false; ir.nets.len()];
                for &n in opts.class_handle_nets.iter() {
                    if let Some(slot) = c.get_mut(n as usize) {
                        *slot = true;
                    }
                }
                c
            },
            ch: {
                let mut ch = crate::native::dirty::DirtyChannel::new(ir);
                // G2 PROBE: the per-net mirror of `--probe`. Installed HERE
                // rather than looked up per store, exactly as `two_state` is
                // resolved into the slot: `probed_nets` is a config vector and
                // the funnel must not ask a side table on every write.
                if !opts.probed_nets.is_empty() {
                    let mut v = vec![false; ir.nets.len()];
                    for &n in opts.probed_nets.iter() {
                        if let Some(slot) = v.get_mut(n as usize) {
                            *slot = true;
                        }
                    }
                    ch.probed = v;
                }
                ch
            },
            buf: vec![0u64; total],
            pending_range: std::cell::RefCell::new(Vec::new()),
        };
        // t0 init: extract the width-wide element init once, broadcast per element.
        for (n, nv) in ir.nets.iter().enumerate() {
            // V1 slice 2: a heap-kind net's slot is DEAD, and its declared init
            // is not for that slot — a `string`'s init is the packed literal,
            // whose bits run far above the element width the slot was sized to.
            // Its real t0 value is the heap's (IEEE §7.5.2: "" / empty), which
            // `SimState` establishes. Initialising the dead slot from an init
            // that does not describe it is how this loop's own invariant
            // assertion first fired.
            if arena.heap[n] {
                continue;
            }
            let s = arena.slots[n];
            // The arena keeps bits above `width` ZERO; the engine's scalar init
            // path word-RESIZES without masking, so if elaborate ever emitted an
            // init with junk above the width the two stores would agree on every
            // READ (both mask) and disagree ONCE on a whole-net write's `changed`
            // verdict — an order-dependent, single-shot divergence. `default_init`
            // sets bits only in `0..width` today; this pins that.
            debug_assert!(
                (s.width..64 * s.words).all(|i| {
                    let (w, b) = ((i / 64) as usize, i % 64);
                    (nv.init.val.get(w).copied().unwrap_or(0) >> b) & 1 == 0
                        && (nv.init.unk.get(w).copied().unwrap_or(0) >> b) & 1 == 0
                }),
                "net {n}: declared init carries bits above its width"
            );
            let mut vplane = vec![0u64; s.words as usize];
            let mut uplane = vec![0u64; s.words as usize];
            for i in 0..s.width {
                let word = (i / 64) as usize;
                let bit = i % 64;
                let v = (nv.init.val.get(word).copied().unwrap_or(0) >> bit) & 1;
                let u = (nv.init.unk.get(word).copied().unwrap_or(0) >> bit) & 1;
                vplane[(i / 64) as usize] |= v << bit;
                uplane[(i / 64) as usize] |= u << bit;
            }
            if s.width > 0 {
                let m = top_mask(s.width);
                vplane[s.words as usize - 1] &= m;
                uplane[s.words as usize - 1] &= m;
            }
            for e in 0..s.elems {
                let base = s.off as usize + (e as usize) * 2 * s.words as usize;
                arena.buf[base..base + s.words as usize].copy_from_slice(&vplane);
                arena.buf[base + s.words as usize..base + 2 * s.words as usize]
                    .copy_from_slice(&uplane);
            }
        }
        Ok(arena)
    }

    /// Would [`build`](Self::build) succeed? The SAME refusals, WITHOUT
    /// allocating the buffer — so the observability rail can report the
    /// storage-level verdict on every run (run.json `native.buildable`) at the
    /// cost of one scan, and `build` calls it first so the two answers are one
    /// predicate rather than two that can drift.
    pub fn buildable(ir: &SimIr, opts: &crate::SimOpts) -> Result<(), &'static str> {
        // FRAME-LOCAL: a subroutine's locals are ordinary nets in `ir.nets`, so
        // this storage would happily give them slots — but their VALUES live in
        // the activation's frame window, not in a net slot, and both the read
        // path and the write funnel are frame-blind. User calls are CORE at S0
        // (revision 4), so an eligible design CAN carry them.
        //
        // S3a lifted the blanket refusal to the SUBSET whose frames never need
        // the module store (`native::frames`), which is what makes delegating
        // `eval_call` to the engine's frame executor byte-identical rather than
        // merely plausible. Everything outside that subset still refuses here,
        // in its own words.
        crate::native::frames::frames_admitted(ir, opts)?;
        let mut off: u64 = 0;
        for nv in &ir.nets {
            match nv.kind {
                NetKind::Wire | NetKind::Reg | NetKind::Logic | NetKind::Integer => {}
                // A6: admitted. It was never a width class — the 64 bits of an
                // f64 fit the ordinary word plane, and the engine stores them
                // there too. What was missing was the FLAG (`Slot::is_real`,
                // stamped onto every read) and the other two arms of the real↔int
                // assignment coercion, now shared as `value::coerce_assign`.
                NetKind::Real => {}
                // V1 slice 2a: admitted. Its slot is dead (see `NetArena::heap`).
                NetKind::DynArray => {}
                // V1 slice 2b: `string` joins it — same routing, and its own
                // shapes (whole-handle assign strips leading NULs, a whole read
                // materializes 8xlen) live in `dyn_read`/`dyn_write`, which the
                // routes above reach.
                NetKind::String => {}
                // V1 slice 2c. Its OPERATIONS ride along: both queue-op tables
                // live in `SimState` and are read by code tier-3 shares.
                NetKind::Queue => {}
                // V1 slice 2d — the last heap kind, and the only one whose
                // WRITE needed a new arm rather than the shared `dyn_write`: an
                // assoc key is an i64 (or a byte string) and cannot ride the
                // `(offset, word)` u32 pairs, so it travels out of band in
                // `Offsets::AssocKey`/`AssocStrKey` and `write_routed` splits on
                // it exactly where `SimState::write_lvalue` does.
                NetKind::Assoc | NetKind::AssocStr => {}
            }
            let words = nwords(nv.width.max(1)).max(1) as u64;
            off += words * 2 * u64::from(nv.array_len.max(1));
            if u32::try_from(off).is_err() {
                return Err("arena exceeds u32 words");
            }
        }
        if usize::try_from(off).is_err() {
            return Err("arena exceeds usize");
        }
        // ── V1 slice 2d: a heap chunk inside a CONCAT lvalue ──
        //
        // ⚠️ Found by the whole-suite backend flip, not by the corpus — and it
        // had been wrong since slice 2a. `NativeKernel::write_routed` routes on
        // `if let [c] = lhs.chunks.as_slice()`, i.e. only when the WHOLE lvalue
        // is one chunk; the engine instead routes PER CHUNK, inside
        // `write_chunk`, whose first question is `dyn_is_handle[net]`. So
        // `{d[0], x} = 8'hAB` sent both chunks to the arena, and the heap one
        // reached `assert_owns` — the instrument named it exactly.
        //
        // ⭐ A8-concat ROUTED it, and it built the follow-on this row named
        // rather than the second spelling it warned about: `write_lvalue` grew a
        // per-chunk `Escape`, so the SPLIT still happens once, here, and the
        // router only says which pieces are not this store's.
        //
        // The old row read: "Refused rather than routed, deliberately. Routing
        // it means splitting the source across chunks and sending each half to a
        // different store, and that split rule already lives in
        // `NetArena::write_lvalue`; a second spelling of it in the router is the
        // §4.5.279 class of defect."
        Ok(())
    }

    /// Count one out-of-range element access, for the run loop to report.
    ///
    /// A FUNCTION rather than the `Cell` bump inline, because it is the whole
    /// diagnostic: `warn_run_range` emits a `Severity::Error` diagnostic, which
    /// the CLI's own sink counts into the process exit code — NOT via
    /// `had_error`, which only the `$error` family sets (`run_tests.rs` records
    /// that a gate was vacuous for exactly that confusion). Any second reader
    /// that resolves its
    /// own element index has to land here — `wprog`'s runtime element load is
    /// that second reader, and a forgotten increment there would turn a design
    /// whose index walks past a memory from FAIL into PASS while every value
    /// stayed identical. The WRITE funnel is the third caller, routed here in the
    /// same slice: it used to bump the cell inline, which is how the refactor
    /// managed to leave its own motivating example un-routed.
    #[inline]
    /// Record one out-of-range access and WHICH KIND it was, in order.
    /// "This store does not own net `net`" — the arena's own statement of the
    /// routing contract, checked at every entry point.
    pub(crate) fn assert_owns(&self, net: u32, site: &str) {
        debug_assert!(
            !self.heap.get(net as usize).copied().unwrap_or(false),
            "net {net} is a heap kind and its value is not in this store, but \
             `{site}` reached the arena with it. Some path indexes `slots` by \
             net id without asking `NativeKernel::is_heap_net` first — that is \
             the bypass, and this assert is how it names itself."
        );
    }

    pub(crate) fn note_bad_index(&self, net: u32, unknown: bool) {
        self.pending_range.borrow_mut().push((net, unknown));
    }

    /// The net whose element block starts at buffer offset `off`, or `u32::MAX`.
    ///
    /// ⚠️ A LINEAR SCAN, and deliberately so: the only caller is the
    /// out-of-range arm of `WOp::LoadIdx`, which is (a) already the cold path
    /// and (b) capped at eight reports per run. The alternative was a `net`
    /// field on the op — but `WOp`'s largest payload is 16 bytes, so a fourth
    /// word would grow EVERY op in the array from 24 to 32 bytes, in the lane
    /// §5.1-ay measured at ~20% of the run, to name an array in a message. The
    /// index is compile-time-unique per net (`off` is a slot's base), so the
    /// scan is exact, not a guess.
    pub(crate) fn net_at_off(&self, off: u32) -> u32 {
        self.slots
            .iter()
            .position(|s| s.off == off)
            .map_or(u32::MAX, |i| i as u32)
    }

    /// The `(val, unk)` plane slices of element `elem` of net `net`.
    #[inline]
    pub fn planes(&self, net: u32, elem: u32) -> (&[u64], &[u64]) {
        self.assert_owns(net, "NetArena::planes");
        let s = self.slots[net as usize];
        debug_assert!(elem < s.elems);
        let base = s.off as usize + (elem as usize) * 2 * s.words as usize;
        (
            &self.buf[base..base + s.words as usize],
            &self.buf[base + s.words as usize..base + 2 * s.words as usize],
        )
    }

    /// Overwrite element `elem` of net `net` — a TEST/mirror helper that
    /// deliberately BYPASSES the dirty/edge channel, so a state planted with it
    /// is invisible to `take_changed` (which is what a differential wants: the
    /// mirror is setup, not a simulated write). Never use it for a write the
    /// scheduler should observe — `write_lvalue` is the funnel that reports.
    /// `pub(crate)`: the S1c write funnel landed, so nothing outside this crate
    /// has a reason to plant state.
    ///
    /// Extra input words are ignored, missing ones zero; the top word is masked
    /// to keep the buffer invariant.
    #[cfg(test)]
    pub(crate) fn set_elem(&mut self, net: u32, elem: u32, val: &[u64], unk: &[u64]) {
        self.assert_owns(net, "NetArena::set_elem");
        let s = self.slots[net as usize];
        debug_assert!(elem < s.elems);
        let m = top_mask(s.width.max(1));
        let base = s.off as usize + (elem as usize) * 2 * s.words as usize;
        for k in 0..s.words as usize {
            let mask = if k + 1 == s.words as usize {
                m
            } else {
                u64::MAX
            };
            self.buf[base + k] = val.get(k).copied().unwrap_or(0) & mask;
            self.buf[base + s.words as usize + k] = unk.get(k).copied().unwrap_or(0) & mask;
        }
    }
}

/// S1b: the arena read path under the EXISTING evaluator. Evaluation semantics
/// are shared with the engine (`EvalCtx` is generic over `NetReader`), so eval
/// parity on THIS path is by construction; what this impl must get right — and
/// what the mirror differential pins — is exactly the read: element indexing,
/// the out-of-range all-X, and top-word masking, mirroring
/// `SimState::read_net`'s flat arm byte-for-byte at the `Value` level.
///
/// Since S2, admitted expression trees bypass this impl entirely: `wprog`
/// loads the two plane words directly. Until S2 slice 4 its admission (in-bounds
/// CONSTANT indices only) also kept the OOB machinery below out of its reach;
/// that slice admitted a RUNTIME index, so `wprog`'s `LoadIdx` reaches the same
/// decision itself — all-X plus `note_bad_index`, the ordered record this arm
/// appends to.
/// Parity is measured (exhaustive battery + pinned corpus sweep), not structural.
///
/// The engine's `warn_run_range` diagnostic on an OOB read is recorded here
/// (`pending_range`) and emitted by the run loop — see that field's doc for why
/// it cannot be emitted at the access.
/// Every other `NetReader` method keeps its default: in an ELIGIBLE design
/// there are no heap kinds and no class handles, so the defaults
/// (`None`/X-poison) are unreachable by construction. **`eval_call` is the
/// exception since S3a** — an eligible design CAN carry subroutine calls, the
/// composite reader on `NativeKernel` is what answers them, and this impl makes
/// reaching the bare store loud rather than defaulting to X.
/// ⚠️ ONE defaulted `NetReader` method is NOT covered by the "no heap kinds, no
/// class handles, no frame calls" argument below: `fd_eof`. It is closed only by
/// `$feof` being OVER-marked as a statement effect (§4.5.291, ROADMAP §5.1) —
/// so the day that over-mark is corrected, `$display("%0d", $feof(fd))` becomes
/// eligible and this reader returns X where the engine returns the live flag.
/// Whoever fixes the over-mark owes this an override.
impl NetReader for NetArena {
    /// The LEAF FAST PATH — two word reads and the shared resize rule.
    ///
    /// ⚠️⚠️ This store had NO fast path until the D4 pre-census, and the reason
    /// it was acceptable stopped being true one slice earlier. `native_eval` is
    /// the only caller, and until D1.5 tier-3 never handed it anything (its
    /// `CompileCtx` refused the expression VM outright); D1.5/D1.6 then routed
    /// every RHS `wprog` declines here, and a profile measured that path at
    /// **19.3% of `struct-heavy`** with `Value::{resize,mask_top}` a further
    /// 10.2% — the 72-byte marshalling §4.5.329–332 removed from `wprog`, still
    /// fully present on the lane that had just become hot (ROADMAP §5.1-az).
    ///
    /// The declines mirror `read_net`'s own routing rather than restating it in
    /// new words: an array (`elems != 1`), a wide or zero-width net, and a
    /// `real` — whose stored bits are an IEEE-754 double and must be read as a
    /// stamped `Value`, not as an integer. The two nets this store does not own
    /// (frame-local, heap) never reach here: the composite reader in
    /// `NativeKernel` answers `read_scalar_words` and asks that question first.
    /// ⚠️ The `is_real` decline is MEASURED UNREACHABLE and kept anyway. A
    /// `panic!` probe there fired in no product path across the whole suite,
    /// because `native_eval::compile`'s NATIVE-TYPE GUARD refuses a real net one
    /// layer up — for the same reason spelled out there, that a `(u64, u64)`
    /// pair structurally cannot carry `is_real`, and with the same measured
    /// consequence (`assign w = r;` printing the raw IEEE pattern
    /// 4613374868287651840 instead of 3).
    ///
    /// ⚠️⚠️ A mutation deleting it therefore SURVIVED, and the fast-path lock
    /// cannot kill it either: that test compares WORDS, and for a real the words
    /// are identical — what differs is the `is_real` stamp this return type has
    /// no room for. So the guard stays as the store-level half of a rule whose
    /// other half already exists, not because a test demands it.
    fn read_scalar_words(&self, net: u32, w: u32, ctx_signed: bool) -> Option<(u64, u64)> {
        let s = *self.slots.get(net as usize)?;
        if s.is_real || s.elems != 1 || s.words != 1 || s.width == 0 || s.width > 64 || w > 64 {
            return None;
        }
        let i = s.off as usize;
        Some(crate::value::scalar_words_resized(
            self.buf[i],
            self.buf[i + 1],
            s.width,
            s.signed,
            w,
            ctx_signed,
        ))
    }

    fn take_deferred_range_kinds(&self) -> Vec<(u32, bool)> {
        std::mem::take(&mut *self.pending_range.borrow_mut())
    }

    /// S3a — **LOUD, not the `None` default.** A subroutine call is answered by
    /// the tier-3 KERNEL's composite reader, which routes it to the engine's
    /// frame executor; the bare arena reaching a call means an evaluation seam
    /// was handed this store instead of that composite, and the `None` default
    /// would answer it with X: a wrong value at exit 0, in a design the gate
    /// reports fully runnable.
    ///
    /// `native::frames::frames_admitted` refuses the two module positions whose
    /// reader is this store (a system-task argument and a delayed continuous
    /// assign). This panic is what makes a seam that enumeration MISSED loud
    /// instead of silent — the same bargain the kernel's `gate_refused!` arms
    /// make, and the reason the S3a gate can be trusted to be complete rather
    /// than merely careful.
    fn eval_call(&self, func: u32, _args: &[Value]) -> Option<Value> {
        panic!(
            "tier-3 arena: subroutine call (func {func}) evaluated through the bare net \
             store — the frame executor is reached through `NativeKernel`'s composite \
             reader, so this is an evaluation seam that was handed the arena alone. \
             Route it through the kernel, or add a `native::frames` row refusing it."
        )
    }

    /// V1 slice 2: this store owns flat slots only, so a heap-kind net must be
    /// routed to `SimState` before it reaches here — which is what
    /// `assert_owns` below checks, and what this asks for.
    fn routes_heap_to_state(&self) -> bool {
        true
    }

    fn read_net(&self, net: u32, word: Option<u32>) -> Value {
        self.assert_owns(net, "NetArena::read_net");
        let s = self.slots[net as usize];
        let w = word.unwrap_or(0);
        // Out-of-range array word reads all-X — NOT a clamp (mirrors the engine:
        // a clamp would silently return a neighbour's value).
        //
        // ✅ S2 OBLIGATION (soundness-review F1) DISCHARGED by A6. The note here
        // said the OOB arm omitted the engine's `v.is_real = slot.is_real`
        // (netread.rs) and was sound only while `build` rejected `NetKind::Real`
        // — which is exactly the row A6 removed, so both arms stamp it now.
        //
        // ⚠️ The obligation was right and the FIRST reason written for it here
        // was wrong, so it is recorded as measured. Rendering does NOT
        // discriminate: `$display("%f", m[9])` prints `0.000000` either way,
        // because an all-X real reads back bits 0 and an all-X integer converts
        // to 0.0. What discriminates is `truthiness` — a real is "nonzero?",
        // an integer is "any X ⇒ UNKNOWN" — so `m[9] ? 1 : 0` answers `0` with
        // the stamp and `X` without it. Dropping this one line was run as a
        // mutation: every other real design in the suite stayed green and that
        // ternary is the whole of what caught it (iverilog agrees: `0`).
        if w >= s.elems {
            self.note_bad_index(net, w == crate::eval::WORD_UNKNOWN);
            let mut v = Value::xs(s.width.max(1), s.signed);
            v.width = s.width;
            v.is_real = s.is_real;
            // ⚠️ `xs` masked to `width.max(1)`, so a ZERO-width slot leaves one `unk`
            // bit above the stamped width — the shape §4.5.368's `resize` assert now
            // catches. No language surface reaches it today (a zero-width array net
            // is not constructible: `[P-1:0]` with `P = 0` sizes to 2 since §4.5.350),
            // which is why it is a side condition rather than a bug — but the
            // invariant must not depend on one, so re-canonicalise here.
            v.mask_top();
            return v;
        }
        let n = s.words as usize;
        let (vp, up) = self.planes(net, w);
        let mut val = Words::zeros(n);
        let mut unk = Words::zeros(n);
        for k in 0..n {
            val[k] = vp[k];
            unk[k] = up[k];
        }
        let m = top_mask(s.width);
        val[n - 1] &= m;
        unk[n - 1] &= m;
        Value {
            val,
            unk,
            width: s.width,
            signed: s.signed,
            // A6: the in-range half of the discharged obligation above. The bits
            // are already the f64; this flag is what stops every consumer of the
            // read (`truthiness`, the arithmetic, the `%f` formatter) from
            // treating them as an integer.
            is_real: s.is_real,
            is_str: false,
        }
    }
}
