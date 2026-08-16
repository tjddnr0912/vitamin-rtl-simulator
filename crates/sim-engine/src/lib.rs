//! sim-engine — event-driven kernel that EXECUTES a frozen `sim_ir::SimIr`.
//!
//! Pipeline position: preprocess → lex → parse → elaborate → sim-ir → **ENGINE**
//! → VCD. v1 entry: [`simulate`] inits the net table from `NetVar.init`, runs the
//! IEEE-1364 stratified scheduler (Active → Inactive → NBA delta loop), evaluates
//! 4-state expressions, drives `vcd-writer` on `$dumpfile`/`$dumpvars`, prints
//! `$display`/`$write`, and stops on `$finish`/`$stop`.
//!
//! DETERMINISM: same `SimIr` → byte-identical VCD + stdout on all 3 OSes. No
//! HashMap iteration ever decides execution order; every ready set is a sorted
//! `Vec` keyed by `tie` = process declaration index; the time wheel is a
//! `BTreeMap`; cont-assigns settle in declaration order; NBA applies in sample
//! (`seq`) order.
//!
//! IMPLEMENTED: fork/join (via `SimOpts.fork_modes`), `$monitor`/`$strobe`
//! postponed-region semantics, real numbers, inlined user task/func, multi-instance
//! hierarchy with hierarchical VCD `$scope`/`$var` names (`SimOpts.net_names`), and
//! per-module timescale scaling of `#delay`/`$time`/`$realtime` (`SimOpts.proc_multipliers`).
//! Arithmetic is a 128-bit lane (unsigned); any operand X/Z poisons the result to X,
//! as does a signed result wider than 64 bits or an unsigned one wider than 128.
//! DEFERRED (Phase-2): `force`/`release`, the full SV 17-region scheduler, full
//! multi-word arithmetic. All three engine-facing side tables ride out-of-band in
//! `SimOpts` and never enter the frozen `SimIr`.

// The shared test harness (`tests/common/mod.rs`) names this crate as
// `sim_engine::…`; aliasing self lets in-crate unit tests `#[path]`-include that
// SAME file (single corpus source) instead of duplicating it. sim-ir precedent.
extern crate self as sim_engine;

mod backend;
mod builtins;
mod eval;
mod exec;
#[cfg(feature = "jit")]
mod jit;
mod levelize;
/// ③층 native backend (doc-21). S0: the design-level eligibility gate only —
/// public because the CLI serializes its verdict (run.json `native`) and the
/// S0 measurement calls it directly.
pub mod native;
mod native_eval;
mod rng;
mod sched;
mod state;
mod value;
mod vcd_thread;
mod width;

#[cfg(test)]
mod width_tests;

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use diag::{LogEvent, LogSink, ProgressEvent, RtlText};
use sim_ir::SimIr;

pub use backend::{
    codegen_coverage, codegen_report, native_eval_coverage, native_eval_coverage_split,
    CodegenCoverage, CodegenReport,
};
/// Re-exported from `elaborate` so callers thread the join-mode side table into
/// `SimOpts.fork_modes` without naming the `elaborate` crate directly.
pub use elaborate::{
    AssignRankTable, CovItem, CovgInstMeta, DeferActTable, DeferMarkTable, DeferRegion,
    ForkModeTable, FuncMeta, FuncTable, JoinMode, NetDeclRangeTable, NetDimsTable, NetNameTable,
    QueueBoundTable, RadixTable, SeverityKind, SeverityTable, Sidecars, TaskCallFunc, TaskCallInfo,
    TaskCallProc,
};
pub use levelize::{
    comb_depth, comb_ranks, fusion_candidates, fusion_candidates_across_copies,
    self_read_write_processes, FusionPair,
};
pub use sched::FinishReason;

use sched::Scheduler;
use state::SimState;

/// Process exit classification (CLI maps this to a numeric exit code).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitClass {
    /// Clean: finished/quiescent with no error-or-fatal diagnostics.
    Ok,
    /// At least one Error-severity diagnostic was emitted (sim still ran).
    HadErrors,
    /// A Fatal diagnostic ended the run.
    Fatal,
}

/// Process-body execution backend (P0a). Selected out-of-band via [`SimOpts`];
/// NEVER enters the frozen `sim_ir::SimIr` (schema hash unaffected). The shared
/// net-write and VCD choke point (`state.rs::write_lvalue`/`emit_vcd_change`) stays
/// on the SHARED side across backends, so only process-body *control flow* differs —
/// VCD/stdout bytes cannot diverge in a backend-specific way (enforced by the P5 gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Tree-walking interpreter (`exec::run_process`) — the REFERENCE semantics.
    ///
    /// ⭐⭐ **PHASE C: this is a TEST INSTRUMENT, not a product surface.** Its job is
    /// to be the readable, obviously-correct statement of what a statement MEANS —
    /// it walks `SimIr` directly, with no compiled form and no second storage — so
    /// that when `vm` and `native` disagree there is something to arbitrate with.
    /// The `oracle` feature is that role made structural: a product build does not
    /// contain this variant at all.
    ///
    /// ⚠️⚠️ **PERMANENTLY EXCLUDED FROM PERFORMANCE WORK, and that is a rule rather
    /// than an observation.** Making the reference faster is how a reference stops
    /// being readable: every specialisation is a second spelling of a rule, and the
    /// second spelling is this repository's defect class (§4.5.279 — the VM drifted
    /// from the interpreter in four independent, silent ways). If a profile ever
    /// names `run_process`, the answer is that the design should not be running here.
    /// Measured for scale, not as a target: picorv32 1.319 s against native's 0.513 s.
    ///
    /// ⚠️ It is still LOAD-BEARING in the oracle build — the VM falls back to it
    /// body-by-body for anything `is_codegen_able` refuses, and tier-3 delegates
    /// frame bodies to it. "Not a product surface" is about the `--backend` FLAG, not
    /// about `run_process` being dead code.
    #[cfg(feature = "oracle")]
    Interpreter,
    /// Bytecode VM (P0a) — **no longer the default (Phase B1).** Codegen-able bodies
    /// (the P9 suspend-free allow-list, `backend::is_codegen_able`) are compiled once
    /// per process template and run on the VM (`sched::scan_arm::vm_run_body`); every
    /// other body falls back to the interpreter, so a design mixing both is normal.
    ///
    /// Measured (release, best-of-5, `tests/perf_baseline.rs`): expression-bound ~2.2x,
    /// structure-bound ~2.8x, wide 100-bit ~1.7x, clock/scheduler-bound ~1.0x (eval is
    /// not the bottleneck there). On a real design (picorv32 + testbench, 40000 cycles,
    /// best-of-7): 1.10 s -> 0.78 s. That is vita against itself; iverilog 13 runs the
    /// same design in 0.58 s + 0.03 s compile, so the VM narrows a gap rather than
    /// closing it.
    ///
    /// Selecting a backend must never change a single output byte. Two gates hold that:
    /// the P5 differential (`tests/backend_equiv.rs`) over the deterministic corpus PLUS
    /// hand-written shapes the generator cannot emit, and — the one that actually found
    /// the four defects that were hiding behind a green P5 — running the entire
    /// workspace suite with this default in place. That obligation now belongs to
    /// [`Backend::Native`]; keep doing it in BOTH directions while two executors exist,
    /// because a corpus differential is far weaker than 5000 real tests.
    #[cfg(feature = "oracle")]
    Bytecode,
    /// ③층 native backend (doc-21) — **THE DEFAULT since Phase B1 (2026-08-16).**
    ///
    /// It became the default on two measurements, not on preference:
    ///
    /// * **Coverage.** Phase A closed every gate row a design can reach. The
    ///   corpus census is **6,470 / 6,470 = 100.00%** with zero refusals, so
    ///   choosing this default does not silently route anyone to another
    ///   executor (ROADMAP §5.1-ap · study/02).
    /// * **Equivalence, then speed.** The flip run — the whole workspace suite
    ///   with this default in place — is byte-identical, and it is the gate that
    ///   has historically found what the corpus differential could not. Only
    ///   after that does speed matter: picorv32, release, interleaved best-of-5,
    ///   **interp 1.319 s / vm 0.838 s / native 0.513 s** (iverilog 13: 0.585 s).
    ///
    /// The three gate layers still exist and still answer: v1 SCOPE
    /// (`native::design_eligibility`), STORAGE (`NetArena::buildable`) and
    /// EXECUTOR (`native::run::executor_rows`). What changed is that none of
    /// them has a row a compiler can produce input for; a malformed sidecar can
    /// still make STORAGE refuse.
    ///
    /// ⚠️ A refusal falls back to [`Backend::Bytecode`] and **says nothing** —
    /// run.json carries `backend_requested` beside the effective `backend` and
    /// `native.refused` names the layer, but there is no diagnostic and the exit
    /// code is 0. That is an honesty gap rather than a correctness one (the VM's
    /// answer is right), and closing it is Phase B4a. Until then, an anchor test
    /// that does not assert `"backend": "native"` cannot tell a native run from
    /// a fallback — which is a mistake this project has actually made.
    #[default]
    Native,
}

