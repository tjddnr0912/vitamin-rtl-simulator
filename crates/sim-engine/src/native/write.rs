//! S1c (doc-21 §5 S1 분해) — **the arena write funnel.**
//!
//! The Value-level mirror of the engine's `write_lvalue_general` → `write_chunk`
//! → `store_words` chain, for the shapes an ELIGIBLE design can produce. What
//! the engine spends branches deciding on every write, an eligible design has
//! already decided:
//!
//! | engine branch | here |
//! |---|---|
//! | real↔int assignment coercion | HALF gone. A real DESTINATION is impossible (`NetKind::Real` cannot build an arena), but a real VALUE is not — `x = $itor(n)/2.0` needs no real net anywhere — so the real→int ROUND arm is mirrored below. Answering only the destination half of this question is what the S1c differential caught |
//! | frame-local write lane | gone, but by REFUSAL, not by argument: user calls are CORE at S0 (rev-4), so an eligible design CAN have frame-local nets — they would get ordinary arena slots and a write would land in the slot instead of the activation's frame window. `NetArena::build` therefore refuses a non-empty `func_table` outright (the READ path is frame-blind too, so this is not a write-only hole). S3 lifts it |
//! | assoc / assoc-str key lanes | gone — heap kinds are rejected by the S0 gate |
//! | dyn-handle / class-field lanes | gone — same |
//! | string / string-element whole-value lane | gone — same |
//! | `forced` (force/release) | gone — `force`/`release` are REJECTED by the design gate (see `native::design_eligibility`), because v1 has no force machinery. Honoring the flag without the machinery would be a half-built feature that reads as support |
//! | 2-state X/Z→0 coercion | KEPT — `two_state_nets` is a CORE sidecar (S0), so it is this funnel's job |
//! | aligned-vs-bit-serial store | KEPT, but the alignment TEST is gone: arena elements are word-aligned by construction |
//!
//! What survives is the geometry, and it is mirrored exactly: chunk widths,
//! MSB-first source consumption across a concat LHS, the signed low-bit
//! position (so `v[-2+:4]` partial-writes and drops its underflowing bits), the
//! out-of-range array-word DROP (never a clamp), and word/bit-granular change
//! detection.
//!
//! ## Not here (S1d obligations, stated so they are not forgotten)
//!
//! - ~~The dirty/edge channel~~ **landed (S1d-2)**: `note_change` +
//!   `accumulate_edge` now hang off the same two store points, so a scheduler
//!   reading `NetArena::take_changed` gets the glitch-accurate intra-slot edge
//!   mask and not merely `changed` (see `native/dirty.rs`).
//! - ~~**`warn_run_range`**~~ — DISCHARGED by S1d-4c-2c. The count lives on
//!   `NetArena::pending_range` and the run loop reports it through the engine's
//!   own emitter. It was mis-sized while it sat here: the note said "values
//!   match without it; stderr does not", but the diagnostic is `Severity::Error`
//!   and therefore the run's EXIT CLASS.
//! - **Wired-AND/OR resolution** (`wired_and_nets`/`wired_or_nets`, CORE at S0)
//!   is a multi-driver SETTLE rule, not a write-funnel rule — S1d owns it.
//! - ~~Effects that never pass through this funnel at all~~ **now REFUSED at the
//!   gate** (`stmt_effect`, added when the `Kernel` trait made the question
//!   structural): the `sim_ir::rhs_is_stmt_effect` family (a seeded
//!   `$random`/`$dist_*` writes its seed back, `$cast(dst,src)`,
//!   the file family) and `sim_ir::systask_writes_net`
//!   (`$readmem*`, `$sformat`, the `$cast` TASK form, the string/heap mutators).
//!   `r = $random(seed)` is NO LONGER eligible. Plumbing a member is what lifts
//!   its reject — `$value$plusargs` was the first (S1d-5, the shared
//!   `exec::plusargs::effect` with a per-store write); until the rest are
//!   plumbed the refusal is what keeps an executor from repeating every draw
//!   in silence.

use sim_ir::{LvalChunk, Lvalue, SelKind, SimIr};

use crate::exec::Offsets;
use crate::native::arena::NetArena;
use crate::value::{top_mask, Value};

