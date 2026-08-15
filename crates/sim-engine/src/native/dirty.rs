//! S1d-2 (doc-21 §5 S1 분해) — **the arena's dirty/edge channel.**
//!
//! The half of scheduling that hangs off the WRITE, not off the loop. The engine
//! maintains it at exactly two points — the two places a stored word actually
//! changes — and the S1c write funnel reproduced those two points without it,
//! which is why `native/write.rs` recorded this as an S1d blocker in the
//! strongest terms available: a scheduler that consumed only `changed` would
//! keep every value correct and still lose a `posedge`.
//!
//! Three things travel here, and the third is the one a value comparison can
//! never see:
//!
//! 1. **`dirty`** — the nets that took a real bit change since the last sweep,
//!    in WRITE order. Membership alone is the changed set: an A→B→A round-trip
//!    inside one slot ends with `cur == prev` and is STILL a change the observer
//!    must see (IEEE §9 fires the glitch once). An endpoint compare drops
//!    exactly those.
//! 2. **`last_blocking_writer`** — who authored the change, so a process is not
//!    re-fired on a net it blocking-wrote itself.
//! 3. **`slot_edge`** — the intra-slot bit0 edge SUMMARY, OR-accumulated per
//!    transition and reset on the net's first dirtying each slot. This is what
//!    recovers the edge KIND for a glitch the endpoints lost, and it is
//!    maintained only for `is_edge_target` nets (every other net pays a bounds
//!    check).
//!
//! Deliberately NOT here — they belong to later pieces, and naming them keeps
//! this module from looking finished:
//!
//! - the continuous-assign dirty worklist (`ca_of_net`/`ca_dirty`, S1d-4's
//!   settle, NOT S1d-3 — that slice delivered the wake decision) — no arena
//!   consumer exists yet;
//! - `emit_probe_change` — genuinely out of scope, `probed_nets` disqualifies
//!   the design at S0;
//! - **`emit_vcd_change` — NOT out of scope, and it constrains this module.**
//!   A `$dumpvars` design is eligible (`vcd_path_override` is a config knob),
//!   and S1d-4's gate is stdout+VCD BYTE identity. Two consequences: (a)
//!   `note_change` must regain the `word` argument the engine carries, since an
//!   array has one VCD id per ELEMENT, and (b) the emitter has to live AT the
//!   store point, not at sweep time — `take_changed` yields one row per net, so
//!   a sweep-time emitter would collapse an intra-slot A→B→A into a single
//!   record and lose exactly the glitch `slot_edge` exists to preserve.
//!
//! And one INHERITED dependency: a `force`d net. The engine's write funnel
//! returns early for `forced[net]` with no `note_change` at all; this store has
//! no force flag, so the same write would fabricate a change — and on a clock,
//! a phantom `posedge`, not merely a wrong value. Designs carrying `force`/
//! `release` are refused by the DESIGN gate (`native::design_eligibility`), and
//! `native::runtime_gate` is what ANDs that with the arena build. A caller that
//! builds an arena without consulting the runtime gate inherits this.

use sim_ir::{FourState, SimIr};

use crate::eval::NetReader;

/// A `Value`'s two planes as the `BitPacked` the VCD writer takes.
pub(crate) fn packed_of(v: &crate::value::Value) -> sim_ir::BitPacked {
    sim_ir::BitPacked {
        val: v.val.to_vec(),
        unk: v.unk.to_vec(),
    }
}

use crate::native::arena::NetArena;

/// One changed net as the sweep hands it to a scheduler: the net, its
/// accumulated intra-slot edge mask, and the activity that blocking-wrote it
/// (`u32::MAX` = any other writer, so re-fire normally).
pub type ChangedNet = (u32, u8, u32);