/// The CLI spelling of a backend, for diagnostics. ONE spelling with the
/// front end's (`cli::frontend`) so a message and `run.json` cannot disagree
/// about what ran — the whole point of B4a's warning is that the two reports
/// are the same report.
fn backend_name(b: Backend) -> &'static str {
    match b {
        #[cfg(feature = "oracle")]
        Backend::Interpreter => "interp",
        #[cfg(feature = "oracle")]
        Backend::Bytecode => "vm",
        Backend::Native => "native",
    }
}

/// N7-REST: one rand field's draw spec — `(field_id, width, signed, lo, hi, constrained)`.
/// `constrained` ⇒ draw within [lo, hi]; else full-width.
pub type RandBound = (u32, u32, bool, i64, i64, bool);

/// N7-REST B2: one rand field's `dist` weighted distribution — `(field_id, entries)`
/// where each entry is `(lo, hi, weight)`: a value (lo==hi) or `[lo:hi]` range whose
/// TOTAL weight is `weight`. `randomize()` weighted-samples the field from these
/// (then a uniform pick within the chosen entry's [lo,hi]).
pub type DistField = (u32, Vec<(i64, i64, i64)>);

/// N7-REST B2: a `randc` (cyclic) field — `(field_id, lo, hi)`. `randomize()` draws
/// a random PERMUTATION of `[lo,hi]`, visiting every value once before repeating
/// (per-instance state in the engine).
pub type RandcField = (u32, i64, i64);

/// N7-REST B-CRV final: one inline `randomize() with {…}` call's per-call extra
/// constraints — `(domain overrides, predicates)`. The engine reads the with-id
/// from a `Const` arg of `ClassRandomize`, INTERSECTS each `(field_id, lo, hi)`
/// override into the class `[lo,hi]` domain, and ANDs the predicates with the
/// class predicates (IEEE §18.7).
pub type RandWithCall = (Vec<(u32, i64, i64)>, Vec<Vec<sim_ir::COp>>);

