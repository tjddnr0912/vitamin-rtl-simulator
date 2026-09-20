//! split part of `state` (mechanical move).

use super::*;

use crate::state::IdCode;

impl SimState<'_> {
    /// Record an ACTUAL bit change on `net`: mark it for the next
    /// `propagate_changes` dirty sweep, then emit the VCD record. This is the
    /// single funnel both `write_chunk` exit paths use — any future mutation
    /// path MUST route through it or the sweep goes blind.
    pub(crate) fn note_change(&mut self, net: u32, word: u32) {
        let i = net as usize;
        if !self.dirty_flag[i] {
            self.dirty_flag[i] = true;
            self.dirty.push(net);
            // GLITCH: first dirtying this slot — start the bit0 edge accumulator
            // fresh. (Later same-slot writes OR into it via `accumulate_edge`.)
            if self.is_edge_target[i] {
                self.slot_edge[i] = 0;
            }
        }
        // SELF-RETRIG: record who authored this change. A BLOCKING write by an
        // executing procedural body (`blocking_writer = Some`) tags the net so the
        // author isn't re-fired on it; every other writer (NBA/cont-assign/
        // clocking/force, `blocking_writer = None`) tags `u32::MAX` = re-fire
        // normally. Overwritten each change, so it is fresh for the next sweep.
        self.last_blocking_writer[i] = self.blocking_writer.unwrap_or(u32::MAX);
        // DIRTY-SETTLE: this net moved, so every continuous assign whose value reads it
        // must be re-evaluated by the next settle pass. Indexed re-borrow because the
        // flag write below aliases `self`. Nets no assign depends on (almost all of
        // them) pay one length check.
        for k in 0..self.ca_of_net.get(i).map_or(0, Vec::len) {
            let ci = self.ca_of_net[i][k] as usize;
            if !self.ca_dirty_flag[ci] {
                self.ca_dirty_flag[ci] = true;
                self.ca_dirty.push(ci as u32);
            }
        }
        self.emit_vcd_change(net, word);
        self.emit_probe_change(net, word);
    }

    /// HEAP-WAKE funnel: the heap CONTENT of handle net `net` moved.
    ///
    /// The twin of [`SimState::note_change`] for storage that has no net word. An
    /// inferred-sensitivity block (`always_comb`/`always_latch`/`@*`) carries its
    /// elaborate read set in `sensitivity.edges`, and a handle net IS in it — the
    /// `comb_read_set` walk inserts the `Signal` under `w.size()`/`w[i]`/`s.len()`
    /// (measured: `always_comb n = w.size()` arms `Level { nets: [t.w] }`). The
    /// wake then runs off the DIRTY sweep, whose only producer was `note_change`,
    /// and no heap mutation reaches that — so every one of them was invisible.
    ///
    /// ⚠️ CALLER OBLIGATION, the same one `note_change` carries: call this ONLY
    /// for an ACTUAL content change. A mark on an unchanged object re-fires its
    /// combinational readers every delta instead of converging.
    ///
    /// NO VCD AND NO PROBE BYTES, and that is a property of the handle rather
    /// than a choice made here: `$dumpvars` skips every DynArray/Queue/Assoc/
    /// AssocStr/String net (variable length has no `$var` form), so `vcd_id` is
    /// `None` and `vcd_word_ids` is empty for one; and `--probe` on one is a LOUD
    /// CLI refusal (`E0001 … is a dynamic-array/queue/string handle`), so
    /// `probed[net]` is false for one. Both were measured, not assumed.
    ///
    /// `ca_dirty` is likewise NOT marked, for a measured reason: `levelize::ca_deps`
    /// refuses to certify any assign with a heap-handle dependency, so every
    /// `assign n = w.size();` is already in `ca_always` and is visited by EVERY
    /// settle pass. Measured: that design answers `0 / 3 / 7` in PRE, which is
    /// verilator's answer exactly. A mark here would be dead code.
    pub(crate) fn note_dyn_change(&self, net: u32) {
        if !self.dyn_wake_observable(net) {
            return;
        }
        // SELF-RETRIG: record the author, exactly as `note_change` does, because
        // the staged mark is consumed a whole delta later — by then the
        // scheduler has cleared `blocking_writer` and the answer would be lost.
        self.dyn_dirty
            .borrow_mut()
            .push((net, self.blocking_writer.unwrap_or(u32::MAX)));
    }

    /// Would a heap mutation of `net` be recorded at all? ONE home for the two
    /// questions [`SimState::note_dyn_change`] asks, so a site that must PAY to
    /// decide "did the content actually change" can decline the cost with the
    /// same predicate the funnel uses rather than a second spelling of it.
    pub(crate) fn dyn_wake_observable(&self, net: u32) -> bool {
        let i = net as usize;
        // Nobody watches this net: no sensitivity names it, so no wake exists.
        if !self.heap_wake_net.get(i).copied().unwrap_or(false) {
            return false;
        }
        // A FRAME-LOCAL handle lives in `dyn_heap[net]` too (one slot per
        // declared local, keyed by net) but is not a module net: no process
        // sensitivity can name it, its stash/restore at call entry and exit is
        // bookkeeping rather than a design-visible mutation, and `read_net` asks
        // exactly this question on the read side.
        !self.frame_local.get(i).copied().unwrap_or(false)
    }

    /// Drain the staged heap-content marks into `out`, ascending by net.
    ///
    /// The CALLER marks them in its own dirty channel; there is no shared apply,
    /// because there is no shared channel — the engine has `st.dirty`/
    /// `st.last_blocking_writer` and tier-3 has `arena.ch`. Same split as
    /// `dyn_heap` itself, which both kernels borrow.
    ///
    /// Ascending + deduplicated so the sweep order does not depend on the order
    /// the mutations happened in (the engine sorts its own `dirty` for the same
    /// reason, and every downstream wake order is pinned to that).
    pub(crate) fn drain_dyn_dirty(&self, out: &mut Vec<(u32, u32)>) {
        out.clear();
        let mut staged = self.dyn_dirty.borrow_mut();
        if staged.is_empty() {
            return;
        }
        out.append(&mut staged);
        // Ascending NET order (byte-identity of the dirty list), STABLE so two
        // authors of one net keep their temporal order: a net marked twice in one
        // delta by the SAME author collapses; marked by two different authors it
        // keeps both, and the temporally later one wins the `last_blocking_writer`
        // slot — which is what `note_change` does too (it overwrites the tag on
        // every change). An unstable sort on `(net, author)` would have let the
        // higher proc id win instead.
        out.sort_by_key(|m| m.0);
        out.dedup();
    }

    /// HEAP-WAKE apply, engine store: put one drained `(net, author)` mark on
    /// THIS store's dirty channel.
    ///
    /// The half of `note_change` a heap change needs and no more: the dirty
    /// flag/list (which IS the changed set the sweep reads) and the SELF-RETRIG
    /// author tag. Deliberately NOT the rest of it —
    ///
    /// * `slot_edge`: a handle net is never an `is_edge_target` (no `@(posedge
    ///   q)` exists; `edge_target_nets` only marks nets an edge sensitivity or an
    ///   edge wait names), so there is no accumulator to reset.
    /// * `ca_dirty`: `levelize::ca_deps` refuses to certify ANY assign with a
    ///   heap-handle dependency, so every one of them is already in `ca_always`
    ///   and is visited by every settle pass. Measured: `assign n = w.size();`
    ///   answers 0 / 3 / 7 in PRE, which is verilator's answer exactly.
    /// * VCD and probe bytes: a handle net has neither channel (see
    ///   [`SimState::note_dyn_change`]).
    ///
    /// Tier-3's twin is the same three lines against `arena.ch`, in
    /// `native::run::propagate` — one rule, two stores, exactly like `dyn_heap`.
    pub(crate) fn mark_heap_dirty(&mut self, net: u32, writer: u32) {
        let i = net as usize;
        if !self.dirty_flag[i] {
            self.dirty_flag[i] = true;
            self.dirty.push(net);
        }
        self.last_blocking_writer[i] = writer;
    }

    /// GLITCH: OR this write's bit0 transition (`old_b0 → current bit0`) into the
    /// net's intra-slot edge accumulator. Called AFTER `note_change` (so the
    /// first-dirty reset has already run), only for `is_edge_target` whole-net /
    /// element-0 writes. `old_b0` is the net's scalar bit0 captured BEFORE the
    /// write.
    pub(crate) fn accumulate_edge(&mut self, net: usize, old_b0: sim_ir::FourState) {
        let new_b0 = scalar_bit0(&self.nets[net].cur);
        let mut m = 0u8;
        if fs_is_posedge(old_b0, new_b0) {
            m |= 1;
        }
        if fs_is_negedge(old_b0, new_b0) {
            m |= 2;
        }
        if old_b0 != new_b0 {
            m |= 4;
        }
        self.slot_edge[net] |= m;
    }

    /// Emit a VCD value_change for the net word that changed. Arrays carry one
    /// id PER ELEMENT (Phase-1.x ⑤ — the v1 VCD only ever showed word 0);
    /// scalars keep the single `vcd_id`.
    pub(crate) fn emit_vcd_change(&mut self, net: u32, word: u32) {
        if !self.dumping {
            return;
        }
        let i = net as usize;
        let width = self.nets[i].width;
        let id = if self.nets[i].vcd_word_ids.is_empty() {
            match self.nets[i].vcd_id {
                Some(id) => id,
                None => return,
            }
        } else {
            match self.nets[i].vcd_word_ids.get(word as usize) {
                Some(Some(id)) => *id,
                _ => return,
            }
        };
        let packed = slice_word(&self.nets[i].cur, width, word);
        self.emit_vcd_packed(id, &packed, width);
    }

    /// Emit a VCD value_change for a net word whose value the CALLER already
    /// has, and whose VCD id the caller already resolved.
    ///
    /// Split out of `emit_vcd_change` for the tier-3 store, which cannot use
    /// that one: it reads `self.nets[i].cur`, and on a native run the values
    /// live in the arena. The id tables (`vcd_id`/`vcd_word_ids`) stay here
    /// because they are static metadata `$dumpvars` fills — the same table for
    /// both stores — so only the VALUE crosses the seam.
    pub(crate) fn emit_vcd_packed(&mut self, id: IdCode, packed: &sim_ir::BitPacked, width: u32) {
        // The `dumping` guard lives HERE too, not only in the callers. The
        // writer gates `value_change` on its own flag but NOT `set_time`, so a
        // caller that forgot would emit stray `#N` records.
        //
        // ⚠️ It is DEFENCE IN DEPTH, not a covered behaviour: with it and
        // `vcd_id_for`'s copy BOTH removed, the gate stays green, because the
        // arena's `vcd_on` tracks `dumping` and so nothing is captured after a
        // `$dumpoff` in the first place. Recorded rather than claimed — a guard
        // that no design can distinguish is worth keeping and worth not
        // pretending about.
        if !self.dumping {
            return;
        }
        if let Some(w) = self.vcd.as_mut() {
            let _ = w.set_time(self.now);
            let _ = w.value_change(id, packed, width);
        }
    }

    /// The VCD id a `(net, word)` change would be reported under, or `None`
    /// when it is not being dumped. The lookup half of `emit_vcd_change`,
    /// callable by a store that holds its own values.
    pub(crate) fn vcd_id_for(&self, net: u32, word: u32) -> Option<(IdCode, u32)> {
        if !self.dumping {
            return None;
        }
        let i = net as usize;
        let width = self.nets[i].width;
        let id = if self.nets[i].vcd_word_ids.is_empty() {
            self.nets[i].vcd_id?
        } else {
            (*self.nets[i].vcd_word_ids.get(word as usize)?)?
        };
        Some((id, width))
    }

    /// OBS-2 (`--probe`): record a `trace.jsonl` change line when a PROBED net's value
    /// changes. INDEPENDENT of VCD dumping (a `--probe` without `$dumpvars` still
    /// traces). Change-deduped against the last emitted value so only real
    /// transitions are logged ("transition만", R-L3). Fast `probed` bool check ⇒
    /// no-op for unprobed nets and for runs with no `--probe` (empty `probed`).
    pub(crate) fn emit_probe_change(&mut self, net: u32, _word: u32) {
        let i = net as usize;
        if self.probed.get(i).copied() != Some(true) {
            return;
        }
        let bits = self.nets[i].cur.clone();
        self.emit_probe_change_from(net, &bits);
    }

    /// The body of [`SimState::emit_probe_change`], over a value the caller
    /// supplies rather than one read from this store.
    ///
    /// A8-probe split it. Tier-3's store point is on the arena, which cannot
    /// reach the sink, so it captures `(net, value)` and the kernel drains here
    /// — and the dedup, the `old` field, the path lookup and the JSON shape stay
    /// in ONE place. The engine's arm above is literally its previous body with
    /// the read hoisted, so its path is unchanged by construction.
    pub(crate) fn emit_probe_change_from(&mut self, net: u32, bits: &sim_ir::BitPacked) {
        let i = net as usize;
        if self.probed.get(i).copied() != Some(true) {
            return;
        }
        let width = self.nets[i].width;
        let newv = crate::fmt_probe_value(bits, width);
        if self.probe_prev[i].as_deref() == Some(newv.as_str()) {
            return; // no actual value change (same-value write / other-word glitch)
        }
        let old = self.probe_prev[i].clone().unwrap_or_default();
        let path = self.net_names.get(i).cloned().unwrap_or_default();
        let mut line = String::from("{\"v\":1,\"t\":");
        line.push_str(&self.now.to_string());
        line.push_str(",\"kind\":\"chg\",\"path\":");
        crate::json_push_str(&mut line, &path);
        line.push_str(",\"old\":");
        crate::json_push_str(&mut line, &old);
        line.push_str(",\"new\":");
        crate::json_push_str(&mut line, &newv);
        line.push('}');
        self.trace_lines.push(line);
        self.probe_prev[i] = Some(newv);
    }

    // ── edge support ─────────────────────────────────────────────────────

    // (R2) The former `snapshot_prev` full-net cur→prev copy at each time
    // advance was DELETED: at the settled point `prev == cur` holds for every
    // net by induction — the only `prev` writers are `propagate_changes`
    // step (c) and the constructor, both setting prev = cur — so the pass was
    // a provable no-op costing O(nets) per timestep. Byte-compare suites
    // (staged/threads/corpus/differential) pin the equivalence.

    // ── force / release (IEEE 1364 §9.3.2; expression forces re-evaluate
    //    continuously via the scheduler's active_forces registry) ─────────

    /// Apply `force lhs = value`: write THROUGH the force flag (a re-force on
    /// an already-forced net must land), then pin the net. `lhs` is a single
    /// whole-net chunk (elaborate-validated).
    pub fn force_write(&mut self, lhs: &Lvalue, value: Value) -> bool {
        let net = lhs.chunks[0].net;
        self.force_lift(net);
        let changed = self.write_lvalue(lhs, value, &Self::FORCE_OFFSETS);
        self.force_pin(net);
        changed
    }

    /// The `Offsets` a force pin uses: one whole-net chunk at `(0, 0)`. Named
    /// because both force write points build it and it must be the same.
    pub(crate) const FORCE_OFFSETS: crate::exec::Offsets = crate::exec::Offsets::Inline {
        buf: [(0, 0); 2],
        len: 1,
    };

    /// Lift the target's force flag so a re-force (or the resumed latent assign)
    /// can LAND — a pin write must go through the very flag it maintains.
    /// Paired with `force_pin`; the pair is what tier-3's own pin write also
    /// calls, so the order is stated once even though the funnel between them
    /// is per-store.
    pub(crate) fn force_lift(&mut self, net: u32) {
        self.forced[net as usize] = false;
    }

    /// Re-pin after a force write. See `force_lift`.
    pub(crate) fn force_pin(&mut self, net: u32) {
        self.forced[net as usize] = true;
    }

    /// Which force keys the nets in `nets` feed, ascending and de-duplicated —
    /// optionally plus every ALWAYS-REEVAL force (a volatile `$time`/`$random`
    /// RHS, or a zero-net constant RHS: both yield a fresh value with frozen
    /// inputs, so a net-sensitivity skip would silently FREEZE them).
    ///
    /// Ascending order is load-bearing, not tidiness: a force's re-pin can write
    /// a net feeding another force, so the executed subset must re-evaluate in
    /// the `BTreeMap` order the old all-forces loop used.
    pub(crate) fn force_keys_for(&self, nets: &[u32], include_always: bool) -> Vec<u32> {
        let mut keys: Vec<u32> = Vec::new();
        for &n in nets {
            if let Some(set) = self.force_net_to_forces.get(&n) {
                keys.extend(set.iter().copied());
            }
        }
        if include_always {
            keys.extend(self.force_always_reeval.iter().copied());
        }
        keys.sort_unstable();
        keys.dedup();
        keys
    }

    /// The live registry entry for a force key, or `None` if an earlier re-pin's
    /// side effects removed it.
    pub(crate) fn force_entry(&self, key: u32) -> Option<(Lvalue, u32, u64)> {
        self.active_forces
            .get(&key)
            .map(|(lv, rhs, mult, _weak)| (lv.clone(), *rhs, *mult))
    }

    /// `release lhs`: unpin. A NET target snaps back to its driver at the next
    /// cont-assign settle (same timestep — the run loop settles every delta);
    /// a VARIABLE keeps the forced value until the next procedural assignment
    /// (no settle entry exists for it) — both fall out of just clearing the flag.
    pub fn release(&mut self, lhs: &Lvalue) {
        self.forced[lhs.chunks[0].net as usize] = false;
    }

    // ── the DECISION half of force/release, split from the WRITE ─────────────
    //
    // Slice #2 threads tier-3 through these. Everything below reads and mutates
    // only the REGISTRY (`active_forces`, `latent_assigns`, the sensitivity
    // sidecars, `forced`) — never a net value — so handing it to a second kernel
    // cannot leak the engine's store. The store operations (evaluate an RHS,
    // write the target) stay with the caller, which is what lets ONE rule drive
    // two funnels. Same shape as slice #1's `clocking_commit_plan` and A1-iii's
    // `TaskWrites`.

    /// `force`/`assign` prologue: settle the registry's priority question and
    /// say whether the caller must now perform the pin write.
    ///
    /// `false` ⇒ a procedural `assign` displaced by a live FORCE: it is parked
    /// as latent (§9.3.1 gives the force priority) and nothing is written.
    pub(crate) fn force_prologue(&mut self, lhs: &Lvalue, rhs: u32, sid: u32) -> bool {
        let net = lhs.chunks[0].net;
        let mult = self.cur_time_mult;
        if self.assign_ranks.contains(&sid) {
            // §9.3.1 proc-assign: an active FORCE keeps priority — park the
            // assign as latent (it takes control at release). Otherwise (re)pin
            // at assign rank (a second assign overrides the first).
            if matches!(self.active_forces.get(&net), Some((.., false))) {
                self.latent_assigns.insert(net, (lhs.clone(), rhs, mult));
                return false;
            }
            self.latent_assigns.remove(&net);
        } else if let Some((plv, prhs, pmult, true)) = self.active_forces.get(&net).cloned() {
            // real force displacing an active assign: park it for release.
            self.latent_assigns.insert(net, (plv, prhs, pmult));
        }
        true
    }

    /// `force`/`assign` epilogue: register for continuous re-evaluation
    /// (IEEE §9.3.2 / §9.3.1) and refresh the RHS net-sensitivity sidecar.
    pub(crate) fn force_epilogue(&mut self, lhs: &Lvalue, rhs: u32, sid: u32) {
        let net = lhs.chunks[0].net;
        let mult = self.cur_time_mult;
        let weak = self.assign_ranks.contains(&sid);
        self.active_forces
            .insert(net, (lhs.clone(), rhs, mult, weak));
        self.register_force_sensitivity(net, rhs);
    }

    /// `release`/`deassign` prologue: update the registry and return the LATENT
    /// procedural assign whose control resumes (§9.3.1) — the caller evaluates
    /// and writes it, then calls `release_epilogue`.
    ///
    /// Also reports whether the target should snap back to its continuous
    /// driver, which is the caller's cue to re-dirty that driver — see
    /// `release_epilogue`'s doc for why that is not automatic.
    pub(crate) fn release_prologue(
        &mut self,
        lhs: &Lvalue,
        sid: u32,
    ) -> Option<(Lvalue, u32, u64)> {
        let net = lhs.chunks[0].net;
        if self.assign_ranks.contains(&sid) {
            // `deassign`: drop the assign wherever it lives. An active STRONG
            // force is untouched; an active assign unpins (the variable HOLDS
            // its value, §9.3.1); a latent assign is just forgotten.
            self.latent_assigns.remove(&net);
            if matches!(self.active_forces.get(&net), Some((.., true))) {
                self.active_forces.remove(&net);
                self.unregister_force_sensitivity(net);
                self.release(lhs);
            }
            return None;
        }
        // `release`: removes the FORCE. A parked proc-assign resumes control
        // (re-pin + re-evaluate NOW, §9.3.1); an active assign is NOT a force
        // and keeps control; otherwise plain unpin.
        match self.active_forces.get(&net) {
            Some((.., true)) => None, // assign active, no force: release is a no-op
            _ => {
                self.active_forces.remove(&net);
                self.unregister_force_sensitivity(net);
                self.release(lhs);
                self.latent_assigns.remove(&net)
            }
        }
    }

    /// Which continuous assigns DRIVE `net` — the cue a `release` owes.
    ///
    /// ⚠️ **This closes a pre-existing silent-wrong that both oracles call.**
    /// `release` on a wire is documented to "snap back to its driver at the next
    /// cont-assign settle", and that was true when the settle re-evaluated every
    /// assign every pass. The dirty-driven settle (§4.5.335) made it false:
    /// clearing `forced` moves no net, so nothing marks the driving assign dirty
    /// and the forced value SURVIVES until some input of that assign happens to
    /// change. Measured on `assign w = a + 2; force w = 8'hF0; release w;` —
    /// iverilog and verilator both report 3, all three vita backends reported
    /// 240 until the next write to `a`.
    ///
    /// Answered from `ir.cont_assigns` rather than from a new LHS→assign map:
    /// a release is a rare event, and a map is state that can go stale. Which
    /// worklist gets marked is the CALLER's — the engine's `st.ca_dirty` and
    /// tier-3's `arena.ch.ca_dirty` are per-store.
    pub(crate) fn drivers_of_net(&self, net: u32) -> Vec<u32> {
        self.ir
            .cont_assigns
            .iter()
            .enumerate()
            .filter(|(_, ca)| ca.lhs.chunks.iter().any(|c| c.net == net))
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// `release` epilogue: re-register the resumed latent assign.
    pub(crate) fn release_epilogue(&mut self, alv: &Lvalue, arhs: u32, amult: u64) {
        let net = alv.chunks[0].net;
        self.active_forces
            .insert(net, (alv.clone(), arhs, amult, true));
        self.register_force_sensitivity(net, arhs);
    }

    // ── C-FORCE-REEVAL-p2: force-RHS net-sensitivity sidecar ─────────────────

    /// Walk a force RHS expression, collecting every design net it READS (so a
    /// per-delta reeval can skip a force whose inputs are unchanged) and whether
    /// it is `volatile` — i.e. it contains a `$time`/`$realtime`/`$stime` or
    /// `$random`/`$urandom`/`$urandom_range` leaf, which yields a DIFFERENT value
    /// each delta even with frozen net inputs. The walk recurses every child
    /// ExprId (children are all `u32` arena indices). A `Signal{net, word}` reads
    /// `net` AND (recursively) its `word` index expr. Defensive on a malformed /
    /// out-of-range ExprId (treat as a volatile leaf so it is always re-evaluated
    /// — never silently dropped).
    pub fn collect_force_reads(&self, eid: u32) -> (Vec<u32>, bool) {
        let mut nets = Vec::new();
        let mut volatile = false;
        self.walk_force_reads(eid, &mut nets, &mut volatile);
        nets.sort_unstable();
        nets.dedup();
        (nets, volatile)
    }

    pub(crate) fn walk_force_reads(&self, eid: u32, nets: &mut Vec<u32>, volatile: &mut bool) {
        use sim_ir::Expr;
        let Some(e) = self.ir.exprs.get(eid as usize) else {
            // Unresolvable node → be conservative: force always re-evaluates.
            *volatile = true;
            return;
        };
        match e {
            Expr::Const { .. } | Expr::ArrayItem { .. } => {}
            Expr::Signal { net, word } => {
                nets.push(*net);
                if let Some(w) = word {
                    self.walk_force_reads(*w, nets, volatile);
                }
            }
            Expr::Select { base, offset, .. } => {
                self.walk_force_reads(*base, nets, volatile);
                self.walk_force_reads(*offset, nets, volatile);
            }
            Expr::Concat { parts } => {
                for &p in parts {
                    self.walk_force_reads(p, nets, volatile);
                }
            }
            Expr::Replicate { count, value } => {
                self.walk_force_reads(*count, nets, volatile);
                self.walk_force_reads(*value, nets, volatile);
            }
            Expr::Unary { operand, .. } => self.walk_force_reads(*operand, nets, volatile),
            Expr::Binary { lhs, rhs, .. } => {
                self.walk_force_reads(*lhs, nets, volatile);
                self.walk_force_reads(*rhs, nets, volatile);
            }
            Expr::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.walk_force_reads(*cond, nets, volatile);
                self.walk_force_reads(*then_e, nets, volatile);
                self.walk_force_reads(*else_e, nets, volatile);
            }
            Expr::SysFunc { which, args } => {
                use sim_ir::SysFuncId as F;
                if matches!(
                    which,
                    F::Time | F::Realtime | F::Stime | F::Random | F::Urandom | F::UrandomRange
                ) {
                    *volatile = true;
                }
                for &a in args {
                    self.walk_force_reads(a, nets, volatile);
                }
            }
            Expr::Call { args, .. } => {
                // A user function call could read state the net-sensitivity map
                // cannot see (statics, side effects). Conservatively volatile so
                // it always re-evaluates (never silently frozen).
                *volatile = true;
                for &a in args {
                    self.walk_force_reads(a, nets, volatile);
                }
            }
        }
    }

    /// Register (or refresh) a force's net-sensitivity in the sidecar. Called at
    /// every `active_forces` insert so the per-delta reeval can target only the
    /// affected forces. `key` is the target net (the `active_forces` map key).
    pub fn register_force_sensitivity(&mut self, key: u32, rhs: u32) {
        // Refresh: a re-force on the same key may change the RHS — drop the old
        // sensitivity first so stale net→force edges never linger.
        self.unregister_force_sensitivity(key);
        let (reads, volatile) = self.collect_force_reads(rhs);
        if volatile || reads.is_empty() {
            // Volatile RHS ($time/$random/…) or a const/zero-net RHS: ALWAYS
            // re-evaluate. A const RHS reads no net, so the net→forces map would
            // never trigger it; treating it as always-reeval preserves today's
            // unconditional behavior (a same-value re-pin is dropped downstream).
            self.force_always_reeval.insert(key);
        } else {
            for n in reads {
                self.force_net_to_forces.entry(n).or_default().insert(key);
            }
        }
    }

    /// Drop a force's net-sensitivity from the sidecar (on release/displace).
    pub fn unregister_force_sensitivity(&mut self, key: u32) {
        self.force_always_reeval.remove(&key);
        // Remove `key` from every net's trigger set; prune emptied entries so
        // the map stays minimal (and the per-delta union loop stays cheap).
        self.force_net_to_forces.retain(|_, set| {
            set.remove(&key);
            !set.is_empty()
        });
    }

    // ── VCD lifecycle (driven by $dumpfile/$dumpvars) ────────────────────

    pub fn open_vcd(&mut self, sink: VcdSink) {
        self.vcd = Some(VcdWriter::new(sink));
    }

    pub fn finalize_vcd(&mut self) {
        if let Some(w) = self.vcd.as_mut() {
            // P2-2: a failed final flush means a truncated waveform — say so
            // (was: `let _ =` swallowed it; exit stayed 0 with no message).
            if let Err(e) = w.flush() {
                self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Warning,
                    code: MsgCode::RunVcdWriteFail,
                    message: format!("VCD flush failed: {e}"),
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(diag::TimeStamp { ticks: self.now }),
                }));
            }
        }
    }
}
impl SimState<'_> {
    /// One W-RUN-DYN-DEGRADE per handle net, callable from `&self` (read path).
    pub(crate) fn dyn_warn_once_at(&self, net: u32, msg: &str) {
        if !self.dyn_warned.borrow_mut().insert(net) {
            return;
        }
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: MsgCode::RunDynDegrade,
            message: msg.to_string(),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.now }),
        }));
    }

    /// v5 (C)-3b: indexed READ of a dyn handle. `idx` is the caller-resolved
    /// word (X/Z or >u32 already mapped to the `u32::MAX` sentinel — the same
    /// rule as static arrays). OOB / X-index / empty / whole-handle reads are
    /// element-width X + warn-once (IEEE: the element default; our elements
    /// are 4-state).
    pub(crate) fn dyn_read(&self, net: u32, idx: Option<u32>) -> Value {
        let nv = &self.ir.nets[net as usize];
        let (w, signed) = (nv.width.max(1), nv.signed);
        let xs = || Value::xs(w, signed);
        let Some(i) = idx else {
            // v7 P2-C: a STRING handle's whole-value read IS its packed
            // materialization (8×len, is_str — context resizing bypassed).
            if nv.kind == NetKind::String {
                let heap = self.dyn_heap.borrow();
                let bytes: &[u8] = match heap.get(net as usize).and_then(|o| o.as_ref()) {
                    Some(DynObj::Str { bytes }) => bytes,
                    _ => &[],
                };
                return Value::from_str_bytes(bytes);
            }
            // a handle has no scalar value surface (elaborate guards at ⑥;
            // defensive here — e.g. a hand-built IR or future regression).
            self.dyn_warn_once_at(net, "dyn handle read without an index");
            return xs();
        };
        match self
            .dyn_heap
            .borrow()
            .get(net as usize)
            .and_then(|o| o.as_ref())
        {
            Some(DynObj::DynArray { elems }) if (i as usize) < elems.len() => {
                elems[i as usize].clone()
            }
            Some(DynObj::Queue { elems }) if (i as usize) < elems.len() => {
                elems[i as usize].clone()
            }
            _ => {
                self.dyn_warn_once_at(net, "dyn index out of range or X (read X)");
                xs()
            }
        }
    }

    // ── N7 class/OOP heap accessors (sibling of the dyn_* family) ──────────
    /// Warn-once (per handle net) for a null/X dereference or a stale-object
    /// access. Never escalates to a fatal — IEEE makes null deref a runtime
    /// error, but vita degrades to X (read) / no-op (write) + this warning so a
    /// faulty testbench does not abort the whole run.
    pub(crate) fn class_warn_null(&self, net: u32, msg: &str) {
        if !self.class_null_warned.borrow_mut().insert(net) {
            return;
        }
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: MsgCode::RunDynDegrade,
            message: msg.to_string(),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.now }),
        }));
    }

    /// The object-id a handle net currently points to, or `None` if it is
    /// `null` (id 0) or holds X/Z. The handle's own integer value is read from
    /// `nets` (a word-less read, which falls through the class branch in
    /// `read_net` to the flat store or the frame window).
    ///
    /// ⭐⭐ **A2-i made the STORE a parameter, and that is the whole slice on the
    /// read side.** This used to be `read_handle_id(&self, net)`, reading
    /// `self.read_net` — the engine's flat store — which is exactly right when
    /// the engine is running and silently wrong when tier-3 is: a native run
    /// leaves that store at its t0 value, so every `obj.f` would dereference
    /// handle `0` and read `null`. Not loud, because `null` has a defined
    /// meaning and the warn that comes with it is a plausible one.
    ///
    /// It is the A1-ii shape verbatim — *the write was already right; the READ
    /// was wrong* — and the fix is A1-ii's: the OPERATION stays here in one
    /// spelling and the STORE arrives as a parameter. `SimState`'s own callers
    /// pass `self`, so the engine path is mechanically byte-identical rather
    /// than merely equivalent.
    pub(crate) fn handle_id_with<N: crate::eval::NetReader + ?Sized>(
        &self,
        nets: &N,
        net: u32,
    ) -> Option<u32> {
        let v = nets.read_net(net, None);
        if v.unk.iter().any(|&u| u != 0) {
            return None; // X/Z handle ⇒ null-like
        }
        match v.val.first().copied().unwrap_or(0) {
            0 => None, // null
            id => Some(id as u32),
        }
    }

    /// The field `(width, signed)` for field-id `field` of the object `id`
    /// belongs to (its DYNAMIC type). `(1,false)` fallback if unknown.
    pub(crate) fn class_field_width(&self, id: u32, field: u32) -> (u32, bool) {
        let cid = self.class_heap.borrow().get(&id).map(|o| o.class_id);
        cid.and_then(|c| self.class_layouts.get(c as usize))
            .map(|l| l.field_width(field))
            .unwrap_or((1, false))
    }

    /// Is field `field` of the object `id` a 4-state type? (2-state ⇒ coerce X/Z→0.)
    /// `true` (no coercion) when the class/field is unknown — conservative.
    pub(crate) fn class_field_four_state(&self, id: u32, field: u32) -> bool {
        let cid = self.class_heap.borrow().get(&id).map(|o| o.class_id);
        cid.and_then(|c| self.class_layouts.get(c as usize))
            .map(|l| l.field_four_state(field))
            .unwrap_or(true)
    }

    /// N7: read field `field` of the object the handle points to. Null/X handle,
    /// a stale object, or a field-id past the layout ⇒ warn-once + X (never a
    /// panic). Returned at the field's natural width; `eval_ctx` resizes to ctx.
    pub(crate) fn class_field_read(&self, net: u32, field: u32) -> Value {
        self.class_field_read_with(self, net, field)
    }

    /// A2-i: [`class_field_read`] against the CALLER's store — see
    /// [`SimState::handle_id_with`] for why the handle read is the part that
    /// had to move. Everything below the handle is `class_heap` and
    /// `class_layouts`, which both kernels borrow, so nothing else routes.
    pub(crate) fn class_field_read_with<N: crate::eval::NetReader + ?Sized>(
        &self,
        nets: &N,
        net: u32,
        field: u32,
    ) -> Value {
        match self.handle_id_with(nets, net) {
            Some(id) => {
                let heap = self.class_heap.borrow();
                match heap.get(&id) {
                    Some(obj) if (field as usize) < obj.fields.len() => {
                        obj.fields[field as usize].clone()
                    }
                    _ => {
                        // CLS-FIELD-RD: fw (heap borrow + layout lookup) is only
                        // used on this cold stale/short arm — compute it here, not
                        // on the hot happy path above.
                        drop(heap);
                        let fw = self.class_field_width(id, field);
                        self.class_warn_null(net, "class field read of a stale/short object (X)");
                        Value::xs(fw.0.max(1), fw.1)
                    }
                }
            }
            None => {
                self.class_warn_null(net, "null/X class handle dereference (read X)");
                Value::xs(1, false)
            }
        }
    }

    /// N7: write field `field` of the object the handle points to. `&self` (the
    /// `RefCell` heap) so a value-method's body — running on the read path — can
    /// still mutate fields. Null/X handle or stale object ⇒ warn-once + no-op
    /// (not a panic). The value is resized to the field's declared width/sign.
    pub(crate) fn class_field_write(
        &self,
        c: &sim_ir::LvalChunk,
        field: u32,
        piece: &Value,
    ) -> bool {
        self.class_field_write_with(self, c, field, piece)
    }

    /// A2-i: [`class_field_write`] against the CALLER's store. Only the HANDLE
    /// read routes (see [`SimState::handle_id_with`]); the resize, the 2-state
    /// coercion and the heap store below are store-independent.
    pub(crate) fn class_field_write_with<N: crate::eval::NetReader + ?Sized>(
        &self,
        nets: &N,
        c: &sim_ir::LvalChunk,
        field: u32,
        piece: &Value,
    ) -> bool {
        let net = c.net;
        let Some(id) = self.handle_id_with(nets, net) else {
            self.class_warn_null(net, "null/X class handle dereference (write ignored)");
            return false;
        };
        let fw = self.class_field_width(id, field);
        // ⚠️ **EQUIVALENT TODAY, and A2-i measured it rather than assuming.** A
        // mutation that drops this resize survives the whole suite INCLUDING an
        // anchor line written to kill it (`p.x = 64'h1234_5678_9ABC_DEF0` really
        // does arrive 64 bits wide against a 32-bit field — instrumented). The
        // reason is on the READ side: `class_field_read` hands back the stored
        // `Value` verbatim and `eval_ctx` then sizes it to the context width,
        // which `patch_class_fields` has already set to the FIELD's own width
        // and sign. So an over-wide store is re-narrowed at every use.
        //
        // Kept because it is the fail-closed direction and because the store
        // then means what the layout says. Fourth time this repo has measured
        // this exact shape — `bind_formal` re-binds a frame formal (§5.1-n),
        // `write_lvalue` re-applies the destination width (A1-i's `H`),
        // `eval_core` pre-sizes a call's actuals (§4.5.338's `formal_width`) —
        // so it is recorded as equivalent, not presented as covered.
        let mut resized = piece.clone().resize_keep_sign(fw.0.max(1), fw.1);
        // A 2-state field (`bit`/`byte`/…) can never hold X/Z — coerce it to 0 (§6.11.3),
        // mirroring the frame-slot (`coerce_two_state_frame`) and module-net 2-state paths.
        if !self.class_field_four_state(id, field) && resized.unk.iter().any(|&u| u != 0) {
            for k in 0..resized.unk.len() {
                resized.val[k] &= !resized.unk[k]; // X (val0/unk1) & Z (val1/unk1) → 0
                resized.unk[k] = 0;
            }
        }
        let mut heap = self.class_heap.borrow_mut();
        match heap.get_mut(&id) {
            Some(obj) if (field as usize) < obj.fields.len() => {
                obj.fields[field as usize] = resized;
            }
            _ => {
                drop(heap);
                self.class_warn_null(net, "class field write to a stale/short object (ignored)");
            }
        }
        false
    }

    /// N7: allocate a fresh object of `class_id`, default-init its fields per the
    /// layout, and return its monotonic object-id (≥1; never recycled). `&self`
    /// (interior-mutable heap) so a ctor invoked on the read path can allocate.
    pub(crate) fn class_alloc(&self, class_id: u32) -> u32 {
        let id = self.class_obj_next.get();
        self.class_obj_next.set(id + 1);
        let fields = match self.class_layouts.get(class_id as usize) {
            Some(layout) => (0..layout.fields.len() as u32)
                .map(|i| layout.default_value(i))
                .collect(),
            None => Vec::new(),
        };
        self.class_heap
            .borrow_mut()
            .insert(id, ClassObj { class_id, fields });
        id
    }

    /// v5 (C)-3b/④: indexed WRITE of a dyn handle. Shared rules: X-index /
    /// bit-select within an element → IGNORED + warn-once (clamping or
    /// auto-grow would silently corrupt). Kind split (iverilog live):
    /// dyn array — any OOB → IGNORED + warn; queue — `q[size] = v` is
    /// push_back-equivalent (IEEE §7.10.1, legal and SILENT, grows by one),
    /// beyond that → IGNORED + warn.
    /// Returns false ALWAYS: dyn content changes do not participate in the net
    /// dirty channel (design §4 — no sensitivity on handles, no VCD records).
    /// `dyn_heap` lazy-create accessor — the `BTreeMap::entry(net).or_insert_with`
    /// replacement for the flat `Vec<Option<DynObj>>` layout. Sets the slot to
    /// `Some(f())` only if it is currently `None`, then hands back `&mut DynObj`.
    /// `net` is always a valid HANDLE NetId (`< ir.nets.len()`), so the slot
    /// exists; the `expect` is unreachable by construction.
    /// §4.5.194: interior-mutable (`RefCell`) lazy-create + scoped-mutate. Sets
    /// the slot to `Some(f())` only if currently `None`, then runs `g` on the
    /// live object with the `borrow_mut` guard scoped to THIS call. `g` MUST NOT
    /// re-touch `dyn_heap` (else `BorrowMutError`); callers do a point mutation
    /// here and defer any `enforce_queue_bound`/warn to AFTER this returns (the
    /// closure form replaces the old `&mut DynObj`-returning `dyn_entry`, whose
    /// escaping reference cannot survive a `RefCell`).
    /// THE funnel for storing a value as a dynamic-container ELEMENT (dyn array or
    /// queue). Every write site must go through it — a site that resizes on its own
    /// silently destroys a string element.
    ///
    /// A `string` element stores the raw byte string: the value carries `is_str` and
    /// its length is dynamic, while the handle net has width 0 (so `w` is 1 here), and
    /// `resize(1)` truncates the whole byte string to one bit. That is exactly how
    /// `string q[$]` first read back EMPTY — `q.size()` was right and every element was
    /// "" — because the queue push did its own `.resize(w)` while the dyn-array element
    /// write had this branch. A `real` element needs no branch: `resize` is a no-op on
    /// `is_real`. Everything else resizes with assignment semantics (§5.5).
    ///
    /// The discriminator is `dyn_str_elem`, the SAME flag that makes the engine store
    /// these elements as byte strings in the first place, so this cannot disagree with
    /// the storage it is coercing for.
    pub(crate) fn coerce_dyn_elem(&self, net: u32, v: &Value, w: u32) -> Value {
        if self
            .dyn_str_elem
            .get(net as usize)
            .copied()
            .unwrap_or(false)
        {
            // §6.16: an INTEGRAL element value crossing into a string element is
            // converted (every 0x00 dropped, unknown bits 0); a string value is kept
            // verbatim. `q.push_back(24'h610062)` is "ab"/2 on both oracles.
            Value::from_str_bytes(&v.to_sv_string_bytes())
        } else {
            v.clone().resize(w)
        }
    }

    pub(crate) fn with_dyn_entry<R>(
        &self,
        net: u32,
        f: impl FnOnce() -> DynObj,
        g: impl FnOnce(&mut DynObj) -> R,
    ) -> R {
        let mut heap = self.dyn_heap.borrow_mut();
        let slot = &mut heap[net as usize];
        if slot.is_none() {
            *slot = Some(f());
        }
        g(slot
            .as_mut()
            .expect("with_dyn_entry: slot just set to Some"))
    }

    /// §4.5.194: allocate a `new[n]` dynamic array into `dyn_heap[net]` — the shared
    /// core of both the `&mut` builtin (`builtins::dispatch` DynNew) and the `&self`
    /// frame executors (`frame_dyn_new`, for a function/task body `loc = new[n]`). `net`
    /// is a validated DynArray handle; `n` is the already-capped element count; `src_net`
    /// is the optional `new[n](src)` copy source. Each element takes its type's IEEE
    /// §7.5.2 default (0 for 2-state, X for 4-state, 0.0 real, "" string).
    pub(crate) fn alloc_dyn_array(&self, net: u32, n: usize, src_net: Option<u32>) {
        let (w, signed) = self
            .ir
            .nets
            .get(net as usize)
            .map(|nv| (nv.width.max(1), nv.signed))
            .unwrap_or((1, false));
        let elem_default = if self.nets[net as usize].is_real {
            Value::from_f64(0.0)
        } else if self
            .dyn_str_elem
            .get(net as usize)
            .copied()
            .unwrap_or(false)
        {
            Value::from_str_bytes(&[])
        } else if self.two_state.get(net as usize).copied().unwrap_or(false) {
            Value::zeros(w, signed)
        } else {
            Value::xs(w, signed)
        };
        let mut elems = vec![elem_default; n];
        if let Some(src_net) = src_net {
            // shared borrow scoped to the prefix-copy (writes the LOCAL `elems`); dropped
            // before the borrow_mut store below (§C6).
            let src_heap = self.dyn_heap.borrow();
            if let Some(DynObj::DynArray { elems: src }) =
                src_heap.get(src_net as usize).and_then(|o| o.as_ref())
            {
                for (dst, s) in elems.iter_mut().zip(src.iter()) {
                    *dst = s.clone();
                }
            }
        }
        // HEAP-WAKE: `new[n]` REPLACES the object, so whether it moved is a
        // comparison against what was there. Paid only when some sensitivity
        // names this net (`dyn_wake_observable`), which is what keeps an
        // allocation-heavy design with no combinational reader at zero cost; the
        // compare is O(n) on an operation that is already O(n). A missing entry
        // IS the empty array (lazy, like every dyn object), so `new[0]` on a
        // never-touched handle is correctly NOT a change.
        let watched = self.dyn_wake_observable(net);
        let moved = {
            let mut heap = self.dyn_heap.borrow_mut();
            let fresh = DynObj::DynArray { elems };
            let moved = watched && !dyn_slot_eq(heap[net as usize].as_ref(), Some(&fresh));
            heap[net as usize] = Some(fresh);
            moved
        };
        if moved {
            self.note_dyn_change(net);
        }
    }

    /// R23: byte-set `s[i] = c` on a `string` net (`$sformatf`-free §6.16.2 element
    /// write, lowered as `SysTaskId::StrPutC`), routed by WHERE that string's bytes live.
    ///
    /// The `StrPutC` handler used to write `dyn_heap[net]` unconditionally. That is the
    /// MODULE-scope string store, and a frame-local `string` is not there — it is
    /// slab-stored in the frame slot (§4.5.167). So `task automatic tk(); string s; s =
    /// "zz"; s[0] = 65;` left `s` as `"zz"` at exit 0, with no diagnostic, while the same
    /// two lines in a module process produced iverilog's `"Az"`. A pre-existing
    /// silent-wrong: it needed no call, no output formal and no frame routing to reproduce.
    /// R23 surfaced it because the loud gate that used to reject `s[i] = f(a, o)` in a
    /// frame body was removed, and removing a gate makes what it masked yours to own.
    ///
    /// Both stores are reachable through `&self` (`dyn_heap` and the frame slab are both
    /// interior-mutable), so this needs no executor change — only the routing question
    /// `read_net` has always asked, asked on the write side too.
    pub(crate) fn str_putc(&self, net: u32, i: u64, c: u8) {
        if c == 0 {
            return; // §6.16.2: writing NUL is ignored (iverilog-pinned)
        }
        if self.frame_local.get(net as usize).copied().unwrap_or(false) {
            let Some((fidx, slot)) = self.frame_route[net as usize] else {
                return;
            };
            let mut bytes = self
                .frame_slot_read(fidx, self.frame_slot_auto[net as usize], slot)
                .to_str_bytes();
            let Some(b) = bytes.get_mut(i as usize) else {
                return; // out of range → no-op, same as the module path below
            };
            *b = c;
            self.frame_slot_write(
                fidx,
                self.frame_slot_auto[net as usize],
                slot,
                Value::from_str_bytes(&bytes),
            );
            return;
        }
        let mut moved = false;
        if let Some(DynObj::Str { bytes }) = self
            .dyn_heap
            .borrow_mut()
            .get_mut(net as usize)
            .and_then(|o| o.as_mut())
        {
            if let Some(b) = bytes.get_mut(i as usize) {
                moved = *b != c;
                *b = c;
            }
        }
        // HEAP-WAKE: an out-of-range index is a no-op above and must stay one here.
        if moved {
            self.note_dyn_change(net);
        }
    }

    /// §4.5.194: `&self` (was `&mut`) — the dyn heap is interior-mutable, so this
    /// element/whole store is reachable from BOTH the `&mut` module path
    /// (`write_chunk`) and the `&self` frame executors (`frame_write_lvalue`, for a
    /// `new[]`-local / snapshotted-formal element write).
    pub(crate) fn dyn_write(
        &self,
        c: &sim_ir::LvalChunk,
        raw_off: u32,
        raw_word: u32,
        piece: &Value,
    ) -> bool {
        let net = c.net;
        let w = self.ir.nets[net as usize].width.max(1);
        // v7 P2-C: STRING whole-handle assignment — strip leading NULs from
        // the packed value (§6.16) and store the bytes. The only legal
        // string lvalue shape; anything narrower falls to the loud arm.
        if self.ir.nets[net as usize].kind == NetKind::String
            && c.word.is_none()
            && c.offset.is_none()
            && c.width.is_none()
        {
            let bytes = piece.to_sv_string_bytes();
            // HEAP-WAKE: a missing entry IS "" (lazy, like every dyn object), so
            // `s = ""` on a never-assigned handle is correctly NOT a change.
            let moved = {
                let mut heap = self.dyn_heap.borrow_mut();
                let fresh = DynObj::Str { bytes };
                let moved = !dyn_slot_eq(heap[net as usize].as_ref(), Some(&fresh));
                heap[net as usize] = Some(fresh);
                moved
            };
            if moved {
                self.note_dyn_change(net);
            }
            return false; // no NET dirty channel (the word never moves); the heap
                          // channel is `note_dyn_change` above
        }
        // N3: a part-select WRITE of a packable-record dyn-ARRAY element
        // (`arr[i].field = v`) — deposit `piece` into the element at `[off +: width]`
        // (read-modify-write). Only a plain `DynArray` element (word + a part-select)
        // takes this path; a queue/assoc/string element part-select falls to the loud
        // arm below. The `(lsb, width)` computation mirrors the module-net `write_chunk`.
        if self.ir.nets[net as usize].kind == NetKind::DynArray
            && c.word.is_some()
            && (c.offset.is_some() || c.width.is_some())
        {
            let off_i = raw_off as i32 as i64;
            let ir = self.ir;
            let fold = |eid: u32| crate::width::const_u32_of_expr(ir, eid);
            let (lsb, width) = match c.kind {
                SelKind::Bit => (off_i, 1u32),
                SelKind::PartConst | SelKind::PartIdxUp => {
                    (off_i, c.width.and_then(fold).unwrap_or(w))
                }
                SelKind::PartIdxDown => {
                    let ww = c.width.and_then(fold).unwrap_or(w);
                    (off_i - ww as i64 + 1, ww)
                }
            };
            // SVPART: an all-2-state record's element net can never hold X/Z (IEEE
            // §6.11.3) — coerce the field's incoming unknown bits to 0, matching the
            // whole-element `'{…}` desugar (which coerces per 2-state field). Mirrors
            // the module-net `write_chunk` coercion (a mixed 2-/4-state record keeps
            // `Logic`, so a 2-state field there stays a documented follow-on).
            let piece_c;
            let piece = if self.two_state[net as usize] && piece.unk.iter().any(|&u| u != 0) {
                let mut v = piece.clone();
                for k in 0..v.unk.len() {
                    v.val[k] &= !v.unk[k];
                    v.unk[k] = 0;
                }
                piece_c = v;
                &piece_c
            } else {
                piece
            };
            let piece_r = piece.clone().resize_keep_sign(width.max(1), false);
            let i = raw_word as usize;
            // Scope the `borrow_mut` to the store; the miss-warn runs after it
            // releases (§C6 — never hold a heap guard across `dyn_warn_once_at`).
            let (hit, moved) = {
                let mut heap = self.dyn_heap.borrow_mut();
                match heap.get_mut(net as usize).and_then(|o| o.as_mut()) {
                    Some(DynObj::DynArray { elems }) if i < elems.len() => {
                        // Deposit each in-range field bit (OOB bits drop, IEEE part-select).
                        let mut cur = elems[i].clone();
                        // Same rule as `frame_part_write`'s deposit, one spelling:
                        // a fully-in-range window is a word-parallel replace, and
                        // anything else keeps the per-bit loop that implements the
                        // IEEE §11.5.1 out-of-range DROP. `copy_bits` is wrong here
                        // for the same reason (it OR-merges into a zero window).
                        if let Some(off) = crate::eval::window_in_range(lsb, width, w) {
                            debug_assert_eq!(cur.width, w, "dyn element narrower than its net");
                            crate::eval::replace_bits(&mut cur, off, &piece_r, 0, width);
                        } else {
                            for k in 0..width {
                                let bp = lsb + k as i64;
                                if bp >= 0 && (bp as u32) < w {
                                    let (bv, bu) = piece_r.get_vu(k);
                                    cur.set_vu(bp as u32, bv, bu);
                                }
                            }
                        }
                        // HEAP-WAKE: a deposit that changes no bit is not a change.
                        let moved = elems[i] != cur;
                        elems[i] = cur;
                        (true, moved)
                    }
                    _ => (false, false),
                }
            };
            if !hit {
                self.dyn_warn_once_at(net, "dyn index out of range or X (write ignored)");
            }
            if moved {
                self.note_dyn_change(net);
            }
            return false;
        }
        if c.word.is_none() || c.offset.is_some() || c.width.is_some() {
            self.dyn_warn_once_at(net, "unsupported dyn lvalue shape (write ignored)");
            return false;
        }
        // ⑤/v6: an assoc element on the u32 pair funnel = a shape the
        // AssocKey/AssocStrKey lane did not claim (a concat chunk, …) —
        // outside the MVP, IGNORED loud. The single-chunk lane
        // (`write_lvalue`) never reaches here.
        if matches!(
            self.ir.nets[net as usize].kind,
            NetKind::Assoc | NetKind::AssocStr
        ) {
            self.dyn_warn_once_at(
                net,
                "assoc element write in an unsupported lvalue shape (ignored)",
            );
            return false;
        }
        let i = raw_word as usize;
        if self.ir.nets[net as usize].kind == NetKind::Queue {
            // A missing entry IS the empty queue: the append lane must be
            // reachable on a never-touched handle (`q[0] = v` creates it). The
            // `borrow_mut` is scoped to `with_dyn_entry`; the bound-enforcement /
            // warn run AFTER it returns (§C6 — no dyn_heap touch in the guard).
            enum QStep {
                /// In-range store; the payload is HEAP-WAKE's "did it move".
                Done(bool),
                Pushed,
                Cap,
                Oob,
            }
            let coerced = self.coerce_dyn_elem(net, piece, w);
            let step = self.with_dyn_entry(
                net,
                || DynObj::Queue {
                    elems: std::collections::VecDeque::new(),
                },
                |obj| {
                    let DynObj::Queue { elems } = obj else {
                        // kind-mismatched entry: unreachable by construction, and
                        // nothing was stored, so nothing moved.
                        return QStep::Done(false);
                    };
                    let len = elems.len();
                    match i.cmp(&len) {
                        std::cmp::Ordering::Less => {
                            let moved = elems[i] != coerced;
                            elems[i] = coerced;
                            QStep::Done(moved)
                        }
                        // The u32::MAX X-sentinel can never land in the Equal arm:
                        // len ≤ the cap, far below the sentinel.
                        std::cmp::Ordering::Equal if len < MAX_DYN_ELEMS => {
                            elems.push_back(coerced);
                            QStep::Pushed
                        }
                        std::cmp::Ordering::Equal => QStep::Cap,
                        std::cmp::Ordering::Greater => QStep::Oob,
                    }
                },
            );
            match step {
                QStep::Pushed => {
                    // HEAP-WAKE: `q[size] = v` grew the queue, always a change.
                    self.note_dyn_change(net);
                    self.enforce_queue_bound(net) // v6 ③ (no-op when unbounded)
                }
                QStep::Cap => self.dyn_warn_once_at(
                    net,
                    "queue exceeds the element cap (1<<24); write-append dropped",
                ),
                QStep::Oob => {
                    self.dyn_warn_once_at(net, "queue index beyond size or X (write ignored)")
                }
                QStep::Done(moved) => {
                    if moved {
                        self.note_dyn_change(net);
                    }
                }
            }
            return false;
        }
        let (hit, moved) = {
            let coerced = self.coerce_dyn_elem(net, piece, w);
            let mut heap = self.dyn_heap.borrow_mut();
            if let Some(DynObj::DynArray { elems }) =
                heap.get_mut(net as usize).and_then(|o| o.as_mut())
            {
                if i < elems.len() {
                    // HEAP-WAKE: a same-value element store is not a change — the
                    // rule `note_change` applies to a net word, applied to the
                    // element, so a self-writing `always_comb` converges here the
                    // way it does there.
                    let moved = elems[i] != coerced;
                    elems[i] = coerced;
                    (true, moved)
                } else {
                    (false, false)
                }
            } else {
                (false, false)
            }
        };
        if !hit {
            self.dyn_warn_once_at(net, "dyn index out of range or X (write ignored)");
        }
        if moved {
            self.note_dyn_change(net);
        }
        false
    }

    /// v5 ⑤: assoc-element WRITE (`a[k] = v`) — the `Offsets::AssocKey` lane.
    /// `None` key = X/Z (invalid index, IEEE §7.8.6): IGNORED + warn-once. A
    /// missing key CREATES the element (§7.8); the value is cast to the
    /// element type (the same `resize(w)` as every other dyn store). Inserts
    /// past the shared cap warn + drop (no silent caps).
    pub(crate) fn assoc_write(&mut self, net: u32, key: Option<i64>, value: &Value) {
        let w = self.ir.nets[net as usize].width.max(1);
        let Some(k) = key else {
            self.dyn_warn_once_at(net, "assoc key is X/Z (write ignored)");
            return;
        };
        // Cap BEFORE the entry borrow (the warn latch needs `&self` while the
        // map borrow holds `&mut self`); replacing an existing key is exempt.
        let (len, exists) = match self
            .dyn_heap
            .borrow()
            .get(net as usize)
            .and_then(|o| o.as_ref())
        {
            Some(DynObj::Assoc { map }) => (map.len(), map.contains_key(&k)),
            _ => (0, false),
        };
        if !exists && len >= MAX_DYN_ELEMS {
            self.dyn_warn_once_at(net, "assoc exceeds the element cap (1<<24); write dropped");
            return;
        }
        // A missing entry IS the empty assoc (lazy, like every dyn object).
        let moved = self.with_dyn_entry(
            net,
            || DynObj::Assoc {
                map: std::collections::BTreeMap::new(),
            },
            |obj| {
                let DynObj::Assoc { map } = obj else {
                    return false;
                };
                let v = value.clone().resize(w);
                // HEAP-WAKE: `insert` hands back the previous binding, so
                // "created or replaced with a different value" is free here.
                map.insert(k, v.clone()) != Some(v)
            },
        );
        if moved {
            self.note_dyn_change(net);
        }
    }

    /// v6: string-keyed assoc WRITE — the `Offsets::AssocStrKey` lane (the
    /// byte-string twin of `assoc_write`; same X-key / cap / create rules).
    pub(crate) fn assoc_str_write(&mut self, net: u32, key: &Option<Vec<u8>>, value: &Value) {
        let w = self.ir.nets[net as usize].width.max(1);
        let Some(k) = key else {
            self.dyn_warn_once_at(net, "assoc key is X/Z (write ignored)");
            return;
        };
        let (len, exists) = match self
            .dyn_heap
            .borrow()
            .get(net as usize)
            .and_then(|o| o.as_ref())
        {
            Some(DynObj::AssocStr { map }) => (map.len(), map.contains_key(k)),
            _ => (0, false),
        };
        if !exists && len >= MAX_DYN_ELEMS {
            self.dyn_warn_once_at(net, "assoc exceeds the element cap (1<<24); write dropped");
            return;
        }
        let moved = self.with_dyn_entry(
            net,
            || DynObj::AssocStr {
                map: std::collections::BTreeMap::new(),
            },
            |obj| {
                let DynObj::AssocStr { map } = obj else {
                    return false;
                };
                let v = value.clone().resize(w);
                map.insert(k.clone(), v.clone()) != Some(v)
            },
        );
        if moved {
            self.note_dyn_change(net);
        }
    }

    /// v6 ③: bounded-queue post-op rule (iverilog live, IEEE §7.10):
    /// whatever the op left beyond size N+1 falls off the TAIL — one rule
    /// reproduces push_back-on-full (= skip), push_front-on-full (back
    /// drops) and insert-on-full (back drops). Loud (W4020 once per net).
    pub(crate) fn enforce_queue_bound(&self, net: u32) {
        let Some(&b) = self.queue_bounds.get(&net) else {
            return;
        };
        let cap = b as usize + 1;
        let mut dropped = false;
        {
            let mut heap = self.dyn_heap.borrow_mut();
            if let Some(DynObj::Queue { elems }) =
                heap.get_mut(net as usize).and_then(|o| o.as_mut())
            {
                while elems.len() > cap {
                    elems.pop_back();
                    dropped = true;
                }
            }
        }
        if dropped {
            // HEAP-WAKE: the tail really left the queue.
            self.note_dyn_change(net);
            self.dyn_warn_once_at(
                net,
                "bounded queue exceeded its bound; tail element(s) dropped",
            );
        }
    }
}

