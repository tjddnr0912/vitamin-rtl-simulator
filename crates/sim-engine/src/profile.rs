//! R14 / ROADMAP §3 ⑭ — the per-body execution profile behind `run.json`'s
//! `processes` object.
//!
//! WHAT THE EXTERNAL REPORT ASKED FOR. A user measuring 20.4 cycle/s against a
//! 440 cycle/s budget asked, as their explicit fallback to a faster scheduler,
//! for "per-process evaluation counts and cumulative time … if we knew which
//! `always_comb` eats the cost we could reduce it on our side". Today `run.json`
//! carries `codegen`, which is a STATIC capability census — it says which
//! process bodies the VM *could* compile, and nothing at all about which ones
//! ran, or how often. This is the dynamic half.
//!
//! WHAT AN "EVALUATION" IS, precisely, because a count nobody can define is a
//! count nobody can act on: one ACTIVATION of the body by the scheduler. A
//! process that suspends on `#5` and resumes counts twice — the two halves are
//! two dispatches and cost two trips through the seam. A continuous assign
//! counts once per settle-fixpoint visit that actually evaluates its RHS (the
//! dirty worklist skips the rest, and a skipped visit costs nothing, so
//! counting it would misattribute).
//!
//! A FORK CHILD is charged to the process TEMPLATE it belongs to, because the
//! scheduler dispatches it by that template — so a `fork`-heavy `initial` shows
//! its children's cost on its own row rather than on rows the source has no
//! names for. That is the useful attribution here: the reader's next action is
//! to edit the block, and the block is what they can edit.
//!
//! DETERMINISM. `evals` is a function of the design and the run options alone —
//! two runs of the same input produce byte-identical counts, which is why they
//! can sit in `run.json` beside `codegen` and inside the determinism golden.
//! `nanos` is WALL CLOCK and is not: it is opt-in behind a second flag, it is
//! excluded from the determinism golden exactly as `wall_s`/`sim_s` are, and it
//! never participates in the row ORDER (rows sort by `evals`, so the file's
//! shape stays deterministic even with timing on).
//!
//! ⚠️ OBSERVER EFFECT, stated rather than hidden. Timing takes two
//! `Instant::now()` per activation (~40 ns on this Mac). For a fat `always_ff`
//! that is noise; for a one-bit continuous assign it can EXCEED the work being
//! measured, so a timed run's `sim_s` is longer than the same run's untimed
//! `sim_s` and the per-row shares are biased toward the cheap rows. Counts do
//! not have this problem. Read `evals` first; reach for `nanos` only to break a
//! tie between two rows with similar counts.

/// What `--obs-procs` / `--obs-procs-time` turned on (rides `SimOpts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProcProfileCfg {
    /// Also accumulate wall-clock nanoseconds per body. See the observer-effect
    /// note above — this is why it is a separate opt-in and not implied.
    pub timed: bool,
}

/// The accumulators. Two parallel vectors per domain rather than a `Vec<struct>`
/// because the hot path touches ONE of them: with timing off, `nanos` is never
/// allocated and never written, so the increment is a single `u64` bump.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProcProfile {
    pub timed: bool,
    /// Activations per ProcId (index = `sim_ir::SimIr.processes` index).
    pub evals: Vec<u64>,
    /// Cumulative nanoseconds per ProcId. EMPTY unless `timed`.
    pub nanos: Vec<u64>,
    /// RHS evaluations per continuous assign (index into `SimIr.cont_assigns`).
    pub ca_evals: Vec<u64>,
    /// Cumulative nanoseconds per continuous assign. EMPTY unless `timed`.
    pub ca_nanos: Vec<u64>,
    /// R2 (round-36): the per-BUILTIN table, folded in at the end of the run
    /// from the interior-mutable [`BuiltinProfile`] the four seams bumped.
    /// EMPTY `rows` means "measured, no builtin ran" — the "not measured" case
    /// is the enclosing `Option<ProcProfile>` being `None`.
    pub builtins: BuiltinCounts,
}

