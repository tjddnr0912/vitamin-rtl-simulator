//! S1d-3 (doc-21 §5 S1 분해) — **the wake decision.**
//!
//! S1d-2 produced the changed set with its intra-slot edge masks; this consumes
//! it and answers the next question: *which processes become ready, in what
//! order*. That is a DECISION, not a value, so the gate compares decisions — the
//! engine's `propagate_changes` pass (a) is the oracle and the ready list is the
//! output compared against it.
//!
//! ## What eligibility buys, stated because it is most of the simplification
//!
//! The engine carries a general ACTIVITY arena: fork children are appended, ties
//! are composites of `(parent_tie, child_idx)`, and a `Ready` names
//! `(tie, proc, block)` because a child runs a sub-chain of its parent's body.
//! None of that survives the S0 gate — `fork_modes` non-empty is a REJECT — so
//! in an eligible design activities are 1:1 with `ir.processes`,
//! `tie == proc == template`, and the resume block is always the process entry.
//! A ready entry collapses to the process id, and `push_sorted`'s tie ordering
//! collapses to ascending process id inside one wake group (`NativeReady::seq`
//! orders the groups and the resume events).
//!
//! ⚠️ Clocking USED to be rejected too, and this file said the engine's
//! `commit_clocking` intercept — which consumes an edge and returns before the
//! process is queued — "has no analogue here". Slice #1 admitted the family and
//! built the analogue: `is_clock_handler` diverts at that exact position. The
//! `fork_modes` simplification above is still load-bearing.
//!
//! ## The three rules that are NOT simplifications
//!
//! 1. **Fire from the intra-slot MASK**, not from an endpoint compare — that is
//!    the whole point of `slot_edge`: an A→B→A clock pulse still ticks once.
//! 2. **A busy process is skipped.** A static edge registration is permanent
//!    (never deregistered), but IEEE does not re-enter an `always` until it
//!    completes and re-arms; a process suspended mid-body is woken through the
//!    waiter path instead (S1d-4). ⚠️ **`busy` has no maintainer here and no
//!    test can set it** — nothing suspends until S1d-4 runs bodies. It is NOT
//!    hypothetical: `always @(posedge clk) begin … @(negedge rst); … end` is
//!    S0-eligible AND arena-buildable (measured), and the engine's busy guard
//!    suppresses a wake for it. S1d-4 owes the maintainer, and those same
//!    designs owe the in-body waiter model this table has no analogue for.
//!
//! 3. **Self-write suppression + timestep dedup.** A process does not fire on a
//!    net it itself blocking-wrote (it saw the value before re-arming), and a
//!    process woken once in a timestep is not re-woken by a later delta's edge
//!    of the same sensitivity (the gated-clock rule) — which is why `seen` is
//!    reset at time advance and at each new event cluster, NOT per delta.
//!
//! ⚠️ A "GATE DEPENDENCY" used to be named here — that the engine reads
//! `last_blocking_writer` LIVE at propagate time while this reads the value
//! snapshotted into the changed tuple, so admitting clocking or force would
//! break the equivalence. Slice #1 had to answer it and MEASURED IT FALSE: the
//! engine snapshots too, in `propagate_changes`'s `edges.extend(…)` prologue,
//! at the same point `take_changed` snapshots here. There is no live read to
//! diverge from, and a clocking commit performed inside the pass lands in the
//! NEXT changed set on both sides. §4.5.338 again — a stated reason does not
//! know when it stopped being true, and this one was never true.

use sim_ir::{EdgeKind, SensKind, SimIr};

use crate::native::dirty::ChangedNet;