/// The per-net channel state. Lives on the arena (as it does on `SimState`)
/// because it is written by the store points inside the write funnel.
pub struct DirtyChannel {
    pub dirty: Vec<u32>,
    pub dirty_flag: Vec<bool>,
    /// Intra-slot bit0 edge summary: bit0 = a posedge occurred, bit1 = a
    /// negedge, bit2 = bit0 changed at all (AnyEdge).
    pub slot_edge: Vec<u8>,
    /// Only these nets maintain `slot_edge` — built by the SAME scan the engine
    /// store uses (`state::changes::edge_target_nets`), not a second one.
    pub is_edge_target: Vec<bool>,
    pub last_blocking_writer: Vec<u32>,
    /// The activity currently executing a body, when the write is a BLOCKING
    /// procedural one. `None` for NBA / continuous-assign / clocking writers.
    pub blocking_writer: Option<u32>,
    /// DIRTY-SETTLE (S1d-4d-1): net → the continuous assigns whose value READS
    /// it, and the worklist of assigns a settle pass must re-evaluate.
    ///
    /// NOT recomputed here — installed from `SimState::ca_of_net`, which
    /// `Scheduler::new` already derived through `levelize::ca_deps`. A second
    /// derivation would be a second answer to "what does this assign read".
    ///
    /// ⚠️ It is not an optimization. Visiting every assign every pass produces
    /// the same VALUES (the write funnel drops a same-value write), which is
    /// what the engine's own comment says — but NOT the same DIAGNOSTICS: an
    /// assign whose RHS reads out of range emits another `E4002` on every
    /// re-read. Measured on picorv32: 6 errors became 9 (the 8-cap plus its
    /// suppression note). The worklist is load-bearing for correctness of the
    /// diagnostic stream, not just for speed.
    pub ca_of_net: Vec<Vec<u32>>,
    pub ca_dirty: Vec<u32>,
    pub ca_dirty_flag: Vec<bool>,
    /// VCD value-changes this store has RECORDED but not yet written
    /// (S1d-4d-2), as `(net, word, value-at-the-store-point)`.
    ///
    /// ⚠️ The VALUE is captured here, not re-read at drain time, and that is
    /// the whole design. Buffering only `(net, word)` would make an A→B→A
    /// round-trip inside one slot emit two records carrying the SAME final
    /// value — the glitch collapse this module's own header warns about, and
    /// the reason the emitter has to live at the store point.
    ///
    /// `now` is NOT captured: every drain seam (a statement boundary, the
    /// settle, the NBA apply) is a span across which `now` does not move, so
    /// stamping at drain equals stamping at store — and a second `now` on this
    /// side is the hazard §4.5.293 removed from the kernel.
    pub vcd_pending: Vec<(u32, u32, sim_ir::BitPacked)>,
    /// Is a dump open? Mirrors `SimState::dumping` so the funnel can skip the
    /// capture entirely on the overwhelmingly common no-waveform run.
    pub vcd_on: bool,
    /// G2 PROBE rail: per net, "`--probe` names this one". Empty when no net is
    /// probed, which is every run without `--probe`, so the capture below costs
    /// one `is_empty` on the hot path.
    ///
    /// The rail's STATE — `probed`, `probe_prev`, `trace_lines`, `net_names` —
    /// all lives on `SimState` and is shared. This mirror exists for the same
    /// reason `vcd_on` does: the funnel is on the arena and cannot reach the
    /// scheduler from inside a store.
    pub probed: Vec<bool>,
    /// Probe value-changes RECORDED but not yet emitted, as `(net, value)`.
    ///
    /// Captured at the store point for the same reason the VCD queue is: the
    /// engine's `emit_probe_change` runs INSIDE `note_change`, so an A→B→A
    /// round-trip inside one slot emits three records there — measured — and a
    /// value re-read at drain time would emit the last one three times.
    ///
    /// ⚠️ Element ZERO regardless of `word`, matching what the engine formats
    /// (`fmt_probe_value` reads the low `width` bits of `cur`). That is not an
    /// array simplification: `--probe` REFUSES an unpacked array at the CLI
    /// (E0001, "v1 can trace only a scalar/vector/packed net"), so no probed net
    /// has a second element on either side.
    pub probe_pending: Vec<(u32, sim_ir::BitPacked)>,
}

impl DirtyChannel {
    pub fn new(ir: &SimIr) -> DirtyChannel {
        let n = ir.nets.len();
        DirtyChannel {
            dirty: Vec::new(),
            dirty_flag: vec![false; n],
            slot_edge: vec![0; n],
            is_edge_target: crate::state::edge_target_nets(ir),
            last_blocking_writer: vec![u32::MAX; n],
            blocking_writer: None,
            ca_of_net: Vec::new(),
            ca_dirty: Vec::new(),
            ca_dirty_flag: Vec::new(),
            vcd_pending: Vec::new(),
            vcd_on: false,
            probed: Vec::new(),
            probe_pending: Vec::new(),
        }
    }

