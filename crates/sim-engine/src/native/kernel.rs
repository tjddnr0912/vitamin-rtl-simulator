//! S1d-4a (doc-21 §5 S1 분해) — **the arena as a second `Kernel` implementor.**
//!
//! The grounding that reshaped this plan: tier-3's executor is not a second
//! executor, it is the `Kernel` trait's second implementor. `compute_effect` and
//! `apply_effect` are already generic over `K: Kernel`, so every statement
//! MEANING — context-sizing, offset sampling, the NBA sample-at-schedule rule,
//! the real→int coercion — is shared code that runs unchanged over this store.
//! Byte-identity with the engine is therefore not a property to test for and
//! hope holds; it is structural, and the differential gate exists to catch the
//! places where THIS file breaks it, not the places the shared executor might.
//!
//! ## What "honest" means for the methods that are not implemented, in TWO kinds
//!
//! 54 declarations (53 without the `jit` feature; S1d-4c-2b added `k_call_fatal`
//! and `k_enter_body` for the body walk). They divide four ways, and three slices moved four of
//! them: **20 store core** · **17 classification
//! predicates** · **17 gate-refused workers** · **0 NOT BUILT**. 20+17+17 = 54
//! WITH the `jit` feature; the default build has 53, because `k_nets` is
//! jit-gated and sits in store core — the arithmetic is the point, so the
//! feature it depends on has to be stated too. S1d-5 moved `k_value_plusargs`
//! from gate-refused to store core (the first `stmt_effect` family member
//! wired; the gate row carves it out). S1d-4b-2
//! implemented `k_dispatch_systask` and `k_sformatf`, S1d-4c-1
//! `k_schedule_nba_at`, and S1d-4c-2a `k_rearm` — the surface is complete, which
//! is NOT the same as the backend being runnable — though as of S1d-4c-2d it
//! IS: the region queues, the delta loop, `busy` and the in-body waiters all
//! exist, and `--backend native` runs the designs three gate layers admit. The counts are counted, not estimated — an early draft said
//! "20 refused" and "16 file methods" and both were wrong (18 and 8).
//!
//! The two-kind split is the correction both reviewers of this slice forced, and
//! it matters more than the arithmetic:
//!
//! - **Gate-refused (18)**: `native::design_eligibility` refuses every design
//!   that can reach them — force/release, queue pop, assoc iteration, class
//!   alloc, `disable fork`, and the ten funnel-outside workers §4.5.291's
//!   `stmt_effect` row still covers (seeded `$random`/`$dist_*`, `$cast`,
//!   the 8 file methods — `$value$plusargs` was the eleventh until S1d-5
//!   wired it). `gate_refused!` names the row.
//! - **NOT BUILT (0)** as of S1d-4c-2a. An ELIGIBLE design reaches every one of these; what keeps them
//!   out of production is `native::runtime_gate` choosing the VM, one layer below
//!   eligibility (§4.5.288's two-layer verdict). `not_built!` says so and names
//!   the slice that builds it. Using the gate-refused wording here — which the
//!   first draft did for `k_sformatf` — points a maintainer at a gate widening
//!   that never happened.
//!
//! Unreachable is not the same as harmless in either kind: a silent no-op would
//! be a wrong answer waiting for the day the layer above it moves.
//!
//! **The predicates are NOT stubbed, and that asymmetry is the whole design.**
//! A `k_*_rhs` predicate answers a CLASSIFICATION question that decides which
//! effect `compute_effect` builds. Hard-coding `false` — the obvious "it can't
//! happen" stub — would not make those statements loud; it would silently route
//! them down the pure-eval path and produce a different statement. So every
//! predicate is answered for real, through `exec::kpred`, the same functions the
//! `Scheduler` impl now calls. One spelling ⇒ the two backends cannot disagree
//! about what a statement IS; only the workers are loud, and only where the gate
//! already guarantees nothing arrives.
//!
//! ## What is deliberately NOT here
//!
//! - **`k_dispatch_systask` and `k_sformatf` are WIRED (S1d-4b-2).** The format
//!   engine takes the arena as a generic reader (4b-1) and `dispatch` now takes
//!   it too, as an `Option` — `None` is the scheduler's own state, which is what
//!   every engine call site passes and what makes those sites byte-identical.
//!
//!   ⚠️ **Threading the FORMATTER was only half of it.** A task's own arguments
//!   are net reads that never pass through the format engine — the fd of
//!   `$fdisplay`, the units of `$timeformat` — and threading the formatter alone
//!   left them reading `sched.st`. Measured: `$fdisplay(fd, …)` with a NET fd
//!   read the untouched engine store, got X, and DROPPED the line with a
//!   bad-descriptor warning, on a design the gate reports fully runnable. Both
//!   are now threaded through `eval_task_arg`.
//!
//!   ⚠️ **And `$timeformat` could not have been refused by task id.** It lowers
//!   to a `Display` plus a `timeformat_stmts` sid, so a refusal keyed on `which`
//!   is structurally the wrong key — an argument for threading rather than
//!   refusing whenever the shape allows it.
//!
//!   **Refused, and why each is a REAL wrong-store read rather than caution:**
//!   `$dumpvars`/`$dumpall`/`$dumpon` (`full_snapshot` walks `&st.nets`
//!   wholesale — S1d-4d owns VCD); `$dumpfile`/`$dumplimit`/`$fclose` (argument
//!   read outside the formatter, and unlike the fd above these have no tier-3
//!   consumer yet); `$writememb`/`$writememh` (reads the MEMORY itself);
//!   `$monitor`/`$strobe` (dispatch only REGISTERS them — it captures ExprIds,
//!   and the render happens in `sched/run_loop.rs::flush_postponed`, which this
//!   seam does not reach; a `$monitor` would print its t0 line and never
//!   re-fire). Every one of these is `eligible: true, buildable: true`, which is
//!   exactly why the refusal has to live here rather than in the design gate.
//!
//!   Gate-refused elsewhere and so unreachable: `$sformat` and `$readmem*`
//!   (`stmt_effect`, `NetWrite::Flat`), the deferred-assert render, `$fmonitor`/
//!   `$fstrobe` (`file_directed_stmts`), `$vita_stage` (`stage`).
//!
//!   Line numbers are deliberately omitted throughout: an earlier version cited
//!   six and the next slice moved three of them without re-pinning. Re-measure
//!   from the store side (`&st.nets`, `st.nets[`, `read_net`, `eval_expr`,
//!   `sched.eval`) rather than from a name list — the first attempt grepped
//!   three spellings and came out 3x low.
//! - **The NBA queues here are a flat `Vec` plus a delayed `BTreeMap`, not the
//!   region machinery.** S1d-4c-1 added the DRAIN (`apply_nba`,
//!   `take_due_delayed`); regions and the delta loop are 4c-2. Entries
//!   are `NbaUpdate` — the ENGINE's type, not a parallel one — so the gate can
//!   compare them field by field, and so 4c inherits a queue whose shape it does
//!   not have to migrate. Draining/regions/delta are 4c.
//! - **`k_rearm` is loud, not a no-op.** Re-arming reads the activity arena's
//!   `is_child` flag and the process's sensitivity kind; that state belongs to
//!   the scheduler 4c builds. A silent no-op would leave a Level process asleep
//!   forever — a hang, not an error — which is precisely the class this file
//!   refuses to introduce.

use std::collections::BTreeMap;

use sim_ir::{LvalChunk, Lvalue, SimIr, SysTaskId};

use crate::builtins::Ctl;
use crate::exec::{Kernel, Offsets};
use crate::native::arena::NetArena;
use crate::sched::{NbaLhs, NbaUpdate};
use crate::value::Value;

/// A method the S0 DESIGN gate makes unreachable. `row` names the eligibility
/// row that refuses it, so a future widening of the gate reads as an instruction
/// rather than a mystery. 17 methods qualify (counted, not estimated).
macro_rules! gate_refused {
    ($m:literal, $row:literal) => {
        panic!(
            "tier-3 native kernel: {} is unreachable — `native::design_eligibility` \
             refuses every design that can call it ({}). Reaching this means the gate \
             widened without wiring the method; wire it or restore the row.",
            $m, $row
        )
    };
}

/// A method that is NOT gate-refused — an ELIGIBLE design reaches it — and is
/// simply not built yet. What keeps it out of production is `native::runtime_gate`
/// choosing the VM, one layer below eligibility (§4.5.288's two-layer verdict).
///
/// The distinction is the whole reason this is a second macro. Both reviewers of
/// this slice found the first version using the gate-refused wording for methods
/// no row refuses, which points a future maintainer at a gate widening that never
/// happened — the exact opposite of what the message is for.
macro_rules! not_built {
    ($m:expr, $slice:expr, $why:expr) => {
        panic!(
            "tier-3 native kernel: {} is NOT built yet ({} builds it) — {}. No \
             eligibility row refuses a design that calls it; `native::runtime_gate` \
             keeping such designs on the VM is what makes this unreachable today, so \
             reaching it means a backend was selected before its kernel was finished.",
            $m, $slice, $why
        )
    };
}

/// The tier-3 `Kernel`: the arena store plus the cold context an `EvalCtx` needs.
///
/// Borrows the IR-derived tables rather than owning them — they are the SAME
/// tables the engine reads, so a width or plusarg can never differ between the
/// two backends by construction.
#[allow(dead_code)]
// S1d-4b's body walk is the production constructor; today
// only the shared-executor differential builds one. Saying that is more
// honest than a fake call site or a widened visibility.
/// See `NativeKernel::wcache`. One slot per ExprId — a direct-indexed vector,
/// not a map: profiling S2 slice 2 found the `BTreeMap` LOOKUP had become the
/// single hottest frame in the native walk (792 samples to the programs' own
/// 367), which is the specialization paying for itself twice over.
pub(crate) type WCache = Vec<Option<WCacheSlot>>;

/// A cached compile for one ExprId: the context it was compiled for, and the
/// program (`None` = a cached DECLINE). A slot holds ONE context; an eid asked
/// under a second `(width, signed)` overwrites it and recompiles. That is a
/// performance choice with no correctness weight — `compile` reads only the IR
/// and the arena's build-time layout, both immutable for a run.
///
/// ⚠️ **Both key fields are unfalsifiable today, measured rather than assumed.**
/// Dropping the `signed` comparison, and never overwriting on a context
/// mismatch, each survive the whole suite: instrumenting `wprog_for` counted
/// ZERO context mismatches across the corpus, the four `examples/`, and the
/// review designs, because elaborate interns nothing (one fresh eid per site)
/// and both callers derive `signed` from the same `wt.get(eid)`. Kept as
/// defence for the day an IR-injection pass shares an eid — and stated here as
/// "unproven", because the earlier version of this note cited a count over
/// ASSIGNMENT sites while S2 slice 2 had already added a second caller
/// (`k_truthy`) with a different width rule.
pub(crate) struct WCacheSlot {
    pub(crate) width: u32,
    pub(crate) signed: bool,
    pub(crate) prog: Option<std::rc::Rc<crate::native::wprog::WProg>>,
}

impl WCacheSlot {
    /// Does this slot answer a request for `(w, signed)`?
    ///
    /// ONE spelling, because two readers now consult the cache (`wprog_for` and
    /// `run_cached_wprog`) and the SIGN half of the key is the part no design
    /// can check: an ExprId is a single source occurrence, so it is always asked
    /// for at one width and one signedness, and dropping `signed` here leaves
    /// the whole suite green (measured). The key still carries it because
    /// `wprog` bakes the operand sign into its comparison ops — a program
    /// compiled signed answers a different question — so the guard is right even
    /// though nothing can currently reach past it. Sharing the predicate is what
    /// keeps the two readers from disagreeing about that.
    fn hits(&self, w: u32, signed: bool) -> bool {
        self.width == w && self.signed == signed
    }
}

/// How one lvalue index expression resolves. See `NativeKernel::icache`.
pub(crate) enum IdxKind {
    /// A compile-time constant index: the offset itself.
    Const(u32),
    /// A width-specialized program; its result goes through the shared rule.
    Prog(std::rc::Rc<crate::native::wprog::WProg>),
    /// Not admitted — the whole lvalue falls back to the generic resolver.
    Generic,
}

