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
//! that frames subroutines is eligible; making that true in the executor is
//! S3's whole job, and S3a took the first bite (`native::frames`).

use sim_ir::{NetKind, SimIr};

use crate::native::arena::NetArena;

use crate::SimOpts;

pub mod arena;
pub mod body;
pub mod dirty;
pub(crate) mod frames;
#[cfg(test)]
mod frames_tests;
pub mod kernel;
#[cfg(test)]
mod kernel_tests;
#[cfg(test)]
mod probe_tests;
pub mod run;
#[cfg(test)]
mod run_tests;
pub mod wake;
pub(crate) mod wprog;
pub mod write;
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
    /// The STORAGE-level half: `NetArena::buildable` accepted this design.
    /// Reported next to `eligible` rather than folded into it because they
    /// answer different questions and the numbers differ — `eligible` is what
    /// v1's SCOPE admits, `buildable` is what today's storage can actually take
    /// (a design with a subroutine TASK, or with a function that reads a module
    /// net, is eligible and not buildable; S3a admitted the store-independent
    /// subset, so the gap narrowed rather than closed).
    pub buildable: bool,
    /// Why the RUNTIME gate — design ∧ storage — said no. `None` ⇒ nothing
    /// refuses this design; it runs natively the moment an executor exists.
    ///
    /// TWO VOCABULARIES, deliberately: when the DESIGN gate refused, this is a
    /// KEY of `reject_reasons` (and, when several fired, the first in that map's
    /// byte-lexicographic order — deterministic, but "a" reason, not "the" one:
    /// removing that feature can expose the next). When the design gate passed
    /// and the STORAGE refused, it is that refusal's own text, which appears in
    /// no map. A consumer joining `refused` back to `reject_reasons` must treat
    /// a miss as the storage case rather than as an error.
    pub refused: Option<&'static str>,
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

