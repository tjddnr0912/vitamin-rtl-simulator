//! split part of `sched` (mechanical move).

use super::*;

impl Scheduler<'_, '_> {
    /// Execute `wait fork;` (IEEE §9.6.1). Counts this process's outstanding
    /// immediate children — every live (non-dead, non-reported) child activity
    /// whose join barrier names this parent (covers the CUMULATIVE set across
    /// all prior `fork ... join_none` and surplus `join_any` children). Returns
    /// `true` if the parent may continue THIS activation (zero outstanding), or
    /// `false` after parking it (`on_child_complete` re-enqueues at `resume_bb`).
    pub(crate) fn exec_wait_fork(&mut self, parent_aid: u32, resume_bb: u32) -> bool {
        let barriers = &self.barriers;
        let outstanding = self
            .activities
            .iter()
            .filter(|c| {
                c.is_child
                    && !c.dead
                    && !c.reported
                    && c.join_ref
                        .is_some_and(|jr| barriers[jr as usize].parent == parent_aid)
            })
            .count() as u32;
        if outstanding == 0 {
            return true; // no live children → fall through immediately
        }
        self.activities[parent_aid as usize].wait_fork = Some(WaitForkPark {
            resume_bb,
            outstanding,
        });
        false // parked; on_child_complete resumes the parent at resume_bb
    }

