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