pub(crate) struct NativeKernel<'i, 'a, 'b> {
    pub(crate) ir: &'i SimIr,
    pub(crate) arena: NetArena,
    /// The engine state, for everything that is NOT net values.
    ///
    /// A BORROW, not a copy, and that is the fix for a hazard the adversarial
    /// review measured. The first version carried its own `now`, `time_mult`,
    /// `prec_mult`, `rng`, `wt` and `plusargs` — and the format engine reads
    /// `now`, `cur_time_mult`, `rng`, `timeformat`, `global_prec_exp` and
    /// `cur_scope` off `&SimState`. So the moment S1d-4b-2 wires
    /// `k_dispatch_systask`, a design like
    /// `$display("t=%t r=%0d n=%0d", $time, $random, n)` — eligible AND
    /// buildable, measured — would have taken its net values from the arena and
    /// its `$time` and `$random` from a DIFFERENT clock and a DIFFERENT stream.
    /// Not a compile error: a silent wrong line.
    ///
    /// `now` and `cur_time_mult` are also not "cold" in the sense the first doc
    /// claimed. `exec/process.rs` rewrites `cur_time_mult` on every process
    /// dispatch and the run loop rewrites `now` every timestep, so a copy is a
    /// staleness bug waiting for its first timestep.
    /// Upgraded from `&SimState` to `&mut Scheduler` in S1d-4b-2: dispatching a
    /// system task needs the output sink, the file table and the assertion side
    /// tables, all reached mutably through the scheduler. The reasoning above is
    /// unchanged — `sched.st` is still the ONE origin for `now`, `cur_time_mult`,
    /// `rng`, `timeformat` and `cur_scope`, and this kernel keeps no copies.
    pub(crate) sched: &'i mut crate::sched::Scheduler<'a, 'b>,
    /// `class_new_sites`, the one classification question that is not a function
    /// of `ir.exprs` (see `exec::kpred`'s module doc). Shared by reference for
    /// the same single-spelling reason the predicates are shared by call.
    pub(crate) class_new_sites: &'i BTreeMap<u32, u32>,
    /// The in-body step ceiling. A CONSTRUCTOR ARGUMENT, not a default: it was
    /// briefly `u64::MAX`, which is not "no opinion" but "no termination guard",
    /// and a runaway combinational body would have spun forever instead of
    /// reporting `F4027`. The gate caught it by comparing against the engine's.
    pub(crate) max_body_steps: u64,
    /// S3a: does this design have subroutine FRAMES at all?
    ///
    /// The composite `NetReader` below has to ask "is this net frame-local"
    /// before every read, and `SimState::frame_local` is a full-length `Vec`
    /// that is all-false when there are none. Hoisting the ONE question that
    /// answers a whole design keeps the leaf load off the hot path for the
    /// designs that have no frames — which is every one tier-3 ran before this
    /// slice. Derived from `func_table`, the same table
    /// `frames_admitted`/`build_func_routing` read.
    pub(crate) has_frames: bool,
    /// A3-ii-b: the open task frames of every process that is SUSPENDED inside
    /// one, keyed by process id.
    ///
    /// The engine's twin is `Scheduler::activities[pi].call_stack`, and the
    /// reason this is a map on the kernel rather than that arena is the S0 gate:
    /// forks are refused, so a tier-3 activity IS its process and there is
    /// nothing for an activity arena to disambiguate. What the two DO share is
    /// the window stash — `frame_window::{stash,restore}_windows_in`, which this
    /// slice extracted so the pop order has one spelling.
    ///
    /// A `BTreeMap` rather than a `Vec` indexed by process: it is empty for every
    /// design that has no parking frame, which is nearly all of them, and its
    /// iteration order is deterministic if a future reader ever needs it.
    pub(crate) parked_frames: BTreeMap<u32, Vec<crate::sched::FrameRec>>,
    /// The NBA queue in engine shape. S1d-4c-1 gave it the drain; regions and
    /// the delta loop are 4c-2.
    pub(crate) nba: Vec<NbaUpdate>,
    pub(crate) nba_seq: u64,
    /// TRANSPORT-delay updates, filed under the tick they are due — the engine's
    /// `delayed_nba`, same type and same key, so the two drains compare directly.
    ///
    /// ⚠️ **S1d-4c-2 OWES THIS QUIESCENCE.** `k_schedule_nba_at` used to be a
    /// `not_built!` panic and is now a silent enqueue, which is a step DOWN the
    /// accuracy ladder for as long as nothing drains it in production. The
    /// engine decides "is there future work" from `Scheduler::delayed_nba`
    /// (`run_loop.rs` computes `next` as the min over `wheel`/`delayed_ca`/
    /// `delayed_nba`) — and that map stays EMPTY on a native run, because every
    /// `k_schedule_nba*` lands here instead. A design whose only pending work is
    /// a transport NBA would therefore be reported quiescent and its update
    /// dropped. 4c-2's loop must fold THIS map into that minimum.
    ///
    /// Not reachable today — `simulate` forces `Backend::Native` to the VM. (An
    /// earlier version of this note pointed at `k_rearm` as a surviving guard;
    /// S1d-4c-2a implemented it, so there is no guard left anywhere on this
    /// path.) Recorded rather than left to be re-derived.
    pub(crate) delayed_nba: BTreeMap<u64, Vec<NbaUpdate>>,
    /// The reused destination for a single-chunk NBA write. The engine keeps one
    /// per flush for a measured reason (2.4M malloc/free pairs on a 40000-cycle
    /// picorv32 run), and carrying the same shape here keeps the write funnel's
    /// input identical rather than merely equivalent.
    nba_scratch_lhs: Lvalue,
    /// The S1d-3 wake table, which until S1d-4c-2a was driven only by its own
    /// differential. `k_rearm` writes it.
    ///
    /// ⚠️ **Same shape as `delayed_nba` above, and worth saying rather than
    /// leaving to be rediscovered**: `k_rearm` went from a `not_built!` panic to
    /// a write that NOTHING in production reads — `WakeTable::wake` has no
    /// production caller, and `NativeKernel` has no production constructor. The
    /// `Kernel` surface being complete is not the backend being runnable.
    ///
    /// ⚠️ And implementing `k_rearm` does NOT by itself give the native path
    /// re-arming: `k_rearm` is called only from `backend.rs` (the VM) and
    /// `jit.rs`. The interpreter's `run_process` calls `sched.rearm(pi)`
    /// DIRECTLY, not through the trait — so 4c-2 must either drive bodies
    /// through the VM path or route its own walk through this method.
    ///
    /// ⚠️ No reset: the t0 state is derived from `kind` alone, so a kernel built
    /// at t > 0 would re-arm every `Level` and dis-arm every `Comb`/`Latch` that
    /// had already run. Every construction site today builds a fresh arena
    /// alongside, so the lifetime is per-run; nothing structurally enforces it.
    pub(crate) wake: crate::native::wake::WakeTable,
    /// The REGION QUEUES and the time wheel (S1d-4c-2c). The engine's
    /// `cur.active` / `cur.inactive` / `wheel`, restated over the collapsed
    /// `Ready`: eligibility refuses forks, so `tie == proc == template` and a
    /// ready entry is `(proc, resume block)`.
    ///
    /// They live on the KERNEL rather than on the run loop because
    /// `k_schedule_resume` is what fills them — a `Terminator::Delay` decides to
    /// suspend inside the SHARED body walk, and only the implementor knows where
    /// the resume is filed.
    pub(crate) active: Vec<NativeReady>,
    pub(crate) inactive: Vec<NativeReady>,
    pub(crate) wheel: BTreeMap<u64, Vec<(bool, NativeReady)>>,
    /// IN-BODY waiters (S1d-4c-2d) — `Scheduler::waiters`, restated.
    ///
    /// Separate from `WakeTable`, which holds the STATIC sensitivity of an
    /// `always @(…)` header. These are the ones a body creates by suspending on
    /// an event mid-execution, and the difference is not cosmetic: a static
    /// Level waiter fires on any change to a watched net, while an in-body one
    /// fires only when the net differs from what it was AT SUSPEND TIME.
    pub(crate) waiters: Vec<NativeWaiter>,
    /// S2 slice 1: width-specialized program cache for the eval funnels,
    /// indexed by ExprId (see `WCache`). `RefCell` because those funnels are
    /// `&self`; the scratch stack is reused across calls for the same reason.
    /// Pure function of the static IR ⇒ no invalidation exists.
    pub(crate) wcache: std::cell::RefCell<WCache>,
    /// S2 slice 3: the INDEX cache — one slot per ExprId for the offset an
    /// lvalue index expression names. `k_resolve_lvalue_offsets` runs once per
    /// assignment (71.9k times on the tier-3 hot design, measured, every one of
    /// them through the generic evaluator including CONSTANT indices), and its
    /// index expressions are tiny, so the win is skipping the evaluator rather
    /// than the arithmetic.
    ///
    /// The offset RULE is not restated: `Const` resolves at compile time
    /// through the same `offset_of_index_value`, and a `Prog` runs the
    /// width-specialized program and hands the resulting value to that same
    /// function. Anything else falls back to `eval::resolve_offsets` whole.
    pub(crate) icache: std::cell::RefCell<Vec<Option<IdxKind>>>,
    pub(crate) wscratch: std::cell::RefCell<Vec<crate::native::wprog::W>>,
    /// **S3 slice 1 — the compiled body, one slot per process template.**
    ///
    /// The tier-3 walk decided per EXECUTION what each statement is
    /// (`compute_effect`), re-resolved every lvalue's offsets and re-asked the
    /// `wprog` cache on every assignment. A `CompiledBody` decides all of that
    /// ONCE per template, and it is also the input cranelift takes
    /// (`jit::compile_body`) — so this is S3's first step rather than a detour
    /// around it.
    ///
    /// Same `VmSlot` type and same decide-once protocol as `SimState::vm_cache`,
    /// deliberately: the compiled form is shared, not re-spelled. What differs is
    /// the `CompileCtx` — tier-3 asks for the plain-scalar specialisation and
    /// REFUSES `Op::EvalNative` (see `CompileCtx`).
    pub(crate) bodies: Vec<crate::backend::VmSlot>,
    /// Pooled register/offset files for `vm_exec`, leased per activation by
    /// `std::mem::take` (an OWNED buffer cannot alias the `&mut self` the kernel
    /// calls need). `nregs`/`noffs` are 1 and ≤1, so this is about not calling
    /// malloc twice per process activation, not about size.
    pub(crate) vm_regs: crate::backend::RegFile,
    pub(crate) vm_offs: crate::backend::OffFile,
    /// Run the compiled body when one exists?
    ///
    /// What this slice must prove is not byte-identity of one function against
    /// itself — it is that two whole EXECUTORS agree over a whole run, and a
    /// funnel that delegates cannot be its own oracle. So both halves have to be
    /// reachable in one binary. In a NON-test build the initialiser is the
    /// literal `true` and this is read once per activation; the switch that can
    /// say otherwise exists only under `cfg(test)`.
    pub(crate) use_compiled: bool,
}

// TEST-ONLY switches for the S3-1 differential.
//
// `USE_COMPILED` forces tier-3 back onto the `run_body` walk.
// `COMPILED_ACTIVATIONS` counts activations that actually ran a compiled body —
// the differential's ANTI-VACUITY reading: without it, a `compiled_for` that
// silently returned `None` everywhere would compare the walk against the walk
// and pass.
//
// Thread-local because `cargo test` runs test functions in parallel and two of
// them walk the same corpus; a process-global would let one test's setting
// decide another test's executor.
#[cfg(test)]
thread_local! {
    pub(crate) static USE_COMPILED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    pub(crate) static COMPILED_ACTIVATIONS: std::cell::Cell<u64> = const {
        std::cell::Cell::new(0)
    };
}

#[cfg(test)]
fn use_compiled_default() -> bool {
    USE_COMPILED.with(|c| c.get())
}
#[cfg(not(test))]
fn use_compiled_default() -> bool {
    true
}

/// One in-body waiter: what it waits for, where it resumes, and — for an
/// `@(sig)` — the watched nets' values at suspend time.
///
/// `arm` is `Some` only for `Level`, exactly as the engine's `Waiter::arm` is.
/// It holds the WHOLE net's raw words rather than a `Value` because that is what
/// the engine compares (`SimState.nets[n].cur`, the packed array), so an
/// `@(mem)` on an array asks the same question on both sides.
pub(crate) struct NativeWaiter {
    pub(crate) cause: sim_ir::WaitCause,
    pub(crate) proc: u32,
    pub(crate) block: u32,
    pub(crate) arm: Option<Vec<Vec<u64>>>,
}

/// A queued activation: which process, and which block it resumes at.
///
/// The engine's `Ready` carries a `tie` as well, because a fork child runs a
/// sub-chain of its parent's body under a composite tie. `fork_modes` non-empty
/// is an S0 reject, so here `tie == proc` and the field would be a second name
/// for the same number — which is worse than absent, because it would look like
/// the two could differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeReady {
    pub(crate) proc: u32,
    pub(crate) block: u32,
}

/// Why the tier-3 kernel refuses to dispatch one system task.
pub(crate) struct SysTaskRefusal {
    pub(crate) label: &'static str,
    pub(crate) slice: &'static str,
    pub(crate) why: &'static str,
}

/// Does the tier-3 kernel refuse to dispatch `which`, and why?
///
/// ONE match with TWO consumers, and that is the point rather than tidiness.
/// `k_dispatch_systask` panics on a `Some`, and `native::run::runnable` refuses
/// the design so nobody ever gets there. Written as two matches — the obvious
/// spelling — the run gate would go stale the first time an arm is added here,
/// and the symptom would be a mid-run panic on a design the gate called runnable.
/// That is precisely the twin-predicate hazard, so the twin is not written.
pub(crate) fn systask_refusal(which: SysTaskId) -> Option<SysTaskRefusal> {
    let (label, slice, why) = match which {
        // `$dumpvars` is WIRED (S1d-4d-2) — `full_snapshot_with` takes the arena
        // as its reader. `$dumpall`/`$dumpon` still are not: they re-snapshot
        // through the same function but from `dispatch`'s own call sites, which
        // this seam does not thread. One more slice, not a subsystem.
        SysTaskId::DumpAll | SysTaskId::DumpOn => (
            "k_dispatch_systask($dumpall/$dumpon)",
            "S1d-4d-3",
            "they re-snapshot through `full_snapshot`, and only the `$dumpvars` \
             call site threads the arena reader so far",
        ),
        // ⚠️ `$monitor`/`$strobe` are WIRED (A5-b). The row that stood here was
        // right about the mechanism and it took two things to lift, not one:
        // the tier-3 run loop now HAS a postponed region (`native::run::
        // flush_postponed`, called at the settled point and at the three
        // terminating arms, as the engine's loop does), and
        // `flush_postponed_with` threads the store into the two places that
        // read nets — the render and, for `$monitor`, the change compare.
        // Registering was never store-bound: `dispatch`'s arms capture ExprIds
        // and metadata and touch no net.
        // `$dumpfile` is WIRED (S1d-4d-2) — but NOT for the reason the first
        // version of this comment gave. It claimed `arg_string` returns early
        // for anything that is not `Expr::Const`, so the task never reads a net.
        // Measured false: it falls through to a VALUE RENDER, and on a native
        // run that read the engine's untouched store — `$dumpfile(nm)` with
        // `nm = 42` wrote a file named `x` instead of `42`, with identical
        // stdout and identical VCD content. What actually makes it safe is that
        // `dispatch_with` now threads the reader into `arg_string_with`.
        // `$dumplimit`/`$fclose` still take INT arguments through `int_arg`,
        // which is not threaded.
        // ⚠️ `$dumplimit`/`$fclose` are WIRED (A5-a). The row that stood here
        // said "the ARGUMENT is read through `int_arg`, not the formatter", and
        // that reason was STALE in two ways: `int_arg` is threaded (it calls
        // `eval_task_arg`) and neither of these two ever used it — each read its
        // own argument with a bare `sched.eval`. Threading those two call sites
        // is the whole fix. §4.5.338's lesson again: a refusal does not know
        // when its own reason stopped being true.
        // ⚠️ `$writememb`/`$writememh` are WIRED (slice #8), and their row was
        // the last entry in this match that a corpus design reached. It said
        // "it reads the MEMORY itself, not a formatted argument", which was
        // exactly right and exactly three reads: the two window bounds (which
        // A1-iii's own note left recorded as still raw) and the per-element
        // value. All three take the threaded reader now.
        // ⚠️ `$dumpoff`/`$dumpflush`/`$monitoron`/`$monitoroff` are deliberately
        // NOT here — and the reason they used to carry ("nothing opens `st.vcd`
        // because `$dumpfile` is refused") DIED when S1d-4d-2 wired the dump
        // tasks. What makes them correct now is direct rather than inherited:
        // `$dumpoff` sets `dumping = false`, and `vcd_id_for` returns `None`
        // while it is false, so the arena's captures are discarded at the drain
        // exactly as the engine's emits are skipped. `$dumpflush` only flushes.
        // Measured: `$dumpoff` standalone, mid-slot and before `$dumpvars` all
        // match the VM byte for byte.
        _ => return None,
    };
    Some(SysTaskRefusal { label, slice, why })
}

