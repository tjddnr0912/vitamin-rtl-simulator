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

use sim_ir::{Lvalue, SimIr, SysTaskId};

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
#[allow(dead_code)] // S1d-4b's body walk is the production constructor; today
                    // only the shared-executor differential builds one. Saying that is more
                    // honest than a fake call site or a widened visibility.
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
        SysTaskId::Monitor | SysTaskId::Strobe => (
            "k_dispatch_systask($monitor/$strobe)",
            "S1d-4d",
            "dispatch only REGISTERS them — it captures ExprIds, and the render \
             happens later in `sched/run_loop.rs::flush_postponed`, which the \
             tier-3 run loop does not restate. A `$monitor` would print its t0 \
             line and then never re-fire, because the store its change detection \
             compares against is the engine's and never moves",
        ),
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
        SysTaskId::DumpLimit | SysTaskId::Fclose => (
            "k_dispatch_systask($dumplimit/$fclose)",
            "S1d-4b-3",
            "the ARGUMENT is read through `int_arg`, not the formatter — a \
             store read this seam does not cover",
        ),
        SysTaskId::WritememB | SysTaskId::WritememH => (
            "k_dispatch_systask($writememb/$writememh)",
            "S1d-4b-3",
            "it reads the MEMORY itself, not a formatted argument",
        ),
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
        NativeKernel {
            ir,
            arena,
            sched,
            class_new_sites,
            max_body_steps,
            nba: Vec::new(),
            nba_seq: 0,
            delayed_nba: BTreeMap::new(),
            nba_scratch_lhs: Lvalue { chunks: Vec::new() },
            wake: crate::native::wake::WakeTable::new(ir),
            active: Vec::new(),
            inactive: Vec::new(),
            wheel: BTreeMap::new(),
            waiters: Vec::new(),
        }
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
        let ir = self.ir;
        for u in batch.drain(..) {
            match u.lhs {
                NbaLhs::One(c) => {
                    scratch.chunks.clear();
                    scratch.chunks.push(c);
                    self.arena.write_lvalue(ir, &scratch, u.sampled, &u.offsets);
                }
                NbaLhs::Many(lv) => {
                    self.arena.write_lvalue(ir, &lv, u.sampled, &u.offsets);
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
        let n = self.arena.pending_range.replace(0);
        for _ in 0..n {
            self.sched.st.warn_run_range("array word index");
        }
        self.drain_vcd();
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
    /// `nets` is the only place the two backends can differ — which is the whole
    /// claim this file rests on, now true by construction rather than by
    /// discipline.
    pub(crate) fn ctx(&self) -> crate::eval::EvalCtx<'_, NetArena> {
        crate::eval::EvalCtx {
            ir: self.ir,
            nets: &self.arena,
            now: self.sched.st.now,
            wt: &self.sched.st.wt,
            time_mult: self.sched.st.cur_time_mult,
            rng: &self.sched.st.rng,
            plusargs: &self.sched.st.plusargs,
        }
    }
}

impl Kernel for NativeKernel<'_, '_, '_> {
    #[cfg(feature = "jit")]
    fn k_nets(&self) -> &dyn crate::eval::NetReader {
        &self.arena
    }

    // ── READ: store-backed, shared rules ──

    fn k_eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value {
        // `Scheduler::eval_for_lvalue`, restated over this store: the IEEE
        // assignment rule is width = max(lhs, self(rhs)), sign = rhs self-sign.
        let lw = self.arena.lvalue_width(self.ir, lhs);
        let sw = self.sched.st.wt.get(rhs);
        self.ctx().eval_ctx(rhs, lw.max(sw.width), sw.signed)
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
        let ir = self.ir;
        self.arena.write_lvalue(ir, lhs, value, offsets);
    }

    fn k_write_scalar(&mut self, lhs: &Lvalue, _net: u32, value: Value) {
        // The compiler proved this destination is a plain whole-net scalar, so
        // the general funnel's offset argument is the constant it would have
        // resolved. The engine's twin takes the same shortcut; `_net` is the id
        // it already knows, and the funnel re-reads it from the chunk — keeping
        // ONE resolution path is worth more here than skipping one index.
        let ir = self.ir;
        let off = Offsets::Inline {
            buf: [(0, 0); 2],
            len: 1,
        };
        self.arena.write_lvalue(ir, lhs, value, &off);
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
        let (sched, arena) = (&mut *self.sched, &self.arena);
        let ctl = crate::builtins::dispatch_with(sched, Some(arena), which, fmt, args, sid);
        // `$dumpoff` turns dumping off; keep the arena's capture flag in step so
        // a long post-`$dumpoff` run stops buffering values the drain would only
        // discard. Correctness does not depend on this — `vcd_id_for` already
        // returns `None` — but capturing for nothing is work, and a flag that
        // tracks its source is easier to reason about than one that does not.
        self.arena.ch.vcd_on = self.sched.st.dumping;
        ctl
    }

    fn k_force(&mut self, _lhs: &Lvalue, _value: Value, _rhs: u32, _sid: u32) {
        gate_refused!("k_force", "statement scan rejects `force`/`release`")
    }
    fn k_release(&mut self, _lhs: &Lvalue, _sid: u32) {
        gate_refused!("k_release", "statement scan rejects `force`/`release`")
    }
    fn k_queue_pop(&mut self, _lhs: &Lvalue, _rhs: u32) -> Value {
        gate_refused!("k_queue_pop", "NetKind scan rejects queue storage")
    }
    fn k_assoc_iter(&mut self, _lhs: &Lvalue, _rhs: u32) -> Value {
        gate_refused!("k_assoc_iter", "the assoc NetKind row; the `foreach`-over-dyn-array form is caught by the `dyn_array`/`stmt_effect` rows instead")
    }
    fn k_disable_fork(&mut self) {
        gate_refused!("k_disable_fork", "statement scan rejects `disable fork`")
    }
    fn k_class_alloc(&mut self, _class_id: u32) -> Value {
        gate_refused!("k_class_alloc", "the `class` sidecar row — `class_new_sites`, the very table `k_class_new_site` reads; `NetKind` has no class variant")
    }

    // The funnel-outside family (§4.5.291): every one of these writes through
    // its own dispatch instead of `write_lvalue`, and the `stmt_effect` reject
    // row refuses any design containing one. Wiring them is what LIFTS that row.

    fn k_random_seeded(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_random_seeded", "`stmt_effect` row (§4.5.291)")
    }
    fn k_dist_seeded(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_dist_seeded", "`stmt_effect` row (§4.5.291)")
    }
    fn k_cast(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_cast", "`stmt_effect` row (§4.5.291)")
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
    fn k_fopen(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fopen", "`stmt_effect` row (§4.5.291)")
    }
    fn k_fgetc(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fgetc", "`stmt_effect` row (§4.5.291)")
    }
    fn k_feof(&mut self, _rhs: u32) -> Value {
        gate_refused!(
            "k_feof",
            "`stmt_effect` row (§4.5.291) — over-marked, ROADMAP §5.1"
        )
    }
    fn k_ungetc(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_ungetc", "`stmt_effect` row (§4.5.291)")
    }
    fn k_fgets(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fgets", "`stmt_effect` row (§4.5.291)")
    }
    fn k_fread(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fread", "`stmt_effect` row (§4.5.291)")
    }
    fn k_fscanf(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fscanf", "`stmt_effect` row (§4.5.291)")
    }
    fn k_sscanf(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_sscanf", "`stmt_effect` row (§4.5.291)")
    }
    fn k_sformatf(&mut self, rhs: u32) -> Value {
        // WIRED (S1d-4b-2) through the same seam. `$sformatf` never needed a
        // funnel-outside write — its value stores through the ordinary lvalue
        // path once rendered; what it needed was the format engine.
        //
        // Nothing reaches it yet: both this and the string CONCAT that elaborate
        // desugars to the same node require a `string` destination, which
        // `NetArena::build` refuses. It is wired rather than left panicking so the
        // remaining blocker is described accurately — that blocker is STORAGE
        // (S3), not the formatter.
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
        let text =
            crate::builtins::format_args_str_with(self.sched.st, &self.arena, f, &rest, None);
        Value::from_str_bytes(text.as_bytes())
    }
}