/// Are two heap slots the SAME CONTENT, as a reader would see them?
///
/// A missing entry IS the empty object — "lazy, like every dyn object" is the
/// rule the read side already implements (`dyn_read` on a `None` slot answers
/// size 0 / "") — so `None` and an empty `Some` must compare equal or every
/// first allocation of an empty array would stage a change nobody can observe.
///
/// ONE home for the question every [`SimState::note_dyn_change`] site asks
/// before it stages a mark.
pub(crate) fn dyn_slot_eq(a: Option<&DynObj>, b: Option<&DynObj>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x == y,
        (Some(x), None) | (None, Some(x)) => x.is_empty(),
        (None, None) => true,
    }
}

/// Which nets any process SENSITIVITY names — the early-out of
/// [`SimState::note_dyn_change`].
///
/// A SUPERSET on purpose. The wake this table gates is a ROUTER, not a guard:
/// over-reporting costs one extra staged mark that the sweep then finds nobody
/// to wake with, while under-reporting is a silent missed wake — the exact
/// defect this funnel exists to close. So it takes every net named by any
/// sensitivity list whatever its `SensKind`, plus every net named by an in-body
/// `@(…)` wait, rather than trying to decide which of them a heap change could
/// really fire.
///
/// Exhaustive over `WaitCause` so a new variant carrying nets is a compile
/// error here rather than a net silently dropped from the set.
///
/// Built once at construction like [`edge_target_nets`], and for the same
/// reason: the net ids are compile-time fixed, so one scan yields the complete
/// set and every mutation afterwards pays a single `Vec<bool>` load.
pub(crate) fn heap_wake_nets(ir: &sim_ir::SimIr) -> Vec<bool> {
    let nnets = ir.nets.len();
    let mut watched = vec![false; nnets];
    let mark = |net: u32, set: &mut Vec<bool>| {
        if (net as usize) < nnets {
            set[net as usize] = true;
        }
    };
    let mark_term = |term: &sim_ir::Terminator, set: &mut Vec<bool>| {
        let sim_ir::Terminator::Wait { cond, .. } = term else {
            return;
        };
        match cond {
            sim_ir::WaitCause::Edge { net, .. } => mark(*net, set),
            sim_ir::WaitCause::Level { nets } => {
                for &n in nets {
                    mark(n, set);
                }
            }
            // No net list: `wait(expr)` (its nets are the expression's, and a
            // heap change cannot move an expression's NET words), a named event,
            // `wait fork`.
            sim_ir::WaitCause::Expr { .. }
            | sim_ir::WaitCause::Named { .. }
            | sim_ir::WaitCause::Fork => {}
        }
    };
    for p in &ir.processes {
        for et in &p.sensitivity.edges {
            mark(et.net, &mut watched);
        }
        for blk in &p.body {
            mark_term(&blk.term, &mut watched);
        }
    }
    // …and the global func/task arena, whose bodies carry waits too — the same
    // second loop `edge_target_nets` needs, for the same reason.
    for blk in &ir.blocks {
        mark_term(&blk.term, &mut watched);
    }
    watched
}