/// `sched::push_sorted` over the collapsed ready: insert keeping `proc`
/// ascending, AFTER every equal entry.
///
/// ⚠️ The `<=` is written to match the engine, NOT because it can be observed
/// here — an earlier version of this doc claimed it was load-bearing and that
/// claim did not survive review. Two entries can never share a `proc` in the
/// accepted class: `WakeTable::wake` dedups per process (`seen` for Edge,
/// waiter CONSUMPTION for Level, and the two maps are kind-disjoint), `arm_t0`
/// visits each process once, and a process has at most one pending resume. So
/// `<=` → `<` is an EQUIVALENT mutation, and saying so is better than a teeth
/// claim no design can back. It stays `<=` because the collapse is a property
/// of today's gate, not of the ordering rule.
pub(crate) fn push_sorted_native(q: &mut Vec<NativeReady>, r: NativeReady) {
    let pos = q.partition_point(|x| x.proc <= r.proc);
    q.insert(pos, r);
}

#[allow(dead_code)] // ditto — `new`/`ctx` have exactly one caller, the gate.
impl<'i, 'a, 'b> NativeKernel<'i, 'a, 'b> {
    pub(crate) fn new(
        ir: &'i SimIr,
        arena: NetArena,
        sched: &'i mut crate::sched::Scheduler<'a, 'b>,
        class_new_sites: &'i BTreeMap<u32, u32>,
        max_body_steps: u64,
    ) -> NativeKernel<'i, 'a, 'b> {
        // The cont-assign dependency map is the SCHEDULER's, already derived
        // through `levelize::ca_deps`; the arena's write funnel maintains the
        // worklist from it rather than deriving a second one.
        let mut arena = arena;
        arena
            .ch
            .install_ca_deps(&sched.st.ca_of_net, ir.cont_assigns.len());
        let has_frames = !sched.st.func_table.is_empty();
        // Built BEFORE the struct literal moves `sched`: the wake table's clocking
        // diversion is keyed on the state's own two tables (see its field doc).
        let wake = crate::native::wake::WakeTable::new(ir, &*sched.st);
        NativeKernel {
            ir,
            arena,
            parked_frames: BTreeMap::new(),
            sched,
            class_new_sites,
            max_body_steps,
            has_frames,
            nba: Vec::new(),
            nba_seq: 0,
            delayed_nba: BTreeMap::new(),
            nba_scratch_lhs: Lvalue { chunks: Vec::new() },
            wake,
            active: Vec::new(),
            inactive: Vec::new(),
            wheel: BTreeMap::new(),
            waiters: Vec::new(),
            wcache: std::cell::RefCell::new((0..ir.exprs.len()).map(|_| None).collect()),
            icache: std::cell::RefCell::new((0..ir.exprs.len()).map(|_| None).collect()),
            wscratch: std::cell::RefCell::new(Vec::new()),
            bodies: (0..ir.processes.len())
                .map(|_| crate::backend::VmSlot::Unchecked)
                .collect(),
            vm_regs: Vec::new(),
            vm_offs: Vec::new(),
            use_compiled: use_compiled_default(),
        }
    }

    /// The compiled body for process template `proc`, compiling and caching on
    /// first sight; `None` ⇒ this template stays on the tier-3 walk.
    ///
    /// `SimState::vm_compiled`'s protocol, over this kernel's own cache: the
    /// codegen-ability question is asked ONCE per template and its answer
    /// remembered on BOTH sides, and the returned `Rc` is an owned handle taken
    /// out of the cache before any `&mut self` kernel call.
    ///
    /// `use_compiled == false` returns `None` BEFORE the cache is consulted, and
    /// without writing to it. Not because the differential would otherwise break
    /// (it builds a fresh kernel per run, so nothing carries over — an earlier
    /// version of this note claimed it did), but because `use_compiled` is not a
    /// property of the TEMPLATE: writing `NotCodegenable` would record an answer
    /// to a different question in the slot that answers "is this body
    /// codegen-able".
    pub(crate) fn compiled_for(
        &mut self,
        proc: usize,
    ) -> Option<std::rc::Rc<crate::backend::CompiledBody>> {
        use crate::backend::VmSlot;
        if !self.use_compiled {
            return None;
        }
        match &self.bodies[proc] {
            VmSlot::Compiled(rc) => return Some(std::rc::Rc::clone(rc)),
            VmSlot::NotCodegenable => return None,
            VmSlot::Unchecked => {}
        }
        // Copy the IR reference out so the immutable read does not borrow `self`
        // across the cache write below (the same dance `vm_compiled` documents).
        let ir: &SimIr = self.ir;
        if !crate::backend::is_codegen_able(
            &ir.stmts,
            &ir.exprs,
            &ir.processes[proc].body,
            self.class_new_sites,
        ) {
            self.bodies[proc] = VmSlot::NotCodegenable;
            return None;
        }
        // `plain_scalar` is a full-length `Vec<bool>` on `SimState`; take it for
        // the duration of the compile and put it back, exactly as `vm_compiled`
        // does, because `compile_body` borrows it while `self` is also borrowed.
        let plain = std::mem::take(&mut self.sched.st.plain_scalar);
        let compiled = std::rc::Rc::new(crate::backend::compile_body(
            &ir.stmts,
            &ir.processes[proc].body,
            Some(&crate::backend::CompileCtx {
                ir,
                wt: &self.sched.st.wt,
                plain: &plain,
                // NOT `Some(..)`: tier-3's RHS path is `wprog`, and `EvalNative`
                // would route around it into the tier-2 expression VM. See
                // `CompileCtx`.
                natives: None,
            }),
        ));
        self.sched.st.plain_scalar = plain;
        self.bodies[proc] = VmSlot::Compiled(std::rc::Rc::clone(&compiled));
        Some(compiled)
    }

    /// Run the cached width-specialised program for `(eid, w, signed)` and hand
    /// back its PLANES, without handing out an `Rc`.
    ///
    /// `wprog_for` is the same cache lookup, but its signature forces a
    /// `Rc::clone` on every hit so the caller can run the program outside the
    /// borrow. That refcount pair is paid 6.3 million times on picorv32
    /// (measured) for programs that are, 47% of the time, a single net read.
    /// Running INSIDE the borrow removes it. The two share the miss path — the
    /// compile still goes through `wprog_for`, so there is one compiler and one
    /// cache-fill, not two.
    ///
    /// `None` means "no admitted program" and is the SAME answer `wprog_for`
    /// gives, including for a cached decline (`slot.prog == None`): the caller
    /// falls back to the generic evaluator, which is the previous path.
    fn run_cached_wprog(&self, eid: u32, w: u32, signed: bool) -> Option<crate::native::wprog::W> {
        {
            let c = self.wcache.borrow();
            if let Some(Some(slot)) = c.get(eid as usize) {
                if slot.hits(w, signed) {
                    let prog = slot.prog.as_ref()?;
                    return Some(prog.run(&self.arena, &mut self.wscratch.borrow_mut()));
                }
            }
        }
        let prog = self.wprog_for(eid, w, signed)?;
        Some(prog.run(&self.arena, &mut self.wscratch.borrow_mut()))
    }

    /// The whole of `lhs = rhs` for a plain whole-net scalar destination that
    /// fits ONE arena word, done entirely in planes.
    ///
    /// `Some(())` means it was done; `None` means "not this shape" and the
    /// caller runs the split path, which IS the previous behaviour.
    ///
    /// What each precondition buys — every one of them is a fact the split path
    /// re-establishes at run time and this one is handed:
    ///
    ///  - `plain_scalar_dest` (the COMPILER's proof, carried in the op) already
    ///    excludes real, frame-local, handle, string, two-state and array
    ///    destinations, so the real→int arm, the X/Z coercion arm and the
    ///    element index all drop out. `force`/`release` is refused by the tier-3
    ///    design gate, so the funnel's `forced` test has nothing to test.
    ///  - `s.words == 1 && s.width <= 64` puts the destination in one word, which
    ///    is what lets the store be `write_chunk_word` — the same plane entry
    ///    §4.5.332 gave the funnel, so this is not a second store.
    ///  - an ADMITTED `WProg` cannot produce a real value (`wprog::compile`
    ///    admits only integral leaves and integral ops), which is the one thing
    ///    the funnel's coercion would still have had to ask.
    ///
    /// The destination width is `s.width`, not a `chunk_width` sum: a proven
    /// plain scalar is one whole net. The context width is the IEEE assignment
    /// rule `max(lhs, self(rhs))`, spelled exactly as `k_eval_for_lvalue`
    /// spells it — with `lvalue_width` replaced by the slot width it is equal to
    /// for this shape.
    fn eval_store_word(&mut self, lhs: &Lvalue, c: &LvalChunk, net: u32, rhs: u32) -> Option<()> {
        let s = self.arena.slots[net as usize];
        // ONE word, and non-empty. `words == nwords(width).max(1)`, so
        // `words == 1` already MEANS `width <= 64` — a separate `width > 64` row
        // was written here first and measured redundant (deleting it leaves the
        // suite green because it can never be the deciding test). What `words`
        // does NOT catch is width 0, whose `nwords` is 0 and whose `.max(1)`
        // makes it look like one word.
        if s.words != 1 || s.width == 0 {
            return None;
        }
        // A6: a REAL destination goes the long way — fail-closed, and MEASURED
        // unreachable today rather than assumed so.
        //
        // What it would prevent: `real r; r = 5;` has an integer RHS that
        // compiles fine, so a real destination here would store the raw integer
        // bits and skip `coerce_assign`'s int→real arm — `5` reinterpreted as an
        // f64 is 2.5e-323, which prints as 0.000000.
        //
        // Why it cannot fire: the op that calls this is only EMITTED for a
        // destination `plain_scalar_dest` accepts, and that predicate reads
        // `SimState::plain_scalar`, whose first clause is `!nets[i].is_real`.
        // One table, both tiers. Deleting this row therefore survives the
        // battery — a `panic!` in its place was never hit by any real design —
        // and it stays anyway: `build_plain_scalar` is one edit away from
        // admitting reals for a real-aware fast path, and the failure that edit
        // would cause here is silent.
        if s.is_real {
            return None;
        }
        let sw = self.sched.st.wt.get(rhs);
        // The destination width, taken from the SLOT instead of walked. Checked
        // rather than argued: `Slot::width` is seeded from `ir.nets[n].width` and
        // never mutated, and for a proven plain scalar `lvalue_width`'s sum is
        // that one chunk — but "the two are equal" is the whole reason this
        // function may skip the walk, so the suite (which runs debug) asks on
        // every execution, the way `k_schedule_nba_scalar` asks about its
        // offsets.
        debug_assert_eq!(
            self.arena.lvalue_width(self.ir, lhs),
            s.width,
            "plain-scalar destination width disagrees with its slot"
        );
        let w = s.width.max(sw.width);
        let r = self.run_cached_wprog(rhs, w, sw.signed)?;
        // `w >= s.width` by construction, so this resize is always a TRUNCATION
        // and the `signed` argument cannot reach `resize_word`'s sign-fill arm.
        // It is spelled anyway, and spelled as the canonical funnel spells it
        // (`Value::resize` takes the value's own signedness, which `run_wprog`
        // sets from this same request), because a shortcut that drops an
        // argument its canonical form carries is a shortcut nobody can check
        // against the canonical form. Measured: passing `false` here leaves the
        // suite green, and that is the proof of deadness, not a hole.
        let (pv, pu) = crate::value::resize_word(r.val, r.unk, w, s.width, sw.signed);
        // `cw` is the DESTINATION width. `write_chunk_word` clamps the window to
        // the net, so passing `w` instead would land the same bits — the
        // difference is only that this spelling states what is being written
        // rather than relying on the clamp to discover it.
        let (arena, forced) = (&mut self.arena, &self.sched.st.forced);
        arena.write_chunk_word(c, 0, 0, pv, pu, s.width, forced);
        Some(())
    }

    /// The cached width-specialized program for `(eid, w, signed)`, compiling
    /// (or caching the decline) on first sight.
    #[inline]
    fn wprog_for(
        &self,
        eid: u32,
        w: u32,
        signed: bool,
    ) -> Option<std::rc::Rc<crate::native::wprog::WProg>> {
        {
            let c = self.wcache.borrow();
            if let Some(Some(slot)) = c.get(eid as usize) {
                if slot.hits(w, signed) {
                    return slot.prog.clone();
                }
            }
        }
        let compiled =
            crate::native::wprog::compile(self.ir, &self.sched.st.wt, &self.arena, eid, w, signed)
                .map(std::rc::Rc::new);
        let mut c = self.wcache.borrow_mut();
        if let Some(slot) = c.get_mut(eid as usize) {
            *slot = Some(WCacheSlot {
                width: w,
                signed,
                prog: compiled.clone(),
            });
        }
        compiled
    }

    /// The specialized offset resolver — `None` means "not this shape", and
    /// the caller runs the generic `eval::resolve_offsets` unchanged.
    ///
    /// Admission mirrors the generic function's own structure rather than
    /// approximating it: at most two chunks (its inline case), integral net
    /// kinds only, and every index expression resolvable by `index_of`. Any
    /// miss declines the WHOLE lvalue, so a partially-specialized resolution
    /// cannot exist — which is also what keeps the E4002 machinery intact, since
    /// a decline is SIDE-EFFECT-FREE. Before S2 slice 4 that held because an
    /// admitted index tree could not reach `warn_run_range` at all; it now holds
    /// because admissibility is decided for every slot before any program runs.
    ///
    /// ⚠️ The kind test is FORWARD DEFENCE, not the complement of anything the
    /// canonical function evaluates here, and review measured both halves of
    /// that: on `NetArena` the assoc side-channel is unreachable for EVERY kind
    /// (`is_assoc`/`is_assoc_str` are the `NetReader` defaults, constant
    /// `false`), and `NetArena::buildable` refuses a design on its FIRST
    /// non-integral net, so an arena existing already implies every net is
    /// integral. Deleting the test leaves the suite green and cannot be forced.
    /// It is kept as a safe superset for the day either of those changes.
    fn fast_offsets(&self, lhs: &Lvalue) -> Option<Offsets> {
        if lhs.chunks.len() > 2 || lhs.chunks.is_empty() {
            return None;
        }
        // TWO PASSES, and the split is a correctness fix rather than a tidy-up.
        //
        // ⚠️ Running a program used to be free to speculate on: an admitted index
        // tree could only load whole nets and in-bounds CONSTANT elements, so it
        // had no side effect and a later decline could simply throw the work away.
        // S2 slice 4 admitted a RUNTIME element load, and that load COUNTS an
        // out-of-range report — so a decline after one had already run made the
        // generic resolver re-evaluate the same index and report it a SECOND time.
        // Both adversarial lenses measured it: `errors=2` on `--backend native`
        // against `1` on the VM, and worse, the duplicate ate the 8-per-run cap so
        // genuinely distinct LATER accesses were dropped. Deciding admissibility
        // for every slot before running any of them makes the speculation
        // side-effect-free by construction, which is the property the old comment
        // claimed the admission gave for free.
        for c in lhs.chunks.iter() {
            if !matches!(
                self.ir.nets.get(c.net as usize)?.kind,
                sim_ir::NetKind::Wire
                    | sim_ir::NetKind::Reg
                    | sim_ir::NetKind::Logic
                    | sim_ir::NetKind::Integer
            ) {
                return None; // assoc/dyn/queue/real/string: generic resolver
            }
            for e in [c.offset, c.word].into_iter().flatten() {
                if !self.index_admits(e) {
                    return None;
                }
            }
        }
        let mut buf = [(0u32, 0u32); 2];
        for (i, c) in lhs.chunks.iter().enumerate() {
            let off = match c.offset {
                None => 0,
                Some(e) => self.index_of(e)?,
            };
            let word = match c.word {
                None => 0,
                Some(e) => self.index_of(e)?,
            };
            buf[i] = (off, word);
        }
        Some(Offsets::Inline {
            buf,
            len: lhs.chunks.len() as u8,
        })
    }

    /// Would `index_of` answer for `eid` — WITHOUT running anything?
    ///
    /// Populates the same cache `index_of` reads, so pass 2 is a hit. It exists
    /// because deciding and evaluating had to be separated: see `fast_offsets`.
    fn index_admits(&self, eid: u32) -> bool {
        self.ensure_index_kind(eid);
        matches!(
            self.icache.borrow().get(eid as usize),
            Some(Some(IdxKind::Const(_))) | Some(Some(IdxKind::Prog(_)))
        )
    }

    /// Fill `icache[eid]` if it is empty. Compiling and CONSTANT-folding only —
    /// no program is run, which is what lets `index_admits` ask the question
    /// without answering it.
    fn ensure_index_kind(&self, eid: u32) {
        if !matches!(self.icache.borrow().get(eid as usize), Some(None)) {
            return; // already decided, or out of range (the caller sees `None`)
        }
        let sw = self.sched.st.wt.get(eid);
        let kind = match self.ir.exprs.get(eid as usize) {
            Some(sim_ir::Expr::Const { .. }) => {
                // Fold through the SAME rule the runtime path uses, so a
                // constant index and a computed one that lands on the same
                // value cannot disagree. A `Const` reads no net, so this is not
                // the speculation `fast_offsets` had to stop doing.
                let v = self.ctx().eval(eid);
                IdxKind::Const(crate::eval::offset_of_index_value(&v))
            }
            _ => match crate::native::wprog::compile(
                self.ir,
                &self.sched.st.wt,
                &self.arena,
                eid,
                sw.width,
                sw.signed,
            ) {
                Some(p) => IdxKind::Prog(std::rc::Rc::new(p)),
                None => IdxKind::Generic,
            },
        };
        self.icache.borrow_mut()[eid as usize] = Some(kind);
    }

    /// One index expression's bit position, cached per ExprId.
    fn index_of(&self, eid: u32) -> Option<u32> {
        self.ensure_index_kind(eid);
        let prog = match self.icache.borrow().get(eid as usize) {
            Some(Some(IdxKind::Const(o))) => return Some(*o),
            // Cloned out of the borrow before running. `wscratch` is a different
            // `RefCell`, so holding both would in fact be legal — measured — but a
            // program that ever reached back into `icache` would deadlock, and the
            // clone costs one refcount bump.
            Some(Some(IdxKind::Prog(p))) => std::rc::Rc::clone(p),
            // `Generic`, or an eid outside `ir.exprs` (defensive).
            _ => return None,
        };
        let v = self.run_wprog(&prog);
        Some(crate::eval::offset_of_index_value(&v))
    }

    /// Test seam for the specialized resolver — the gate compares it against
    /// `eval::resolve_offsets` directly rather than only through a run.
    #[cfg(test)]
    pub(crate) fn fast_offsets_for_test(&self, lhs: &Lvalue) -> Option<Offsets> {
        self.fast_offsets(lhs)
    }

    /// Mutate the arena while the kernel — and therefore its caches — stays
    /// ALIVE. The first version of the offset gate rebuilt the kernel for each
    /// mirrored state, so the `icache` was only ever consulted against the
    /// state that filled it, and a mutation that froze a `Prog` result into a
    /// `Const` (pure staleness) passed both of this slice's tests. A cache test
    /// that never re-reads across a state change is not a cache test.
    #[cfg(test)]
    pub(crate) fn arena_mut_for_test(&mut self) -> &mut NetArena {
        &mut self.arena
    }

    /// Run a compiled program and stamp its result as a `Value`.
    fn run_wprog(&self, prog: &crate::native::wprog::WProg) -> Value {
        let r = prog.run(&self.arena, &mut self.wscratch.borrow_mut());
        let mut v = Value::zeros(prog.width(), prog.signed());
        v.val[0] = r.val;
        v.unk[0] = r.unk;
        v
    }

    /// Move the transport updates due at `now` into this tick's NBA batch.
    ///
    /// The engine does this with `delayed_nba.remove(&next)` at the top of a
    /// timestep; the `seq` sort in `apply_nba` is what then interleaves them with
    /// updates scheduled during the tick, in original statement order. Splitting
    /// the two queues but sharing the sort is the whole mechanism.
    pub(crate) fn take_due_delayed(&mut self, now: u64) {
        if let Some(ups) = self.delayed_nba.remove(&now) {
            self.nba.extend(ups);
        }
    }

    /// Drain the NBA batch into the arena — the engine's `apply_nba`, restated
    /// over this store.
    ///
    /// Restated rather than shared because the engine's version is a method on
    /// `Scheduler` that writes `self.st`; what is shared is the thing that
    /// matters, `write_lvalue`, which is the S1c funnel both backends use. The
    /// two things this must not get wrong are the `seq` sort (NBA order is
    /// statement order, not queue order) and the fact that a `NbaLhs::One`
    /// borrows the engine's scratch `Lvalue` rather than allocating.
    pub(crate) fn apply_nba(&mut self) {
        let mut batch = std::mem::take(&mut self.nba);
        batch.sort_by_key(|u| u.seq);
        let mut scratch =
            std::mem::replace(&mut self.nba_scratch_lhs, Lvalue { chunks: Vec::new() });
        for u in batch.drain(..) {
            match u.lhs {
                NbaLhs::One(c) => {
                    scratch.chunks.clear();
                    scratch.chunks.push(c);
                    self.write_routed(&scratch, u.sampled, &u.offsets);
                }
                NbaLhs::Many(lv) => {
                    self.write_routed(&lv, u.sampled, &u.offsets);
                }
            }
        }
        self.nba_scratch_lhs = scratch;
        self.nba = batch;
    }

    /// Report every out-of-range array access the arena has recorded since the
    /// last drain, through the ENGINE's emitter.
    ///
    /// Calling `SimState::warn_run_range` rather than re-emitting is what keeps
    /// the message text, the `Severity::Error` that sets the exit class, the
    /// 8-per-run cap and its "further suppressed" note identical without any of
    /// them being restated. The arena can only COUNT (see `pending_range`).
    ///
    /// ⚠️ ORDERING is NOT this function's job, and an earlier version of this
    /// doc said the opposite twice over. It claimed the late report was "a real
    /// difference … not observable in the CLI because stdout and diagnostics
    /// are separate destinations". Both halves were measured false: `-l/--log`
    /// (and `2>&1`) is a SINGLE destination, and two diagnostics — a range
    /// report and the `$error` whose own argument caused it — go to the same
    /// stream regardless.
    ///
    /// What actually orders the report is `NetReader::take_deferred_range_reports`,
    /// drained inside `format_args_str_with`: that function holds the alternate
    /// reader and the sink at once, so it reports after the argument reads and
    /// before the caller emits its line. This drain is the BACKSTOP for accesses
    /// no formatter follows — a terminator condition, an NBA apply, a body that
    /// ends the run. `now` is unchanged across that span, so the timestamp is
    /// right either way.
    pub(crate) fn drain_range_diags(&mut self) {
        self.drain_range_reports();
        self.drain_vcd();
        self.drain_probe();
    }

    /// The RANGE half of the drain, on `&self` — one spelling, TWO callers
    /// (`drain_range_diags` and `eval_call`; the counted number, not an estimate —
    /// an earlier version of this line said three).
    ///
    /// It is split out because the second caller is on the READ path: `eval_call`
    /// has to report what the ARGUMENT reads earned before the callee starts
    /// printing, and it only holds `&self`. Going through the `NetReader` method
    /// rather than `arena.pending_range` directly is deliberate — that method IS
    /// this store's answer, and routing every drain through it is what keeps the
    /// override load-bearing instead of decorative.
    ///
    /// ⚠️ A third site drains the SAME counter without going through here:
    /// `format_args_str_with`'s guard calls `NetReader::take_deferred_range_reports`
    /// on whatever reader it was handed. That is why the override, not this
    /// function, is the single channel — and why deleting the override would now
    /// silence out-of-range diagnostics on the whole backend rather than only
    /// reorder them. The blast radius grew with this slice; saying so is the point.
    pub(crate) fn drain_range_reports(&self) {
        for unknown in crate::eval::NetReader::take_deferred_range_kinds(self) {
            self.sched.st.warn_run_index("array word index", unknown);
        }
    }

    /// Write out the VCD value-changes the arena recorded, through the ENGINE's
    /// emitter and its id tables.
    ///
    /// Drained at the same seams as the range diagnostics because both are the
    /// same problem: the store detected something at a point where it could not
    /// reach the sink.
    ///
    /// ⚠️ The buffer is NOT the only order the file has — an earlier version of
    /// this sentence said it was. `$dumpvars` (preamble + `dump_initial`) and
    /// `$dumpoff` write to the same file SYNCHRONOUSLY, bypassing the buffer.
    /// What keeps the interleaving right is stronger and elsewhere: the walk
    /// drains at EVERY statement boundary, so no buffered record can outlive the
    /// statement before a `$dump*` statement runs. `now` does not move across a
    /// drain span, so stamping at drain equals stamping at store.
    /// The PROBE twin of [`NativeKernel::drain_vcd`], drained at the same seams
    /// and for the same reason: the store point that records is on the arena and
    /// the sink is on the scheduler.
    ///
    /// `now` is not captured on the arena side — every drain seam is a span
    /// across which it does not move, so stamping at drain equals stamping at
    /// store. That is the VCD queue's argument verbatim, and it matters more
    /// here because the probe record carries `"t"` in its JSON.
    pub(crate) fn drain_probe(&mut self) {
        if self.arena.ch.probe_pending.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.arena.ch.probe_pending);
        for (net, bits) in &pending {
            self.sched.st.emit_probe_change_from(*net, bits);
        }
        let mut p = pending;
        p.clear();
        self.arena.ch.probe_pending = p;
    }

    pub(crate) fn drain_vcd(&mut self) {
        if self.arena.ch.vcd_pending.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.arena.ch.vcd_pending);
        for (net, word, packed) in &pending {
            if let Some((id, width)) = self.sched.st.vcd_id_for(*net, *word) {
                self.sched.st.emit_vcd_packed(id, packed, width);
            }
        }
        // Hand the capacity back: a dumping run does this once per statement.
        let mut p = pending;
        p.clear();
        self.arena.ch.vcd_pending = p;
    }

    /// Every pending activation, as `(tick, inactive, proc, block)` — the twin of
    /// `Scheduler::pending_resumes_for_test`, in the same shape so the two can be
    /// compared without either side naming the other's types.
    #[cfg(test)]
    pub(crate) fn pending_resumes_for_test(&self) -> Vec<(u64, bool, u32, u32)> {
        let now = self.sched.st.now;
        let mut v: Vec<(u64, bool, u32, u32)> = Vec::new();
        for r in &self.active {
            v.push((now, false, r.proc, r.block));
        }
        for r in &self.inactive {
            v.push((now, true, r.proc, r.block));
        }
        for (&t, evs) in &self.wheel {
            for (inactive, r) in evs {
                v.push((t, *inactive, r.proc, r.block));
            }
        }
        v
    }

    /// The read context: `SimState::mk_eval_ctx` with exactly ONE field changed.
    ///
    /// Every other field is read from `self.st`, not copied at construction, so
    /// on THIS path `nets` is the only place the two backends can differ.
    /// Since S2 it is no longer the only evaluator: `k_eval_for_lvalue` runs
    /// the width-specialized `wprog` spelling for admitted trees, whose parity
    /// is measured (the exhaustive width-4 battery and the pinned corpus
    /// sweep), not structural — the admission plus those anchors are what
    /// stand where "by construction" stood.
    pub(crate) fn ctx(&self) -> crate::eval::EvalCtx<'_, Self> {
        crate::eval::EvalCtx {
            ir: self.ir,
            nets: self,
            now: self.sched.st.now,
            wt: &self.sched.st.wt,
            time_mult: self.sched.st.cur_time_mult,
            rng: &self.sched.st.rng,
            plusargs: &self.sched.st.plusargs,
        }
    }

    /// Is `net` a subroutine frame slot — i.e. does its value live in the
    /// activation window rather than in an arena slot?
    ///
    /// `has_frames` short-circuits the whole question for a design with no
    /// subroutines; `frame_local` is the ENGINE's table, built once by
    /// `build_func_routing` from the same `func_table` the gate reads, so the
    /// two backends cannot disagree about which nets are frame slots.
    #[inline]
    /// **THE tier-3 write funnel.** Every store this backend performs goes
    /// through here, and it exists because the alternative was seven call sites
    /// (`k_write_lvalue`, `k_write_scalar`, the NBA apply's two arms, and the
    /// settle's three) each spelling the same routing question.
    ///
    /// Today it answers that question one way — the arena owns the store — so
    /// this is mechanically the seven `arena.write_lvalue` calls it replaced.
    /// The reason to have it anyway is V1 slice 2 (ROADMAP §5.1-c): a heap-kind
    /// net's value does not live in a net slot at all, it lives in
    /// `SimState::dyn_heap` keyed by net id, so admitting `string`/`queue`/
    /// `dyn_array`/`assoc` means a WRITE-SIDE ROUTER — and the READ side already
    /// has one (`read_net` below, and `SimState::read_net`'s own bitmap
    /// dispatch), while the write side had no counterpart.
    ///
    /// ⚠️ It has to live on the KERNEL, not on `NetArena`: the destination for a
    /// heap net is `self.sched.st`, and the arena does not hold the scheduler.
    /// That is also why the arena's own `write_lvalue` stays exactly as it is —
    /// it remains the flat store's funnel, and this is the layer above it that
    /// chooses a store.
    pub(crate) fn write_routed(&mut self, lhs: &Lvalue, value: Value, offsets: &Offsets) -> bool {
        // V1 slice 2: a heap-kind net's elements are not in the flat store, so
        // this write belongs to `SimState::dyn_heap`. Mirrors the engine's own
        // `write_chunk`, whose FIRST check is the same one.
        if let [c] = lhs.chunks.as_slice() {
            // V1 slice 2d: the ASSOC lanes first. An assoc key is an i64 (or a
            // byte string) and cannot ride the `(offset, word)` u32 pairs, so
            // `resolve_offsets` carries it out of band and `as_slice()` yields
            // `&[]` — which means the `unwrap_or((0, 0))` below would have
            // silently turned every key into 0 before handing it to `dyn_write`,
            // whose own arm for that shape is a loud "unsupported lvalue shape"
            // + IGNORE. Measured before this arm existed: `aa[3] = 7; aa[9] = 11`
            // stored nothing and read back `x x 0 0` against the VM's `7 11 2 1`.
            //
            // The split is the same one `SimState::write_lvalue` makes, at the
            // same point (after the real→int coercion the value already carries),
            // and it delegates to the same two methods — the key resolution has
            // already happened, in the SHARED `resolve_offsets`, through THIS
            // kernel's reader.
            if let Offsets::AssocKey(key) = offsets {
                self.sched.st.assoc_write(c.net, *key, &value);
                return false; // heap content never enters the dirty channel
            }
            if let Offsets::AssocStrKey(key) = offsets {
                self.sched.st.assoc_str_write(c.net, key, &value);
                return false;
            }
            // A3-ii-a: the FRAME lane, and it is the exact mirror of the read
            // path's — `NetReader::read_net` above has routed a frame-local net to
            // `SimState` since S3a, and this side had no counterpart. Harmless
            // while tier-3 never executed a frame body (no statement could name a
            // frame slot: `frames_admitted`'s module-body row forbids it); the
            // moment the walk drives one, a body's `s = x + y` would otherwise
            // land in the arena's DEAD slot for that net while every read came
            // from the window.
            //
            // The split is `SimState::write_lvalue_general`'s own, delegating to
            // the same method: single chunk, `frame_local`, and NOT a non-string
            // dyn handle (a frame-local dyn array keeps its elements in the heap,
            // so it belongs to the lane above; a frame-local `string` is
            // `dyn_is_handle` too but is slab-stored and belongs here).
            //
            // `false` — a frame slot is not a flat-store net, so it never enters
            // the dirty channel. Same as the engine's lane, and it is what stops
            // a frame-local write from waking a process.
            if self.is_frame_local(c.net)
                && (!self.is_heap_net(c.net)
                    || self.ir.nets[c.net as usize].kind == sim_ir::NetKind::String)
            {
                self.sched.st.frame_write_lvalue(lhs, value);
                return false;
            }
            if self.is_heap_net(c.net) {
                let (off, word) = offsets.as_slice().first().copied().unwrap_or((0, 0));
                // `&self` — the heap is interior-mutable (§4.5.194), which is
                // what lets this run without disturbing the `&mut arena` borrow.
                return self.sched.st.dyn_write(c, off, word, &value);
            }
            // A2-i: the CLASS FIELD lane, the exact mirror of the read path's —
            // and the pair `class ∧ word.is_some()` is `SimState::write_chunk`'s
            // own, delegating to the same method. A bare word-less write is the
            // HANDLE ID itself (`h = new`, `h = null`, a ref copy); that value
            // belongs in this store's slot and falls through.
            //
            // `false` — a heap field is not a flat-store net, so it never enters
            // the dirty channel (`class_field_write` returns `false` for the
            // engine too, and it is what stops `obj.f = 1` from waking a process
            // sensitive to the handle).
            if self.is_class_handle(c.net) && c.word.is_some() {
                let (_, word) = offsets.as_slice().first().copied().unwrap_or((0, 0));
                return self
                    .sched
                    .st
                    .class_field_write_with(&*self, c, word, &value);
            }
        }
        let ir = self.ir;
        // ── CONCAT with a chunk this store does not own (A8-concat) ──────────
        //
        // V1 slice 2 refused this shape and named the follow-on exactly: give
        // the funnel a per-chunk escape rather than spell the split twice. The
        // decision is made HERE because only the kernel knows which store owns a
        // net; the SLICING stays in `write_lvalue_escaping`, which is the one
        // spelling of it.
        //
        // ⚠️ The escaped pieces are applied AFTER the arena's chunks, and that
        // is not observable: the chunks of one concat lvalue are disjoint
        // destinations and nothing reads between them — the same argument
        // A1-iii's `TaskWrites::Collect` makes, for the same reason (the borrow
        // that makes the collect necessary).
        if lhs.chunks.len() > 1 {
            let mask: Vec<bool> = lhs.chunks.iter().map(|c| self.is_heap_net(c.net)).collect();
            if mask.iter().any(|&b| b) {
                let mut esc = crate::native::write::Escape {
                    mask: &mask,
                    taken: Vec::new(),
                };
                let changed = {
                    let (arena, forced) = (&mut self.arena, &self.sched.st.forced);
                    arena.write_lvalue_escaping(ir, lhs, value, offsets, forced, &mut esc)
                };
                let mut any = changed;
                for (idx, piece) in esc.taken.into_iter() {
                    let c = &lhs.chunks[idx];
                    let (off, word) = offsets.as_slice().get(idx).copied().unwrap_or((0, 0));
                    any |= self.sched.st.dyn_write(c, off, word, &piece);
                }
                return any;
            }
        }
        // The force flags are `SimState`'s — ONE table, threaded rather than
        // mirrored. The split borrow is field-disjoint (`arena` mutable,
        // `sched.st.forced` shared).
        let (arena, forced) = (&mut self.arena, &self.sched.st.forced);
        arena.write_lvalue(ir, lhs, value, offsets, forced)
    }

    /// The tier-3 pin write: through THIS store's funnel, with the target's
    /// force flag lifted so a re-force (or a resumed latent assign) can land.
    ///
    /// The lift/pin pair is `SimState`'s (`force_lift`/`force_pin`) — one table,
    /// one order, two funnels. `SimState::force_write` is the engine's twin and
    /// differs only in which funnel sits between them.
    pub(crate) fn force_write(&mut self, lhs: &Lvalue, value: Value) -> bool {
        let net = lhs.chunks[0].net;
        self.sched.st.force_lift(net);
        let changed = self.write_routed(lhs, value, &crate::state::SimState::FORCE_OFFSETS);
        self.sched.st.force_pin(net);
        changed
    }

    /// Mark every continuous assign that DRIVES `net` dirty in THIS store's
    /// worklist. The `release` half of the fix at `SimState::drivers_of_net`;
    /// the engine has the same two lines against `st.ca_dirty`.
    pub(crate) fn redirty_drivers_of(&mut self, net: u32) {
        for ci in self.sched.st.drivers_of_net(net) {
            let i = ci as usize;
            if i < self.arena.ch.ca_dirty_flag.len() && !self.arena.ch.ca_dirty_flag[i] {
                self.arena.ch.ca_dirty_flag[i] = true;
                self.arena.ch.ca_dirty.push(ci);
            }
        }
    }

    /// The tier-3 twin of `Scheduler::reeval_active_forces` (IEEE §9.3.2 /
    /// §9.3.1 continuous re-evaluation), against the arena.
    ///
    /// The fixpoint's SHAPE — seed from this delta's changed nets plus every
    /// always-reeval force, re-pin in ascending key order, re-select from the
    /// nets that actually moved, budget over the live-force count — is the
    /// engine's, expressed through the same `SimState` helpers
    /// (`force_keys_for`/`force_entry`). What differs is the two store
    /// operations and the SEED: the engine reads `st.dirty`, tier-3 the arena's.
    pub(crate) fn reeval_active_forces(&mut self) {
        let dirty = std::mem::take(&mut self.arena.ch.dirty);
        let mut keys = self.sched.st.force_keys_for(&dirty, true);
        self.arena.ch.dirty = dirty;

        let saved = self.sched.st.cur_time_mult;
        let mut budget = self.sched.st.active_forces.len().saturating_add(2);
        while !keys.is_empty() && budget > 0 {
            budget -= 1;
            let mut next_changed: Vec<u32> = Vec::new();
            for &k in &keys {
                let Some((lv, rhs, mult)) = self.sched.st.force_entry(k) else {
                    continue;
                };
                self.sched.st.cur_time_mult = mult;
                let v = self.k_eval_for_lvalue(&lv, rhs);
                if self.force_write(&lv, v) {
                    next_changed.push(k);
                }
            }
            keys = self.sched.st.force_keys_for(&next_changed, false);
        }
        self.sched.st.cur_time_mult = saved;
    }

    /// Is this net's value in the ENGINE's heap rather than in a slot of THIS
    /// store?
    ///
    /// Sourced from the ARENA's own `heap` map, not from `SimState`'s
    /// `dyn_is_handle`. Both are derived from the same `ir.nets` kinds, so this
    /// is one rule with one derivation rather than two spellings — and the
    /// arena's is the one that is guaranteed to exist here: it is built with the
    /// store it describes, in `NetArena::build`.
    pub(crate) fn is_heap_net(&self, net: u32) -> bool {
        self.arena.heap.get(net as usize).copied().unwrap_or(false)
    }

    /// A2-i. Reads the ARENA's bitmap, not `SimState`'s, so the one consumer
    /// that holds no kernel (`wprog::compile`) asks the same question.
    pub(crate) fn is_class_handle(&self, net: u32) -> bool {
        self.arena.class.get(net as usize).copied().unwrap_or(false)
    }

    pub(crate) fn is_frame_local(&self, net: u32) -> bool {
        self.has_frames
            && self
                .sched
                .st
                .frame_local
                .get(net as usize)
                .copied()
                .unwrap_or(false)
    }
}

