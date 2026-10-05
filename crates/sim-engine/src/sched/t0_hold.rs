//! The time-0 HOLD of the continuous assigns that reach an effectful call (ROADMAP §2
//! 🆕 AB, §4.5.590): such an assign is not evaluated before the first time-0 batch has
//! run, so the function it calls runs at time 0 on the inputs the time-0 processes
//! wrote rather than first on the declared defaults — exceptions under **The release**.
//!
//! **What it fixes.** The time-0 settle evaluates every continuous assign before any
//! process has run, so `assign y = f(a);` called `f` on `a = x` and then again once the
//! `initial` wrote `a`. A pure `f` leaves nothing behind, but a `$display` printed one
//! extra line, a `unique case` reported a miss at time 0, an `assert` or `$error`
//! failed and set exit 1, a `$fatal` ended the run. iverilog 13 and verilator 5.052
//! both call `f` once at time 0 with the written value (`f t=0 x=0 z=1`, then the
//! design's own lines) and stay silent.
//!
//! **Which assigns are held** ([`T0Hold::build`]). The seed set is every assign whose
//! rhs or lhs index reaches a user call AND (a reached callee is not effect-free —
//! `levelize::func_effect_free`: its body declines `func_read_deps`' walk, or a system
//! task is reachable from it — OR the assign reads a heap handle, through which a class
//! method's body is out of the walk's sight), minus the assign PRE evaluates exactly
//! once: certified by `ca_deps` with an EMPTY read set and not a multi-driver member
//! (nothing a time-0 process writes can reach it, so holding it would only reorder a
//! race neither oracle decides — `f(1'b0, 1'b1)`: verilator calls it before the
//! `initial`, iverilog after). The held set is then closed downstream: every assign
//! reading a net a held assign drives is held too, so no assign settles on a held net's
//! declared default (`assign v = (w === 1'bz);` below a held `w` settled `v = 1`, the
//! first batch saw a posedge iverilog never fires, and the release took it back).
//!
//! **Where the hold applies.** Every settle before the first batch is taken — the seed
//! settle, the initializer re-settle and the loop-top settle in front of the first
//! batch — skips a held assign (the wave that releases it marks it dirty; it is not
//! kept on the worklist meanwhile); so do the multi-driver resolution (a group with a
//! held member), `schedule_delayed_cas` and the copy-net repair. A held
//! DELAYED assign still drives its initial `x` (iverilog reads `x` during `[0, d)`),
//! sized from the lvalue and the rhs's static width without evaluating the rhs.
//!
//! **The release** ([`Scheduler::settle_releasing_t0`], tier-3 twin
//! `native::run::settle_releasing_t0`). The first settle after the first batch is taken
//! (at run start when there is none) settles the non-held assigns to a fixpoint, then
//! releases the held ones in dependency WAVES, settling after each: a held assign's wave
//! comes once every OTHER held assign driving a net in its read set is released. A chain
//! of held assigns through distinct nets, fed by no cycle, evaluates each link once, on
//! its settled input. Three shapes do not (ROADMAP §2 🆕 AB residue):
//! - a CYCLE of held assigns: when no held assign is ready the remaining ones — the cycle
//!   and everything waiting on it — are released in one wave, and the fixpoint runs
//!   them in declaration order, so a link downstream of the cycle can run on a held
//!   net's `z` before the cycle settles (`assign o2 = f(o1, 6);` below a three-assign
//!   ring: `f6 t=0 v=z`, then again on `x` and `0`);
//! - the order is kept per NET, not per bit or element, so a chain whose links read and
//!   drive one shared vector or array net (`assign c[k+1] = f(c[k]);`, generate
//!   instances chained through `o[k]`) is such a cycle: five links run `f` 9 times
//!   (iverilog 10, verilator 13), six instances 18 times (both oracles 6);
//! - a callee `func_read_deps` declines contributes no body reads to the assign's read
//!   set, so its wave can come before the held assign driving a net that body reads:
//!   `f2` reading `w1` in its body ran on `w1 = z` and again on `0` (both oracles once,
//!   on `0`; the pre-hold binary ran it on `x` first too).
//!
//! A held UNCERTIFIED assign (in `ca_always`: delayed, multi-driver member, impure or
//! heap-reading) is visited by every pass of every later wave once released, as
//! `ca_always` is visited by every pass of every settle; a chain of N such links is
//! evaluated O(N²) times during the release. A net first dirtied by the waves
//! with no definite bit leaves the change list: a `z → x` hop is no event for iverilog,
//! the arming's T0 X-DROP rule (`arm_processes`), applied again here because the hold
//! moved those writes past the arming.
//!
//! **Cost and byte identity.** Building the held set and its wave graph, and the wave
//! bookkeeping of the whole release, are linear in the assigns, their read sets and their
//! lvalue chunks. Each wave's settle visits what that wave marked dirty and what that
//! moved, the held members of `ca_always` (released ones evaluated, unreleased ones
//! dropped), the multi-driver groups with a held member and the held delayed assigns —
//! nothing else, since by the closure no other assign reads a held net — so the
//! release is linear in the held graph except for the held uncertified assigns above.
//! With an empty seed set `active` is false from construction and every settle is the
//! pre-hold fixpoint, monomorphised without a hold test (`settle_cont_assigns_inner::
//! <false>`, one test per settle call); the release never happens. A design with no
//! call at all is not even walked.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// The hold's per-design state — built once by [`T0Hold::build`] in `Scheduler::new`,
/// read by both settle loops through the `Scheduler` (boxed there, so the hot
/// scheduler struct grows by one pointer). Its vectors stay empty when nothing is held.
pub(crate) struct T0Hold {
    /// Per continuous assign: held until the release.
    held: Vec<bool>,
    /// Per continuous assign: released by a wave.
    released: Vec<bool>,
    /// Per held assign: the distinct held-driven nets it drives.
    drives: Vec<Vec<u32>>,
    /// Per held assign: how many held-driven nets in its read set still wait for an
    /// unreleased held driver other than itself. Zero = its wave may come.
    unsat: Vec<u32>,
    /// Per held-driven net: its unreleased held drivers, and the held assigns reading it.
    nets: BTreeMap<u32, HeldNet>,
    /// The held assigns whose `unsat` has reached zero and that wait for the next wave.
    ready: Vec<u32>,
    /// Held assigns not yet released.
    pending: usize,
    /// Some assign is held and the release has not finished.
    active: bool,
    /// The first time-0 batch has been taken (or there is none): the next settle
    /// releases.
    releasing: bool,
    /// The release's waves are running (`settle_releasing_t0`, after its first settle).
    in_waves: bool,
    /// The held members of `ca_always`, ascending: the always-visited set inside the
    /// waves.
    held_always: Vec<u32>,
    /// The multi-driver groups (indices into `md_groups`) with a held member: the
    /// groups resolved inside the waves.
    md_held: Vec<usize>,
    /// The held delayed assigns, ascending: the ones `schedule_delayed_cas` visits
    /// inside the waves.
    held_delayed: Vec<u32>,
}