/// Which nets are EDGE targets — statically edge-sensitive `always` processes
/// plus every procedural `@(posedge x)` wait, in the process-local bodies and in
/// the global func/task arena. Compile-time-fixed net ids, so one scan at
/// construction yields the complete set; the intra-slot edge mask (`slot_edge`)
/// is then maintained and consulted only for these nets.
///
/// Extracted so the engine store and the tier-3 arena build it from ONE
/// spelling: a second scan that missed the func/task arena would leave a clock
/// net untracked, and the symptom is not a wrong value but a `posedge` that
/// never fires — invisible to any value comparison.
pub(crate) fn edge_target_nets(ir: &sim_ir::SimIr) -> Vec<bool> {
    let nnets = ir.nets.len();
    let mut is_edge_target = vec![false; nnets];
    let mark_edge = |net: u32, set: &mut Vec<bool>| {
        if (net as usize) < nnets {
            set[net as usize] = true;
        }
    };
    for p in &ir.processes {
        if p.sensitivity.kind == sim_ir::SensKind::Edge {
            for et in &p.sensitivity.edges {
                mark_edge(et.net, &mut is_edge_target);
            }
        }
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
    for blk in &ir.blocks {
        if let sim_ir::Terminator::Wait {
            cond: sim_ir::WaitCause::Edge { net, .. },
            ..
        } = &blk.term
        {
            mark_edge(*net, &mut is_edge_target);
        }
    }
    is_edge_target
}