// No PRODUCTION caller yet: the S1d executor is what will drive this funnel, and
// until it exists only the S1c differential (`native/tests.rs`) does. Saying that
// with `allow(dead_code)` is more honest than widening the visibility to silence
// it — the surface stays `pub(crate)`, which is all any caller will ever need
// (the `Offsets` it takes is crate-private by construction).
#[allow(dead_code)]
impl NetArena {
    /// Write `value` into the LHS chunks of `lhs`, MSB-first source consumption
    /// (Verilog concat-LHS). Returns true if ANY stored bit changed.
    ///
    /// `offsets[i]` is the already-EVALUATED bit offset / array word of
    /// `lhs.chunks[i]`, resolved by the SHARED `eval::resolve_offsets` at the
    /// caller's sampling moment — the same function the engine binds, so an X/Z
    /// index cannot be dropped by one side and written by the other.
    ///
    /// `ir` supplies the const-expr arena a part-select width folds through
    /// (`c.width` is an ExprId, not a literal). Folding it per write mirrors the
    /// engine exactly; hoisting the fold to build time is a legitimate S2/S3
    /// move, but only once a profile asks for it (doc-18's rule).
    pub(crate) fn write_lvalue(
        &mut self,
        ir: &SimIr,
        lhs: &Lvalue,
        value: Value,
        offsets: &Offsets,
    ) -> bool {
        self.write_lvalue_inner(ir, lhs, value, offsets, true)
    }

    /// The general funnel with the one-word entry TURNED OFF — the differential's
    /// other side, and the reason it is not vacuous.
    ///
    /// `word_entry` is a literal at both call sites, so the production one folds
    /// the branch away; the alternative (comparing the delegating funnel against
    /// itself) is a test that cannot fail.
    #[cfg(test)]
    pub(crate) fn write_lvalue_general_for_test(
        &mut self,
        ir: &SimIr,
        lhs: &Lvalue,
        value: Value,
        offsets: &Offsets,
    ) -> bool {
        self.write_lvalue_inner(ir, lhs, value, offsets, false)
    }

    #[inline]
    fn write_lvalue_inner(
        &mut self,
        ir: &SimIr,
        lhs: &Lvalue,
        value: Value,
        offsets: &Offsets,
        word_entry: bool,
    ) -> bool {
        for c in &lhs.chunks {
            self.assert_owns(c.net, "NetArena::write_lvalue");
        }
        // The assoc key variants carry their key OUT of band and `as_slice()`
        // yields `&[]` for them — a shape this funnel has no arm for. Heap kinds
        // are S0-rejected so they cannot arrive, but resting on a `NetReader`
        // DEFAULT (`is_assoc` = false) for that is thinner than saying it here.
        if matches!(offsets, Offsets::AssocKey(_) | Offsets::AssocStrKey(_)) {
            debug_assert!(false, "assoc lvalue reached the arena funnel");
            return false;
        }
        let offsets = offsets.as_slice();
        debug_assert_eq!(
            offsets.len(),
            lhs.chunks.len(),
            "one (offset,word) per chunk"
        );
        // ── real→int assignment coercion (IEEE 1364 §6.2) ──
        //
        // The DESTINATION can never be real here (a `NetKind::Real` net cannot
        // build an arena), but the VALUE can: `x = $itor(n)/2.0` needs no real
        // NET anywhere, and neither does `$bitstoreal(…)` or a real literal that
        // survives to runtime. Both engine arms with a real destination are
        // therefore unreachable; this one is not, and omitting it stored the raw
        // IEEE-754 bits where the engine rounds — a silent wrong value in a
        // design the gate calls eligible. (Found by the S1c differential, which
        // is why the shape is now one of its pinned designs.)
        //
        // Verbatim from the engine, including the detail that a single-chunk LHS
        // rounds to the NET's width and signedness — NOT the chunk's, so
        // `x[3] = 1.5` rounds at 32 bits and the resize below takes bit 0.
        let value = if value.is_real {
            // `chunks[0]` is indexed only INSIDE the single-chunk arm — the
            // engine is empty-safe by construction (a guarded `if` plus a
            // short-circuiting `&&`), and a mirror that panics where the
            // original returns is not a mirror.
            let w = if let [c0] = lhs.chunks.as_slice() {
                self.slots[c0.net as usize].width
            } else {
                lhs.chunks.iter().map(|c| self.chunk_width(ir, c)).sum()
            };
            let signed =
                matches!(lhs.chunks.as_slice(), [c0] if self.slots[c0.net as usize].signed);
            crate::value::real_to_int_round(value.to_f64().unwrap_or(0.0), w.max(1), signed)
        } else {
            value
        };
        // Total destination bit width = sum of chunk widths.
        let total: u32 = lhs.chunks.iter().map(|c| self.chunk_width(ir, c)).sum();

        // ── ONE-WORD ENTRY (§4.5.332) ──
        //
        // The canonical rule, entered a level lower. Everything above this point
        // has already run — the real→int coercion and the destination width — so
        // what is delegated is the tail: resize the source to `total`, then store
        // it. The measured shape of a real design is that this tail is nearly all
        // of it: on picorv32, 4.01 M of 4.06 M chunk writes are a whole net, 99.1%
        // of them ≤ 64 bits, and the funnel plus its `resize` was 18.9% of the run.
        //
        // Admission is one question — does the DESTINATION fit a single arena word
        // — and the source follows from it: `total <= 64` makes `resize_word`'s
        // extension branch unreachable for a source wider than a word, so only the
        // low word can matter, exactly as the general `resize` copies only
        // `nwords(min(from,to))` words. `is_real` is already false here (the
        // coercion above is what makes it so), and `is_str` is irrelevant once the
        // value is bits — `resize` drops the flag rather than reading it.
        if let [c0] = lhs.chunks.as_slice() {
            let s = self.slots[c0.net as usize];
            if word_entry && s.words == 1 && total <= 64 {
                let (raw_off, raw_word) = offsets.first().copied().unwrap_or((0, 0));
                let (pv, pu) = crate::value::resize_word(
                    value.val.first().copied().unwrap_or(0),
                    value.unk.first().copied().unwrap_or(0),
                    value.width,
                    total.max(1),
                    value.signed,
                );
                return self.write_chunk_word(c0, raw_off, raw_word, pv, pu, total);
            }
        }

        let src = value.resize(total.max(1));

        // Single-chunk LHS — the dominant case (a whole net, or one bit-/part-
        // select). With one chunk the sliced "piece" IS `src` exactly.
        if lhs.chunks.len() == 1 {
            let (raw_off, raw_word) = offsets.first().copied().unwrap_or((0, 0));
            return self.write_chunk(ir, &lhs.chunks[0], raw_off, raw_word, &src);
        }

        let mut changed = false;
        let mut src_hi = total; // next source bit (exclusive top)
        for (idx, chunk) in lhs.chunks.iter().enumerate() {
            let cw = self.chunk_width(ir, chunk);
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
            changed |= self.write_chunk(ir, chunk, raw_off, raw_word, &piece);
        }
        changed
    }