/// Caller-tunable knobs. All have deterministic, documented defaults.
#[derive(Debug, Clone)]
pub struct SimOpts {
    /// Overrides the `$dumpfile` path (e.g. CLI `-o`). `None` ⇒ use the RTL's
    /// `$dumpfile` argument.
    pub vcd_path_override: Option<String>,
    /// `$timescale` unit string for the VCD preamble (e.g. `"1ns"`).
    pub timescale_unit: String,
    /// VCD `$date` stamp — taken verbatim so output stays deterministic.
    pub vcd_date: String,
    /// Max delta cycles per time-step before the infinite-delta guard fires.
    pub max_deltas: u64,
    /// Cap on BLOCK STEPS a single process activation may execute WITHOUT suspending.
    ///
    /// A distinct question from `max_deltas`, and it used to borrow that number, which
    /// made a plain `for (i = 0; i < 500000; i++)` in an `initial` fatal at 1,000,000
    /// steps and reported it as "zero-delay loop / combinational oscillation" — a cause
    /// that was not present. A loop with no feedback and no delay is legal and finite;
    /// what the counter actually observes is only that ONE ACTIVATION has run a long
    /// time. So it gets its own budget, its own message, and a much larger default: the
    /// guard exists to stop a genuinely unbounded loop from hanging the run, not to
    /// bound how much work a testbench may do at time 0 (vector-file parsing and memory
    /// preloads routinely exceed a million steps).
    ///
    /// Default 100,000,000 — a hundred times the old shared limit, which covers the
    /// reported real site (a time-0 CAVP vector parse) with room, while still tripping an
    /// unbounded loop in about a second rather than the ten a billion would take.
    pub max_body_steps: u64,
    /// CLASS-HEAP-CAP: max live class objects before a graceful fatal fires. The
    /// class heap is never garbage-collected, so an unbounded `new()` in a loop
    /// would grow without limit; this bounds it to a loud `F-RUN-CLASS-LIMIT`
    /// (implicit `$finish`) instead of an OOM. Default 1_000_000 (≈160 MiB) —
    /// far above any N7-level testbench's live-object count.
    pub max_class_objs: u64,
    /// Hard cap on advanced simulation time (ticks). `None` ⇒ unbounded.
    pub time_limit: Option<u64>,
    /// Join-mode side table from `elaborate::elaborate_with_modes`, keyed
    /// `(template ProcId, join_bb)`. EMPTY for fork-free designs (the default), so
    /// every existing `SimOpts::default()` caller is unaffected. The engine's
    /// fork-mode lookup is total-or-fatal: a `Terminator::Fork` with no matching
    /// entry aborts the run at t0 rather than fabricating a (wrong) mode.
    pub fork_modes: ForkModeTable,
    /// Per-NetId hierarchical name table from `elaborate::elaborate_with_sidecars`.
    /// EMPTY by default (every existing caller unaffected): the VCD writer then
    /// falls back to a flat `top` scope + synthetic `n{i}` names. When populated it
    /// drives real hierarchical `$scope`/`$var` output. Never enters the golden IR.
    pub net_names: NetNameTable,
    /// Per-ProcId time multiplier `M = 10^(unit_exp − global_prec_exp)` from
    /// `elaborate::elaborate_with_timescale`, for `$time`/`$realtime` scaling
    /// (`$time = now / M`). EMPTY ⇒ multiplier 1 (the 1ns/1ns base). Never golden.
    pub proc_multipliers: Vec<u64>,
    /// Parallel per-ProcId `S = 10^(prec_exp − global_prec_exp)` table (module's
    /// OWN precision step in global ticks) for the IEEE two-stage `#delay`
    /// conversion: a real delay rounds to the module precision FIRST
    /// (`round(d × M/S)`), then scales by `S`. EMPTY ⇒ S=1 for every process ⇒
    /// byte-identical to the prior single `round(d × M)` (single-timescale
    /// designs and every existing `SimOpts::default()` caller unaffected).
    pub proc_prec_mults: Vec<u64>,
    /// Process-body execution backend (P0a). Default [`Backend::Bytecode`] — the VM
    /// compiles the bodies it can and falls back to the interpreter body-by-body for the
    /// rest, so a caller that never sets this gets the faster executor with the same
    /// output bytes. Rides out-of-band (never enters the frozen `SimIr`).
    pub backend: Backend,
    /// Severity side table from `elaborate::elaborate_with_timescale`, keyed by
    /// StmtId: marks `$fatal`/`$error`/`$warning`/`$info` statements (lowered as
    /// `SysTaskId::Display`). EMPTY for severity-free designs (the default), so
    /// every existing caller is unaffected. Never enters the golden IR.
    pub severities: SeverityTable,
    /// `$timeformat` side table: StmtIds of `$timeformat` calls (lowered as no-op
    /// `SysTaskId::Display`, the severity/assert_ctl pattern). EMPTY for designs
    /// without `$timeformat` (the default). Never enters the golden IR.
    pub timeformat_stmts: std::collections::BTreeSet<u32>,
    /// OBS-3: `$vita_stage` StmtIds (no-op Display the engine intercepts for stage.jsonl).
    pub stage_stmts: std::collections::BTreeSet<u32>,
    /// Whole-handle copy markers (§7.10 `dst = src` deep copy): no-op Display
    /// StmtId → (dst_net, src_net). EMPTY default. Never golden.
    pub handle_copy_stmts: std::collections::BTreeMap<u32, (u32, u32)>,
    /// Queue-slice markers (§7.10.1 `dst = src[a:b]`): no-op Display StmtIds
    /// whose args are [dst, src, a, b]. EMPTY default. Never golden.
    pub queue_slice_stmts: std::collections::BTreeSet<u32>,
    /// Global precision exponent (power of 10, in seconds, of one simulation
    /// tick) for `%t` unit scaling — −9 (the 1ns/1ns base) by default. Computed
    /// alongside `proc_multipliers` by the CLI timescale wiring. Never golden.
    pub global_prec_exp: i8,
    /// Default-radix side table (P1-5): StmtId → 2/8/16 for the b/o/h print
    /// variants. EMPTY by default (decimal everywhere). Never enters the IR.
    pub radixes: RadixTable,
    /// Assign-rank side table (§9.3.1): StmtIds of Force/Release stmts that are
    /// procedural `assign`/`deassign` (weak rank — a real force overrides them;
    /// release hands control back). EMPTY by default. Never enters the IR.
    pub assign_ranks: AssignRankTable,
    /// Bounded-queue side table (v6 ③): handle NetId → declared bound N
    /// (`[$:N]`, max size N+1). Any queue op that ends beyond the bound has
    /// its TAIL truncated + W4020 (iverilog live). EMPTY ⇒ all unbounded.
    pub queue_bounds: QueueBoundTable,
    /// Per-ProcId instance path (`"tb.u1"`) for `%m` (P2-11). EMPTY ⇒ `%m`
    /// renders the legacy flat `top`. Never enters the IR.
    pub proc_scopes: Vec<String>,
    /// OBS-1b coverage manifest (per covergroup instance → hit-bitmap net ids). The
    /// engine reads each bitmap's FINAL value at end-of-run to build the
    /// `SimResult.coverage` summary for `coverage.json`. EMPTY ⇒ no covergroups.
    /// Never enters the IR (golden-neutral).
    pub coverage_manifest: Vec<CovgInstMeta>,
    /// OBS-2: net ids to trace for `trace.jsonl` (`--probe`). On each CHANGE of a
    /// probed net the engine records a `{v,t,kind:"chg",path,old,new}` line in
    /// `SimResult.trace`. EMPTY ⇒ no probing (byte-identical). Never enters the IR.
    pub probed_nets: Vec<u32>,
    /// Unpacked-array dims (Phase-1.x ⑤): array NetId → per-dim `(lo, size)`.
    /// SPARSE — an absent array means 1-D 0-based, so per-element VCD names
    /// fall back to `mem[k]`. EMPTY by default. Never enters the IR.
    pub net_dims: NetDimsTable,
    /// DECLARED packed `(msb, lsb)` for nets stored with a normalized range (a NEGATIVE
    /// low bound). Drives the VCD `$var` label only. EMPTY ⇒ byte-identical.
    pub net_decl_ranges: NetDeclRangeTable,
    /// `$fmonitor`/`$fstrobe` call-site StmtIds — for these the postponed capture takes
    /// `args[0]` as a file descriptor. EMPTY ⇒ every monitor/strobe goes to stdout.
    pub file_directed_stmts: std::collections::BTreeSet<u32>,
    /// The synthesized declaration-initializer ProcIds, in INITIALIZATION order.
    /// `arm_processes` runs these to completion BEFORE arming anything and then skips
    /// them, so initialization precedes every user process and produces no event
    /// (IEEE 1800 §6.21). EMPTY ⇒ no declaration initializers ⇒ nothing changes.
    pub init_procs: Vec<u32>,
    /// Worker-thread budget (P4-T1, CLI `--threads`/`-j`). `1` (the default) is
    /// the exact single-thread path; `≥2` moves VCD file writes onto a dedicated
    /// writer thread behind an order-preserving bounded FIFO. CONTRACT: output
    /// (VCD/stdout/exit) is byte-identical for every value — N changes
    /// wall-clock only (enforced by `tests/threads.rs`).
    pub threads: u32,
    /// P2-E: ProcIds of `final` blocks — never armed at t0; run ONCE (in
    /// ascending ProcId order) after the main loop ends, whatever the finish
    /// reason. EMPTY default keeps every existing caller unchanged.
    pub final_procs: std::collections::BTreeSet<u32>,
    /// N4 clocking: source NetIds to snapshot into the preponed buffer at each
    /// time advance. EMPTY ⇒ no clocking ⇒ byte-identical.
    pub clocking_inputs: std::collections::BTreeSet<u32>,
    /// N4 clocking: marked commit-handler ProcId → `[(holding_net, source_net)]`.
    pub clocking_commit: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// N4 clocking output pairs: ProcId → `[(source_net, holding_net)]`.
    /// At each clocking-edge commit, drive `source_net = current_value(holding_net)`.
    /// EMPTY ⇒ no output clocking ⇒ byte-identical to designs without OUTPUT ports.
    pub clocking_outputs: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// S1 gate/assign rise·fall·turnoff delay: cont-assign index → (rise, fall,
    /// turnoff). Populated only when the folded delay values are NOT all equal
    /// (so `#5`, `#(3,3)`, no-delay carry no entry → the uniform `ContAssign.delay`
    /// is used). EMPTY ⇒ byte-identical to designs with uniform/no delays.
    pub ca_delays: std::collections::BTreeMap<u32, (u32, u32, u32)>,
    /// Runtime plusargs (v7, `+name[=value]` with the '+' stripped, CLI
    /// order). `$test$plusargs` prefix-probes them; `$value$plusargs`
    /// converts the first match's remainder. Pure runtime input — never
    /// hashed into artifacts.
    pub plusargs: Vec<String>,
    /// §16.4 deferred-assert flush markers: marker StmtId → maturation region.
    /// EMPTY default ⇒ no deferred asserts (every existing caller byte-identical).
    pub defer_marks: DeferMarkTable,
    /// §16.4 deferred-assert actions: action StmtId → (marker StmtId, region).
    pub defer_acts: DeferActTable,
    /// B1 frame-call metadata, index-aligned to `ir.funcs`. EMPTY default ⇒ no
    /// automatic/recursive functions ⇒ every existing caller byte-identical.
    pub func_table: FuncTable,
    /// N1: FuncId → subroutine name, index-aligned to `func_table`. `%m` inside a
    /// frame body only. EMPTY default ⇒ `%m` = module scope (byte-identical).
    pub func_names: Vec<String>,
    /// B2 frame-call: process-body task-call sites (executor-facing).
    pub task_calls_proc: TaskCallProc,
    /// B2 frame-call: nested (task-body) task-call sites (`run_task`-facing).
    pub task_calls_func: TaskCallFunc,
    /// SVPART: NetIds of 2-state variables — the engine coerces X/Z→0 on every
    /// write (IEEE §6.11.3). EMPTY default ⇒ no 2-state nets ⇒ byte-identical.
    /// One-shot `vita` only (the staged trailer does not serialise it; 2-state
    /// INIT-to-0 rides the golden SimIr and so works on both paths).
    pub two_state_nets: std::collections::BTreeSet<u32>,
    /// N3 Phase 2 heterogeneous heap: DynArray handle NetIds whose ELEMENTS are
    /// `real` / `string`. The engine flags the net `is_real` (real) or routes the
    /// element store through the string heap (string), and fills `new[]` with the
    /// element's IEEE default (0.0 / ""). One-shot `vita` + staged trailer.
    pub real_elem_dyn_nets: std::collections::BTreeSet<u32>,
    pub string_elem_dyn_nets: std::collections::BTreeSet<u32>,
    /// WAND/WOR: NetIds whose MULTI-driver resolution is wired-AND / wired-OR
    /// (instead of the default wire resolution). One-shot `vita` only.
    pub wired_and_nets: std::collections::BTreeSet<u32>,
    pub wired_or_nets: std::collections::BTreeSet<u32>,
    // ── N7 class/OOP (out-of-band, golden-free; one-shot `vita` only) ──
    /// NetIds that are class handles (drives `State.class_is_handle`).
    pub class_handle_nets: std::collections::BTreeSet<u32>,
    /// `new` allocation sites: StmtId → class_id.
    pub class_new_sites: std::collections::BTreeMap<u32, u32>,
    /// Per-class field layout: `class_layouts[class_id]` = `[(width, signed,
    /// four_state)]` in stable field-id order.
    pub class_layouts: Vec<Vec<(u32, bool, bool)>>,
    /// SW1: per-class folded field initializers, parallel to `class_layouts`
    /// (`[class_id][field_id]` → `Some(bits)` if the field has a `= const`
    /// initializer, else `None`). Drives the `new` default-init (IEEE §8.8).
    pub class_field_inits: Vec<Vec<Option<sim_ir::BitPacked>>>,
    /// N7-REST: per-class `rand` fields with folded constraint bounds.
    /// `class_rand[class_id]` = `[(field_id, width, signed, lo, hi, ranged)]`.
    /// `ranged` ⇒ `randomize()` draws `dist_uniform(lo, hi)`; else a full-width draw.
    pub class_rand: Vec<Vec<RandBound>>,
    /// N7-REST B2: per-class general constraint predicates (postfix programs over
    /// candidate rand-field values). `randomize()` draws from `class_rand`'s domains
    /// then keeps a candidate only when every predicate evaluates true (rejection
    /// sampling). Out-of-band sidecar (IR-0).
    pub class_constraints: Vec<Vec<Vec<sim_ir::COp>>>,
    /// N7-REST B2: per-class `dist` weighted distributions (field → entries).
    pub class_dist: Vec<Vec<DistField>>,
    /// N7-REST B2: per-class `randc` cyclic fields.
    pub class_randc: Vec<Vec<RandcField>>,
    /// N7-REST B-CRV final: per-call inline `randomize() with {…}` constraints,
    /// indexed by the with-id Const arg of each `ClassRandomize`. EMPTY ⇒ none.
    pub randomize_with: Vec<RandWithCall>,
    /// Virtual dispatch table: `class_vtable[class_id][vslot]` = concrete FuncId.
    pub class_vtable: Vec<Vec<u32>>,
    /// Per method-call-site dispatch: key (StmtId/ExprId) → `(vslot, static_fid)`.
    pub class_calls: std::collections::BTreeMap<u32, (Option<u32>, u32)>,
    /// Class field-read `Signal` ExprId → `(field_width, field_signed)`. Patches
    /// the width table (a field Signal's net is the 32-bit handle, not the field).
    pub class_field_widths: std::collections::BTreeMap<u32, (u32, bool)>,
    /// SVA-REST: StmtIds of synthesized assertion FIRE reports. Suppressed while
    /// assertions are disabled by a standing `$assertoff`/`$assertkill`.
    pub assert_fire: std::collections::BTreeSet<u32>,
    /// SVA-REST: `$assertoff`/`$asserton`/`$assertkill` control-site StmtId → kind
    /// (0 = off, 1 = on, 2 = kill). Lowered as no-op `Display`; the engine flips the
    /// global assertion-enable on reach.
    pub assert_ctl: std::collections::BTreeMap<u32, u8>,
}

