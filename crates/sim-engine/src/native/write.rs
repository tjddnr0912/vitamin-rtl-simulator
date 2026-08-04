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
//!   `$value$plusargs`, the file family) and `sim_ir::systask_writes_net`
//!   (`$readmem*`, `$sformat`, the `$cast` TASK form, the string/heap mutators).
//!   `r = $random(seed)` is NO LONGER eligible. Plumbing them is what lifts the
//!   reject; until then the refusal is what keeps an executor from repeating
//!   every draw in silence.

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
        // `mem[k] = …`). None ⇒ index 0. An out-of-range word write is IGNORED
        // (E-RUN-RANGE) — clamping to the last element would silently corrupt a
        // valid neighbour. This is also where an X/Z index lands: the shared
        // resolver maps it to the far-out-of-range sentinel.
        let word = if c.word.is_some() {
            if raw_word >= s.elems {
                // Counted, not clamped and not silent — the run loop reports it
                // through the engine's own `warn_run_range` (see
                // `NetArena::pending_range`). The engine emits here; dropping it
                // turned an `ExitClass::HadErrors` run into a clean PASS.
                self.pending_range
                    .set(self.pending_range.get().saturating_add(1));
                return false;
            }
            raw_word
        } else {
            0
        };
        let net_w = s.width;

        // The low (LSB) net-bit position is SIGNED — an underflowing indexed
        // part-select (`v[-2+:4]`, `v[1-:3]`) extends below bit 0, and only the
        // in-range bits are written (the low OOB bits are dropped, NOT shifted up
        // or wrapped). `raw_off` arrives as the offset's 32-bit 2's-complement;
        // sign-extend it (bit positions never exceed i32 range).
        let off_i = raw_off as i32 as i64;
        let (lsb, width) = match c.kind {
            SelKind::Bit => {
                if c.offset.is_none() && c.width.is_none() {
                    (0i64, net_w) // whole net
                } else {
                    (off_i, 1)
                }
            }
            SelKind::PartConst | SelKind::PartIdxUp => (
                off_i,
                c.width
                    .and_then(|eid| crate::width::const_u32_of_expr(ir, eid))
                    .unwrap_or(net_w),
            ),
            SelKind::PartIdxDown => {
                let w = c
                    .width
                    .and_then(|eid| crate::width::const_u32_of_expr(ir, eid))
                    .unwrap_or(net_w);
                (off_i - (w as i64) + 1, w)
            }
        };

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
