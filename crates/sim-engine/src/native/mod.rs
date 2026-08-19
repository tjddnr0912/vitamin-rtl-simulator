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
// Differential too — it drives `run_tests::agree` (tier-3 vs the engine), so it
// belongs to the same feature for the same reason.
#[cfg(all(test, feature = "oracle"))]
mod frames_tests;
pub mod kernel;
#[cfg(test)]
mod kernel_tests;
#[cfg(test)]
mod probe_tests;
pub mod run;
// ⚠️ B2': `oracle` as well as `test`. Every differential in this module compares
// tier-3 against the interpreter or the VM, which is exactly what the oracle
// feature carries — so in a product-shape build there is nothing for them to
// compare against and the `Backend::{Interpreter,Bytecode}` spellings they use
// do not exist. Gated as a MODULE rather than per-test: the ten sites are all
// the same fact, and a `#[cfg]` per assertion would rot.
#[cfg(all(test, feature = "oracle"))]
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

/// The `stmt_effect` family members tier-3 HAS wired — the carve-out the gate
/// row subtracts (§4.5.304's `$value$plusargs` pattern, generalized once A1
/// made the list grow).
///
/// Every entry is the CANONICAL predicate the walk itself dispatches on
/// (`exec::kpred`), never a second spelling, so the gate admits exactly the
/// statements the executor answers. The CLASSIFICATION
/// (`sim_ir::rhs_is_stmt_effect`) is untouched: tier-2's compile gate still
/// sees one family, and this only stops counting the wired members.
///
/// ⚠️ Naming a member here whose `k_*` still refuses is a SILENT-WRONG, not a
/// compile error — the design runs natively and its effect lands in the engine's
/// store. That is why each addition ships with a differential AND an absolute
/// anchor (ROADMAP §5.1-e: as tier-3 delegates, the differential goes blind).
pub fn stmt_effect_wired(exprs: &[sim_ir::Expr], rhs: u32) -> bool {
    use crate::exec::kpred;
    // S1d-5 (§4.5.304): parse/match/convert are the shared
    // `exec::plusargs::effect`; only the destination write is the store's.
    kpred::value_plusargs_rhs(exprs, rhs)
        // A1-i: `q.pop_front()` / `q.pop_back()`. Store-INDEPENDENT — the pop
        // mutates `SimState::dyn_heap` (one object, both backends) and reads no
        // net value, so tier-3 DELEGATES to the engine's own impl and the
        // destination rides `apply_effect`'s `k_write_lvalue`.
        || kpred::queue_pop_rhs(exprs, rhs)
        // A1-ii: the REF-ARG writers. Their bodies moved to
        // `exec::stmt_effect`, generic over `Kernel`, so the operand reads
        // (`k_eval`) and the ref-arg write (`k_write_lvalue`) both land in the
        // calling kernel's store instead of `Scheduler::eval`'s.
        || kpred::random_seeded_rhs(exprs, rhs)
        || kpred::dist_seeded_rhs(exprs, rhs)
        || kpred::cast_rhs(exprs, rhs)
        || kpred::assoc_iter_rhs(exprs, rhs)
        // A1-iv-a: `$sscanf` only. It scans a STRING, so unlike its seven file
        // siblings it needs no file-table plumbing — source through `k_eval`,
        // destinations through `k_write_lvalue`, and `scan_run` is generic over
        // `Kernel` now. The fd family stays refused until A1-iv-b.
        || kpred::sscanf_rhs(exprs, rhs)
        // A1-iv-b: six of the seven fd members. Their bodies moved to
        // `exec::stmt_effect` beside the others; the FILE TABLE needed no
        // routing at all (it lives in `SimState`, one object both backends see,
        // exactly like `dyn_heap`), only three narrow table seams.
        || kpred::fopen_rhs(exprs, rhs)
        || kpred::fgetc_rhs(exprs, rhs)
        || kpred::feof_rhs(exprs, rhs)
        || kpred::ungetc_rhs(exprs, rhs)
        || kpred::fgets_rhs(exprs, rhs)
        || kpred::fscanf_rhs(exprs, rhs)
        // A1-iv-c: the last member. `$fread` merges each element with its PRIOR
        // value, so it is the only one that reads its own destination — three
        // more seams (`k_read_net`, `k_array_base`, `k_warn_readmem`) and the
        // family's last raw engine write is gone.
        || kpred::fread_rhs(exprs, rhs)
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
        // #10: pure DIAGNOSTIC metadata (file:line:col + instance for severity
        // reports) — read only at emit time, orthogonal to execution. Never a
        // reject axis.
        severity_locs: _,
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
        // A4-a: WIRED for a PROCESS-LEVEL fork. `exec_fork_into` is the engine's
        // own bookkeeping and the walk supplies only the queue, so what is left
        // to refuse is the two shapes tier-3 still has no answer for — a fork
        // INSIDE a frame (`native::frames`'s "a task frame that FORKS" row) and
        // a bare `wait fork` (the executor row). Both are counted elsewhere,
        // under their own names.
        fork_modes: _,
        // A8-a: see the deleted `handle_copy` row below — store-independent.
        handle_copy_stmts: _,
        // CORE since V1 slice 2c. Both queue-OPERATION tables live in `SimState`
        // and are read by code tier-3 already shares: `queue_slice_stmts` is
        // consulted inside `builtins::dispatch`, and `queue_bounds` by
        // `SimState::enforce_queue_bound` — an `&self` method over the heap that
        // touches no net store at all.
        //
        // What DID need work is the slice's BOUND expressions: `run_queue_slice`
        // read them through `Scheduler::eval`, which is hard-wired to the
        // engine's own nets, so a native `q2 = q[a:b]` clamped against X and
        // yielded the empty queue. This row could only open together with
        // threading the reader into that helper (see `run_queue_slice`'s doc).
        queue_slice_stmts: _,
        queue_bounds: _,
        // A7: see the deleted `coverage` row below — desugared to ordinary IR.
        coverage_manifest: _,
        // A8-probe: WIRED, so it disqualifies nothing (see the note below).
        probed_nets: _,
        // Slice #6: see the deleted `stage` row below — two argument reads.
        stage_stmts: _,
        // Slice #1: see the deleted `clocking` row below — three tables in
        // `SimState`, which both kernels borrow; what was missing was routing.
        clocking_inputs: _,
        clocking_commit: _,
        clocking_outputs: _,
        // A8-b: see the deleted `deferred_assert` row below — the maturation
        // reads no net; what was missing were the two regions.
        defer_marks: _,
        defer_acts: _,
        // ── A2-i: the plain-OOP sidecars are no longer a refusal ────────────
        // A class handle net is an ordinary `Logic` slot holding an object id;
        // the object's FIELDS live in `SimState::class_heap`, which both kernels
        // borrow. So these tables describe storage tier-3 routes to rather than
        // machinery it lacks — the two rows below (`class_crv`, `class_virtual`)
        // are what is genuinely missing. `_` here rather than deleted from the
        // pattern: an exhaustive destructure is what forces the next sidecar
        // added to `SimOpts` to be classified here on purpose.
        class_handle_nets: _,
        class_new_sites: _,
        class_layouts: _,
        class_field_inits: _,
        class_rand: _,
        class_constraints: _,
        class_dist: _,
        class_randc: _,
        randomize_with: _,
        class_vtable: _,
        class_calls: _,
        class_field_widths: _,
        // Slice #4: see the deleted `file_directed` row below — one fd read.
        file_directed_stmts: _,
    } = opts;

    // ⭐⭐ **A8-a: `handle_copy` was PURELY CONSERVATIVE — zero kernel code.**
    // A whole-handle copy (`d2 = d1`, IEEE §7.10) lowers to a no-op `Display`
    // plus a StmtId → `(dst_net, src_net)` marker, and `builtins::dispatch`
    // answers it BEFORE anything renders: it deep-clones `dyn_heap[src]` into
    // `dyn_heap[dst]` and re-applies the queue bound. Every one of those is
    // `SimState` — one object both kernels borrow, keyed by NET ID — and the
    // two net ids come from the sidecar rather than from an evaluation. **No
    // net value is read on any path**, so there was nothing to route.
    //
    // Measured, not argued: `dyn_heap` (V1 slice 2), `handle_copy_stmts`,
    // `queue_bounds` and `dyn_warn_once_at`'s latch are the whole surface, and
    // tier-3 has routed through `dispatch_with` since S1d-4b. The same shape as
    // V1 slice 1 (SVA) and A1-i (`k_queue_pop`): the refusal named a feature,
    // not a mechanism this backend lacked.
    // ⭐⭐ **A7: `coverage` was CONSERVATIVE too — the same discovery V1 slice 1
    // made about SVA.** A covergroup is not a runtime mechanism: elaborate
    // DESUGARS `cg.sample()` into ordinary bit-set assignments on a bitmap net
    // (`1 << (v & 63)`, or the explicit-bin equivalent) and `get_coverage()`
    // into ordinary arithmetic over that net, so what reaches the engine is
    // plain IR the tier-3 walk has always executed correctly.
    //
    // ⚠️ ONE thing genuinely was store-dependent and it is not in the walk at
    // all: the end-of-run `coverage.json` summary read
    // `st.nets[it.bitmap_net].cur`, the ENGINE's flat store, which a native run
    // never writes — so lifting this row without that fix would have reported
    // 0.00% coverage at exit 0. The bits are now harvested from whichever store
    // ran, before the arena is dropped (`simulate`'s `cover_bits`).

    // ⭐ The G2 PROBE rail is WIRED (A8-probe). The row's reason was true and
    // stayed true — `emit_probe_change` really is called from the engine's
    // `note_change` — and slice #6 measured that this is exactly what separated
    // it from `stage`, which shared the row's comment while riding no hook at
    // all. What made it cheap anyway is that the rail's STATE is all on
    // `SimState` (`probed`, `probe_prev`, `trace_lines`, `net_names`): the only
    // store-bound part was the VALUE, so the arena captures it at its own store
    // point and `NativeKernel::drain_probe` hands it to the one shared emitter,
    // exactly as the VCD queue has done since S1d-4d-2.
    //
    // ⚠️ The failure mode the row protected against was NOT a crash: an
    // unwired probe run produces `trace.jsonl` with the t0 lines and nothing
    // after, at exit 0 — a G2 artifact that is present and wrong, which is the
    // same shape A7's `coverage.json` had.
    // ⭐ **Slice #6: `stage` was TWO READS.** ⚠️ It was in the same sentence as
    // `probe` and that grouping was wrong: `$vita_stage` is not a change hook at
    // all, it is an explicit call site that elaborate lowers to a no-op
    // `Display` plus this StmtId set. The rail's state (`stage_lines`,
    // `stage_idx`, `stage_enabled`) is `SimState`, borrowed by both kernels.
    // What was store-bound was `run_vita_stage`'s label and value reads, which
    // used a bare `sched.eval` — so a native run would have written a
    // `stage.jsonl` describing the ENGINE's untouched slots. Silently wrong
    // rather than absent, exactly like A7's `coverage.json`.
    // ⭐ **Slice #1: `clocking` was ROUTING plus one position.** A clocking block
    // is not a runtime mechanism either: elaborate mints a HOLDING net per item,
    // aliases `cb.sig` to it, and emits a marked `always @(clk);` handler with a
    // NULL body. What reaches the engine is ordinary IR plus three `SimState`
    // tables (`clocking_inputs`/`clocking_commit`/`clocking_outputs`) and the
    // `preponed_buf` — all borrowed by both kernels, exactly `dyn_heap`'s
    // situation.
    //
    // Store-bound were the two ENDS: `snapshot_preponed` read the SOURCE nets and
    // the commit read/wrote the HOLDING nets, both through the engine's store. On
    // a native run that means every `cb.sig` commits whatever the engine's slot
    // happens to hold — and X is a defined value, so this would have been silent
    // (the A1-ii shape). Both are threaded now; the plan/apply split is what lets
    // one decision drive two write points (`clocking_commit_plan`).
    //
    // The POSITION is the third piece and it is not a table: the engine performs
    // the commit inside `propagate_changes` pass (a), after the fire/busy/
    // self-write tests and before the multi-net dedup, then `continue`s. Tier-3's
    // `WakeTable::wake` diverts a handler at that exact point.
    //
    // ⚠️ Recorded because it is NOT this row's business to fix: vita's clocking
    // model commits at edge DETECTION rather than in the Observed region, so an
    // Active-region `always @(posedge clk)` reads THIS edge's sample where
    // verilator (which does support clocking blocks) reads the PREVIOUS one. That
    // is a deliberate hand-IEEE simplification of the engine's, documented at
    // `clocking_commit_plan`, and both vita backends now share it.
    // ⭐ **A8-b: `deferred_assert` was CONSERVATIVE — the machinery was already
    // shared, only the two REGIONS were missing.** §16.4.3 renders a deferred
    // action's text at REACH, so what is enqueued is a `String`; maturation reads
    // no net at all. The one store-bound line is the render inside `try_defer`,
    // and `dispatch_with` has threaded that since S1d-4b.
    //
    // What tier-3 lacked was the OBSERVED and REACTIVE positions in its region
    // cascade — the same shape as A5-b's postponed region, and added the same
    // way (`native::run::mature_deferred`, called where the engine calls it,
    // plus `drain_deferred_on_finish` at the three terminating arms).
    // ⭐⭐ **A2-i SPLIT THIS ROW IN THREE, and the census is why.** It used to be
    // one family over twelve sidecars — "how much OOP" — which meant a design
    // declaring `class C; int f; endclass` and one solving a constraint were
    // refused by the same word. Measured over the whole suite: of the 160
    // designs this row alone was blocking, **121 never execute `randomize()`
    // and have no virtual call site**. For those, class support is ROUTING and
    // nothing else — a class handle net is an ordinary `Logic` slot holding an
    // object id, and the object's fields live in `SimState::class_heap`, which
    // both kernels borrow (exactly `dyn_heap`'s situation, V1 slice 2).
    //
    // So the three rows are cut by what the design DOES, not by which sidecar
    // elaborate happened to emit — every one of the 160 carries `class_rand`
    // and `class_vtable` tables, because those are per-CLASS and exist the
    // moment a `rand` field or a method is declared. A table nobody reads
    // refuses nothing.
    //
    // ⚠️⚠️ **VIRTUAL DISPATCH WAS ALMOST THE SECOND ROW, and measuring killed
    // it.** The plan had one: `resolve_virtual_call` is answered on whichever
    // reader `eval_core` holds, and `NativeKernel`'s own doc names it as the
    // question its "no class handles exist" argument does NOT cover. But the
    // function reads `args[0]` — the receiver handle's VALUE, already evaluated
    // by the caller in the caller's store — plus `class_heap` and
    // `class_vtable`, and BOTH composites forward it to `st`. Nothing in it
    // touches a net.
    //
    // Measured before deleting the row rather than after: a three-level
    // hierarchy with an override at each level, an inherited method, and a base
    // handle re-pointed at a derived object runs natively and agrees with both
    // other backends (`a_virtual_call_dispatches_dynamically_on_tier_3`). A row
    // that refuses designs the executor gets right is a rung DOWN the ladder,
    // which costs more than the row buys.
    //
    // ⭐ It also turned up an oracle limit worth recording: **iverilog 13 gets
    // this wrong.** For `B h = d; h.who()` it calls `B::who` (static), where
    // IEEE §8.20 requires `D::who`. vita's three backends agree with the LRM, so
    // this is one of the few places the repo is ahead of its own oracle — and
    // the reason the virtual test is pinned by hand rather than by `vvp`.
    // ⚠️ **A2-ii WIRED IT, and the row's own reason was the map.** It said
    // `class_randomize_run` reads its receiver through `Scheduler::eval_ctx_top`
    // — the engine's nets — and that was exactly right and exactly one line.
    // Everything below the handle is `class_heap`, the four per-class sidecar
    // tables (`class_rand`/`class_constraints`/`class_dist`/`class_randc`), the
    // inline-`with` overrides and the RNG: all `SimState`, all borrowed by both
    // kernels, so the draw, the constraint solve and the field writes needed no
    // routing at all. The status write is the second half — a funnel-OUTSIDE
    // write, through A1-iii's `TaskWrites` sink.
    //
    // Measured before deleting: the raw-read pin counts four untreaded reads in
    // `crv_draw.rs` and the other three are `$writemem*`'s, which is a different
    // family with its own row.
    // ⭐ **Slice #4: `file_directed` was ONE READ.** `$fmonitor`/`$fstrobe` share
    // the frozen `Monitor`/`Strobe` task ids — the only thing that marks
    // `args[0]` as a descriptor is this table — and `split_file_directed`
    // evaluated that descriptor with a bare `sched.eval`, the engine's nets.
    // Everything else was already shared: the capture goes into
    // `SimState::postponed` (A5-b's region), the file table is `SimState` too
    // (A1-iv-b), and the render is `flush_postponed_with`, threaded since A5-b.
    //
    // Same shape as A5-a, down to the wording of the reason: an unthreaded fd
    // read on a native run returns whatever the engine's slot holds, so the
    // monitor would write to a descriptor the design never opened. §4.5.338
    // again — the family was named for a feature, not for missing machinery.

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
    // - ⭐ **`force`/`release` is WIRED (slice #2)** and this row is gone. The
    //   machinery it named turned out to be three pieces, and only the last was
    //   really missing: the per-net force flag is `SimState::forced` — ONE table
    //   for both stores, now THREADED into the arena funnel rather than mirrored
    //   — the §9.3.1/§9.3.2 registry (`active_forces`, `latent_assigns`, the RHS
    //   sensitivity sidecars, the assign/deassign weak rank) is `SimState` too
    //   and both kernels borrow it, and what tier-3 lacked was the continuous
    //   RE-EVALUATION fixpoint, which is now the engine's shape expressed
    //   through the same `SimState` helpers (`force_keys_for`/`force_entry`)
    //   with only the eval, the write and the dirty SEED per-store.
    //
    //   ⚠️ The old row said the write funnel "deliberately does NOT carry the
    //   flag — honoring it without the machinery would read as support while
    //   every `force` silently did nothing". That was the right call while the
    //   row stood; the funnel carries it now, at the same per-CHUNK point the
    //   engine gates, so `{a, b} = x` with only `a` forced drops one chunk.
    //
    //   ⚠️⚠️ Admitting the family also uncovered a PRE-EXISTING silent-wrong
    //   that both oracles call — see `SimState::drivers_of_net`. `release` on a
    //   wire cleared the flag and nothing re-dirtied the driving continuous
    //   assign, so the forced value survived until some input of that assign
    //   happened to move (iverilog/verilator 3, all three vita backends 240).
    //   Fixed for both stores, in the family this row was gating.
    // - ⭐⭐ **`disable fork` IS CORE SINCE A4-d, and this is the row that used to
    //   stand here.** It counted only `DisableKind::Fork`, because a plain
    //   `disable <named block>` is the break/continue idiom and needs nothing at
    //   runtime (elaborate lowers it as a diagnostic-shaped marker plus a sibling
    //   `Goto`, so the engine executes it as `StmtEffect::Nop`).
    //
    //   What the fork spelling needed was not machinery either. IEEE §9.6.3's
    //   kill reads no net value — `Scheduler::k_disable_fork` walks `activities`
    //   and `barriers` transitively and cancels §16.4 reports in `st.postponed`,
    //   all of it shared — so the tier-3 method is one delegated line. The two
    //   things that were genuinely missing were at the DISPATCH choke, not here:
    //   `cur_aid` (the root of the kill set) and the drop of an already-dead
    //   activity (how the resume entries filed before the kill get discarded).
    //
    //   ⚠️ With this gone the design gate has no family left that a design can
    //   reach. The loop below is kept, `_`-free, so a NEW `Stmt` kind has to be
    //   classified rather than silently swallowed; what is pinned today is that
    //   the map comes back empty.
    let mut stmt_effect = 0usize;
    for s in &ir.stmts {
        match s {
            // CORE since slice #2 — kept as an explicit arm rather than folded
            // into the catch-all, because an exhaustive-looking `match` that
            // silently swallows a family is how this gate would stop noticing a
            // NEW statement kind. Their machinery is `k_force`/`k_release`.
            sim_ir::Stmt::Force { .. } | sim_ir::Stmt::Release { .. } => {}
            sim_ir::Stmt::Disable { .. } => {}
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
                // The WIRED members are carved out by `stmt_effect_wired` (see
                // its doc for why the list, not the classification, is what
                // moves). What remains counted is the family tier-3 has not
                // answered yet.
                if sim_ir::rhs_is_stmt_effect(&ir.exprs, *rhs)
                    && !stmt_effect_wired(&ir.exprs, *rhs)
                {
                    stmt_effect += 1;
                }
            }
            sim_ir::Stmt::SysTask { which, .. } => {
                // FLAT only. The original reason — "a heap mutator writes too,
                // but its design is already refused by the storage-kind row
                // above" — DIED with V1 slice 2, which admits `DynArray`,
                // `String` and `Queue`. The split is still right, for a reason
                // that is now direct rather than inherited: a heap mutator does
                // not write a NET, it mutates `dyn_heap`, which lives in
                // `SimState` and is the SAME object for both backends. Tier-3
                // reaches it through the shared `builtins::dispatch`, so there
                // is nothing for an executor to reproduce. Verified: a design
                // whose only statements are `q.push_back(a)` / `d = new[n]` /
                // `s.itoa(v)` runs natively and matches the VM byte for byte.
                //
                // ⚠️ It matched only after the mutators' ARGUMENTS were threaded
                // through the reader (`eval_task_arg`/`eval_task_arg_ctx`) —
                // they read `Scheduler::eval`, i.e. the engine's own nets.
                // A1-iii: all three FLAT-writing task ids are wired. Their
                // destination writes are COLLECTED by `TaskWrites::Collect` and
                // applied through the calling kernel's funnel, so `$sformat`,
                // `$readmemb/h` and the `$cast` TASK form no longer refuse.
                //
                // ⚠️ `NetWrite::Heap` was never counted here and never needed to
                // be: `write_lvalue` routes a heap-kind net to `dyn_heap` by net
                // id, and that object is shared by both backends.
                if sim_ir::systask_net_write(*which) == sim_ir::NetWrite::Flat
                    && !matches!(
                        which,
                        sim_ir::SysTaskId::Sformat
                            | sim_ir::SysTaskId::ReadmemB
                            | sim_ir::SysTaskId::ReadmemH
                            | sim_ir::SysTaskId::Cast
                    )
                {
                    stmt_effect += 1;
                }
            }
            sim_ir::Stmt::NonblockingAssign { .. } => {}
        }
    }
    flag(&mut out, "stmt_effect", stmt_effect);

    // ── heap-storage net kinds — the doc's `*_dyn_nets` intent, done right:
    // a PLAIN `int q[$]` has no sidecar entry at all, so the only complete
    // detector is the net table itself. The match is `_`-free: a new NetKind
    // is a format bump AND a forced classification here.
    for net in &ir.nets {
        match net.kind {
            // Flat 4-state storage — the S1 arena's own ground (R1).
            NetKind::Wire | NetKind::Reg | NetKind::Logic | NetKind::Integer => {}
            // A6: `real` joined the flat kinds. Its 64 bits ARE ordinary word
            // storage — what it needed was `Slot::is_real` (stamped onto every
            // read) and the two missing arms of the real<->int assignment
            // coercion, now one shared `value::coerce_assign`. The row above
            // this one used to say it was "an S2 WIDTH CLASS"; the width was
            // never the problem.
            NetKind::Real => {}
            // V1 slice 2: every heap-storage kind is CORE. Their values live in
            // `SimState::dyn_heap`, keyed by NET ID rather than by a handle, and
            // `NativeKernel` already borrows that `SimState` — so admitting them
            // was routing (a write funnel, a total reader, `HeapRouted`), never
            // a second store. 2a `DynArray` · 2b `String` · 2c `Queue` ·
            // 2d `Assoc`/`AssocStr`.
            //
            // ⚠️ What is NOT core is the SHAPES that mutate them from inside a
            // call — `q.pop_front()`, `aa.first(i)`, `foreach` — and those are
            // refused by `stmt_effect` under their own name, not by these rows.
            NetKind::DynArray | NetKind::Queue | NetKind::String => {}
            NetKind::Assoc | NetKind::AssocStr => {}
        }
    }

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