impl Default for SimOpts {
    fn default() -> Self {
        SimOpts {
            vcd_path_override: None,
            timescale_unit: "1ns".to_string(),
            vcd_date: "vitamin-sim".to_string(),
            max_deltas: 1_000_000,
            max_body_steps: 100_000_000,
            max_class_objs: 1_000_000,
            time_limit: None,
            fork_modes: ForkModeTable::new(),
            net_names: Vec::new(),
            proc_multipliers: Vec::new(),
            proc_prec_mults: Vec::new(),
            // ⚠️ TWO SPELLINGS OF ONE DEFAULT, and they have disagreed before.
            // §4.5.336 measured that this literal did NOT track the enum's
            // `#[default]`, so flipping the derive alone moved only the CLI half
            // of the suite. Keep them together, and flip both when flipping.
            backend: Backend::Native,
            severities: SeverityTable::new(),
            timeformat_stmts: std::collections::BTreeSet::new(),
            stage_stmts: std::collections::BTreeSet::new(),
            handle_copy_stmts: std::collections::BTreeMap::new(),
            queue_slice_stmts: std::collections::BTreeSet::new(),
            global_prec_exp: -9,
            radixes: RadixTable::new(),
            assign_ranks: AssignRankTable::new(),
            queue_bounds: QueueBoundTable::new(),
            proc_scopes: Vec::new(),
            coverage_manifest: Vec::new(),
            probed_nets: Vec::new(),
            net_dims: NetDimsTable::new(),
            net_decl_ranges: NetDeclRangeTable::new(),
            file_directed_stmts: std::collections::BTreeSet::new(),
            init_procs: Vec::new(),
            threads: 1,
            plusargs: Vec::new(),
            final_procs: std::collections::BTreeSet::new(),
            clocking_inputs: std::collections::BTreeSet::new(),
            clocking_commit: std::collections::BTreeMap::new(),
            clocking_outputs: std::collections::BTreeMap::new(),
            ca_delays: std::collections::BTreeMap::new(),
            defer_marks: DeferMarkTable::new(),
            defer_acts: DeferActTable::new(),
            func_table: FuncTable::new(),
            func_names: Vec::new(),
            task_calls_proc: TaskCallProc::new(),
            task_calls_func: TaskCallFunc::new(),
            two_state_nets: std::collections::BTreeSet::new(),
            real_elem_dyn_nets: std::collections::BTreeSet::new(),
            string_elem_dyn_nets: std::collections::BTreeSet::new(),
            wired_and_nets: std::collections::BTreeSet::new(),
            wired_or_nets: std::collections::BTreeSet::new(),
            class_handle_nets: std::collections::BTreeSet::new(),
            class_new_sites: std::collections::BTreeMap::new(),
            class_layouts: Vec::new(),
            class_field_inits: Vec::new(),
            class_rand: Vec::new(),
            class_constraints: Vec::new(),
            class_dist: Vec::new(),
            class_randc: Vec::new(),
            randomize_with: Vec::new(),
            class_vtable: Vec::new(),
            class_calls: std::collections::BTreeMap::new(),
            class_field_widths: std::collections::BTreeMap::new(),
            assert_fire: std::collections::BTreeSet::new(),
            assert_ctl: std::collections::BTreeMap::new(),
        }
    }
}