    /// A fork child has reached its barrier's join_bb. Decrement and, on the firing
    /// condition for the mode, re-enqueue the parent at `resume_bb` exactly once.
    pub(crate) fn on_child_complete(&mut self, join_ref: u32, child_aid: u32) {
        // Per-child fire-once: a child may reach its join at most once. A second
        // report would under-decrement `outstanding` and fire an All-barrier EARLY.
        debug_assert!(
            !self.activities[child_aid as usize].reported,
            "internal error: child {child_aid} reported completion twice"
        );
        self.activities[child_aid as usize].reported = true;
        // P3-1: the reporting child is DEAD past this point (its run_process
        // returns Step::Done right after; children never re-arm) — recycle.
        self.free_activities.push(child_aid);

        // v8 `wait fork`: capture the forking parent BEFORE the barrier may be
        // recycled below — used by the wait-fork hook at the end.
        let parent_aid = self.barriers[join_ref as usize].parent;

        let b = &mut self.barriers[join_ref as usize];
        debug_assert!(
            b.outstanding > 0,
            "internal error: barrier {join_ref} outstanding underflow"
        );
        b.outstanding -= 1;
        if b.outstanding == 0 {
            // Every child has reported: nothing references this barrier anymore
            // (the parent resume below reads its fields by value) — recycle.
            self.free_barriers.push(join_ref);
        }
        let fire = match b.mode {
            JoinMode::All => b.outstanding == 0, // last child
            JoinMode::Any => true,               // first child (later guarded by `fired`)
            JoinMode::None => false,             // never (parent already continued)
        };
        if fire && !b.fired {
            b.fired = true;
            let parent = b.parent;
            let resume_bb = b.resume_bb;
            // Stage-1 fork-in-frame: if the parent forked from INSIDE a suspendable task
            // frame, it resumes at the frame's PC — set the top frame's `bb` here (the
            // `Ready.block` below is IGNORED while in_frame; `run_process` reads the frame
            // bb). A top-level (non-frame) parent has an empty call_stack → no-op, and the
            // `Ready.block = resume_bb` drives it exactly as before (byte-identical).
            if let Some(f) = self.activities[parent as usize].call_stack.last_mut() {
                f.bb = resume_bb;
            }
            let tie = self.activities[parent as usize].tie;
            // Re-enqueue the parent at resume_bb THIS instant (Active region).
            // Surplus children (join_any) stay live and run to completion; their
            // later on_child_complete sees `fired == true` → no-op.
            push_sorted(
                &mut self.cur.active,
                Ready {
                    tie,
                    proc: parent,
                    block: resume_bb,
                },
            );
        }

        // v8 `wait fork`: this completion also counts against the parent's
        // parked wait-fork set (a join_none/join_any-surplus child reports to
        // its OWN barrier above, but its parent may be blocked on `wait fork`).
        // Decrement and resume the parent once its last child reports.
        let wf_resume = {
            let pa = &mut self.activities[parent_aid as usize];
            if let Some(wf) = pa.wait_fork.as_mut() {
                wf.outstanding = wf.outstanding.saturating_sub(1);
                if wf.outstanding == 0 {
                    Some(wf.resume_bb)
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(resume_bb) = wf_resume {
            self.activities[parent_aid as usize].wait_fork = None;
            let tie = self.activities[parent_aid as usize].tie;
            push_sorted(
                &mut self.cur.active,
                Ready {
                    tie,
                    proc: parent_aid,
                    block: resume_bb,
                },
            );
        }
    }
}
// ── helpers ──────────────────────────────────────────────────────────────

/// Insert keeping sorted by `tie` (stable; equal ties keep insertion order).
pub(crate) fn push_sorted(q: &mut Vec<Ready>, r: Ready) {
    let pos = q.partition_point(|x| x.tie <= r.tie);
    q.insert(pos, r);
}

/// Child tie = `(parent_tie+1)` in the high 16 bits, child declaration index in
/// the low 16. `parent` is ALWAYS a top-level process (nested fork is an
/// elaborate ERROR), so `parent_tie ∈ [0, nproc)` is a small dense int and the
/// shift is applied EXACTLY ONCE — never chained. The `+1` offset makes children
/// sort STRICTLY AFTER their parent for all `parent_tie` (including 0), while
/// preserving relative parent ordering and declaration order among siblings. v1
/// limits — ≤ 65534 top-level processes and ≤ 65536 children per fork — are
/// ENFORCED at the spawn site (FORK-TIE-CAP in `exec_fork`); above them the high
/// or low half would overflow/alias, so this helper is only ever reached within
/// the safe range.
pub(crate) fn compose_child_tie(parent_tie: u32, child_idx: u32) -> u32 {
    ((parent_tie + 1) << 16) | (child_idx & 0xFFFF)
}

/// GLITCH: edge firing from a net's intra-slot `slot_edge` accumulator mask
/// (bit0 = posedge occurred, bit1 = negedge, bit2 = bit0 changed at all). For a
/// net written ONCE in the slot the mask is exactly `{is_posedge, is_negedge,
/// prev!=cur}` of the single `prev→cur` transition, so this returns the same as
/// `edge_fires(kind, prev, cur)` — only an A→B→A glitch (which the endpoint
/// compare loses) diverges, recovering the IEEE §9 "fire once per slot".
pub(crate) fn edge_fires_slot(mask: u8, kind: EdgeKind) -> bool {
    match kind {
        EdgeKind::Posedge => mask & 1 != 0,
        EdgeKind::Negedge => mask & 2 != 0,
        EdgeKind::AnyEdge => mask & 4 != 0,
    }
}

/// MULTI-DRIVER: fold one driver value `d` into accumulator `acc` by IEEE 1364
/// 4-state WIRE resolution, bitwise per word. Encoding (val,unk): (0,0)=0,
/// (1,0)=1, (0,1)=X, (1,1)=Z. Z (=val&unk) is the identity — it yields to the
/// other driver; two equal non-Z bits keep the value; two differing non-Z bits
/// (a 0/1 conflict, or any X) resolve to X. Commutative + associative, so
/// folding every driver from an all-Z start is order-independent (matches the
/// oracle table verified across all 16 (a,b) pairs). `acc` and `d` share width.
pub(crate) fn resolve_wire_into(acc: &mut Value, d: &Value) {
    let n = acc.val.len();
    for w in 0..n {
        let av = acc.val[w];
        let au = acc.unk[w];
        let bv = d.val.get(w).copied().unwrap_or(0);
        let bu = d.unk.get(w).copied().unwrap_or(0);
        let az = av & au; // acc bit is Z
        let bz = bv & bu; // driver bit is Z
        let same = !(av ^ bv) & !(au ^ bu); // (av,au) == (bv,bu)
        let take_b = az; // acc Z → take the driver
        let take_a = !az & (bz | same); // driver Z, or equal → keep acc
        let x_bits = !az & !bz & !same; // both non-Z and differ → X
        acc.val[w] = (take_b & bv) | (take_a & av);
        acc.unk[w] = (take_b & bu) | (take_a & au) | x_bits;
    }
    acc.mask_top();
}

/// MULTI-DRIVER (WAND): fold a driver into `acc` by IEEE wired-AND resolution
/// (oracle-verified 16 pairs). Z is the identity; a 0 on any driver forces 0;
/// two 1s give 1; anything else with an X gives X.
pub(crate) fn resolve_wand_into(acc: &mut Value, d: &Value) {
    for w in 0..acc.val.len() {
        let (av, au) = (acc.val[w], acc.unk[w]);
        let bv = d.val.get(w).copied().unwrap_or(0);
        let bu = d.unk.get(w).copied().unwrap_or(0);
        let az = av & au;
        let bz = bv & bu;
        let take_b = az;
        let take_a = !az & bz;
        let rest = !az & !bz;
        let a0 = !av & !au;
        let b0 = !bv & !bu;
        let a1 = av & !au;
        let b1 = bv & !bu;
        let one = rest & a1 & b1;
        let xx = rest & !(a0 | b0) & !(a1 & b1);
        acc.val[w] = (take_b & bv) | (take_a & av) | one;
        acc.unk[w] = (take_b & bu) | (take_a & au) | xx;
    }
    acc.mask_top();
}

/// MULTI-DRIVER (WOR): fold a driver into `acc` by IEEE wired-OR resolution.
/// Z is the identity; a 1 on any driver forces 1; two 0s give 0; else X.
pub(crate) fn resolve_wor_into(acc: &mut Value, d: &Value) {
    for w in 0..acc.val.len() {
        let (av, au) = (acc.val[w], acc.unk[w]);
        let bv = d.val.get(w).copied().unwrap_or(0);
        let bu = d.unk.get(w).copied().unwrap_or(0);
        let az = av & au;
        let bz = bv & bu;
        let take_b = az;
        let take_a = !az & bz;
        let rest = !az & !bz;
        let a0 = !av & !au;
        let b0 = !bv & !bu;
        let a1 = av & !au;
        let b1 = bv & !bu;
        let one = rest & (a1 | b1);
        let xx = rest & !(a1 | b1) & !(a0 & b0);
        acc.val[w] = (take_b & bv) | (take_a & av) | one;
        acc.unk[w] = (take_b & bu) | (take_a & au) | xx;
    }
    acc.mask_top();
}

/// S1: effective inertial delay for a delayed continuous-assign / gate-prim
/// value change `old → new` with distinct rise/fall/turnoff specs (IEEE 1364
/// §7.14 / §28 — confirmed against iverilog 13.0 as ATOMIC-at-max, not per-bit
/// separate arrival). The whole net updates at `now + D` where `D` is the MAX,
/// over every bit `i` that actually changes (`old[i] != new[i]`), of that bit's
/// DESTINATION-based delay:
///   new bit → 1  : `rise`
///   new bit → 0  : `fall`
///   new bit → z  : `turnoff`
///   new bit → x  : `min(rise, fall, turnoff)`
/// First drive (`old == None`) treats ALL bits as changed. `get_vu` 4-state
/// encoding: (0,0)=0, (1,0)=1, (0,1)=X, (1,1)=Z. This is only reached when the
/// value actually changed, so at least one bit differs; the `rise` fallback for
/// the impossible no-change case is harmless.
pub(crate) fn transition_delay(
    old: Option<&Value>,
    new: &Value,
    rise: u32,
    fall: u32,
    toff: u32,
) -> u32 {
    let x_delay = rise.min(fall).min(toff);
    let mut d: Option<u32> = None;
    for i in 0..new.width {
        let (nv, nu) = new.get_vu(i);
        // A bit is "changed" if there is no prior value (first drive) or the old
        // bit differs from the new bit.
        let changed = match old {
            None => true,
            Some(o) => {
                // Compare only within the old value's width; bits beyond it (the
                // first-drive case for wider news) count as changed.
                if i < o.width {
                    o.get_vu(i) != (nv, nu)
                } else {
                    true
                }
            }
        };
        if !changed {
            continue;
        }
        let bit_d = match (nv, nu) {
            (1, 0) => rise, // → 1
            (0, 0) => fall, // → 0
            (1, 1) => toff, // → z
            _ => x_delay,   // → x  (0,1)
        };
        d = Some(d.map_or(bit_d, |m| m.max(bit_d)));
    }
    d.unwrap_or(rise)
}
