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
}

/// The tier-3 net store: every net of the design, in slot form.
pub struct NetArena {
    pub slots: Vec<Slot>,
    /// The single flat storage buffer. Top-word bits beyond `width` are kept
    /// ZERO as an invariant (writes mask; reads may therefore copy words
    /// verbatim and only re-mask the top word, mirroring the engine's read).
    pub buf: Vec<u64>,
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
        // FRAME-LOCAL: a subroutine's locals are ordinary nets in `ir.nets`, so
        // this storage would happily give them slots — but their VALUES live in
        // the activation's frame window, not in a net slot, and both the read
        // path and the write funnel here are frame-blind. User calls are CORE at
        // S0 (revision 4), so an eligible design CAN carry them; refusing here
        // makes the mitigation structural instead of a comment, and puts it in
        // the same place as the `Real` refusal. S3 owns lifting it.
        if !opts.func_table.is_empty() {
            return Err("frame-local storage: S3 (subroutine frames)");
        }
        let mut slots = Vec::with_capacity(ir.nets.len());
        let mut off: u64 = 0;
        for (n, nv) in ir.nets.iter().enumerate() {
            match nv.kind {
                NetKind::Wire | NetKind::Reg | NetKind::Logic | NetKind::Integer => {}
                NetKind::Real => return Err("real: S2 width class"),
                NetKind::DynArray
                | NetKind::Queue
                | NetKind::Assoc
                | NetKind::AssocStr
                | NetKind::String => return Err("heap kind: outside R1 storage"),
            }
            let words = nwords(nv.width.max(1)).max(1) as u32;
            let elems = nv.array_len.max(1);
            let slot = Slot {
                off: u32::try_from(off).map_err(|_| "arena exceeds u32 words")?,
                words,
                width: nv.width,
                elems,
                signed: nv.signed,
                two_state: opts.two_state_nets.contains(&(n as u32)),
            };
            slots.push(slot);
            off += u64::from(words) * 2 * u64::from(elems);
        }
        let total = usize::try_from(off).map_err(|_| "arena exceeds usize")?;
        let mut arena = NetArena {
            slots,
            buf: vec![0u64; total],
        };
        // t0 init: extract the width-wide element init once, broadcast per element.
        for (n, nv) in ir.nets.iter().enumerate() {
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

    /// The `(val, unk)` plane slices of element `elem` of net `net`.
    #[inline]
    pub fn planes(&self, net: u32, elem: u32) -> (&[u64], &[u64]) {
        let s = self.slots[net as usize];
        debug_assert!(elem < s.elems);
        let base = s.off as usize + (elem as usize) * 2 * s.words as usize;
        (
            &self.buf[base..base + s.words as usize],
            &self.buf[base + s.words as usize..base + 2 * s.words as usize],
        )
    }

    /// Overwrite element `elem` of net `net` (test/mirror helper until the S1c
    /// write funnel lands). Extra input words are ignored, missing ones zero;
    /// the top word is masked to keep the buffer invariant.
    pub fn set_elem(&mut self, net: u32, elem: u32, val: &[u64], unk: &[u64]) {
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
/// parity is by construction; what this impl must get right — and what the
/// mirror differential pins — is exactly the read: element indexing, the
/// out-of-range all-X, and top-word masking, mirroring `SimState::read_net`'s
/// flat arm byte-for-byte at the `Value` level.
///
/// Deliberately NOT here yet: the engine's `warn_run_range` diagnostic on an
/// OOB read (a stderr side effect owned by the run pipeline — S1d must emit it
/// for stdout/stderr byte-parity), and the `read_scalar_words` fast path (an
/// S2 concern; the default `None` keeps every read on the mirrored main path).
/// Every other `NetReader` method keeps its default: in an ELIGIBLE design
/// there are no heap kinds, no class handles, and no frame calls, so the
/// defaults (`None`/X-poison) are unreachable by construction.
impl NetReader for NetArena {
    fn read_net(&self, net: u32, word: Option<u32>) -> Value {
        let s = self.slots[net as usize];
        let w = word.unwrap_or(0);
        // Out-of-range array word reads all-X — NOT a clamp (mirrors the engine:
        // a clamp would silently return a neighbour's value).
        //
        // S2 OBLIGATION (soundness-review F1): the engine's OOB arm also stamps
        // `v.is_real = slot.is_real` (netread.rs). This arm omits it, which is
        // sound TODAY solely because `build` rejects `NetKind::Real` — the
        // differential structurally cannot catch the omission before Real is
        // admitted (a Real design cannot build an arena). Whoever adds the S2
        // real width class must carry an `is_real` slot flag through BOTH the
        // OOB and in-range arms, and add a Real leg to the mirror test.
        if w >= s.elems {
            let mut v = Value::xs(s.width.max(1), s.signed);
            v.width = s.width;
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
            is_real: false,
            is_str: false,
        }
    }
}