/// The RUNTIME gate: **design gate ∧ arena build**, the two halves that must
/// agree before `--backend native` may run anything.
///
/// This exists as one function because the S1d obligation is exactly that they
/// stop disagreeing: `design_eligibility` alone says yes to designs
/// `NetArena::build` refuses (a subroutine design), and an executor wired to
/// the design gate alone would run one of those on storage that cannot hold it.
/// The reason string is the design gate's first reject family, or the storage's
/// own refusal.
pub fn runtime_gate(ir: &SimIr, opts: &SimOpts) -> Result<(), &'static str> {
    let e = design_eligibility(ir, opts);
    if let Some(r) = e.refused {
        return Err(r);
    }
    Ok(())
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
        // CORE since V1 slice 1. SVA is not a runtime mechanism at all: elaborate
        // DESUGARS `assert property(@(clk) a |-> b)` into `always @(clk) if (a &&
        // !b) $error(…)`, so what reaches the engine is ordinary IR plus these two
        // StmtId tables — and BOTH are read inside the SHARED `builtins::dispatch`
        // (`assert_ctl` flips `st.assert_disabled`, `assert_fire` is the set that
        // flip suppresses), which tier-3 already routes through. Refusing here
        // bought nothing and cost 760 of the 2,834 measured fall-backs (§4.5.336).
        //
        // The shapes that genuinely need machinery are refused ELSEWHERE, by their
        // own names: SVA liveness and `cover property` register a `final_procs`
        // entry, so `native::run::executor_rows` refuses them as `final` blocks,
        // and §16.4 deferred assertions are the separate `deferred_assert` row
        // (their maturation hooks are not in tier-3's region cascade).
        assert_fire: _,
        assert_ctl: _,
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
    flag(&mut out, "file_directed", file_directed_stmts.len());

    // ── statement-level families with no v1 machinery ──────────────────────
    // THREE families, none of them sidecar-borne — only a scan of `ir.stmts`,
    // the arena EVERY body's statements live in (process and subroutine alike),
    // finds them.
    //
    // The third (`stmt_effect`) was carried as a note from S1c until the
    // `Kernel` trait made the question structural: an `impl Kernel` cannot be
    // written without answering all of these, so "decide later" stopped being
    // available and the gate answers instead. Wiring them later is what LIFTS
    // the reject, not what was needed to add it.
    //
    // - `force`/`release`: a whole machine v1 does not have (the per-net force
    //   flag that suppresses every normal driver, the continuous re-evaluation,
    //   the assign/deassign weak rank). The write funnel deliberately does NOT
    //   carry the flag — honoring it without the machinery would read as support
    //   while every `force` silently did nothing. Note procedural
    //   `assign`/`deassign` ALSO lower to Force/Release, so this row is the only
    //   thing that catches them (`assign_ranks` is a core sidecar).
    // - `disable fork` only. A plain `disable <named block>` is the
    //   break/continue idiom and needs NOTHING here: elaborate lowers it as a
    //   diagnostic-shaped marker plus a sibling `Goto` that does the actual
    //   control flow (and rejects every non-lexically-enclosing target loudly),
    //   so the engine executes it as `StmtEffect::Nop`. Counting all `Disable`
    //   rejected whole designs for a statement with no runtime effect —
    //   costlier here than for the VM, which loses one body while tier-3 has no
    //   body-level fallback at all.
    let mut force_release = 0usize;
    let mut disable_fork = 0usize;
    let mut stmt_effect = 0usize;
    for s in &ir.stmts {
        match s {
            sim_ir::Stmt::Force { .. } | sim_ir::Stmt::Release { .. } => force_release += 1,
            sim_ir::Stmt::Disable { scope_kind, .. } => {
                if matches!(scope_kind, sim_ir::DisableKind::Fork) {
                    disable_fork += 1;
                }
            }
            // EFFECTS THAT NEVER PASS THROUGH THE WRITE FUNNEL. A seeded
            // `$random`/`$dist_*` writes its seed back, `$cast` writes its
            // destination, `$value$plusargs` writes its output, and the whole
            // file family advances descriptor state — all INSIDE the call, not
            // through `write_lvalue`. `$readmem*` writes a memory net and
            // `$sformat` writes a packed destination the same way.
            //
            // BOTH halves go through a canonical, `_`-free predicate in sim-ir:
            // `rhs_is_stmt_effect` (the SAME function the tier-2 VM's compile
            // gate consults — two spellings would let the backends disagree
            // about one statement) and `systask_writes_net`. The SysTask half
            // was written here as a three-id `matches!` first, and its implicit
            // catch-all immediately cost a miss: the TASK form of `$cast` writes
            // its destination exactly as `$sformat` does and was accepted.
            //
            // ⚠️ This row's criterion is "writes a NET from inside the call". It
            // is NOT the full list of what an executor must reproduce: every
            // `$display`/`$fdisplay`/`$dumpvars`/`$writemem*` reads nets and
            // mutates output or descriptor state. Those are runtime SERVICES,
            // answered by whoever implements `k_dispatch_systask`, not by this
            // gate. Reading this row as "the SysTasks tier-3 must worry about"
            // would understate the work.
            //
            // ⚠️ `$sformatf` (the FUNCTION form) is deliberately NOT here: its
            // only effect is the rendered value, written through the ordinary
            // funnel. That is true only for an executor that routes statements
            // through `compute_effect`/`apply_effect` — the tier-2 VM bypasses
            // them and therefore has to exclude it. The tier-3 plan is to BE a
            // `Kernel` impl, so this exclusion is correct and conditional on it.
            sim_ir::Stmt::BlockingAssign { rhs, .. } => {
                // `$value$plusargs` is CARVED OUT (S1d-5): `k_value_plusargs`
                // is wired through the shared `exec::plusargs::effect`, so the
                // first member of this family runs instead of refusing. The
                // carve-out is the canonical `value_plusargs_rhs` — the same
                // predicate the walk dispatches on (process.rs), so the gate
                // admits exactly the statements the executor answers. The
                // CLASSIFICATION (`rhs_is_stmt_effect`) is untouched: tier-2's
                // compile gate still sees one family, and this row only stops
                // counting the wired member.
                if sim_ir::rhs_is_stmt_effect(&ir.exprs, *rhs)
                    && !crate::exec::kpred::value_plusargs_rhs(&ir.exprs, *rhs)
                {
                    stmt_effect += 1;
                }
            }
            sim_ir::Stmt::SysTask { which, .. } => {
                // FLAT only: a heap mutator writes too, but its design is
                // already refused by the storage-kind row above, and counting it
                // here would double-book the same statement.
                if sim_ir::systask_net_write(*which) == sim_ir::NetWrite::Flat {
                    stmt_effect += 1;
                }
            }
            sim_ir::Stmt::NonblockingAssign { .. } => {}
        }
    }
    flag(&mut out, "force_release", force_release);
    flag(&mut out, "disable_fork", disable_fork);
    flag(&mut out, "stmt_effect", stmt_effect);

    // ── heap-storage net kinds — the doc's `*_dyn_nets` intent, done right:
    // a PLAIN `int q[$]` has no sidecar entry at all, so the only complete
    // detector is the net table itself. The match is `_`-free: a new NetKind
    // is a format bump AND a forced classification here.
    let mut dyn_n = 0usize;
    let mut queue_n = 0usize;
    let mut assoc_n = 0usize;
    let mut string_n = 0usize;
    let mut real_n = 0usize;
    for net in &ir.nets {
        match net.kind {
            // Flat 4-state storage — the S1 arena's own ground (R1).
            NetKind::Wire | NetKind::Reg | NetKind::Logic | NetKind::Integer => {}
            // `real` is an f64 slot = an S2 WIDTH CLASS, not v1 core, and
            // `NetArena::build` already refuses it. Saying so here keeps the
            // published eligibility number from counting designs the storage
            // cannot take (differential-review find: the two gates disagreed).
            NetKind::Real => real_n += 1,
            NetKind::DynArray => dyn_n += 1,
            NetKind::Queue => queue_n += 1,
            NetKind::Assoc | NetKind::AssocStr => assoc_n += 1,
            NetKind::String => string_n += 1,
        }
    }
    flag(&mut out, "real", real_n);
    flag(&mut out, "dyn_array", dyn_n);
    flag(&mut out, "queue", queue_n);
    flag(&mut out, "assoc", assoc_n);
    flag(&mut out, "string", string_n);

    // The storage-level half. `buildable` is allocation-free, so asking on every
    // run costs one scan — worth it: run.json then carries BOTH numbers and the
    // eligible-vs-buildable gap is measured rather than hand-counted.
    let storage = NetArena::buildable(ir, opts);
    let eligible = out.is_empty();
    let refused = if !eligible {
        // The first reject family names it; the full map is right there for detail.
        out.keys().next().copied()
    } else {
        storage.err()
    };
    NativeEligibility {
        eligible,
        buildable: storage.is_ok(),
        refused,
        reject_reasons: out,
    }
}
