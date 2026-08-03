//! ③층(native compiled) backend — S0: the DESIGN-LEVEL eligibility gate.
//!
//! doc-21 §4.1: a native backend OWNS net storage (that is what R1 *means*), so
//! there is no body-level fallback — the interpreter cannot see the native
//! arena's nets. A design is therefore either **wholly** eligible or **wholly**
//! on the existing engine, and this module answers that question from the
//! frozen `SimIr` + the `SimOpts` sidecars alone (a static property; no
//! simulation state). run.json's `native` object serializes the verdict
//! (doc-19 R-L0), which is how the S0 eligibility measurement over real
//! designs is taken.
//!
//! ## What this gate is — and is not (S0 scope)
//!
//! This is the design-level HALF of the full v1 gate: it disqualifies on
//! FEATURE FAMILIES that live in sidecars or net kinds (doc-21 §4.3's reject
//! bucket). The body-level half — "can the S3 compiler emit code for every
//! statement of every body" — arrives WITH the S3 compiler, which is the only
//! honest place that question can be answered. Until then `eligible: true`
//! reads as an UPPER BOUND: no design-level disqualifier was found. That is
//! exactly what the S0 stop-judgment needs (if even the upper bound is 0% on
//! real designs, v1's scope is wrong and gets redrawn before any backend code
//! is written).
//!
//! ## One deliberate deviation from §4.3 as first written
//!
//! §4.3 (revision 1) put `func_table`/`task_calls_*` — user subroutine calls —
//! in the reject bucket. Revision 4 absorbed T1/T2 into S3 ("바디 코드 생성:
//! 정지 + **호출** 포함"), and S3's stop-judgment is literally "호출을 삼키지
//! 못하면 중단": a tier-3 that refuses calls has the same 0%-coverage failure
//! mode tier-2 measured on `bench/keccak`. So calls are CORE here. A design
//! that frames subroutines is eligible; making that true in the compiler is
//! S3's whole job.

use sim_ir::{NetKind, SimIr};

use crate::SimOpts;

pub mod arena;
#[cfg(test)]
mod probe_tests;
// The SHARED corpus/harness source, included exactly ONCE for both test
// modules (clippy duplicate_mod forbids a per-file include) — see the
// `extern crate self as sim_engine` note in lib.rs.
#[cfg(test)]
#[path = "../../tests/common/mod.rs"]
mod test_common;
#[cfg(test)]
mod tests;

/// The S0 verdict for one design. `eligible` ⇔ `reject_reasons` is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEligibility {
    /// No design-level disqualifier found (see module docs: an upper bound
    /// until the S3 body-level gate exists).
    pub eligible: bool,
    /// Reject FAMILY → count of offending items. The unit varies per family
    /// (nets for storage kinds, table entries for sidecars) — a row answers
    /// "how much of this feature exists", and ANY non-zero row disqualifies.
    /// Keys are stable snake_case strings (run.json consumers may pin them).
    pub reject_reasons: std::collections::BTreeMap<&'static str, u32>,
}

/// Record `n` offending items under `family` (no-op when `n == 0`, so a clean
/// family never fabricates a zero row).
fn flag(out: &mut std::collections::BTreeMap<&'static str, u32>, family: &'static str, n: usize) {
    if n > 0 {
        *out.entry(family).or_insert(0) += n as u32;
    }
}