/// S3a — **the tier-3 read path is a COMPOSITE, and the split is the store
/// boundary.**
///
/// Module nets come from the arena; frame slots come from the engine's frame
/// window/slab, which is not part of either net store. Calls delegate to the
/// engine's frame executor (`SimState::run_frame_call`) rather than being
/// restated — the whole S3a argument, and `native::frames::frames_admitted` is
/// what makes it byte-identical: an admitted subroutine body names no net
/// outside its own window, so nothing inside the call reaches the flat store
/// this backend leaves untouched.
///
/// Everything not overridden keeps `NetReader`'s default, and the reason is the
/// same one `NetArena`'s impl gives: an ELIGIBLE design has no heap kinds, no
/// class handles and no `real`, so the defaults are unreachable by construction.
///
/// ⚠️ That argument does NOT cover `resolve_virtual_call`, and an earlier version
/// of this sentence claimed "the three overrides are exactly the three questions a
/// FRAME asks" while `eval_core` asks a FOURTH one on the same reader, one line
/// earlier. It is forwarded below, and what actually keeps it unreachable is the
/// S0 `class` row rather than anything about this store.
impl crate::eval::NetReader for NativeKernel<'_, '_, '_> {
    fn read_net(&self, net: u32, word: Option<u32>) -> Value {
        // ⭐ A2-i: a class FIELD select, and it is checked FIRST for the reason
        // `SimState::read_net` checks it first — a method's `this` is BOTH a
        // class handle and a frame-local net, and the field must win or the read
        // lands on the window slot holding the object id.
        //
        // ⚠️ It cannot delegate whole, unlike the two lanes below: the field
        // lives in the shared `class_heap`, but the HANDLE that names it is in
        // THIS store. `class_field_read_with` takes the reader for exactly that
        // — passing `self` here is what makes `obj.f` read the object this run
        // allocated rather than the engine's t0 `null`.
        if self.is_class_handle(net) {
            if let Some(field) = word {
                return self.sched.st.class_field_read_with(self, net, field);
            }
        }
        // Two nets this store does not own: a frame-local (S3a) and — since V1
        // slice 2 — a heap kind, whose elements live in `SimState::dyn_heap`.
        // Both delegate to the ENGINE's `read_net`, which routes on its own
        // bitmaps; restating either rule here is how two stores diverge.
        if self.is_frame_local(net) || self.is_heap_net(net) {
            return self.sched.st.read_net(net, word);
        }
        self.arena.read_net(net, word)
    }

    /// The leaf fast path is the ARENA's answer (today: the `NetReader` default
    /// `None`, i.e. no fast path). A frame slot must not take it either —
    /// `SimState`'s own implementation bails on `frame_local` for the same
    /// reason — so the composite simply never offers one.
    fn read_scalar_words(&self, _net: u32, _w: u32, _ctx_signed: bool) -> Option<(u64, u64)> {
        None
    }

    // ── V1 slice 2: the capabilities this store does NOT own ──────────────
    //
    // Every method below reads state that lives ONLY on `SimState` — the dyn/
    // assoc/string heaps, the `foreach` iteration context, the file table — so
    // the arena has no counterpart and delegation is unconditionally right,
    // independent of which gate rows are open.
    //
    // ⚠️ They are here because the census in ROADMAP §5.1-c found tier-3 was
    // SILENTLY taking `NetReader`'s defaults for fourteen of them, and every one
    // of those defaults returns a PLAUSIBLE value — `None` that the caller
    // X-poisons, `false` meaning "not an assoc", an `xs`. That is harmless only
    // while the gate refuses every design that can reach them, and it stops
    // being harmless the moment a row opens: opening `dyn_array` for slice 2a
    // made `q.size()` read `x` on the line after `q = new[4]`, because
    // `dyn_size`'s default is `None`. The composite is now TOTAL over the
    // trait, and `the_composite_reader_overrides_every_netreader_method` keeps
    // it that way — a new trait method with a default must be answered here on
    // purpose rather than inherited by accident.

    /// `false` — this composite IS the routing (`read_net` above asks
    /// `is_heap_net`), so asking `SimState::eval_expr_with` to wrap it again
    /// would be a second decision on the same question.
    fn routes_heap_to_state(&self) -> bool {
        false
    }

    fn dyn_size(&self, net: u32) -> Option<u64> {
        self.sched.st.dyn_size(net)
    }
    fn dyn_values(&self, net: u32) -> Option<Vec<Value>> {
        self.sched.st.dyn_values(net)
    }
    fn dyn_warn(&self, net: u32, msg: &str) {
        self.sched.st.dyn_warn(net, msg)
    }
    fn array_item(&self, index: bool) -> Value {
        self.sched.st.array_item(index)
    }
    fn swap_array_item(&self, v: Option<(Value, u64)>) -> Option<(Value, u64)> {
        self.sched.st.swap_array_item(v)
    }
    fn str_bytes(&self, net: u32) -> Option<Vec<u8>> {
        self.sched.st.str_bytes(net)
    }
    fn str_byte_at(&self, net: u32, i: usize) -> Option<u8> {
        self.sched.st.str_byte_at(net, i)
    }
    fn is_assoc(&self, net: u32) -> bool {
        self.sched.st.is_assoc(net)
    }
    fn assoc_read(&self, net: u32, key: Option<i64>) -> Value {
        self.sched.st.assoc_read(net, key)
    }
    fn assoc_exists(&self, net: u32, key: Option<i64>) -> Option<bool> {
        self.sched.st.assoc_exists(net, key)
    }
    fn is_assoc_str(&self, net: u32) -> bool {
        self.sched.st.is_assoc_str(net)
    }
    fn assoc_str_read(&self, net: u32, key: &Option<Vec<u8>>) -> Value {
        self.sched.st.assoc_str_read(net, key)
    }
    fn assoc_str_exists(&self, net: u32, key: &Option<Vec<u8>>) -> Option<bool> {
        self.sched.st.assoc_str_exists(net, key)
    }
    fn fd_eof(&self, fd: u32) -> Value {
        self.sched.st.fd_eof(fd)
    }

    fn take_deferred_range_kinds(&self) -> Vec<bool> {
        // The ARENA's counter, not the engine's: `SimState` reports at the
        // access, so it has none, and draining a second source here would double
        // count nothing while silently dropping the arena's.
        //
        // Both of THIS file's drains go through `drain_range_reports`, which calls
        // this method — so the override is what ORDERS an out-of-range report, not
        // bookkeeping. (`format_args_str_with`'s guard is a third drain of the same
        // counter and calls this method directly; see `drain_range_reports`. An
        // earlier version of this line said "every drain goes through
        // `drain_range_reports`", which that guard contradicts.)
        self.arena.take_deferred_range_kinds()
    }

    /// ⚠️ **The drain is not decoration — it is the ORDER.**
    ///
    /// The engine reports an out-of-range read AT THE ACCESS
    /// (`SimState::read_net` → `warn_run_range`); this store can only COUNT, and
    /// somebody holding the sink reports for it. `eval_core`'s `Expr::Call` arm
    /// evaluates every actual and THEN calls this, so an argument that read past
    /// a memory leaves a pending count on the way in — and `run_frame_call`
    /// prints through the sink. Without this drain the callee's `$display` comes
    /// out BEFORE the E4002 its own argument earned, while the VM emits them the
    /// other way round: same values, same exit class, a different stream.
    ///
    /// Measured on `function f(x); $display(...); endfunction  r = f(mem[9]);` —
    /// the adversarial soundness review of S3a found it, and it is the same class
    /// §4.5.298 closed for the module path (the format engine drains because it
    /// is the one place holding both the reader and the sink). S3a opened a
    /// SECOND order-visible seam; this is that seam's drain.
    fn eval_call(&self, func: u32, args: &[Value]) -> Option<Value> {
        self.drain_range_reports();
        // A3-iii: the ARENA, not `self` — the kernel holds `&mut Scheduler` and
        // cannot lend both at once, which is `k_dispatch_systask`'s split. One
        // level down `eval_ctx_with_reader` wraps it in `HeapRouted`, so a
        // FRAME-LOCAL net still comes back from the activation window while a
        // module net comes from this store. Passing `None` here is what made an
        // admitted body's module read return the engine's t0 value, which is
        // exactly the S3a row this narrows.
        self.sched
            .st
            .run_frame_call_with(Some(&self.arena), func, args)
    }

    /// N7 virtual dispatch, forwarded for the same reason `eval_call` is:
    /// `eval_core` asks THIS reader for the target immediately before asking it
    /// to run the call, so a composite that answers one and defaults the other
    /// would dispatch a virtual method to its STATIC target — silently, unlike
    /// the arena's loud `eval_call`.
    ///
    /// ⚠️ Unreachable today, and by a DIFFERENT gate than the one the impl doc
    /// cites: `class_calls` is counted by the S0 `class` row, so a design with a
    /// virtual call is refused before eligibility, not merely absent from the
    /// arena. Forwarded anyway — a one-line correct answer beats a silent wrong
    /// one the day that row moves. (Found by the S3a differential review.)
    fn resolve_virtual_call(&self, call_eid: u32, static_fid: u32, args: &[Value]) -> u32 {
        self.sched
            .st
            .resolve_virtual_call(call_eid, static_fid, args)
    }

    fn formal_width(&self, func: u32, i: usize) -> Option<(u32, bool)> {
        self.sched.st.formal_width(func, i)
    }

    fn formal_is_string(&self, func: u32, i: usize) -> bool {
        self.sched.st.formal_is_string(func, i)
    }
}