impl ProcProfile {
    /// Size the accumulators for one design. Called ONCE, from `simulate`, and
    /// only when the profile was requested.
    pub fn new(cfg: ProcProfileCfg, n_procs: usize, n_cas: usize) -> Self {
        Self {
            timed: cfg.timed,
            evals: vec![0; n_procs],
            nanos: if cfg.timed {
                vec![0; n_procs]
            } else {
                Vec::new()
            },
            ca_evals: vec![0; n_cas],
            ca_nanos: if cfg.timed {
                vec![0; n_cas]
            } else {
                Vec::new()
            },
            builtins: BuiltinCounts {
                timed: cfg.timed,
                rows: std::collections::BTreeMap::new(),
            },
        }
    }

    /// Record one process-body activation.
    ///
    /// `get_mut`, not `[]`: a stale/short accumulator must not panic a
    /// simulation over a REPORTING side table. The `debug_assert` is what makes
    /// the silent branch honest — a debug build fails loudly if the sizing ever
    /// drifts from `ir.processes.len()`.
    #[inline]
    pub(crate) fn bump_proc(&mut self, tmpl: usize, nanos: u64) {
        debug_assert!(
            tmpl < self.evals.len(),
            "R14 profile sized from ir.processes"
        );
        if let Some(c) = self.evals.get_mut(tmpl) {
            *c += 1;
        }
        if let Some(t) = self.nanos.get_mut(tmpl) {
            *t += nanos;
        }
    }

    /// Record one continuous-assign RHS evaluation.
    #[inline]
    pub(crate) fn bump_ca(&mut self, ci: usize, nanos: u64) {
        debug_assert!(
            ci < self.ca_evals.len(),
            "R14 profile sized from ir.cont_assigns"
        );
        if let Some(c) = self.ca_evals.get_mut(ci) {
            *c += 1;
        }
        if let Some(t) = self.ca_nanos.get_mut(ci) {
            *t += nanos;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// R2 (round-36) — the PER-BUILTIN profile, the second half of `--obs-procs`.
// ─────────────────────────────────────────────────────────────────────────────
//
// WHAT THE EXTERNAL REPORT ASKED FOR, and what this is not. Their `initial` at
// `tb_aes_top:729` is ONE row worth 60% of the run, because it calls a whole
// vector-driver stack and every nested cost is summed into the caller. They
// asked, in priority order, for (1) a call tree down to task granularity and
// (2), failing that, per-builtin cumulative time for `$fgets`/`$sscanf`/string
// ops/queue ops. THIS IS (2). It is not a call tree and does not pretend to be:
// it splits a process row into "time this body spent inside the simulator's own
// builtins" versus the rest, which is the half a profile can attribute with a
// stable identity today. Why (1) is a separate slice, and what it needs first,
// is written up in the OBS SPEC (doc-19 §4.9) with the measurement that decided
// it.
//
// THE IDENTITY is the builtin's NAME (`sim_ir::systask_name`/`sysfunc_name`),
// not an index — a `$sscanf` row means the same thing in every design, every run
// and every version, which is exactly what an index would not. A builtin has no
// declaration site to report: it is not declared in the user's source, so the
// `file:line:col` that identifies a PROCESS row has no counterpart here. (The
// CALL SITE does have one, and per-site rows are the natural follow-on; the SPEC
// says why they are not in this slice.)
//
// ATTRIBUTION — stated here because "do not double-count" is a hard requirement:
//
//  * `calls` is invocations. No nesting question exists for it.
//  * `nanos` is **SELF (exclusive) time**: the wall clock of one invocation
//    MINUS the wall clock of any builtin invoked inside it. `$display("%s",
//    $sformatf(…))` is a real nesting — the argument evaluation runs the inner
//    builtin inside the outer one's dispatch — so an inclusive convention would
//    count that span twice and the column would not add up. With SELF time the
//    rows are disjoint and `Σ nanos` is a true simulator-builtin subtotal.
//  * That sum is nonetheless CONTAINED IN the process rows above: a builtin runs
//    inside whichever body called it, so `processes.items[].time_s` already
//    includes it. `run.json` says so in `builtins.attribution` and
//    `builtins.included_in_processes` rather than leaving a reader to guess.
//
// DETERMINISM is the same split as the process profile: `calls` is a function of
// the design and the run options alone and rides the determinism golden; `nanos`
// is wall clock, exists only under `--obs-procs-time`, and never participates in
// the row ORDER (rows sort by `calls`, then by NAME — a total order, so two runs
// are byte-identical).
//
// ⚠️ OBSERVER EFFECT, worse here than for processes. Two `Instant::now()` per
// invocation is ~40 ns on this Mac, and a `.len()` on a short string costs less
// than that. Read `calls` first; `time_s` is for separating two rows whose call
// counts are comparable.

/// Cumulative accumulator for ONE builtin name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuiltinAcc {
    /// Invocations. Deterministic.
    pub calls: u64,
    /// SELF (exclusive) nanoseconds — see the attribution note above. Always 0
    /// unless the run asked for `--obs-procs-time`.
    pub nanos: u64,
}

/// The finished per-builtin table handed back on [`ProcProfile`].
///
/// `BTreeMap` and not a `Vec`: the key is a `&'static str` from the two name
/// tables, so iteration order is a total order over NAMES and cannot depend on
/// insertion order — i.e. on the design's execution order, which is exactly what
/// a `HashMap` would have leaked into a file this rail promises is byte-stable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuiltinCounts {
    pub timed: bool,
    pub rows: std::collections::BTreeMap<&'static str, BuiltinAcc>,
}

/// One open invocation. Returned by [`BuiltinProfile::enter`] and consumed by
/// [`BuiltinProfile::leave`]; carrying the enclosing invocation's inner-time
/// total in the VALUE rather than in a side stack is what makes the pair
/// reentrant without allocating.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BuiltinFrame {
    t0: Option<std::time::Instant>,
    saved_nested: u64,
}