/// A net some held assign drives.
struct HeldNet {
    /// Its held drivers not yet released.
    rem: u32,
    /// Its held drivers.
    drivers: Vec<u32>,
    /// The held assigns whose read set contains it.
    readers: Vec<u32>,
}

impl T0Hold {
    /// Nothing held: every query answers "not held", nothing is allocated.
    fn inactive() -> T0Hold {
        T0Hold {
            held: Vec::new(),
            released: Vec::new(),
            drives: Vec::new(),
            unsat: Vec::new(),
            nets: BTreeMap::new(),
            ready: Vec::new(),
            pending: 0,
            active: false,
            releasing: false,
            in_waves: false,
            held_always: Vec::new(),
            md_held: Vec::new(),
            held_delayed: Vec::new(),
        }
    }

    /// The held set and its wave order. `deps` is `levelize::ca_deps`' answer (read set,
    /// certified) per assign, `windows` the frame layout and `heap` the heap-handle test
    /// that call takes, `ca_md` the multi-driver membership. Linear in the assigns, their
    /// read sets and their lvalue chunks.
    pub(crate) fn build(
        ir: &sim_ir::SimIr,
        deps: &[(BTreeSet<u32>, bool)],
        windows: &[(u32, u32)],
        heap: &dyn Fn(u32) -> bool,
        ca_md: &[bool],
    ) -> T0Hold {
        // No call anywhere in the design: nothing can be held, and nothing is walked.
        if !ir
            .exprs
            .iter()
            .any(|e| matches!(e, sim_ir::Expr::Call { .. }))
        {
            return T0Hold::inactive();
        }
        let nca = ir.cont_assigns.len();
        // Per assign: the callees its rhs and lhs index expressions reach, or `None`
        // when the walk met an expression id it cannot resolve (held).
        let reached: Vec<Option<Vec<u32>>> = ir
            .cont_assigns
            .iter()
            .map(|c| {
                let mut out = Vec::new();
                let index = c
                    .lhs
                    .chunks
                    .iter()
                    .flat_map(|k| [k.word, k.offset, k.width]);
                let ok = reached_callees(ir, c.rhs, &mut out)
                    && index.flatten().all(|e| reached_callees(ir, e, &mut out));
                ok.then_some(out)
            })
            .collect();
        if !reached
            .iter()
            .any(|r| r.as_ref().is_none_or(|v| !v.is_empty()))
        {
            return T0Hold::inactive();
        }
        let free = crate::levelize::func_effect_free(ir, windows, heap);
        let mut held = vec![false; nca];
        for ci in 0..nca {
            let effectful = match &reached[ci] {
                None => true,
                Some(v) if v.is_empty() => continue,
                Some(v) => v
                    .iter()
                    .any(|&f| !free.get(f as usize).copied().unwrap_or(false)),
            };
            let (reads, certified) = &deps[ci];
            let heap_read = reads.iter().any(|&n| heap(n));
            let once = *certified && reads.is_empty() && !ca_md[ci];
            held[ci] = (effectful || heap_read) && !once;
        }
        let mut queue: Vec<u32> = (0..nca as u32).filter(|&ci| held[ci as usize]).collect();
        if queue.is_empty() {
            return T0Hold::inactive();
        }
        // The downstream closure: an assign reading a net a held assign drives is held,
        // transitively (a worklist over the per-net readers, each assign queued once).
        let mut readers_of: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (ci, (reads, _)) in deps.iter().enumerate() {
            for &n in reads {
                readers_of.entry(n).or_default().push(ci as u32);
            }
        }
        while let Some(ci) = queue.pop() {
            for k in &ir.cont_assigns[ci as usize].lhs.chunks {
                for &r in readers_of.get(&k.net).map_or(&[][..], Vec::as_slice) {
                    if !held[r as usize] {
                        held[r as usize] = true;
                        queue.push(r);
                    }
                }
            }
        }
        // The wave graph, per net. A held assign waits for every OTHER held driver of
        // each net in its read set. By the closure no assign outside the held set reads
        // a held net, so these direct edges are the whole held dependency order — a
        // search through non-held drivers can never reach a held one.
        let mut drives: Vec<Vec<u32>> = vec![Vec::new(); nca];
        let mut nets: BTreeMap<u32, HeldNet> = BTreeMap::new();
        for ci in (0..nca).filter(|&ci| held[ci]) {
            let mut mine: Vec<u32> = ir.cont_assigns[ci]
                .lhs
                .chunks
                .iter()
                .map(|k| k.net)
                .collect();
            mine.sort_unstable();
            mine.dedup();
            for &n in &mine {
                let hn = nets.entry(n).or_insert_with(|| HeldNet {
                    rem: 0,
                    drivers: Vec::new(),
                    readers: Vec::new(),
                });
                hn.rem += 1;
                hn.drivers.push(ci as u32);
            }
            drives[ci] = mine;
        }
        let mut unsat = vec![0u32; nca];
        for ci in (0..nca).filter(|&ci| held[ci]) {
            for &n in &deps[ci].0 {
                if let Some(hn) = nets.get_mut(&n) {
                    hn.readers.push(ci as u32);
                    if hn.rem > u32::from(drives[ci].binary_search(&n).is_ok()) {
                        unsat[ci] += 1;
                    }
                }
            }
        }
        let ready: Vec<u32> = (0..nca as u32)
            .filter(|&ci| held[ci as usize] && unsat[ci as usize] == 0)
            .collect();
        let pending = held.iter().filter(|&&h| h).count();
        T0Hold {
            held,
            released: vec![false; nca],
            drives,
            unsat,
            nets,
            ready,
            pending,
            active: true,
            releasing: false,
            in_waves: false,
            held_always: Vec::new(),
            md_held: Vec::new(),
            held_delayed: Vec::new(),
        }
    }