/// Outcome of a run. The VCD + stdout are the side effects; this is the summary.
/// (Not `Eq`: the OBS-1b `coverage` field carries `f64` percents.)
#[derive(Debug, Clone, PartialEq)]
pub struct SimResult {
    pub finish_reason: FinishReason,
    pub sim_time: u64,
    pub exit_class: ExitClass,
    pub vcd_path: Option<String>,
    /// OBS-1b: end-of-run functional-coverage summary (N5 covergroups). `None` ⇒ the
    /// design had no covergroup instances (empty `coverage_manifest`).
    pub coverage: Option<CoverageSummary>,
    /// OBS-2: `trace.jsonl` lines — one `{v,t,kind:"chg",path,old,new}` per probed-net
    /// CHANGE, in emission (time) order. `None` ⇒ no `--probe` (empty `probed_nets`).
    pub trace: Option<Vec<String>>,
    /// OBS-3: `stage.jsonl` lines — one `{v,t,kind:"stage",…}` per `$vita_stage` call
    /// (time order). `None` ⇒ `+STAGE_TRACE` was not set (no capture).
    pub stage: Option<Vec<String>>,
    /// T0: the VM-coverage report behind run.json's `codegen` object — computed
    /// from the SAME walk and the SAME `class_new_sites` copy the compile gate
    /// reads (`st.class_new_sites`), so the log cannot disagree with what the
    /// executor did. A static property of the design: always present, one
    /// allow-list walk per process template.
    pub codegen: CodegenReport,
    /// S0 (doc-21 §7.3): the ③층 eligibility verdict — serialized as run.json's
    /// `native` object. Always present; static per (design, run options) — NOT
    /// per design alone: an instrumented run (`--probe`, stage capture) is
    /// ineligible by design, doc-21 §4.3.
    pub native: native::NativeEligibility,
    /// The executor that ACTUALLY ran process bodies. Differs from the requested
    /// `SimOpts.backend` when `Backend::Native` fell back (see the resolution in
    /// `simulate`), which is why run.json reports this one.
    pub backend: Backend,
}

/// OBS-2: format a net's 4-state value as an MSB..LSB binary string (`bit_char`
/// semantics: 0/1/x/z), matching the VCD writer. Deterministic. A scalar (width 1)
/// yields a single char.
fn fmt_probe_value(bits: &sim_ir::BitPacked, width: u32) -> String {
    let w = width.max(1);
    let mut s = String::with_capacity(w as usize);
    for i in (0..w).rev() {
        let word = (i / 64) as usize;
        let shift = i % 64;
        let v = bits.val.get(word).map_or(0, |x| (x >> shift) & 1);
        let u = bits.unk.get(word).map_or(0, |x| (x >> shift) & 1);
        s.push(match (v, u) {
            (0, 0) => '0',
            (1, 0) => '1',
            (0, 1) => 'x',
            _ => 'z',
        });
    }
    s
}

/// OBS-2: append `s` as a JSON string literal (quotes + minimal RFC-8259 escaping).
/// Net paths and 4-state values are normally escape-free, but a `\`-escaped SV
/// identifier could appear — so quote/backslash/control are escaped for safety.
fn json_push_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// OBS-1b: end-of-run functional-coverage summary (N5 covergroups), computed from
/// `SimOpts.coverage_manifest` + the final hit-bitmap net values. Serialized to
/// `coverage.json` by the CLI.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverageSummary {
    pub groups: Vec<CovgResult>,
}

/// One covergroup INSTANCE's coverage: overall weighted-average percent + per-item
/// breakdown. `coverage_pct` mirrors `synth_cover_get` (`c.get_coverage()`) EXACTLY.
#[derive(Debug, Clone, PartialEq)]
pub struct CovgResult {
    pub instance: String,
    pub coverage_pct: f64,
    pub items: Vec<CovItemResult>,
}

/// One coverage item's result: covered-bin count out of `num_bins` and its percent.
#[derive(Debug, Clone, PartialEq)]
pub struct CovItemResult {
    pub name: String,
    pub is_cross: bool,
    pub num_bins: u32,
    pub covered_bins: u32,
    pub coverage_pct: f64,
}

/// A `Write` sink that forwards RTL text to a `LogSink` as `RtlOutput` events.
/// This is the default `$display` sink so output is captured through `diag`.
/// (v1: `sim_time` is left `None` — threading live time through a `Write`
/// adapter is a minor follow-up; each `$display` is one write.)
struct LogWrite<'a> {
    sink: &'a dyn LogSink,
}