/// Static edge + level registrations plus the per-timestep wake bookkeeping.
pub struct WakeTable {
    /// net → [(edge kind, process)] for statically edge-sensitive processes.
    net_to_edge: Vec<Vec<(EdgeKind, u32)>>,
    /// net → [process] for statically LEVEL-sensitive processes (`always @(a)`,
    /// `always @(a or b)`). The engine wakes these through the waiter `retain`
    /// (pass (b)) rather than the edge map, and the gate found the omission on
    /// its first run: a level-sensitive design woke one process on the engine
    /// and none here.
    net_to_level: Vec<Vec<u32>>,
    /// A static level waiter is CONSUMED when it fires — the engine's `retain`
    /// removes it, and it returns only when the process completes and re-arms.
    /// Until bodies can run (S1d-4) nothing re-arms, so this only ever falls;
    /// modelling it anyway is what keeps the decision comparable.
    level_armed: Vec<bool>,
    /// CHANGE SEQUENCE at the static level waiter's arm — the engine's
    /// `Waiter::arm_seq` for the waiter `arm_sensitivity` pushes: 0 at time 0,
    /// the sequence at the completion that re-armed it. A net changed at or
    /// below it is not this waiter's event (a write made earlier in the batch
    /// the process ran in).
    level_arm_seq: Vec<u64>,
    /// Does this process have a NON-EMPTY sensitivity read set?
    ///
    /// `arm_sensitivity` builds its waiter only `if !nets.is_empty()`, so a
    /// process whose inferred read set is empty (`always_comb o = 1'b0;`, a
    /// self-timed bare `always`) registers nothing and re-arming it must register
    /// nothing either.
    ///
    /// Derived for EVERY kind, not only the level-ish ones, and that is
    /// deliberate: with it keyed on kind as well, an `Edge` process was
    /// unarmable for TWO independent reasons, and a mutation that let `k_rearm`
    /// arm an Edge process became invisible — the guard silently covered for the
    /// match. One condition per question: this answers "is there a read set",
    /// `k_rearm`'s match answers "should this kind re-arm at all".
    has_level_nets: Vec<bool>,
    /// Per-process: suspended mid-body. S1d-4 maintains it; all-false until
    /// bodies can suspend, which is why rule 2 above is stated rather than
    /// merely implemented.
    pub busy: Vec<bool>,
    /// TIMESTEP-scoped dedup markers, and the touched list so the reset is
    /// O(#woken) rather than O(#processes).
    seen: Vec<bool>,
    marked: Vec<u32>,
    /// Is this process a marked CLOCKING-COMMIT handler (`always @(clk);` with a
    /// Null body, minted by elaborate for a `clocking` block)?
    ///
    /// It is answered HERE rather than by the caller because the engine answers
    /// it here — inside pass (a), after the fire/busy/self-write tests and
    /// BEFORE the multi-net dedup, then `continue`s. Both halves of that
    /// position matter: a handler must not be queued to Active (its body is a
    /// no-op), and it must not enter `seen`/`marked` either, because the engine's
    /// `continue` runs before the marker is set.
    is_clock_handler: Vec<bool>,
}