    /// Total destination bit-width of an lvalue (Σ chunk widths) — the engine's
    /// `lvalue_width`, which seeds the RHS context width.
    pub(crate) fn lvalue_width(&self, ir: &SimIr, lhs: &Lvalue) -> u32 {
        // `.max(1)` exactly as the engine clamps it — a 0-width destination must
        // still seed a 1-bit RHS context, or the two sides size the value
        // differently.
        lhs.chunks
            .iter()
            .map(|c| self.chunk_width(ir, c))
            .sum::<u32>()
            .max(1)
    }

    /// The LOW (LSB) net-bit position this chunk writes at, given its already-known
    /// width. SIGNED: an indexed part-select can extend below bit 0 (`v[-2+:4]`,
    /// `v[1-:3]`), and only the in-range bits are stored.
    ///
    /// Extracted so the word funnel below and the general `write_chunk` cannot
    /// disagree about WHICH BITS a chunk names — the `- cw + 1` of the descending
    /// form is one character, and two copies of it silently pick a different window.
    /// `cw` is passed rather than refolded because the caller already holds it
    /// (`write_lvalue` folded it to size the source), and refolding is what would
    /// let the two answers drift apart.
    fn chunk_lsb(&self, c: &LvalChunk, raw_off: u32, cw: u32) -> i64 {
        // `raw_off` arrives as the offset's 32-bit 2's-complement; sign-extend it
        // (bit positions never exceed i32 range).
        let off_i = raw_off as i32 as i64;
        match c.kind {
            SelKind::Bit => {
                if c.offset.is_none() && c.width.is_none() {
                    0 // whole net
                } else {
                    off_i
                }
            }
            SelKind::PartConst | SelKind::PartIdxUp => off_i,
            SelKind::PartIdxDown => off_i - (cw as i64) + 1,
        }
    }

    /// The array element this chunk writes, or `None` when the write must be
    /// DROPPED because the index is out of range (never clamped — clamping would
    /// silently corrupt a valid neighbour). Counting the drop is a side effect, so
    /// this is also the one place that decides an E4002/W4029 is owed.
    fn chunk_elem(&self, c: &LvalChunk, raw_word: u32, elems: u32) -> Option<u32> {
        if c.word.is_none() {
            return Some(0);
        }
        if raw_word >= elems {
            // Counted, not clamped and not silent — the run loop reports it
            // through the engine's own `warn_run_range` (see
            // `NetArena::pending_range`). The engine emits here; dropping it
            // turned an `ExitClass::HadErrors` run into a clean PASS.
            self.note_bad_index(raw_word == crate::eval::OFF_UNKNOWN);
            return None;
        }
        Some(raw_word)
    }