    /// The lanes the waves visit, from the scheduler's own tables once it has built
    /// them: the held members of `ca_always`, the multi-driver groups with a held
    /// member, the held delayed assigns. Nothing when nothing is held.
    pub(crate) fn index_lanes(
        &mut self,
        ca_always: &[u32],
        md_groups: &[(u32, Vec<usize>, u8)],
        delayed: &[u32],
    ) {
        if !self.active {
            return;
        }
        self.held_always = ca_always
            .iter()
            .copied()
            .filter(|&ci| self.held[ci as usize])
            .collect();
        self.held_always.sort_unstable();
        self.md_held = (0..md_groups.len())
            .filter(|&mi| md_groups[mi].1.iter().any(|&c| self.held[c]))
            .collect();
        self.held_delayed = delayed
            .iter()
            .copied()
            .filter(|&ci| self.held[ci as usize])
            .collect();
    }

    /// The held members of `ca_always` (inside the waves).
    pub(crate) fn held_always(&self) -> &[u32] {
        &self.held_always
    }

    /// The multi-driver groups with a held member (inside the waves).
    pub(crate) fn md_held_groups(&self) -> &[usize] {
        &self.md_held
    }

    /// The held delayed assigns (inside the waves).
    pub(crate) fn held_delayed(&self) -> &[u32] {
        &self.held_delayed
    }