/// The live accumulators.
///
/// ⚠️ INTERIOR MUTABILITY IS THE WHOLE DESIGN. The four seams that see a builtin
/// run are not all `&mut`: `builtins::dispatch_with` holds `&mut Scheduler`,
/// `exec::apply_effect` holds `&mut impl Kernel` whose net reader it can only
/// borrow immutably, `EvalCtx::eval_sysfunc_ctx` is `&self`, and the `&self`
/// frame executor in `state/frame_eval.rs` is `&self` by name. A `&mut`
/// accumulator would have needed a different plumbing story at each one; with
/// `Cell`/`RefCell` all four reach the SAME object through a shared reference —
/// the pattern `SimState::rng` (`RngCells`) and `dyn_heap` already use.
#[derive(Debug, Default)]
pub struct BuiltinProfile {
    timed: bool,
    acc: std::cell::RefCell<std::collections::BTreeMap<&'static str, BuiltinAcc>>,
    /// Nanoseconds accumulated by builtins invoked INSIDE the innermost open
    /// invocation. See [`Self::leave`].
    nested: std::cell::Cell<u64>,
}

impl BuiltinProfile {
    /// Allocate for one run. Constructed only when `--obs-procs` was given, so
    /// `SimState::builtin_prof == None` is the whole cost on a run without it.
    pub fn new(cfg: ProcProfileCfg) -> Self {
        Self {
            timed: cfg.timed,
            acc: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            nested: std::cell::Cell::new(0),
        }
    }

    /// Open one invocation. Reads the clock only when timing was asked for, so
    /// an `--obs-procs` (counts-only) run pays one `bool` test here.
    #[inline]
    pub(crate) fn enter(&self) -> BuiltinFrame {
        BuiltinFrame {
            t0: self.timed.then(std::time::Instant::now),
            // Park the enclosing invocation's inner-time accumulator and start
            // this one's at zero, so `leave` can subtract exactly the time spent
            // in builtins nested inside THIS call.
            saved_nested: self.nested.replace(0),
        }
    }