impl Kernel for NativeKernel<'_, '_, '_> {
    #[cfg(feature = "jit")]
    fn k_nets(&self) -> &dyn crate::eval::NetReader {
        // `self`, not `&self.arena` (S3a): whoever asks for "this kernel's
        // reader" must get the composite, or a call it evaluates hits the
        // arena's loud `eval_call` — the arena alone is half a store.
        self
    }

    // ── READ: store-backed, shared rules ──

    fn k_eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value {
        // `Scheduler::eval_for_lvalue`, restated over this store: the IEEE
        // assignment rule is width = max(lhs, self(rhs)), sign = rhs self-sign.
        let lw = self.arena.lvalue_width(self.ir, lhs);
        let sw = self.sched.st.wt.get(rhs);
        let w = lw.max(sw.width);
        // S2: the width-specialized fast path. Admission (uniform width AND
        // sign, ≤64 bits, constant shift amounts, and array indices that are
        // themselves admitted — see `wprog`) is what
        // makes it byte-identical; a decline is cached and falls through to
        // the generic evaluator, which IS the previous path.
        if let Some(prog) = self.wprog_for(rhs, w, sw.signed) {
            return self.run_wprog(&prog);
        }
        self.ctx().eval_ctx(rhs, w, sw.signed)
    }

    fn k_eval(&self, eid: u32) -> Value {
        // HEAP-ROUTED, deliberately: `SimState::eval_expr_with` is the one frame
        // that holds both the heap owner and this reader (V1 slice 2), so a
        // `string` key or a heap-backed operand answers from `dyn_heap` instead
        // of reaching `assert_owns` on an arena that owns no slot for it.
        self.sched.st.eval_expr_with(&self.arena, eid)
    }
    fn k_ir(&self) -> &SimIr {
        self.ir
    }
    fn k_lvalue_width(&self, lhs: &Lvalue) -> u32 {
        self.arena.lvalue_width(self.ir, lhs)
    }
    fn k_self_width(&self, eid: u32) -> (u32, bool) {
        // The width table is IR-derived and shared; both kernels read the one
        // `SimState` carries.
        let sw = self.sched.st.wt.get(eid);
        (sw.width, sw.signed)
    }
    fn k_eval_ctx(&self, eid: u32, ctx_width: u32, ctx_signed: bool) -> Value {
        // Through THIS COMPOSITE, not through `&self.arena`: a subroutine
        // actual is an ordinary caller expression, so it can name a heap net
        // (`t.run(q[0])`) as well as a flat one, and the composite is the reader
        // that routes both. `eval_ctx_with_reader` is `eval_expr_with`'s
        // context-sized twin (V1 slice 2c) — same routing decision, `eval_ctx`
        // instead of `eval`, so a formal narrower than its actual truncates here
        // rather than after the fact.
        self.sched
            .st
            .eval_ctx_with_reader(self, eid, ctx_width, ctx_signed)
    }
    fn k_frame_base(&self, func: u32) -> u32 {
        self.sched.st.func_table[func as usize].base_net
    }
    fn k_task_call_site(&self, proc: u32, bb: u32) -> Option<crate::TaskCallInfo> {
        self.sched.st.task_calls_proc.get(&(proc, bb)).cloned()
    }
    fn k_nested_call_site(&self, global_bb: u32) -> Option<crate::TaskCallInfo> {
        self.sched.st.task_calls_func.get(&global_bb).cloned()
    }
    fn k_callee_is_driven(&self, callee: u32) -> bool {
        self.sched.st.suspendable_tasks.contains(&callee)
    }
    fn k_enter_driven_frame(
        &mut self,
        callee: u32,
        in_vals: &[(u32, Value)],
        dyn_snaps: &[(u32, u32)],
    ) -> Vec<(u32, Option<crate::state::DynObj>)> {
        self.sched.st.enter_driven_frame(callee, in_vals, dyn_snaps)
    }
    fn k_exit_driven_frame(
        &mut self,
        callee: u32,
        out_binds: &[(u32, Lvalue)],
        dyn_stash: Vec<(u32, Option<crate::state::DynObj>)>,
    ) -> Vec<(Lvalue, Value)> {
        self.sched
            .st
            .exit_driven_frame(callee, out_binds, dyn_stash)
    }
    fn k_call_site_runnable(&self, proc: u32, bb: u32) -> bool {
        crate::exec::frame_call::site_runnable(
            self.sched.st.ir,
            &self.sched.st.suspendable_tasks,
            self.sched.st.task_calls_proc.get(&(proc, bb)),
        )
    }
    fn k_run_subset_task(
        &mut self,
        callee: u32,
        in_vals: &[(u32, Value)],
        dyn_snaps: &[(u32, u32)],
        out_binds: &[(u32, Lvalue)],
    ) -> Vec<(Lvalue, Value)> {
        // DELEGATION, and every store this touches is `SimState`'s own
        // frame/heap state, which this kernel BORROWS rather than mirrors.
        //
        // ⚠️⚠️ **This comment used to end "the nets … are not touched here at
        // all", and A3-iii measured that FALSE.** It was true of the CALL
        // PROTOCOL — the inputs are evaluated by `k_eval_ctx` above and the
        // scalar outputs written by `k_write_lvalue` after this returns — and
        // said nothing about the callee's BODY, which reads whatever module nets
        // it names. S3a's row made that unreachable; narrowing that row to WRITES
        // made it reachable, and the arena has to go with it. Measured at exit 0
        // with no diagnostic: `while (getnext(i, v) == 1)` over a module array
        // returned 0 on the first call, so the loop body never ran and the design
        // printed its `PASS` line and nothing else.
        //
        // §4.5.338 inside a delegation's justification: a comment saying "this
        // does not touch X" does not know when it starts to.
        self.sched
            .st
            .run_subset_task_with(Some(&self.arena), callee, in_vals, dyn_snaps, out_binds)
    }
    fn k_file_read_byte(&mut self, fd: u32) -> Option<u8> {
        // The file table lives in `SimState`, which this kernel borrows — one
        // object, both backends, exactly like `dyn_heap`.
        crate::builtins::file_read_byte(self.sched, fd)
    }
    fn k_file_unget(&mut self, fd: u32, b: u8) {
        self.sched
            .st
            .read_state
            .entry(fd)
            .or_default()
            .pushback
            .push(b);
    }
    fn k_read_net(&self, net: u32, word: Option<u32>) -> Value {
        // THIS kernel's `NetReader`, not `self.arena` directly. ⚠️ The first
        // version of this method WAS `self.arena.read_net(...)`, and that is a
        // second spelling of a routing decision the `NetReader` impl below
        // already owns: a frame-local (S3a) and a heap kind (V1 slice 2) are
        // NOT the arena's, and reaching past the router to the store would send
        // `$fread`'s prior-value read into `assert_owns` for either.
        //
        // The route through here is what a mutation swapping the receiver for
        // `self.sched.st` proves: the anchor's partial read then prints
        // `4546xxxx` instead of `4546beef`, because the engine's copy of that
        // memory never saw the native run's writes.
        crate::eval::NetReader::read_net(self, net, word)
    }
    fn k_array_base(&self, net: u32) -> Option<u64> {
        crate::builtins::declared_array_base(&self.sched.st.net_dims, net)
    }
    fn k_warn_readmem(&mut self, msg: String) {
        Kernel::k_warn_readmem(self.sched, msg)
    }
    fn k_file_open(&mut self, name: &str, mode: Option<&str>) -> u32 {
        // The file TABLE lives in `SimState`, which this kernel borrows — one
        // object, both backends, exactly like `dyn_heap`. Nothing to route.
        Kernel::k_file_open(self.sched, name, mode)
    }
    fn k_file_eof(&mut self, fd: u32) -> Option<bool> {
        Kernel::k_file_eof(self.sched, fd)
    }
    fn k_file_ungetc(&mut self, fd: u32, byte: u8) -> bool {
        Kernel::k_file_ungetc(self.sched, fd, byte)
    }
    fn k_assoc_iter_cur_key(&self, rhs: u32) -> Option<u32> {
        self.sched.st.assoc_iter_cur_key(rhs)
    }
    fn k_assoc_iter_compute(&self, rhs: u32, cur: Option<Value>) -> (Option<(u32, Value)>, i32) {
        // The LOCATE half walks `dyn_heap`, one object both backends share; the
        // current key came in through `k_eval`, i.e. through THIS store.
        self.sched.st.assoc_iter_compute(rhs, cur)
    }

    fn k_eval_native(&self, prog: &crate::native_eval::NativeProg) -> Value {
        // `native_eval::run` already takes `&dyn NetReader`, so the tier-2
        // compiled-expression VM runs over the arena with no second copy. The
        // scratch is per-call here (the engine reuses one behind a RefCell —
        // an allocation choice, not a semantic one).
        let mut scratch = crate::native_eval::NativeScratch::default();
        crate::native_eval::run(prog, &self.arena, &mut scratch)
    }

    fn k_resolve_lvalue_offsets(&self, lhs: &Lvalue) -> Offsets {
        if let Some(fast) = self.fast_offsets(lhs) {
            return fast;
        }
        crate::eval::resolve_offsets(&self.ctx(), lhs)
    }

    fn k_delay_ticks(&self, eid: u32) -> u64 {
        // The RULE is `eval::delay_ticks_of`, shared with the engine. It was
        // restated here first, and the gate caught that the restatement had
        // dropped the X/Z guard and the `u64::MAX` saturation sentinel — an
        // unbounded delay fired at t+0. Sharing it is what makes that class of
        // divergence unrepresentable rather than merely fixed.
        let v = self.ctx().eval(eid);
        crate::eval::delay_ticks_of(&v, self.sched.st.cur_time_mult, self.sched.st.cur_prec_mult)
    }

    fn k_truthy(&self, eid: u32) -> bool {
        // Conditions are the OTHER half of the walk's evaluation: measured on
        // the tier-3 hot design, `k_truthy` ran 21.7k times to
        // `k_eval_for_lvalue`'s 71.9k, and every one of them took the generic
        // path because a comparison's shape (wide operands, one-bit result)
        // was outside slice 1's admission. The specialization is the
        // EVALUATION only — the verdict comes from the same `truthiness` the
        // generic path uses, over the same `Value`.
        let sw = self.sched.st.wt.get(eid);
        if let Some(r) = self.run_cached_wprog(eid, sw.width, sw.signed) {
            // The planes go straight to the rule. Wrapping them in a `Value`
            // first was measured at 11.6% of a picorv32 run (`one_word_value`),
            // and the verdict is the same function either way —
            // `truthiness_word` is the plane-level entry point of the very
            // `truthiness` this used to call, not a second answer.
            //
            // The program is run INSIDE the cache borrow (`run_cached_wprog`)
            // rather than through an `Rc` handed out of it: the width asked for
            // is `sw.width`, so `prog.width()` was the same number, and it is
            // now read from the request instead of from the program.
            let m = crate::value::low_mask(sw.width);
            return matches!(
                crate::eval::truthiness_word(r.val, r.unk, m),
                crate::eval::Tri::True
            );
        }
        self.ctx().truthy(eid)
    }

    fn k_truthy_value(&self, v: &Value) -> bool {
        matches!(self.ctx().truthiness(v), crate::eval::Tri::True)
    }

    fn k_enter_body(&mut self, tmpl: u32) {
        // The SAME function the engine calls — `cur_time_mult`/`cur_prec_mult`/
        // `cur_scope` live on `sched.st`, which this kernel borrows rather than
        // copies, so there is one origin and one spelling.
        crate::exec::enter_body(self.sched.st, tmpl as usize);
    }

    fn k_call_fatal(&self) -> bool {
        self.sched.st.call_fatal.get()
    }

    fn k_drain_diags(&mut self) {
        self.drain_range_diags();
    }

    fn k_now(&self) -> u64 {
        // `sched.st` is the ONE origin for `now` (see the field doc): the arena
        // holds net VALUES and nothing else, so a delay and a `$time` in the same
        // body cannot come from two clocks.
        self.sched.st.now
    }

    fn k_delta_budget(&self) -> u64 {
        // Read through the borrowed scheduler for the same single-origin reason
        // `now` is: `SimOpts::max_deltas` reaches exactly one field, and a
        // constructor argument here would be a second place for it to be wrong.
        self.sched.k_delta_budget()
    }

    fn k_time_limit(&self) -> Option<u64> {
        self.sched.k_time_limit()
    }

    fn k_suspend_on(&mut self, proc: u32, block: u32, cause: &sim_ir::WaitCause) {
        // `Scheduler::suspend_on`, restated over this store. The one thing that
        // is not a transcription is the arm snapshot: the engine clones
        // `nets[n].cur` (a `BitPacked`), this copies the slot's raw words. Both
        // are "the whole net as it stands now", which is what the fire test
        // compares against.
        let arm = match cause {
            sim_ir::WaitCause::Level { nets } => Some(
                nets.iter()
                    .map(|&n| self.arena.net_words(n).to_vec())
                    .collect(),
            ),
            _ => None,
        };
        self.waiters.push(NativeWaiter {
            cause: cause.clone(),
            proc,
            block,
            arm,
        });
    }

    fn k_schedule_resume(&mut self, proc: u32, block: u32, tick: u64, inactive: bool) {
        // `Scheduler::schedule_resume`, restated over the collapsed ready. The
        // `tick == now` test is what puts a `#0` (and a `#d` that evaluated to
        // zero ticks) into THIS timestep's Inactive region rather than onto the
        // wheel — the wheel is keyed by time, so a same-time entry there would be
        // drained only after the timestep it belongs to had already ended.
        let r = NativeReady { proc, block };
        if tick == self.sched.st.now {
            if inactive {
                push_sorted_native(&mut self.inactive, r);
            } else {
                push_sorted_native(&mut self.active, r);
            }
        } else {
            self.wheel.entry(tick).or_default().push((inactive, r));
        }
    }

    fn k_max_deltas(&self) -> u64 {
        self.max_body_steps
    }

    /// CONTROL: re-arm after `Return`. (Lives here, with the other implemented
    /// control methods, rather than under the refused-workers banner it was
    /// first filed under.)
    fn k_rearm(&mut self, proc: u32) {
        // The LAST unbuilt method (S1d-4c-2a). `Scheduler::rearm`, restated over
        // the wake table S1d-3 built:
        //
        // - a fork CHILD never re-arms. Unreachable here — `fork_modes` non-empty
        //   is an S0 reject, so an activity is 1:1 with a process and `is_child`
        //   is always false — but the engine's first line is a guard on it, and
        //   omitting the reason would leave a reader wondering which of the two
        //   is wrong.
        // - `Edge` and `Initial` MUST NOT re-arm: an edge registration is
        //   permanent (`net_to_edge` is read, not consumed), so re-registering
        //   would make the process fire 2^k times on edge k. `Initial` is
        //   one-shot.
        // - `Comb`/`Latch`/`Level` MUST re-arm: their waiter is CONSUMED when it
        //   fires, so without this they wake once and never again.
        //
        // The asymmetry is the whole content of this method, and it is the same
        // asymmetry `WakeTable::new` encodes at t0 (`level_armed = kind ==
        // Level`, because `arm_processes` QUEUES Comb/Latch rather than arming
        // them).
        //
        // `Initial` shares the do-nothing arm to mirror the engine's spelling,
        // and it is protected a second way: an `initial` process never carries a
        // read set, so `rearm_level` would no-op even if this arm let it through.
        // Measured over the corpus rather than assumed — which is why folding
        // `Initial` into the other arm is an EQUIVALENT mutation, not a gap.
        let tmpl = proc as usize;
        match self.ir.processes[tmpl].sensitivity.kind {
            sim_ir::SensKind::Edge | sim_ir::SensKind::Initial => {}
            sim_ir::SensKind::Comb | sim_ir::SensKind::Latch | sim_ir::SensKind::Level => {
                self.wake.rearm_level(proc)
            }
        }
    }

    fn k_mark_fatal(&mut self) {
        // The SAME `Scheduler::mark_fatal` the engine calls. This used to set a
        // local `self.fatal` flag that NOTHING in the workspace read — so the
        // engine emitted `RunBodyStepLimit` and set `had_fatal` (which drives the
        // process exit class) while the native side hit its step limit in total
        // silence. A loud→silent step, on the one path a `correct-or-loud`
        // project can least afford to take it: the guard exists to report a
        // runaway body, and a runaway body that reports nothing is the failure.
        self.sched.mark_fatal();
    }

    // ── WRITE: store-backed ──

    fn k_write_lvalue(&mut self, lhs: &Lvalue, value: Value, offsets: &Offsets) {
        self.write_routed(lhs, value, offsets);
    }

    fn k_park_frames(&mut self, proc: u32, mut frames: Vec<crate::sched::FrameRec>) {
        // The WINDOW half first, and it is the engine's own function: an
        // automatic frame's window is live on the SHARED `SimState::frame_stack`
        // while its activation runs, so parking without popping it would leave
        // another process's `frame_slot_read` looking at this frame's locals.
        // Popped top-first, which is why the order lives in one place.
        crate::exec::frame_window::stash_windows_in(self.sched.st, &mut frames);
        self.parked_frames.insert(proc, frames);
    }

    fn k_take_frames(&mut self, proc: u32) -> Vec<crate::sched::FrameRec> {
        let Some(mut frames) = self.parked_frames.remove(&proc) else {
            return Vec::new();
        };
        crate::exec::frame_window::restore_windows_in(self.sched.st, &mut frames);
        frames
    }

    fn k_eval_write_scalar(&mut self, lhs: &Lvalue, net: u32, rhs: u32) {
        // The whole assignment in planes when the destination is one word —
        // see `eval_store_word`. `None` falls through to the trait default's
        // two calls, spelled here because a `Kernel` method cannot call its own
        // default once it is overridden.
        if let [c0] = lhs.chunks.as_slice() {
            if self.eval_store_word(lhs, c0, net, rhs).is_some() {
                return;
            }
        }
        let value = self.k_eval_for_lvalue(lhs, rhs);
        self.k_write_scalar(lhs, net, value);
    }

    fn k_eval_nba_scalar(&mut self, lhs: &Lvalue, rhs: u32) {
        // The NBA twin. It cannot store — the update is QUEUED, and
        // `NbaUpdate::sampled` is a `Value` — so what this saves is the two
        // things that surround the sample rather than the sample itself: the
        // `lvalue_width` walk of the IR (the destination is a proven whole net,
        // so its width is the slot's) and the `Rc` clone `wprog_for` forces.
        // The queue entry it builds is the one `k_schedule_nba_scalar` builds,
        // and that method is still the only spelling of it.
        //
        // ⚠️ A6 asymmetry, deliberate: this twin does NOT need the `is_real`
        // decline its STORE twin (`eval_store_word`) grew. It cannot store — it
        // queues a `Value`, and the coercion happens where the update lands, in
        // the write funnel, which is the same place the engine's NBA reaches it.
        // The sampling width is the same too (`max(slot, self(rhs))` is what
        // `k_eval_for_lvalue` computes from `lvalue_width`, and for a real net
        // that IS the slot width). `r <= r + 0.5` takes the generic path anyway,
        // because `wprog` declines the real net read.
        if let [c0] = lhs.chunks.as_slice() {
            let s = self.arena.slots[c0.net as usize];
            if s.words == 1 && s.width > 0 {
                let sw = self.sched.st.wt.get(rhs);
                debug_assert_eq!(
                    self.arena.lvalue_width(self.ir, lhs),
                    s.width,
                    "plain-scalar NBA destination width disagrees with its slot"
                );
                let w = s.width.max(sw.width);
                if let Some(r) = self.run_cached_wprog(rhs, w, sw.signed) {
                    let mut v = Value::zeros(w, sw.signed);
                    v.val[0] = r.val;
                    v.unk[0] = r.unk;
                    self.k_schedule_nba_scalar(lhs, v);
                    return;
                }
            }
        }
        let value = self.k_eval_for_lvalue(lhs, rhs);
        self.k_schedule_nba_scalar(lhs, value);
    }

    fn k_write_scalar(&mut self, lhs: &Lvalue, _net: u32, value: Value) {
        // The compiler proved this destination is a plain whole-net scalar, so
        // the general funnel's offset argument is the constant it would have
        // resolved. The engine's twin takes the same shortcut; `_net` is the id
        // it already knows, and the funnel re-reads it from the chunk — keeping
        // ONE resolution path is worth more here than skipping one index.
        let off = Offsets::Inline {
            buf: [(0, 0); 2],
            len: 1,
        };
        self.write_routed(lhs, value, &off);
    }

    fn k_schedule_nba(&mut self, lhs: &Lvalue, value: Value) {
        // Sample the dynamic LHS index NOW, before the push — the Active-region
        // rule that makes `a[i] <= x; i = i+1;` use the OLD `i`.
        let offsets = self.k_resolve_lvalue_offsets(lhs);
        let seq = self.nba_seq;
        self.nba_seq += 1;
        self.nba.push(NbaUpdate {
            seq,
            lhs: NbaLhs::of(lhs),
            sampled: value,
            offsets,
        });
    }

    fn k_schedule_nba_scalar(&mut self, lhs: &Lvalue, value: Value) {
        debug_assert_eq!(
            self.k_resolve_lvalue_offsets(lhs).as_slice(),
            &[(0, 0)],
            "specialised NBA destination must resolve to the constant offsets"
        );
        let seq = self.nba_seq;
        self.nba_seq += 1;
        let chunk = lhs.chunks[0].clone();
        self.nba.push(NbaUpdate {
            seq,
            lhs: NbaLhs::One(chunk),
            sampled: value,
            offsets: Offsets::Inline {
                buf: [(0, 0); 2],
                len: 1,
            },
        });
    }

    fn k_schedule_nba_at(&mut self, lhs: &Lvalue, value: Value, ticks: u64) {
        // TRANSPORT delay: filed under `now + ticks`, exactly as the engine does.
        //
        // ⚠️ The interleave-by-`seq` this enables is, today, a rule with no
        // production reader — measured: removing the `sort_by_key` from the
        // ENGINE's `apply_nba` leaves the whole package green, because the run
        // loop drains NBA to empty before the timestep advances, so the bucket
        // always extends an EMPTY queue and per-bucket push order is already
        // ascending. Keeping the sort in both is right (the rule is IEEE's, and
        // 4c-2's delta loop is where a non-empty merge becomes reachable), but it
        // is not a mechanism either backend currently exercises.
        // The index is sampled NOW, before the push, like every other NBA.
        let offsets = self.k_resolve_lvalue_offsets(lhs);
        let seq = self.nba_seq;
        self.nba_seq += 1;
        let at = self.sched.st.now.saturating_add(ticks);
        self.delayed_nba.entry(at).or_default().push(NbaUpdate {
            seq,
            lhs: NbaLhs::of(lhs),
            sampled: value,
            offsets,
        });
    }

    // ── CLASSIFICATION: one spelling, shared with the engine's impl ──
    //
    // Answered for real, never stubbed. See the module doc: a `false` here does
    // not make a statement loud, it makes it a DIFFERENT statement.

    fn k_queue_pop_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::queue_pop_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_random_seeded_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::random_seeded_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_dist_seeded_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::dist_seeded_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_cast_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::cast_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_value_plusargs_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::value_plusargs_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fopen_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fopen_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fgetc_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fgetc_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_feof_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::feof_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_ungetc_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::ungetc_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fgets_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fgets_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fread_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fread_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fscanf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fscanf_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_sscanf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::sscanf_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_sformatf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::sformatf_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_assoc_iter_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::assoc_iter_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_rhs_is_stmt_effect_family(&self, rhs: u32) -> bool {
        sim_ir::rhs_is_stmt_effect(self.ir.exprs.as_slice(), rhs)
    }
    fn k_class_new_site(&self, sid: u32) -> Option<u32> {
        self.class_new_sites.get(&sid).copied()
    }

    // ── REFUSED WORKERS: loud, each naming the row that makes it unreachable ──

    fn k_dispatch_systask(
        &mut self,
        which: SysTaskId,
        fmt: Option<u32>,
        args: &[u32],
        sid: u32,
    ) -> Ctl {
        // WIRED (S1d-4b-2). The format engine takes the arena as its net reader;
        // everything else a task touches — output sink, file table, `$time`, the
        // RNG, the assertion side tables — comes from the scheduler, which is
        // where those live for BOTH backends. That split is the whole reason this
        // kernel borrows the scheduler rather than copying its fields.
        //
        // The refused arms are not convenience: they read the store WITHOUT going
        // through the formatter, so the reader parameter never reaches them and
        // they would render `SimState`'s nets while every other value came from
        // the arena — one wrong line in an otherwise right run.
        // `design_eligibility` does not refuse them, so this does.
        if let Some(r) = systask_refusal(which) {
            not_built!(r.label, r.slice, r.why)
        }
        // `$dumpvars` takes its t0 value snapshot from the ARENA (S1d-4d-2) and
        // then turns on the store-point capture. Everything else about it — the
        // header, the scope/var declarations, the filter — reads IR and
        // metadata, so it needs nothing from this seam.
        // A1-iii: the three funnel-outside task writes (`$sformat`'s destination,
        // `$readmem*`'s memory fill, the `$cast` TASK form's `dst`) are COLLECTED
        // rather than written, because only this kernel holds the funnel that
        // reaches the store it owns. `TaskWrites::Direct` — what the engine
        // passes — is literally the `sched.st.write_lvalue` those sites made
        // before, so its path is unchanged by construction.
        let mut pending: Vec<(Lvalue, Value, Offsets)> = Vec::new();
        let (sched, arena) = (&mut *self.sched, &self.arena);
        let ctl = crate::builtins::dispatch_with(
            sched,
            Some(arena),
            &mut crate::builtins::TaskWrites::Collect(&mut pending),
            which,
            fmt,
            args,
            sid,
        );
        // Applied AFTER the dispatch returns rather than inside it: the borrow of
        // `self.sched`/`self.arena` above is what makes the collect necessary in
        // the first place. Nothing inside these three arms reads back what it
        // just wrote (`$readmem` parses tokens and fills; `$sformat` renders its
        // arguments before touching the destination), so the deferral is not
        // observable — and every one of them lands through `write_routed`, which
        // is the single place that answers "which store owns this net".
        for (lv, v, off) in pending {
            self.k_write_lvalue(&lv, v, &off);
        }
        // `$dumpoff` turns dumping off; keep the arena's capture flag in step so
        // a long post-`$dumpoff` run stops buffering values the drain would only
        // discard. Correctness does not depend on this — `vcd_id_for` already
        // returns `None` — but capturing for nothing is work, and a flag that
        // tracks its source is easier to reason about than one that does not.
        self.arena.ch.vcd_on = self.sched.st.dumping;
        ctl
    }

    /// WIRED (slice #2). The REGISTRY rule is `SimState`'s and is the engine's —
    /// `force_prologue` settles the §9.3.1 priority question (a procedural
    /// `assign` displaced by a live force is parked latent and writes nothing)
    /// and `force_epilogue` registers the RHS for continuous re-evaluation. What
    /// this kernel supplies is the WRITE, through its own funnel.
    fn k_force(&mut self, lhs: &Lvalue, value: Value, rhs: u32, sid: u32) {
        if !self.sched.st.force_prologue(lhs, rhs, sid) {
            return;
        }
        self.force_write(lhs, value);
        self.sched.st.force_epilogue(lhs, rhs, sid);
    }
    /// WIRED (slice #2). `release_prologue` owns the four §9.3.1/§9.3.2 arms and
    /// hands back the latent procedural assign whose control resumes, if any;
    /// this kernel evaluates and pins it, then re-dirties the target's
    /// continuous drivers so a released WIRE snaps back in this settle rather
    /// than at the next input change (see `SimState::drivers_of_net`).
    fn k_release(&mut self, lhs: &Lvalue, sid: u32) {
        let resumed = self.sched.st.release_prologue(lhs, sid);
        if let Some((alv, arhs, amult)) = resumed {
            let saved = self.sched.st.cur_time_mult;
            self.sched.st.cur_time_mult = amult;
            let v = self.k_eval_for_lvalue(&alv, arhs);
            self.force_write(&alv, v);
            self.sched.st.cur_time_mult = saved;
            self.sched.st.release_epilogue(&alv, arhs, amult);
        }
        self.redirty_drivers_of(lhs.chunks[0].net);
    }
    fn k_queue_pop(&mut self, lhs: &Lvalue, rhs: u32) -> Value {
        // WIRED (A1-i). DELEGATED, not restated: `Scheduler::k_queue_pop` is
        // store-INDEPENDENT, so asking it produces the answer this kernel would
        // produce. Every input it reads is one of
        //   * `ir.exprs` / `ir.nets`      — the frozen IR,
        //   * `SimState::dyn_heap`        — the heap, keyed by NET ID and the
        //                                   SAME object for both backends
        //                                   (V1 slice 2's whole point),
        //   * `st.lvalue_width` / `st.wt` — widths folded from the IR
        //                                   (`chunk_width` reads declared
        //                                   widths and const-folds a part-select
        //                                   width expr; no slot VALUE),
        //   * `dyn_warn_once_at`          — the shared warn-once latch.
        // and it reads NO net value. The pop's own destination is not written
        // here at all: `apply_effect` writes the returned value through
        // `k_write_lvalue`, which is THIS kernel's funnel.
        //
        // ⚠️ The row that used to keep this unreachable moved twice — the
        // `NetKind` scan until slice 2c, then `stmt_effect`. It is now carved
        // out by `stmt_effect_wired`, so this arm RUNS.
        Kernel::k_queue_pop(self.sched, lhs, rhs)
    }
    fn k_assoc_iter(&mut self, lhs: &Lvalue, rhs: u32) -> Value {
        // WIRED (A1-ii). The whole body is `exec::stmt_effect::assoc_iter`,
        // generic over `Kernel`, so the current-key read and the key write both
        // land in this store while the locate half walks the shared heap.
        crate::exec::stmt_effect::assoc_iter(self, lhs, rhs)
    }
    fn k_disable_fork(&mut self) {
        gate_refused!("k_disable_fork", "statement scan rejects `disable fork`")
    }
    fn k_class_alloc(&mut self, class_id: u32) -> Value {
        // WIRED (A2-i), and it is a DELEGATION for the same reason A1-i's
        // `k_queue_pop` is one: nothing in it reads a net. `class_alloc` mints an
        // id from a `Cell`, default-inits the fields from `class_layouts`, and
        // inserts into `class_heap` — all `SimState`, one object both kernels
        // borrow. The allocation CAP is part of the same chokepoint and must not
        // be restated here, so the whole arm is the engine's.
        //
        // The handle VALUE it returns is written by `apply_effect` through
        // `k_write_lvalue`, i.e. into THIS store's slot — which is exactly the
        // half that is not shared.
        Kernel::k_class_alloc(self.sched, class_id)
    }

    // The funnel-outside family (§4.5.291): every one of these writes through
    // its own dispatch instead of `write_lvalue`, and the `stmt_effect` reject
    // row refuses any design containing one. Wiring them is what LIFTS that row.

    fn k_random_seeded(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::random_seeded(self, rhs)
    }
    fn k_dist_seeded(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::dist_seeded(self, rhs)
    }
    fn k_cast(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::cast(self, rhs)
    }
    fn k_value_plusargs(&mut self, rhs: u32) -> Value {
        // The first `stmt_effect` family member WIRED (S1d-5): parse/match/
        // convert are the shared `exec::plusargs::effect` (one spelling with
        // the engine — the wide-radix fix landed there for both at once); only
        // the destination write is this store's. The gate row now admits
        // exactly this member (`value_plusargs_rhs` carve-out in
        // `design_eligibility`), so the design that reaches here RUNS.
        let (status, write, warn) =
            crate::exec::plusargs::effect(self.ir, &self.sched.st.plusargs, rhs);
        if let Some((radix, text)) = warn {
            // Same emitter as the engine's consumer — one spelling, and the
            // warning lands at the same statement position in the merged
            // stream on both backends.
            self.sched.st.warn_plusargs_invalid(radix, &text);
        }
        if let Some((lv, v)) = write {
            let off = self.k_resolve_lvalue_offsets(&lv);
            self.k_write_lvalue(&lv, v, &off);
        }
        status
    }
    fn k_fopen(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fopen(self, rhs)
    }
    fn k_fgetc(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fgetc(self, rhs)
    }
    fn k_feof(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::feof(self, rhs)
    }
    fn k_ungetc(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::ungetc(self, rhs)
    }
    fn k_fgets(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fgets(self, rhs)
    }
    fn k_fread(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fread(self, rhs)
    }
    fn k_fscanf(&mut self, rhs: u32) -> Value {
        crate::exec::stmt_effect::fscanf(self, rhs)
    }
    fn k_sscanf(&mut self, rhs: u32) -> Value {
        // WIRED (A1-iv-a). `$sscanf` scans a STRING, so the whole body is
        // store-routed and file-table-free: the source comes through `k_eval`
        // and every destination through `k_write_lvalue`.
        crate::exec::stmt_effect::sscanf(self, rhs)
    }
    fn k_sformatf(&mut self, rhs: u32) -> Value {
        // WIRED (S1d-4b-2) through the same seam. `$sformatf` never needed a
        // funnel-outside write — its value stores through the ordinary lvalue
        // path once rendered; what it needed was the format engine.
        //
        // ⚠️ "Nothing reaches it yet: both this and the string CONCAT that
        // elaborate desugars to the same node require a `string` destination,
        // which `NetArena::build` refuses" — that is what this comment said, and
        // it was already FALSE when it was written. A PACKED destination
        // (`reg [63:0] p; p = $sformatf(...)`) takes the same node and no row
        // refuses it; measured running natively on the pre-S3a binary as well as
        // this one. Only the STRING destination is refused, and by the S0
        // `string` net-kind row rather than by the arena. (Found by the S3a
        // differential review.)
        // A non-`SysFunc` rhs is unreachable (`compute_effect` guards on
        // `k_sformatf_rhs`), and the ENGINE answers it with an empty string. This
        // returned `not_built!` — a panic — which made two `Kernel` implementors
        // disagree on one input in a slice whose thesis is that they cannot. It
        // is also not "not built": it is an impossible input, a third category
        // the two macros deliberately do not have. Match the engine.
        let Some(sim_ir::Expr::SysFunc { args, .. }) = self.ir.exprs.get(rhs as usize) else {
            return Value::from_str_bytes(&[]);
        };
        let (f, rest) = (args.first().copied(), args.get(1..).unwrap_or(&[]).to_vec());
        // The reader is `self`, not `&self.arena`: `$sformatf("%0d", f(x))` is a
        // call in an argument, and only the composite answers one. Both are
        // SHARED reborrows of `*self`, so this compiles where the sibling
        // `k_dispatch_systask` (which needs `&mut Scheduler` for the sink and
        // the file table) cannot — which is why that one keeps a gate row.
        let text = crate::builtins::format_args_str_with(self.sched.st, self, f, &rest, None);
        Value::from_str_bytes(text.as_bytes())
    }
}