    /// Install the cont-assign dependency map and seed the worklist exactly as
    /// `Scheduler::new` does: EVERY assign starts dirty, because nothing has
    /// been evaluated yet and the first settle must behave like a full pass.
    pub fn install_ca_deps(&mut self, ca_of_net: &[Vec<u32>], nca: usize) {
        self.ca_of_net = ca_of_net.to_vec();
        self.ca_dirty_flag = vec![true; nca];
        self.ca_dirty = (0..nca as u32).collect();
    }
}

impl NetArena {
    /// The WHOLE net's raw words — every element, both planes.
    ///
    /// The arm snapshot an in-body `@(sig)` needs. It is the whole net rather
    /// than element 0 because that is what the engine snapshots
    /// (`SimState.nets[n].cur` is the packed array), so `@(mem)` on an array
    /// compares the same thing on both sides.
    pub(crate) fn net_words(&self, net: u32) -> &[u64] {
        let s = self.slots[net as usize];
        let lo = s.off as usize;
        let n = 2 * s.words as usize * s.elems as usize;
        &self.buf[lo..lo + n]
    }

    /// Bit 0 of element 0 — the scalar the edge predicates read.
    pub(crate) fn scalar_bit0(&self, net: u32) -> FourState {
        let s = self.slots[net as usize];
        let base = s.off as usize;
        let v = self.buf[base] & 1;
        let u = self.buf[base + s.words as usize] & 1;
        match (v, u) {
            (0, 0) => FourState::Zero,
            (1, 0) => FourState::One,
            (0, 1) => FourState::X,
            _ => FourState::Z,
        }
    }

    /// A net took a real bit change. Mirrors the engine's `note_change`,
    /// including the detail that the FIRST dirtying of a slot resets the edge
    /// accumulator (later same-slot writes OR into it).
    ///
    /// CALLER OBLIGATION: on an `is_edge_target` net this must be PAIRED with
    /// `accumulate_edge` for the same transition. Calling it alone RESETS the
    /// mask and records nothing in its place, which reads as "the net changed
    /// but no edge occurred" — the engine hit exactly this and documents it at
    /// its third store point (`commit_clocking_sample`).
    pub(crate) fn note_change(&mut self, net: u32, word: u32) {
        let i = net as usize;
        if !self.ch.dirty_flag[i] {
            self.ch.dirty_flag[i] = true;
            self.ch.dirty.push(net);
            if self.ch.is_edge_target[i] {
                self.ch.slot_edge[i] = 0;
            }
        }
        self.ch.last_blocking_writer[i] = self.ch.blocking_writer.unwrap_or(u32::MAX);
        // DIRTY-SETTLE: this net moved, so every continuous assign that reads it
        // must be re-evaluated by the next settle pass. The engine's third store
        // effect, at the same point.
        for k in 0..self.ch.ca_of_net.get(i).map_or(0, Vec::len) {
            let ci = self.ch.ca_of_net[i][k] as usize;
            if !self.ch.ca_dirty_flag[ci] {
                self.ch.ca_dirty_flag[ci] = true;
                self.ch.ca_dirty.push(ci as u32);
            }
        }
        // VCD: capture the CHANGED WORD's value NOW. `word` is why this
        // parameter came back — an array carries one VCD id per ELEMENT, so a
        // record built from word 0 would name the wrong variable.
        if self.ch.vcd_on {
            let v = self.read_net(net, Some(word));
            self.ch
                .vcd_pending
                .push((net, word, crate::native::dirty::packed_of(&v)));
        }
        // G2 PROBE: the same store-point capture, for the same reason. The
        // engine emits from inside its own `note_change`; this side cannot
        // reach the sink from here, so it records and the kernel drains.
        //
        // ⚠️ Element ZERO regardless of `word`, because that is what the engine
        // formats — not a simplification, a mirror. The dedup that turns this
        // into "one record per real change" is `probe_prev`, on the shared
        // state, so both backends dedup with one spelling.
        if self.ch.probed.get(i).copied() == Some(true) {
            // ⚠️ `Some(0)`, and passing `Some(word)` instead SURVIVES the
            // battery — measured. `word` is the ELEMENT index, and `--probe`
            // refuses an unpacked array at the CLI (E0001), so every probed net
            // has `elems == 1` and the two spellings cannot differ. `0` is kept
            // because it states the property the engine's formatter relies on
            // (`fmt_probe_value` reads the low `width` bits of `cur`) rather
            // than relying on a CLI check three layers away to make `word` zero.
            let v = self.read_net(net, Some(0));
            self.ch
                .probe_pending
                .push((net, crate::native::dirty::packed_of(&v)));
        }
    }