    /// Close one invocation and charge it.
    ///
    /// The three lines that make the column add up: `elapsed` is this
    /// invocation's INCLUSIVE time, `inner` is what builtins nested inside it
    /// reported while it was open, and `elapsed - inner` is its SELF time. The
    /// enclosing invocation then resumes with `saved + elapsed` as its own inner
    /// total — `elapsed`, not `elapsed - inner`, because the WHOLE of this call
    /// (its own work and its callees') is nested inside that one.
    ///
    /// `saturating_sub`, not `-`: the clock is monotonic but the two reads are
    /// taken at different nesting depths, and a reporting side table must not be
    /// able to panic a simulation over a rounding artefact.
    #[inline]
    pub(crate) fn leave(&self, name: &'static str, f: BuiltinFrame) {
        let elapsed = f.t0.map_or(0, |t0| t0.elapsed().as_nanos() as u64);
        let inner = self.nested.replace(f.saved_nested.saturating_add(elapsed));
        let mut acc = self.acc.borrow_mut();
        let slot = acc.entry(name).or_default();
        slot.calls += 1;
        slot.nanos = slot.nanos.saturating_add(elapsed.saturating_sub(inner));
    }

    /// Freeze into the reportable table.
    pub fn finish(&self) -> BuiltinCounts {
        BuiltinCounts {
            timed: self.timed,
            rows: self.acc.borrow().clone(),
        }
    }
}

/// R2: the profile label of a severity task.
///
/// ⚠️ `$info`/`$warning`/`$error`/`$fatal` all lower to `SysTaskId::Display`
/// plus a `severities` sidecar entry, so the ID alone cannot name them and TWO
/// seams have to un-fold the same table (`builtins::dispatch`'s label helper and
/// the `&self` frame executor's severity arm). One spelling, here, so the two
/// cannot drift into calling the same construct different things.
pub(crate) fn severity_builtin_name(sev: crate::SeverityKind) -> &'static str {
    match sev {
        crate::SeverityKind::Info => "$info",
        crate::SeverityKind::Warning => "$warning",
        crate::SeverityKind::Error => "$error",
        crate::SeverityKind::Fatal => "$fatal",
        // A `unique`/`priority` violation is a PARSER desugar onto the
        // `$warning` shape, not a task the user wrote — name the construct.
        crate::SeverityKind::UniqueViolation => "unique/priority check",
    }
}

#[cfg(test)]
mod builtin_tests {
    use super::*;

    /// SELF time is exclusive: an inner invocation's span is charged to the
    /// inner row and SUBTRACTED from the outer one. Without this the two rows
    /// would double-count the nested span, which is the failure the
    /// `attribution` field exists to rule out.
    ///
    /// The assertion is on the ORDERING and the SUM, not on absolute
    /// nanoseconds — wall clock is not reproducible, and a test that pinned it
    /// would be measuring this Mac.
    #[test]
    fn nested_time_is_charged_once() {
        let p = BuiltinProfile::new(ProcProfileCfg { timed: true });
        let outer = p.enter();
        std::thread::sleep(std::time::Duration::from_millis(4));
        let inner = p.enter();
        std::thread::sleep(std::time::Duration::from_millis(8));
        p.leave("$inner", inner);
        p.leave("$outer", outer);
        let c = p.finish();
        let o = c.rows["$outer"].nanos;
        let i = c.rows["$inner"].nanos;
        assert!(i > o, "inner slept twice as long: inner={i} outer={o}");
        // The outer row must NOT contain the inner span. Total ≈ 12 ms, so an
        // outer that still carried the inner 8 ms would be ≳ 11 ms.
        assert!(o < 7_000_000, "outer kept the nested span: {o} ns");
    }

    /// Counts are the deterministic half and do not need timing on.
    #[test]
    fn counts_without_timing_are_free_of_the_clock() {
        let p = BuiltinProfile::new(ProcProfileCfg { timed: false });
        for _ in 0..3 {
            let f = p.enter();
            p.leave("$display", f);
        }
        let c = p.finish();
        assert_eq!(c.rows["$display"].calls, 3);
        assert_eq!(c.rows["$display"].nanos, 0, "untimed run must report 0");
        assert!(!c.timed);
    }
}