    /// Are the release's waves running?
    #[inline]
    pub(crate) fn in_waves(&self) -> bool {
        self.in_waves
    }

    /// The release's first settle is done; its waves begin.
    pub(crate) fn begin_waves(&mut self) {
        self.in_waves = self.active;
    }

    /// Some assign is held and the release has not finished. Read ONCE per settle by
    /// both loops (it cannot change inside one), so a design with nothing held pays
    /// no per-assign test.
    #[inline]
    pub(crate) fn active(&self) -> bool {
        self.active
    }

    /// Is assign `ci` held right now?
    #[inline]
    pub(crate) fn held_now(&self, ci: usize) -> bool {
        self.active && self.held[ci] && !self.released[ci]
    }

    /// The first time-0 batch has been taken (or there is none): the next settle
    /// releases. Called by both run loops.
    pub(crate) fn begin_release(&mut self) {
        self.releasing = self.active;
    }

    /// Is the next settle the release?
    #[inline]
    pub(crate) fn release_due(&self) -> bool {
        self.active && self.releasing
    }

    /// The next wave, ascending, marked released: every unreleased held assign none of
    /// whose read nets has an unreleased held driver other than itself (Kahn, counted per
    /// net). When none qualifies — a cycle through held assigns, which includes every
    /// chain whose links read and drive one shared net — the remaining held assigns are
    /// released together. `None` once every held assign is released.
    pub(crate) fn next_wave(&mut self) -> Option<Vec<u32>> {
        if self.pending == 0 {
            return None;
        }
        let mut wave = std::mem::take(&mut self.ready);
        let together = wave.is_empty();
        if together {
            wave = (0..self.held.len() as u32)
                .filter(|&ci| self.held[ci as usize] && !self.released[ci as usize])
                .collect();
        }
        wave.sort_unstable();
        for &ci in &wave {
            self.released[ci as usize] = true;
        }
        self.pending -= wave.len();
        if together {
            return Some(wave);
        }
        // A net is satisfied for a reader once no held driver of it other than the reader
        // itself is unreleased: at `rem == 1` only for the one driver left, if it reads
        // the net; at `rem == 0` for every other reader. Each net crosses each step once,
        // so the whole release is linear in the held graph.
        let mut next = Vec::new();
        for &ci in &wave {
            for i in 0..self.drives[ci as usize].len() {
                let n = self.drives[ci as usize][i];
                let Some(hn) = self.nets.get_mut(&n) else {
                    continue;
                };
                hn.rem -= 1;
                let mut gain: Vec<u32> = Vec::new();
                match hn.rem {
                    1 => {
                        if let Some(&d) = hn.drivers.iter().find(|&&d| !self.released[d as usize]) {
                            if hn.readers.contains(&d) {
                                gain.push(d);
                            }
                        }
                    }
                    0 => {
                        for &r in &hn.readers {
                            if !self.released[r as usize]
                                && self.drives[r as usize].binary_search(&n).is_err()
                            {
                                gain.push(r);
                            }
                        }
                    }
                    _ => {}
                }
                for r in gain {
                    self.unsat[r as usize] -= 1;
                    if self.unsat[r as usize] == 0 {
                        next.push(r);
                    }
                }
            }
        }
        self.ready = next;
        Some(wave)
    }

    /// The release has finished: nothing is held any more.
    pub(crate) fn finish(&mut self) {
        self.active = false;
        self.releasing = false;
        self.in_waves = false;
    }

    /// The release's X-DROP: the nets on `dirty` that were not on it before the waves
    /// (`before`) and whose settled value has no definite bit in any element.
    pub(crate) fn x_drop<N: NetReader + ?Sized>(
        nets: &N,
        ir: &sim_ir::SimIr,
        dirty: impl IntoIterator<Item = u32>,
        before: &BTreeSet<u32>,
    ) -> Vec<u32> {
        dirty
            .into_iter()
            .filter(|n| {
                !before.contains(n) && !crate::alias::settled_has_definite_bit(nets, ir, *n)
            })
            .collect()
    }
}