/// S0 (doc-21 §5/§7.3): can tier-3 v1 take this WHOLE design? Answered from
/// the IR + sidecars only.
///
/// The `SimOpts` destructure below is exhaustive ON PURPOSE (no `..` rest
/// pattern): adding a sidecar to `SimOpts` without classifying it here is a
/// compile error, not a silent eligibility over-claim. Config knobs and
/// core-v1 sidecars (§4.3's "코어" bucket) bind to `_`; every reject family
/// is counted.
pub fn design_eligibility(ir: &SimIr, opts: &SimOpts) -> NativeEligibility {
    let mut out = std::collections::BTreeMap::new();
    let SimOpts {
        // ── config knobs — not design features ──────────────────────────────
        vcd_path_override: _,
        timescale_unit: _,
        vcd_date: _,
        max_deltas: _,
        max_body_steps: _,
        max_class_objs: _,
        time_limit: _,
        backend: _,
        threads: _,
        plusargs: _,
        // ── core-v1 sidecars (§4.3 "코어") — v1 must support, not disqualify ─
        net_names: _,
        proc_multipliers: _,
        proc_prec_mults: _,
        global_prec_exp: _,
        net_dims: _,
        net_decl_ranges: _,
        proc_scopes: _,
        ca_delays: _,
        assign_ranks: _,
        two_state_nets: _,
        wired_and_nets: _,
        wired_or_nets: _,
        radixes: _,
        severities: _,
        init_procs: _,
        final_procs: _,
        // `$timeformat` is runtime print state, orthogonal to codegen — the
        // compiled body calls the same formatting runtime the interpreter uses.
        timeformat_stmts: _,
        // Frame calls are CORE by the revision-4 amendment (module docs): S3
        // compiles suspendable/framed subroutine calls or tier-3 v1 fails its
        // own stop-judgment.
        func_table: _,
        func_names: _,
        task_calls_proc: _,
        task_calls_func: _,
        // Subsumed reject refinements: every member of these two sets is a
        // DynArray/Queue-kind handle net, already counted by the net-kind scan
        // below — counting them again would double-book the same net.
        real_elem_dyn_nets: _,
        string_elem_dyn_nets: _,
        // ── v1-reject sidecars (§4.3) — each non-empty table disqualifies ───
        fork_modes,
        handle_copy_stmts,
        queue_slice_stmts,
        queue_bounds,
        coverage_manifest,
        probed_nets,
        stage_stmts,
        clocking_inputs,
        clocking_commit,
        clocking_outputs,
        defer_marks,
        defer_acts,
        class_handle_nets,
        class_new_sites,
        class_layouts,
        class_field_inits,
        class_rand,
        class_constraints,
        class_dist,
        class_randc,
        randomize_with,
        class_vtable,
        class_calls,
        class_field_widths,
        assert_fire,
        assert_ctl,
        file_directed_stmts,
    } = opts;

    flag(&mut out, "fork", fork_modes.len());
    flag(&mut out, "handle_copy", handle_copy_stmts.len());
    // Queue OPERATIONS (slices, bounds) — queue STORAGE is counted by kind below.
    flag(
        &mut out,
        "queue_ops",
        queue_slice_stmts.len() + queue_bounds.len(),
    );
    flag(&mut out, "coverage", coverage_manifest.len());
    // The G2 probe/stage rails ride the interpreter's change hooks; tier-3 v1
    // does not reproduce them (doc-21 §4.3), so an instrumented run stays on
    // the existing engine rather than silently losing its trace.
    flag(&mut out, "probe", probed_nets.len());
    flag(&mut out, "stage", stage_stmts.len());
    flag(
        &mut out,
        "clocking",
        clocking_inputs.len() + clocking_commit.len() + clocking_outputs.len(),
    );
    flag(
        &mut out,
        "deferred_assert",
        defer_marks.len() + defer_acts.len(),
    );
    // One family for the whole class/OOP/CRV surface — twelve sidecars, one
    // verdict. The count is the sum of their entries (a rough "how much OOP").
    flag(
        &mut out,
        "class",
        class_handle_nets.len()
            + class_new_sites.len()
            + class_layouts.len()
            + class_field_inits.len()
            + class_rand.len()
            + class_constraints.len()
            + class_dist.len()
            + class_randc.len()
            + randomize_with.len()
            + class_vtable.len()
            + class_calls.len()
            + class_field_widths.len(),
    );
    flag(&mut out, "sva", assert_fire.len() + assert_ctl.len());
    flag(&mut out, "file_directed", file_directed_stmts.len());

    // ── heap-storage net kinds — the doc's `*_dyn_nets` intent, done right:
    // a PLAIN `int q[$]` has no sidecar entry at all, so the only complete
    // detector is the net table itself. The match is `_`-free: a new NetKind
    // is a format bump AND a forced classification here.
    let mut dyn_n = 0usize;
    let mut queue_n = 0usize;
    let mut assoc_n = 0usize;
    let mut string_n = 0usize;
    for net in &ir.nets {
        match net.kind {
            // Flat 4-state storage + real: the S1 arena's own ground (R1/R2).
            NetKind::Wire | NetKind::Reg | NetKind::Logic | NetKind::Integer | NetKind::Real => {}
            NetKind::DynArray => dyn_n += 1,
            NetKind::Queue => queue_n += 1,
            NetKind::Assoc | NetKind::AssocStr => assoc_n += 1,
            NetKind::String => string_n += 1,
        }
    }
    flag(&mut out, "dyn_array", dyn_n);
    flag(&mut out, "queue", queue_n);
    flag(&mut out, "assoc", assoc_n);
    flag(&mut out, "string", string_n);

    NativeEligibility {
        eligible: out.is_empty(),
        reject_reasons: out,
    }
}