impl Write for LogWrite<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buf).into_owned();
        self.sink.emit(LogEvent::RtlOutput(RtlText {
            text,
            sim_time: None,
        }));
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// THE entry point. Executes `ir`, driving the VCD file + RTL output through
/// `sink`. `$display`/`$write` text is emitted as `LogEvent::RtlOutput`.
pub fn simulate(ir: &SimIr, sink: &dyn LogSink, opts: SimOpts) -> SimResult {
    // RTL output sink routes $display/$write text through the LogSink.
    let out: Box<dyn Write + '_> = Box::new(LogWrite { sink });

    let mut st = SimState::new(
        ir,
        out,
        sink,
        opts.timescale_unit.clone(),
        opts.vcd_date.clone(),
        opts.vcd_path_override.clone(),
    );
    st.net_names = opts.net_names.clone();
    // OBS-2: arm the `--probe` trace tap. `probe_prev` starts at each probed net's
    // CONSTRUCTION value (pre-t0, usually all-x) — armed BEFORE the event loop, so
    // a value first driven at t0 IS logged as the first `chg` (old = the
    // construction default). Only same-value writes are suppressed (R-L3
    // transition-only) — the t0 initialization edge is real signal history.
    if !opts.probed_nets.is_empty() {
        let n = st.nets.len();
        st.probed = vec![false; n];
        st.probe_prev = vec![None; n];
        for &id in &opts.probed_nets {
            let i = id as usize;
            if i < n {
                st.probed[i] = true;
                st.probe_prev[i] = Some(fmt_probe_value(&st.nets[i].cur, st.nets[i].width));
            }
        }
    }
    st.proc_multipliers = opts.proc_multipliers.clone();
    st.proc_prec_mults = opts.proc_prec_mults.clone();
    // ③층: resolve the EFFECTIVE executor once. `Backend::Native` falls back to
    // the VM whenever the runtime gate refuses the design — and, today, always,
    // because no native executor exists yet (S1d). `SimResult.backend` carries
    // the result so run.json reports what RAN; the CLI reports the request
    // beside it, because a fall-back only the wall-clock could reveal would be
    // exactly the wrong-log doc-19 §3 forbids.
    // S0: the ③층 verdict, taken here — while `opts` is still WHOLE (the
    // scheduler consumes `opts.fork_modes` by value further down, and a late
    // read would see an emptied table and silently call a fork design eligible).
    // Computed ONCE: `refused` IS the runtime gate's answer, so the backend
    // resolution below and run.json read the same verdict.
    let native_eligibility = native::design_eligibility(ir, &opts);
    // S1d-4c-2c: the THIRD layer — "can the executor that exists today run it".
    // `design_eligibility.refused` already ANDs scope with storage; these are the
    // rows the run loop itself adds (no cont-assign settle, no in-body waiter, no
    // refused system task). Asked through `executor_rows` rather than `runnable`
    // so the design gate is evaluated ONCE and the published verdict cannot come
    // from a different evaluation than the executed decision.
    let mut native_eligibility = native_eligibility;
    if native_eligibility.refused.is_none() {
        // PUBLISH the third layer, not just decide with it. An earlier version
        // of this slice kept the executor's refusal in a local and serialized
        // the two-layer struct, so run.json said `refused: null` on every
        // fall-back the new rows caused — a G2 rail whose entire job is to
        // explain `backend != backend_requested`, answering "nothing refused
        // this". Both adversarial reviews found it independently.
        native_eligibility.refused = native::run::executor_rows(ir, &opts).err();
    }
    let native_refusal = native_eligibility.refused;
    let effective_backend = match opts.backend {
        // ⚠️ B2': the FALL-BACK TARGET is what the `oracle` feature gates, and
        // that is the whole mechanism. With the oracle backends compiled out
        // there is nothing to fall back TO, so a gate refusal has to be reported
        // rather than routed around — which is the ladder rise Phase B is for
        // (B4b turns the W4030 warning below into an error there).
        #[cfg(feature = "oracle")]
        Backend::Native if native_refusal.is_some() => Backend::Bytecode,
        b => b,
    };
    // ── B4b: WITH NO FALLBACK TARGET, A REFUSAL IS FATAL ──
    //
    // ⚠️⚠️ **This is a HOLE B2' opened, not an enhancement.** Gating the
    // fall-back arm above removed the only consumer of the gate's verdict in
    // this build — and `simulate` then ran the design on tier-3 ANYWAY.
    // Measured with a forced refusal in a `--no-default-features` binary: the
    // design ran, exit 0, no diagnostic. The gate said "out of scope" and
    // nothing listened, which is precisely the class this project refuses.
    //
    // Fatal rather than a warning, and the reason is the same ladder argument
    // that made B4a a warning, read the other way: with the VM compiled out
    // there is no correct-support option left, so the choice is loud-or-wrong
    // rather than loud-or-correct. `fatal_run` is the graceful form — it latches
    // `had_fatal`/`finished`, so the run ends with a non-zero exit class instead
    // of panicking in `NetArena::build`'s `expect` (which is what a STORAGE
    // refusal would otherwise reach).
    #[cfg(not(feature = "oracle"))]
    if let Some(row) = native_refusal {
        st.fatal_run(&format!(
            "backend `native` cannot run this design ({row}), and this build \
             carries no other executor — the `oracle` backends are compiled out"
        ));
    }
    // ── B4a: THE SWAP IS NO LONGER SILENT ──
    //
    // The verdict has always been PUBLISHED — run.json carries
    // `backend_requested` beside `backend` and `native.refused` names the layer.
    // But published is not the same as said: a fall-back you have to go looking
    // for is one nobody looks for. §5.1-o is the incident — a design run with
    // `--backend native` had actually fallen back, the outputs matched iverilog
    // exactly, and it read as "tier-3 agrees" until run.json was opened.
    //
    // ⚠️ A WARNING, NOT AN ERROR, and the reason is the accuracy ladder rather
    // than politeness. The fall-back is not a wrong answer, it is a slow one:
    // byte-identity across the executors is a gate, so the VM's answer is the
    // native one. Making this `exit != 0` in the default build would trade
    // correct-support for loud, which is a rung DOWN. It is promoted to an error
    // only in the build where the fall-back target is not compiled at all
    // (`--no-default-features`, Phase B4b), because there the choice is
    // loud-or-wrong rather than loud-or-correct.
    //
    // ⚠️ POPULATION ZERO TODAY, and that is stated rather than hidden. Phase A
    // closed every gate row a compiler can produce input for, so no source
    // reaches this. It is written fail-closed so that the day a new row is added
    // this reports it without anyone remembering to; its teeth are a corrupted
    // sidecar, which is the same technique
    // `native_gate::the_runtime_gate_is_exactly_design_and_storage` uses.
    if effective_backend != opts.backend {
        use diag::{Diagnostic, LogEvent, MsgCode, Severity};
        sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: MsgCode::RunBackendFallback,
            message: format!(
                "requested backend `{}` cannot run this design ({}); ran on `{}` instead \
                 — the result is unaffected, the speed is",
                backend_name(opts.backend),
                native_refusal.unwrap_or("no reason recorded"),
                backend_name(effective_backend),
            ),
            location: None,
            context: Vec::new(),
            sim_time: None,
        }));
    }
    st.backend = effective_backend;
    st.severities = opts.severities.clone();
    st.timeformat_stmts = opts.timeformat_stmts.clone();
    st.stage_stmts = opts.stage_stmts.clone();
    st.handle_copy_stmts = opts.handle_copy_stmts.clone();
    st.queue_slice_stmts = opts.queue_slice_stmts.clone();
    st.global_prec_exp = opts.global_prec_exp;
    st.radixes = opts.radixes.clone();
    st.assign_ranks = opts.assign_ranks.clone();
    st.queue_bounds = opts.queue_bounds.clone();
    st.proc_scopes = opts.proc_scopes.clone();
    st.net_dims = opts.net_dims.clone();
    st.net_decl_ranges = opts.net_decl_ranges.clone();
    st.file_directed_stmts = opts.file_directed_stmts.clone();
    st.init_procs = opts.init_procs.clone();
    st.threads = opts.threads;
    st.plusargs = opts.plusargs.clone();
    // OBS-3: `$vita_stage` captures to stage.jsonl only under `+STAGE_TRACE`; without
    // it every `$vita_stage` is a pure no-op (suppressed, no capture). Accept the
    // bare flag AND a `+STAGE_TRACE=<val>` form (plusargs conventionally carry `=v`),
    // but NOT a `STAGE_TRACE`-prefixed neighbour like `+STAGE_TRACEX`.
    st.stage_enabled = st
        .plusargs
        .iter()
        .any(|p| p == "STAGE_TRACE" || p.starts_with("STAGE_TRACE="));
    st.final_procs = opts.final_procs.clone();
    // N4 clocking: source nets to snapshot (ordered, deterministic) + commit handlers.
    st.clocking_inputs = opts.clocking_inputs.iter().copied().collect();
    st.clocking_commit = opts.clocking_commit.clone();
    st.clocking_outputs = opts.clocking_outputs.clone();
    st.ca_delays = opts.ca_delays.clone();
    st.defer_marks = opts.defer_marks.clone();
    st.defer_acts = opts.defer_acts.clone();
    // WAND/WOR: per-net multi-driver resolution kind (one-shot only).
    st.wired_and_nets = opts.wired_and_nets.clone();
    st.wired_or_nets = opts.wired_or_nets.clone();
    // SVPART: mark 2-state nets so write_chunk coerces X/Z→0 (one-shot only).
    for &n in &opts.two_state_nets {
        if (n as usize) < st.two_state.len() {
            st.two_state[n as usize] = true;
        }
    }
    // N3 Phase 2 heterogeneous heap: flag `real r[]` element-real handles `is_real`
    // (so element read/write use the real value path) and `string s[]` handles as
    // string-element (byte-string store). EMPTY ⇒ golden-neutral.
    for &n in &opts.real_elem_dyn_nets {
        if let Some(slot) = st.nets.get_mut(n as usize) {
            slot.is_real = true;
        }
    }
    for &n in &opts.string_elem_dyn_nets {
        if (n as usize) < st.dyn_str_elem.len() {
            st.dyn_str_elem[n as usize] = true;
        }
    }
    // N7 class/OOP: install the class sidecars (out-of-band, golden-free). EMPTY
    // ⇒ class_is_handle all-false ⇒ byte-identical for every prior design.
    for &n in &opts.class_handle_nets {
        if (n as usize) < st.class_is_handle.len() {
            st.class_is_handle[n as usize] = true;
        }
    }
    st.class_new_sites = opts.class_new_sites.clone();
    st.class_layouts = opts
        .class_layouts
        .iter()
        .enumerate()
        .map(|(ci, fields)| crate::state::ClassLayout {
            fields: fields.clone(),
            inits: opts.class_field_inits.get(ci).cloned().unwrap_or_default(),
        })
        .collect();
    st.class_vtable = opts.class_vtable.clone();
    st.class_rand = opts.class_rand.clone();
    st.class_constraints = opts.class_constraints.clone();
    st.class_dist = opts.class_dist.clone();
    st.class_randc = opts.class_randc.clone();
    st.randomize_with = opts.randomize_with.clone();
    // CLS-CALL-VEC: index per-call-site dispatch info by ExprId (O(1) Vec) instead
    // of a BTreeMap (O(log n)) — siblings (class_vtable/class_is_handle) are Vec
    // too. Non-class designs keep an EMPTY Vec (get() returns None for all eids ⇒
    // byte-identical, zero allocation); only class designs pay the exprs-length Vec.
    if !opts.class_calls.is_empty() {
        let mut cc = vec![None; ir.exprs.len()];
        for (&eid, &v) in &opts.class_calls {
            cc[eid as usize] = Some(v);
        }
        st.class_calls = cc;
    }
    // B1 frame-call: install the sidecar, derive the per-net routing tables, and
    // REBUILD the width table so `Expr::Call` widths come from the func metadata.
    // Order is load-bearing: `func_table` must be on `st` before routing/width.
    // EMPTY table ⇒ no-op (routing all-false, width rebuild byte-identical).
    st.func_table = opts.func_table.clone();
    st.func_names = opts.func_names.clone();
    st.build_func_routing();
    // N7: a class field-read Signal's net is the 32-bit handle, so the field's
    // own width/sign is handed in and applied DURING the pass — a post-hoc patch
    // reached the leaf only, leaving every operator above it unsigned/32-bit.
    st.wt = crate::width::WidthTable::build_with(ir, &st.func_table, &opts.class_field_widths);
    // WIDE-ARITH-CAP: a multi-word `*`/`/`/`%`/`**` wider than the cap is poisoned
    // to X at eval (the kernels would otherwise stall). Warn ONCE here, by a
    // single static scan, so the degradation is loud, not silent. Uses each
    // arith node's self-width — the width every reachable eval sees, since a
    // wider assignment context would need a net wider than MAX_NET_WIDTH (which
    // elaborate already rejects).
    if ir.exprs.iter().enumerate().any(|(i, e)| {
        matches!(
            e,
            sim_ir::Expr::Binary {
                op: sim_ir::BinOp::Mul
                    | sim_ir::BinOp::Div
                    | sim_ir::BinOp::Mod
                    | sim_ir::BinOp::Pow,
                ..
            }
        ) && st.wt.width(i as u32) > crate::eval::WIDE_ARITH_CAP
    }) {
        sink.emit(LogEvent::Diagnostic(diag::Diagnostic {
            severity: diag::Severity::Warning,
            code: diag::MsgCode::RunWideArith,
            message: format!(
                "multi-word arithmetic exceeds the {}-bit width cap; result poisoned to X \
                 (the kernel would otherwise stall — narrow the operands)",
                crate::eval::WIDE_ARITH_CAP
            ),
            location: None,
            context: Vec::new(),
            sim_time: None,
        }));
    }
    // SVA-REST: assertion-control sidecars (gated fires + `$assertoff/on/kill` sites).
    st.assert_fire = opts.assert_fire.clone();
    st.assert_ctl = opts.assert_ctl.clone();
    st.task_calls_proc = opts.task_calls_proc.clone(); // B2
    st.task_calls_func = opts.task_calls_func.clone(); // B2
    st.max_class_objs = opts.max_class_objs; // CLASS-HEAP-CAP

    // Round-14 V3/V4: recompute the suspendable-task set from the SimIr func arena — no
    // serialized sidecar (format_version 22 unchanged), identical on the one-shot and
    // staged paths. `run_process` routes these tasks through the call-stack path.
    // r18: `base_nets` (each func's frame base, from `func_table`, itself the elaborate
    // `func_metas` threaded verbatim) makes the classifier frame-aware — a task that
    // writes an out-of-frame (module/instance) net is suspendable, not loud.
    let base_nets: Vec<u32> = st.func_table.iter().map(|m| m.base_net).collect();
    // §4.5.208: `has_hier_call` forces a frame task with a deferred hier enable suspendable —
    // consistently with elaborate (both derive it from the same serialized `FuncMeta`).
    let force_suspend: Vec<bool> = st.func_table.iter().map(|m| m.has_hier_call).collect();
    // R23 §3.1: the nested call sites' COPY-OUT destinations, reduced through the shared
    // `sim_ir::call_out_nets` — elaborate calls the same function over the same table
    // (`task_calls_func` is threaded verbatim), so both computes agree by construction. A
    // call whose output actual escapes the calling frame's window routes that caller here,
    // to `run_process`, whose copy-out goes through the `write_lvalue` funnel.
    let call_out_nets = sim_ir::call_out_nets(
        st.task_calls_func
            .iter()
            .map(|(b, info)| (*b, info.out_binds.as_slice())),
    );
    st.suspendable_tasks = sim_ir::compute_suspendable_tasks(
        &st.ir.funcs,
        &st.ir.blocks,
        &st.ir.stmts,
        &st.ir.exprs,
        &base_nets,
        &force_suspend,
        &call_out_nets,
    );

    // A7: the FINAL hit-bitmap of every coverage item, harvested from whichever
    // store actually ran.
    //
    // ⚠️ This exists because the coverage summary below reads
    // `st.nets[it.bitmap_net].cur` — the ENGINE's flat store — which a native run
    // never writes. Coverage itself needed no work at all: elaborate DESUGARS
    // `cg.sample()` into ordinary bit-set assignments on a bitmap net and
    // `get_coverage()` into ordinary arithmetic over it, so the tier-3 walk has
    // been executing covergroups correctly all along (the same discovery V1
    // slice 1 made about SVA). What it could not do is REPORT them: the arena is
    // dropped at the end of the native arm, so the bits had to be taken before
    // it goes.
    //
    // `None` on the engine path, and that is what keeps it byte-identical: the
    // summary falls through to exactly the read it always made.
    let mut cover_bits: Option<Vec<value::Value>> = None;
    // ⚠️ B4b: `fatal_run` above latched `had_fatal`/`finished`, but latching is
    // not skipping — measured, the run still walked into `NetArena::build`'s
    // `expect` and PANICKED. A graceful fatal has to also decline to execute, so
    // the executor selection asks whether the design is still runnable at all.
    let reason = if st.finished {
        crate::sched::FinishReason::Finish
    } else if effective_backend == Backend::Native {
        // ③층 (S1d-4c-2c): the design passed all three gate layers, so the tier-3
        // run loop owns the whole simulation — there is no body-level fallback
        // (doc-21 §4.1: a native backend owns net storage, so the interpreter
        // cannot see its nets). The scheduler is still constructed, as the HOST
        // for everything that is not a net value: the output sink, the file
        // table, `now`, the RNG.
        //
        // `class_new_sites` is cloned BEFORE the scheduler takes `&mut st`, and
        // it is provably empty here (the `class` eligibility row counts it), so
        // the clone is free and the map is the same one the engine would read.
        let class_new_sites = opts.class_new_sites.clone();
        let arena = native::arena::NetArena::build(ir, &opts)
            .expect("the runtime gate ANDs the arena build, so a refused build cannot get here");
        let mut sched = Scheduler::new(
            &mut st,
            opts.max_deltas,
            opts.max_body_steps,
            opts.time_limit,
            opts.fork_modes,
        );
        let mut nk = native::kernel::NativeKernel::new(
            ir,
            arena,
            &mut sched,
            &class_new_sites,
            opts.max_body_steps,
        );
        let reason = native::run::run(&mut nk, ir);
        // …before `nk` (and the arena inside it) is dropped. Read through
        // `NetReader::read_net`, the composite — not `arena.read_net` — so a
        // bitmap net that is heap-kind or frame-local would still be answered
        // by whoever owns it rather than from a dead slot. Neither is
        // constructible from `cover.rs` today (a bitmap is a packed `logic`
        // declared at module scope); asking through the funnel costs nothing
        // and is the answer that stays right if that changes.
        if !opts.coverage_manifest.is_empty() {
            use crate::eval::NetReader as _;
            cover_bits = Some(
                opts.coverage_manifest
                    .iter()
                    .flat_map(|inst| inst.items.iter())
                    .map(|it| nk.read_net(it.bitmap_net, None))
                    .collect(),
            );
        }
        reason
    } else {
        let mut sched = Scheduler::new(
            &mut st,
            opts.max_deltas,
            opts.max_body_steps,
            opts.time_limit,
            opts.fork_modes,
        );
        // t0 structural settle. If it can't converge (cont-assign oscillator),
        // stop immediately with DeltaLimit rather than running on a divergent t0.
        if sched.settle_cont_assigns().is_some() {
            sched.arm_processes();
            let reason = sched.run();
            #[cfg(feature = "jit")]
            if std::env::var_os("VITA_JIT_STATS").is_some() {
                crate::jit::jit_stats();
            }
            // P2-E: end-of-simulation `final` blocks (zero-time one-shots),
            // whatever the finish reason — including the delta-limit path's
            // else arm below NOT running them (a divergent t0 has no
            // meaningful end-of-sim state).
            sched.run_finals();
            reason
        } else {
            FinishReason::DeltaLimit
        }
    };

    st.finalize_vcd();

    // G2 FST breadth: a `.fst` dump target is produced by transcoding the sidecar
    // VCD. Dropping the writer flushes + closes the sidecar (single-threaded for
    // FST, so there is no writer thread to join); then transcode to the real
    // `.fst` path and remove the sidecar. A transcode failure is LOUD — the
    // waveform must never silently vanish (same MsgCode as a VCD write failure);
    // the sidecar VCD is left in place for debugging.
    if let Some(fst_path) = st.fst_target.take() {
        st.vcd = None; // flush + close the sidecar VCD file
        let tmp = format!("{fst_path}.vcdtmp");
        let version = format!("vitamin-sim {}", env!("CARGO_PKG_VERSION"));
        let date = st.vcd_date.clone();
        match vcd_writer::fst::transcode_vcd_to_fst(
            std::path::Path::new(&tmp),
            std::path::Path::new(&fst_path),
            &version,
            &date,
        ) {
            Ok(()) => {
                let _ = std::fs::remove_file(&tmp);
            }
            Err(e) => {
                st.sink.emit(LogEvent::Diagnostic(diag::Diagnostic {
                    severity: diag::Severity::Warning,
                    code: diag::MsgCode::RunVcdWriteFail,
                    message: format!("FST transcode failed for '{fst_path}': {e}"),
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(diag::TimeStamp { ticks: st.now }),
                }));
            }
        }
    }

    let exit_class = if st.had_fatal {
        ExitClass::Fatal
    } else if st.had_error.get() {
        ExitClass::HadErrors
    } else {
        ExitClass::Ok
    };

    sink.emit(LogEvent::Progress(ProgressEvent {
        message: format!("simulation ended ({:?}) at time {}", reason, st.now),
    }));

    // OBS-1b: build the end-of-run functional-coverage summary from the manifest +
    // final hit-bitmap values. Mirrors `synth_cover_get` EXACTLY (same countones —
    // 1-bits excluding X/Z — same per-item `covered*100.0/num_bins`, same
    // coverpoint-then-cross accumulation ORDER, same `sum/max(total_weight,1)` weighted
    // average) so `coverage.json` can never disagree with `c.get_coverage()`.
    let coverage = (!opts.coverage_manifest.is_empty()).then(|| {
        // A7: ONE flat index across every instance's items, walked in the same
        // nested order `cover_bits` was collected in. A per-instance index would
        // be a second spelling of that order, and getting it wrong reports one
        // covergroup's bins under another's name.
        let mut flat = 0usize;
        let groups = opts
            .coverage_manifest
            .iter()
            .map(|inst| {
                let mut sum = 0.0f64;
                let mut total_weight: u64 = 0;
                let items = inst
                    .items
                    .iter()
                    .map(|it| {
                        let i = flat;
                        flat += 1;
                        // The store that RAN. `None` (the engine path) is the
                        // read this always made; `Some` is the tier-3 arena's
                        // harvest, taken before the arena was dropped.
                        let (val, unk): (&[u64], &[u64]) = match &cover_bits {
                            Some(bits) => (&bits[i].val, &bits[i].unk),
                            None => {
                                let bp = &st.nets[it.bitmap_net as usize].cur;
                                (&bp.val, &bp.unk)
                            }
                        };
                        let mut covered: u32 = 0;
                        for (k, &v) in val.iter().enumerate() {
                            covered += (v & !unk.get(k).copied().unwrap_or(0)).count_ones();
                        }
                        let pct = if it.num_bins == 0 {
                            0.0
                        } else {
                            f64::from(covered) * 100.0 / f64::from(it.num_bins)
                        };
                        // Coverpoints with num_bins==0 are EXCLUDED from the average
                        // (not a coverage target); crosses always count (weight 1).
                        if it.is_cross || it.num_bins > 0 {
                            sum += f64::from(it.weight) * pct;
                            total_weight += u64::from(it.weight);
                        }
                        CovItemResult {
                            name: it.name.clone(),
                            is_cross: it.is_cross,
                            num_bins: it.num_bins,
                            covered_bins: covered,
                            coverage_pct: pct,
                        }
                    })
                    .collect();
                let coverage_pct = if total_weight == 0 {
                    0.0
                } else {
                    sum / total_weight as f64
                };
                CovgResult {
                    instance: inst.inst.clone(),
                    coverage_pct,
                    items,
                }
            })
            .collect();
        CoverageSummary { groups }
    });

    // OBS-2: hand off the accumulated trace lines (Some iff `--probe` was set).
    let trace = (!opts.probed_nets.is_empty()).then(|| std::mem::take(&mut st.trace_lines));
    // OBS-3: hand off stage lines (Some iff `+STAGE_TRACE` armed the capture).
    let stage = st
        .stage_enabled
        .then(|| std::mem::take(&mut st.stage_lines));

    SimResult {
        finish_reason: reason,
        sim_time: st.now,
        exit_class,
        vcd_path: st.vcd_path.clone(),
        coverage,
        trace,
        stage,
        // T0: `st.class_new_sites` (not `opts.`) — the copy the VM compile gate
        // itself reads, so this is the real gate's verdict, not a re-derivation.
        codegen: backend::codegen_report(ir, &st.class_new_sites),
        native: native_eligibility,
        backend: effective_backend,
    }
}

/// Convenience: run a simulation capturing RTL output into a `String` and the
/// VCD into the file named by `$dumpfile`/`override`. Returns (result, stdout).
/// Primarily for tests + a simple CLI path.
pub fn simulate_capture(ir: &SimIr, opts: SimOpts) -> (SimResult, String) {
    let buf = Rc::new(RefCell::new(String::new()));
    let sink = CaptureSink { buf: buf.clone() };
    let result = simulate(ir, &sink, opts);
    let s = buf.borrow().clone();
    (result, s)
}

struct CaptureSink {
    buf: Rc<RefCell<String>>,
}

impl LogSink for CaptureSink {
    fn emit(&self, event: LogEvent) {
        if let LogEvent::RtlOutput(t) = event {
            self.buf.borrow_mut().push_str(&t.text);
        }
    }
}