impl WakeTable {
    pub(crate) fn new(ir: &SimIr, st: &crate::state::SimState) -> WakeTable {
        let mut net_to_edge = vec![Vec::new(); ir.nets.len()];
        for (pi, p) in ir.processes.iter().enumerate() {
            if p.sensitivity.kind == SensKind::Edge {
                for et in &p.sensitivity.edges {
                    if (et.net as usize) < net_to_edge.len() {
                        net_to_edge[et.net as usize].push((et.kind, pi as u32));
                    }
                }
            }
        }
        // THREE kinds feed the engine's `arm = None` Level waiter, not one:
        // `arm_sensitivity` treats `Level | Comb | Latch` identically, because
        // `always_comb`/`always_latch`/`@(*)` carry their elaborate-inferred read
        // set in the same `edges` list. Registering only `Level` left every
        // combinational process — the most common synthesizable shape, and what
        // the corpus's own `gen_comb_chain` template emits — permanently asleep.
        let mut net_to_level = vec![Vec::new(); ir.nets.len()];
        let mut has_level_nets = vec![false; ir.processes.len()];
        for (pi, p) in ir.processes.iter().enumerate() {
            has_level_nets[pi] = !p.sensitivity.edges.is_empty();
            if matches!(
                p.sensitivity.kind,
                SensKind::Level | SensKind::Comb | SensKind::Latch
            ) {
                for et in &p.sensitivity.edges {
                    if (et.net as usize) < net_to_level.len() {
                        net_to_level[et.net as usize].push(pi as u32);
                    }
                }
            }
        }
        WakeTable {
            net_to_edge,
            net_to_level,
            // …and their t0 ARM STATE agrees too: `arm_processes` ARMS a `Level`
            // block (it waits for the first event) and, since §4.5.584, a
            // `Comb`/`Latch` block as well — its implicit time-0 pass waits for
            // the first promotion (`NativeKernel::t0_implicit`), and a change
            // before it, the time-0 settle's included, must wake it as it wakes a
            // level block. Before, a Comb/Latch block was queued into the first
            // batch and had no waiter until that run re-armed it.
            //
            // ⚠️ `&& !edges.is_empty()` — the engine's `arm_sensitivity` pushes a
            // waiter only for a non-empty read set, so a process with an EMPTY
            // read set has no waiter there and must have none here. The
            // condition was latent until `always @*` became `Level`: every prior
            // `Level` came from an explicit `@(a or b)`, which always names nets.
            // Caught by `s1d4c2a_rearm_matches_the_engine_on_both_halves_of_the_asymmetry`.
            level_armed: ir
                .processes
                .iter()
                .map(|p| {
                    matches!(
                        p.sensitivity.kind,
                        SensKind::Level | SensKind::Comb | SensKind::Latch
                    ) && !p.sensitivity.edges.is_empty()
                })
                .collect(),
            level_arm_seq: vec![0; ir.processes.len()],
            has_level_nets,
            busy: vec![false; ir.processes.len()],
            seen: vec![false; ir.processes.len()],
            marked: Vec::new(),
            is_clock_handler: {
                // Derived from the SAME two tables `clocking_commit_plan` reads,
                // so "which procs are handlers" has one spelling: a second one
                // that drifted would divert a proc the plan then declines (a
                // process silently never run) or fail to divert one it accepts
                // (its no-op body queued, and worse, marked `seen`).
                let mut v = vec![false; ir.processes.len()];
                for &p in st.clocking_commit.keys().chain(st.clocking_outputs.keys()) {
                    if (p as usize) < v.len() {
                        v[p as usize] = true;
                    }
                }
                v
            },
        }
    }

    /// Does this net's accumulated mask satisfy `kind`? The engine's
    /// `edge_fires_slot`, shared rather than restated.
    #[inline]
    fn fires(mask: u8, kind: EdgeKind) -> bool {
        crate::sched::edge_fires_slot(mask, kind)
    }

    /// Which processes this changed set wakes, in the order the engine queues
    /// them (ascending tie == ascending process id).
    ///
    /// The output is SORTED here, and that sort is load-bearing: the engine
    /// inserts each woken process by tie, so the queue is tie-ascending
    /// regardless of the order its changed nets are visited in. Measurement
    /// agrees — reversing this function's input changes nothing, because the
    /// sort makes the input order irrelevant. (An earlier version of this
    /// comment claimed the two orders "agree by construction"; a mutation that
    /// reversed the input and still passed showed that was not the reason.)
    pub fn wake(
        &mut self,
        changed: &[ChangedNet],
        last_change_seq: &[u64],
        out: &mut Vec<u32>,
        clocked: &mut Vec<u32>,
    ) {
        out.clear();
        clocked.clear();
        for &(net, mask, writer) in changed {
            for k in 0..self.net_to_edge[net as usize].len() {
                let (kind, proc) = self.net_to_edge[net as usize][k];
                if !Self::fires(mask, kind) {
                    continue;
                }
                if self.busy[proc as usize] || writer == proc {
                    continue;
                }
                // N4 clocking: a marked commit handler fired on its clocking
                // edge. Diverted to `clocked` — the caller performs the commit —
                // and NOT queued, exactly where the engine's `commit_clocking`
                // intercept sits. Deliberately BEFORE the `seen` test, so a
                // handler registered on two nets that both fire in one delta
                // commits twice here as it does there; the engine's `continue`
                // is likewise reached before its marker is set.
                if self.is_clock_handler[proc as usize] {
                    clocked.push(proc);
                    continue;
                }
                if self.seen[proc as usize] {
                    continue;
                }
                self.seen[proc as usize] = true;
                self.marked.push(proc);
                out.push(proc);
            }
        }
        // Static LEVEL sensitivity (the engine's pass (b), static arm): fires
        // when ANY watched net changed AFTER the arm (CHANGE SEQUENCE) and the
        // waiter's own process did not blocking-write it. No timestep dedup —
        // the waiter is CONSUMED instead, which is a stronger condition and the
        // one the engine uses.
        for &(net, _, writer) in changed {
            for k in 0..self.net_to_level[net as usize].len() {
                let proc = self.net_to_level[net as usize][k];
                if !self.level_armed[proc as usize]
                    || writer == proc
                    || last_change_seq[net as usize] <= self.level_arm_seq[proc as usize]
                {
                    continue;
                }
                self.level_armed[proc as usize] = false;
                out.push(proc);
            }
        }
        out.sort_unstable();
    }