    /// Width (in bits) a single lvalue chunk writes — the engine's `chunk_width`.
    pub(crate) fn chunk_width(&self, ir: &SimIr, c: &LvalChunk) -> u32 {
        let net_w = self.slots[c.net as usize].width;
        match c.kind {
            // whole-net write: offset/width None.
            SelKind::Bit => {
                if c.offset.is_none() && c.width.is_none() {
                    net_w
                } else {
                    1
                }
            }
            SelKind::PartConst | SelKind::PartIdxUp | SelKind::PartIdxDown => {
                // `c.width` is an ExprId (a const-expr edge like `Add(Sub(msb,lsb),1)`),
                // NOT a literal — fold it to its value.
                c.width
                    .and_then(|eid| crate::width::const_u32_of_expr(ir, eid))
                    .unwrap_or(net_w)
            }
        }
    }

    /// Write one chunk's `piece` into its destination bits.
    fn write_chunk(
        &mut self,
        ir: &SimIr,
        c: &LvalChunk,
        raw_off: u32,
        raw_word: u32,
        piece: &Value,
    ) -> bool {
        let s = self.slots[c.net as usize];
        // SVPART: a 2-state variable can never hold X/Z (IEEE §6.11.3) — coerce
        // every unknown bit of the incoming value to 0 before it lands.
        let coerced;
        let piece = if s.two_state && piece.unk.iter().any(|&u| u != 0) {
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
        // `raw_word` is the caller-evaluated array index (the runtime `k` of
        // `mem[k] = …`). None ⇒ index 0. This is also where an X/Z index lands:
        // the shared resolver maps it to the far-out-of-range sentinel.
        let Some(word) = self.chunk_elem(c, raw_word, s.elems) else {
            return false;
        };
        let net_w = s.width;
        let width = self.chunk_width(ir, c);
        let lsb = self.chunk_lsb(c, raw_off, width);

        // WORD-PARALLEL store: a whole-element write. The engine needs three more
        // conditions here (`base % 64 == 0`, and the element occupying whole store
        // words) because it packs elements bit-contiguously; an arena element is
        // word-aligned BY CONSTRUCTION, so the geometry test collapses to "is this
        // the whole element".
        if lsb == 0 && width == net_w {
            return self.store_words(c.net, word, piece);
        }

        // GLITCH: capture bit 0 BEFORE the mutation so a same-slot re-write
        // (A→B→A) records its real B→A transition — the endpoint compare would
        // see only A==A. Gated on `is_edge_target`, so non-clock nets pay a
        // bounds check.
        let track_edge = self.ch.is_edge_target[c.net as usize];
        let old_b0 = if track_edge {
            self.scalar_bit0(c.net)
        } else {
            sim_ir::FourState::Zero
        };
        // Bit-serial: only the in-range destination bits are written.
        let mut changed = false;
        let base = s.off as usize + (word as usize) * 2 * s.words as usize;
        for i in 0..width {
            let dst = lsb + i as i64;
            if dst < 0 || dst as u32 >= net_w {
                continue;
            }
            let (v, u) = piece.get_vu(i);
            let bit = dst as u32;
            let vi = base + (bit / 64) as usize;
            let ui = base + s.words as usize + (bit / 64) as usize;
            let m = 1u64 << (bit % 64);
            let nv = if v != 0 {
                self.buf[vi] | m
            } else {
                self.buf[vi] & !m
            };
            let nu = if u != 0 {
                self.buf[ui] | m
            } else {
                self.buf[ui] & !m
            };
            if nv != self.buf[vi] || nu != self.buf[ui] {
                self.buf[vi] = nv;
                self.buf[ui] = nu;
                changed = true;
            }
        }
        if changed {
            self.note_change(c.net, word);
            if track_edge {
                self.accumulate_edge(c.net, old_b0);
            }
        }
        changed
    }

    /// One chunk's store on the RAW PLANES, for a destination that occupies a
    /// single arena word (`s.words == 1`, so `net_w <= 64`).
    ///
    /// This is `write_chunk` with the `Value` taken away and the two store shapes
    /// — whole element and in-word window — collapsed into the one they always
    /// were at this size: a masked read-modify-write of two adjacent words. It
    /// asks the SAME two questions through the SAME two helpers (`chunk_elem`,
    /// `chunk_lsb`), so which element and which bits a chunk names has exactly one
    /// spelling; what differs is only how the bits get there.
    ///
    /// `cw` is the chunk's width, already folded by the caller.
    pub(crate) fn write_chunk_word(
        &mut self,
        c: &LvalChunk,
        raw_off: u32,
        raw_word: u32,
        mut pv: u64,
        mut pu: u64,
        cw: u32,
    ) -> bool {
        let s = self.slots[c.net as usize];
        // SVPART: a 2-state variable can never hold X/Z (IEEE §6.11.3) — coerce
        // every unknown bit of the incoming value to 0 before it lands. The
        // general path clones and loops; at one word it is two instructions, and
        // doing it unconditionally is the same result as doing it only when some
        // unknown bit is set.
        if s.two_state {
            pv &= !pu; // X (val0/unk1) & Z (val1/unk1) → 0
            pu = 0;
        }
        let Some(word) = self.chunk_elem(c, raw_word, s.elems) else {
            return false;
        };
        let net_w = s.width;
        let lsb = self.chunk_lsb(c, raw_off, cw);

        // The destination window, clipped to the net on BOTH ends: an indexed
        // part-select may hang off either edge and only the in-range bits are
        // written (the out-of-range ones are dropped, never shifted or wrapped).
        let hi = (lsb + cw as i64).clamp(0, net_w as i64) as u32;
        let lo = lsb.clamp(0, net_w as i64) as u32;
        if hi <= lo {
            return false; // the window missed the net entirely
        }
        // `drop` is how many LOW source bits fell off the bottom; `hi > lo` is what
        // bounds it below `cw <= 64`, so neither shift can be undefined.
        let drop = (lo as i64 - lsb) as u32;
        let m = top_mask(hi) & !top_mask(lo);
        let nv = ((pv >> drop) << lo) & m;
        let nu = ((pu >> drop) << lo) & m;

        let base = s.off as usize + (word as usize) * 2;
        let (ov, ou) = (self.buf[base], self.buf[base + 1]);
        let (wv, wu) = ((ov & !m) | nv, (ou & !m) | nu);
        if wv == ov && wu == ou {
            return false;
        }
        // GLITCH: capture bit 0 BEFORE the mutation so a same-slot re-write
        // (A→B→A) records its real B→A transition — the endpoint compare would
        // see only A==A. Gated on `is_edge_target`, so non-clock nets pay a
        // bounds check.
        let track_edge = self.ch.is_edge_target[c.net as usize];
        let old_b0 = if track_edge {
            self.scalar_bit0(c.net)
        } else {
            sim_ir::FourState::Zero
        };
        self.buf[base] = wv;
        self.buf[base + 1] = wu;
        self.note_change(c.net, word);
        if track_edge {
            self.accumulate_edge(c.net, old_b0);
        }
        true
    }

    /// The word-parallel whole-element store — the one place a whole element's
    /// words change. Word-granular change detection with the top-word mask, so
    /// the arena's "no bits above `width`" invariant holds after every write.
    fn store_words(&mut self, net: u32, word: u32, piece: &Value) -> bool {
        let s = self.slots[net as usize];
        // GLITCH: bit 0 before the mutation (see the bit-serial twin).
        let track_edge = self.ch.is_edge_target[net as usize];
        let old_b0 = if track_edge {
            self.scalar_bit0(net)
        } else {
            sim_ir::FourState::Zero
        };
        let base = s.off as usize + (word as usize) * 2 * s.words as usize;
        // `s.words == nwords(width).max(1)` by construction (the allocation used
        // it) — using the slot's own count keeps the indices provably in bounds.
        let nw = s.words as usize;
        let m = top_mask(s.width);
        let mut changed = false;
        for k in 0..nw {
            let mut nv = piece.val.get(k).copied().unwrap_or(0);
            let mut nu = piece.unk.get(k).copied().unwrap_or(0);
            if k == nw - 1 {
                nv &= m;
                nu &= m;
            }
            let vi = base + k;
            let ui = base + s.words as usize + k;
            if self.buf[vi] != nv || self.buf[ui] != nu {
                self.buf[vi] = nv;
                self.buf[ui] = nu;
                changed = true;
            }
        }
        if changed {
            self.note_change(net, word);
            if track_edge {
                self.accumulate_edge(net, old_b0);
            }
        }
        changed
    }
}