/// Collect into `out` every user function `eid` calls, at any depth. An exhaustive
/// `_`-free match, so a new `Expr` variant must be decided here; `false` = an id that
/// does not resolve, which the caller holds rather than reads as "calls nothing".
fn reached_callees(ir: &sim_ir::SimIr, eid: u32, out: &mut Vec<u32>) -> bool {
    use sim_ir::Expr as E;
    let Some(e) = ir.exprs.get(eid as usize) else {
        return false;
    };
    match e {
        E::Const { .. } => true,
        // The with-clause iterator value: a leaf, no sub-expression.
        E::ArrayItem { .. } => true,
        E::Signal { net: _, word } => word.is_none_or(|w| reached_callees(ir, w, out)),
        E::Select {
            base,
            offset,
            width,
            kind: _,
        } => [*base, *offset, *width]
            .iter()
            .all(|&s| reached_callees(ir, s, out)),
        E::Concat { parts } => parts.iter().all(|&p| reached_callees(ir, p, out)),
        E::Replicate { count, value } => {
            reached_callees(ir, *count, out) && reached_callees(ir, *value, out)
        }
        E::Unary { op: _, operand } => reached_callees(ir, *operand, out),
        E::Binary { op: _, lhs, rhs } => {
            reached_callees(ir, *lhs, out) && reached_callees(ir, *rhs, out)
        }
        E::Ternary {
            cond,
            then_e,
            else_e,
        } => [*cond, *then_e, *else_e]
            .iter()
            .all(|&s| reached_callees(ir, s, out)),
        E::SysFunc { which: _, args } => args.iter().all(|&a| reached_callees(ir, a, out)),
        E::Call { func, args } => {
            out.push(*func);
            args.iter().all(|&a| reached_callees(ir, a, out))
        }
    }
}

impl Scheduler<'_, '_> {
    /// The per-pass hold step of `settle_cont_assigns_inner`, run only while
    /// `t0_hold.active()`: drop every held assign from `pass` (its release marks it dirty
    /// again), and drive a held delayed assign's initial `x` (sized from the
    /// lvalue and the rhs's static width — the rhs is not evaluated). `true` when that
    /// drive changed a net. The tier-3 twin is `native::run::hold_t0_pass`.
    pub(super) fn hold_t0_pass(&mut self, pass: &mut Vec<u32>) -> bool {
        let held: Vec<u32> = pass
            .iter()
            .copied()
            .filter(|&ci| self.t0_hold.held_now(ci as usize))
            .collect();
        if held.is_empty() {
            return false;
        }
        pass.retain(|&ci| !self.t0_hold.held_now(ci as usize));
        let mut changed = false;
        for ci in held.into_iter().map(|c| c as usize) {
            if self.st.ir.cont_assigns[ci].delay.is_some() && self.delayed_owes_initial_x(ci) {
                let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
                let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
                let w = self.st.lvalue_width(&lhs).max(self.st.wt.get(ca_rhs).width);
                let offs = self.resolve_lvalue_offsets(&lhs);
                changed |= self.st.write_lvalue(&lhs, Value::xs(w, false), &offs);
            }
        }
        changed
    }

    /// The release settle (module doc): the non-held assigns to a fixpoint, then the
    /// held ones wave by wave, then the X-DROP of the nets only the waves dirtied.
    /// Same contract as `settle_cont_assigns`. Tier-3 twin:
    /// `native::run::settle_releasing_t0`.
    pub(super) fn settle_releasing_t0(&mut self) -> Option<bool> {
        let mut any = self.settle_cont_assigns_inner::<true>()?;
        let before: BTreeSet<u32> = self.st.dirty.iter().copied().collect();
        self.t0_hold.begin_waves();
        while let Some(wave) = self.t0_hold.next_wave() {
            for ci in wave {
                if !self.st.ca_dirty_flag[ci as usize] {
                    self.st.ca_dirty_flag[ci as usize] = true;
                    self.st.ca_dirty.push(ci);
                }
            }
            any |= self.settle_cont_assigns_inner::<true>()?;
        }
        self.t0_hold.finish();
        let drop = T0Hold::x_drop(
            &*self.st,
            self.st.ir,
            self.st.dirty.iter().copied(),
            &before,
        );
        if !drop.is_empty() {
            for &n in &drop {
                self.st.dirty_flag[n as usize] = false;
            }
            let mut v = std::mem::take(&mut self.st.dirty);
            v.retain(|n| self.st.dirty_flag[*n as usize]);
            self.st.dirty = v;
        }
        Some(any)
    }
}