    /// Re-arm a static level waiter — what the engine does when the process
    /// completes and `arm_sensitivity` runs again (`k_rearm` is the caller).
    ///
    /// The EMPTY-READ-SET guard is not defensive: `arm_sensitivity` builds its
    /// waiter only `if !nets.is_empty()`, so a process with no inferred reads
    /// registers nothing and re-arming it must register nothing either. Without
    /// this the two diverged on `always_comb o = 1'b0;` — engine "no live
    /// waiter", kernel "armed" — measured on a design the gate reports eligible
    /// and buildable. It was invisible only because `net_to_level` has no entry
    /// for such a process either, so `wake` never read the bit; that is an
    /// accident, not a design, and 4c-2's `busy` / quiescence work reads this
    /// state directly.
    ///
    /// Calling it twice without an intervening fire leaves one `true`, and so
    /// does the engine: `arm_sensitivity` keeps at most one live static waiter
    /// per activity and refreshes its `arm_seq` on a second arm
    /// (`Activity::static_level_live`); it used to push a second (measured
    /// 1 → 2 → 3). In a run every activation start consumes the arm (a fire, or
    /// `take_t0_trigger`), so the sequence is the unit differential's, not a
    /// run's.
    /// `n_level_waiters` is absent here; it is a fast-path counter guarding
    /// whether the engine runs its level pass at all, and this table always runs
    /// its own.
    pub fn rearm_level(&mut self, proc: u32, arm_seq: u64) {
        if !self.has_level_nets[proc as usize] {
            return;
        }
        self.level_armed[proc as usize] = true;
        self.level_arm_seq[proc as usize] = arm_seq;
    }

    /// Deliver the implicit time-0 pass of `always_comb` / `always_latch` `proc` as a
    /// trigger — the engine's `Scheduler::take_t0_trigger`, which carries the
    /// argument: `Drop` when `busy`; `Run` with no read set or a live static arm,
    /// which starting the pass consumes; otherwise `RunSuperseding` (woken, its start
    /// pending in `active`). A base activity's id is its process id here (`arm_t0`
    /// queues base activities only).
    pub(crate) fn take_t0_trigger(&mut self, proc: u32) -> crate::sched::T0Trigger {
        use crate::sched::T0Trigger;
        let p = proc as usize;
        if self.busy[p] {
            return T0Trigger::Drop;
        }
        if !self.has_level_nets[p] {
            return T0Trigger::Run;
        }
        if !self.level_armed[p] {
            return T0Trigger::RunSuperseding;
        }
        self.level_armed[p] = false;
        T0Trigger::Run
    }

    /// The kernel-side twin of `Scheduler::edge_registration_count`.
    #[cfg(test)]
    pub fn edge_registration_count(&self, proc: u32) -> usize {
        self.net_to_edge
            .iter()
            .flat_map(|v| v.iter())
            .filter(|(_, p)| *p == proc)
            .count()
    }

    /// Read/write the arm state of one process — the differential's only way to
    /// observe what `k_rearm` did. Test-only: production reads it through `wake`.
    #[cfg(test)]
    pub fn level_armed_for_test(&self, proc: u32) -> bool {
        self.level_armed[proc as usize]
    }

    #[cfg(test)]
    pub fn set_level_armed_for_test(&mut self, proc: u32, v: bool) {
        self.level_armed[proc as usize] = v;
    }

    /// A new event cluster (time advance, a `#0` batch, the NBA region) — the
    /// dedup markers are TIMESTEP-scoped, so an edge produced by the new cluster
    /// must be able to re-fire a process already woken this timestep.
    pub fn reset_edge_seen(&mut self) {
        for p in self.marked.drain(..) {
            self.seen[p as usize] = false;
        }
    }
}