    /// OR this transition into the net's intra-slot edge mask. `old_b0` is bit 0
    /// BEFORE the mutation — captured by the caller, because after the store it
    /// is gone. Mirrors the engine's `accumulate_edge` and shares its predicates
    /// (`fs_is_posedge`/`fs_is_negedge`), which are iverilog-pinned over all
    /// twelve 4-state transitions.
    pub(crate) fn accumulate_edge(&mut self, net: u32, old_b0: FourState) {
        let new_b0 = self.scalar_bit0(net);
        let mut m = 0u8;
        if crate::state::fs_is_posedge(old_b0, new_b0) {
            m |= 1;
        }
        if crate::state::fs_is_negedge(old_b0, new_b0) {
            m |= 2;
        }
        if old_b0 != new_b0 {
            m |= 4;
        }
        self.ch.slot_edge[net as usize] |= m;
    }

    /// The arena's twin of `SimState::commit_clocking_sample` — commit a
    /// whole-net value into a clocking holding net (or, for an output clockvar,
    /// into the driven source net), blocking + same-slot, marking it changed.
    ///
    /// The masked store and the change verdict are the SHARED
    /// `value::store_sample_words`; what is per-store is reaching the planes (a
    /// window into `buf`, no resize — a slot's word count is fixed at build) and
    /// the edge bookkeeping below, which is this module's mirrored pair.
    ///
    /// CALLER OBLIGATION discharged here, not deferred: a holding net can itself
    /// be an edge target (`@(posedge cb.sig)`), so `note_change` is PAIRED with
    /// `accumulate_edge` — alone it resets the mask and records nothing in its
    /// place, which is the exact hazard `note_change`'s doc names and which the
    /// engine hit at this same store point.
    pub(crate) fn commit_clocking_sample(&mut self, net: u32, v: &crate::value::Value) -> bool {
        self.assert_owns(net, "NetArena::commit_clocking_sample");
        let s = self.slots[net as usize];
        let nw = s.words as usize;
        let m = crate::value::top_mask(s.width.max(1));
        let track_edge = self.ch.is_edge_target[net as usize];
        let old_b0 = if track_edge {
            self.scalar_bit0(net)
        } else {
            FourState::Zero
        };
        let base = s.off as usize;
        let (lo, hi) = self.buf[base..base + 2 * nw].split_at_mut(nw);
        let changed = crate::value::store_sample_words(lo, hi, m, v);
        if changed {
            self.note_change(net, 0);
            if track_edge {
                self.accumulate_edge(net, old_b0);
            }
        }
        changed
    }

    /// Take this delta's changed set, ASCENDING, with each net's edge mask and
    /// authoring writer — the engine's `propagate_changes` prologue.
    ///
    /// Ascending, not write order: the engine sorts to reproduce the order its
    /// old full-table scan produced, and every downstream wake order (and thus
    /// every `$display` interleaving) is pinned to it. Clears the flags, keeps
    /// the Vec's capacity.
    #[allow(dead_code)] // S1d-3's region queues are the production consumer; today
                        // only the channel differential drives it. Saying that is more honest than
                        // widening the visibility to silence the lint.
    pub(crate) fn take_changed(&mut self, out: &mut Vec<ChangedNet>) {
        out.clear();
        let mut cand = std::mem::take(&mut self.ch.dirty);
        cand.sort_unstable();
        for &n in &cand {
            self.ch.dirty_flag[n as usize] = false;
            out.push((
                n,
                self.ch.slot_edge[n as usize],
                self.ch.last_blocking_writer[n as usize],
            ));
        }
        cand.clear();
        self.ch.dirty = cand;
    }
}
