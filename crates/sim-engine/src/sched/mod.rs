//! Event scheduler: time wheel + IEEE-1364 stratified region queues
//! (Active → Inactive → NBA), deterministic ready ordering, the delta loop,
//! the infinite-delta cap, NBA sample/apply, and net-change propagation with
//! edge detection.

use std::collections::BTreeMap;
use std::rc::Rc;

use sim_ir::{BitPacked, EdgeKind, EdgeTerm, Lvalue, RegionTag, SensKind, Terminator, WaitCause};

use elaborate::{ForkModeTable, JoinMode};

use crate::builtins::{format_args_str, write_out};
use crate::eval::{EvalCtx, NetReader};
use crate::exec::{run_process, Kernel, Offsets, Step};
use crate::state::SimState;
use crate::value::Value;
use crate::DeferRegion;

// ---- split parts (mechanical refactor) ----
mod kernel;
mod propagate;
mod run_loop;
mod scan_arm;
mod wait_fork;
pub(crate) use scan_arm::*;
pub(crate) use wait_fork::*;

/// A schedulable process resume. `proc` is a runtime ACTIVITY id (index into
/// `Scheduler::activities`), NOT a declaration index: top-level processes seed
/// activities `0..nproc` 1:1, and `fork` APPENDS child activities (id ≥ nproc).
/// `tie` is the deterministic intra-region order key (doc-06 tie-break); for a
/// top-level activity it equals the declaration index, for a fork child it is the
/// composite of `(parent_tie, child_idx)` from [`compose_child_tie`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ready {
    pub tie: u32,
    pub proc: u32,
    pub block: u32,
}

/// Per-activity private state. Top-level processes are pre-seeded 1:1 with
/// `ir.processes`; fork children are appended (id ≥ `ir.processes.len()`). The
/// arena only ever GROWS — ids are never reused or reindexed — so any `Ready`
/// stored by value in `wheel`/`waiters`/`net_to_edge` stays valid after a later
/// fork appends.
/// Round-14 V3/V4: one entry on an activity's suspendable-task call-stack. While the
/// stack is non-empty, `run_process` executes the TOP frame's task CFG (from the global
/// `ir.blocks` arena at `bb`) instead of the base process body — so a `Delay`/`Wait` in
/// a task body suspends the whole activity (the stack is preserved on the `Activity`).
/// `Return` pops the frame, copies `out_binds` to the caller's lvalues, and resumes the
/// parent at `ret_bb`.
pub(crate) struct FrameRec {
    /// The suspendable task's `FuncId` (index into `func_table` / `ir.funcs`).
    pub callee: u32,
    /// Current block in `ir.blocks` for this frame.
    pub bb: u32,
    /// Where the PARENT resumes on `Return`: a base-process-local block id when this is
    /// the only frame (parent is the process), else a global `ir.blocks` id (parent is a
    /// task frame). The pop site interprets it by whether the stack is then empty.
    pub ret_bb: u32,
    /// `(callee out-/inout-slot, caller lvalue)` — copied out at `Return`.
    pub out_binds: Vec<(u32, sim_ir::Lvalue)>,
    /// Round-14 V3/V4 Phase 3: this frame's AUTOMATIC window while the activity is
    /// SUSPENDED. During execution the window lives on the shared `frame_stack` (top =
    /// current, `None` here); on a `Delay`/`Wait` in this frame it is STASHED here
    /// (popped off `frame_stack`) so interleaving activities can't corrupt the shared
    /// stack, and RESTORED (pushed back) when the activity resumes. `None` for a static
    /// callee (its slab lives in `static_store`, not `frame_stack`). Stage-2 fork-in-frame:
    /// a `WindowSlot::Shared(h)` here is a Case-B task frame (parent) or its fork arm — the
    /// stashed slot is just the arena handle; the window data stays in `frame_windows`.
    pub window: Option<crate::state::WindowSlot>,
    /// T1-9: this activation's frame-local dyn-array STASH — the heap objects the OUTER
    /// activation held in those net-keyed slots, taken at entry and put back at `Return`.
    ///
    /// It lives HERE, on the per-activity call stack, rather than on a shared stack,
    /// because a suspendable frame's entry and exit are separated by arbitrary scheduler
    /// activity: A can enter this task, suspend, let B enter the SAME task, then resume
    /// and exit first. A shared stack would hand A back B's array. Riding the call stack
    /// makes the lifetime exactly the activation's, which is what the stash means.
    ///
    /// Empty for the overwhelming majority of frames (a callee with no dyn-array local).
    pub dyn_stash: Vec<(u32, Option<crate::state::DynObj>)>,
    /// This frame's OWN frame-local dyn-array contents while the activity is SUSPENDED —
    /// the heap-object twin of `window`, parked and unparked at exactly the same two
    /// points. Non-empty only across a suspend, and only on the TOP frame (the heap slots
    /// hold the top frame's values; outer activations live in the `dyn_stash` above them).
    ///
    /// This is what lets two CONCURRENT activations of one task each keep their own array:
    /// `dyn_stash` alone is sound only when their lifetimes NEST, which a `fork` breaks.
    pub dyn_parked: Vec<(u32, Option<crate::state::DynObj>)>,
    /// Has this frame executed a `fork`? Its arms run IN it and read its frame-locals, so
    /// its dyn-array slots must stay in the shared heap (see `park_frame_dyn`) — a parked
    /// array is absent, not shared, and the arm would read X.
    pub forked: bool,
    /// Is this a fork ARM frame (built by `exec_fork` for an in-frame fork), as opposed
    /// to a real CALLEE frame pushed by a `Call`?
    ///
    /// Load-bearing, not cosmetic: the child-completion intercept compares this frame's
    /// `bb` against the barrier's `join_bb`, and the two live in DIFFERENT numbering
    /// spaces unless the frame is an arm. An arm's `bb` and an in-frame fork's `join_bb`
    /// are both global `ir.blocks` ids; a callee frame's `bb` is a global id while a
    /// TOP-LEVEL fork's `join_bb` is a process-local block index. Without this flag a
    /// plain `fork tk(1); tk(2); join` — where each child merely CALLS a suspendable task
    /// — also reached the intercept with `call_stack.len() == 1`, and any numeric
    /// collision between the two spaces killed the child mid-task: its remaining body
    /// vanished at exit 0 and `exit_arm_frame` tore down a window that was not an arm's.
    pub is_arm: bool,
}

pub(crate) struct Activity {
    /// Round-14 V3/V4: suspendable-task call-stack. Empty for the overwhelming majority
    /// of activities (the base process runs directly); non-empty only while inside a
    /// suspendable task call. `run_process` reads the top frame's CFG when non-empty.
    pub call_stack: Vec<FrameRec>,
    /// Index into `ir.processes` for the body/sensitivity TEMPLATE this activity
    /// runs. Multiple activities may share a template (a child runs a different BB
    /// sub-chain of the SAME `body` Vec as its parent).
    pub template: u32,
    /// Deterministic ordering key (top-level: == template; child: composite).
    pub tie: u32,
    /// If this activity is a fork child, the barrier id it reports completion to.
    /// `None` for top-level processes.
    pub join_ref: Option<u32>,
    /// Role bit: is this a spawned fork child? Children never re-arm.
    pub is_child: bool,
    /// Completion-report guard: set true the FIRST time this child reaches its
    /// barrier's join_bb. A second report is an internal error (double-decrement).
    /// Always `false` for top-level activities.
    pub reported: bool,
    /// P2-E `disable fork`: a killed descendant. Stale queue/waiter/wheel
    /// entries survive; the single dispatch choke (`run_body`) drops them.
    pub dead: bool,
    /// v8 `wait fork`: when `Some`, this (parent) activity is parked until all
    /// its outstanding immediate children report completion. `None` otherwise.
    pub wait_fork: Option<WaitForkPark>,
    /// `true` while this activity is SUSPENDED MID-BODY — it has run from its
    /// `entry` and hit a blocking control (`#delay` / `wait` / `@(...)` /
    /// `fork` / `wait fork`) without yet reaching `Return`. An edge-sensitive
    /// `always`'s permanent `net_to_edge` entry must NOT re-trigger it from
    /// `entry` while busy (IEEE: a process is not re-entered until it completes
    /// and re-arms). Set on `Step::Suspended`, cleared on `Step::Done`. In-body
    /// waiter wakes (the `resume` block) are unaffected — only the static
    /// top-sensitivity re-fire is gated.
    pub busy: bool,
    /// Incarnation counter for this activity SLOT (§16.4 deferred-assert keying):
    /// bumped each time the slot is re-issued to a new fork child via
    /// `free_activities`. Distinguishes a completed activation's pending deferred
    /// report from a later activation that recycled the same `aid`. Top-level
    /// processes never recycle, so their generation stays 0.
    pub gen: u32,
}

/// A process blocked on `wait fork;` (IEEE §9.6.1) — parked until all of its
/// outstanding forked children report completion. Tracked directly on the
/// parent activity (not a `JoinBarrier`) because the cumulative child set spans
/// every prior `fork ... join_none` / surplus `join_any` child, which report to
/// their OWN barriers; the count here is decremented by `on_child_complete`.
pub(crate) struct WaitForkPark {
    /// Continuation BB where the parent resumes once `outstanding` hits 0.
    pub resume_bb: u32,
    /// Count of the parent's still-running immediate children.
    pub outstanding: u32,
}

/// One live fork's join barrier.
pub(crate) struct JoinBarrier {
    /// Activity id of the parent that is (or will be) blocked here.
    pub parent: u32,
    /// The join convergence BB (Fork.join), in the parent's template body. Used as
    /// the child-completion sentinel; NEVER fetched as a real block.
    pub join_bb: u32,
    /// Parent's continuation BB (Fork.resume_bb), in the parent's template body.
    pub resume_bb: u32,
    /// Join mode recovered from the elaborate side table.
    pub mode: JoinMode,
    /// Count of children that have NOT yet reached the join.
    pub outstanding: u32,
    /// Has the parent already been resumed past this barrier? (fire-once guard.)
    pub fired: bool,
}

/// A pending nonblocking LHS update: RHS sampled in Active, applied in NBA.
/// An NBA update's destination.
///
/// The queue outlives the activation that pushed it, so the destination must be OWNED —
/// and `Lvalue` owns a `Vec<LvalChunk>`, which made every nonblocking assignment a
/// malloc at push and a free at apply. On picorv32 + testbench (40000 cycles) that is
/// 2,474,446 allocation pairs, and the free side alone measured 25.8 ms of the NBA
/// region's 117 ms.
///
/// Measured on the same run: 2,463,015 of those 2,474,446 updates — **99.5%** — are a
/// SINGLE chunk. `One` stores that chunk by value, so the overwhelmingly common
/// nonblocking assignment allocates nothing at all. `Many` keeps the old owned `Lvalue`
/// for a concat LHS (`{a,b} <= x`).
pub(crate) enum NbaLhs {
    One(sim_ir::LvalChunk),
    Many(Lvalue),
}

impl NbaLhs {
    /// Take an owned destination from a borrowed one, allocating only for a concat LHS.
    pub(crate) fn of(lhs: &Lvalue) -> Self {
        match lhs.chunks.as_slice() {
            [c] => NbaLhs::One(c.clone()),
            _ => NbaLhs::Many(lhs.clone()),
        }
    }
}

pub(crate) struct NbaUpdate {
    pub seq: u64,
    pub lhs: NbaLhs,
    pub sampled: Value,
    /// Per-chunk `(bit-offset, array-word)` sampled in the ACTIVE region when the
    /// `<=` executed (so `a[i] <= x; i = i+1;` / `m[k] <= x;` use the OLD `i`/`k`),
    /// one per `lhs.chunks`.
    pub offsets: Offsets,
}

/// One simulation time's three region buckets. Active/Inactive hold process
/// resumes + continuous-assign re-evals; NBA holds sampled updates.
#[derive(Default)]
struct SlotQueues {
    active: Vec<Ready>,
    inactive: Vec<Ready>,
}

/// A process suspended on a Wait condition.
struct Waiter {
    cause: WaitCause,
    ready: Ready,
    /// For an IN-BODY `@(sig)`/`@(*)` (a `WaitCause::Level` from `suspend_on`):
    /// the net values snapshot AT ARM TIME, one per `Level.nets`. The waiter fires
    /// only when a net differs from this snapshot — so a change that completed
    /// BEFORE the wait armed (e.g. the t0 `X→init` settle done by another initial
    /// block before `@(sig)` suspended) does NOT spuriously trigger it. `None` for
    /// a STATIC always/comb sensitivity (those re-fire on any change, by design).
    arm: Option<Vec<BitPacked>>,
}

/// One INERTIAL-delay continuous-assign write: `(cont-assign index,
/// generation, lhs, value, per-chunk (offset, word))`. Applied when the
/// simulation reaches the scheduled tick IF the generation still matches
/// `ca_gen[ci]` — a later RHS change bumps the generation, so the stale
/// pending write is silently dropped (IEEE inertial pulse filtering: a pulse
/// narrower than the delay never reaches the LHS; iverilog-pinned live).
type DelayedWrite = (u32, u64, Lvalue, Value, Offsets);

/// V2A-frame (§4.5.173): a frame task call's copy-ins, split by kind — scalar
/// `(formal slot, evaluated value)` pairs and dynamic-array snapshot `(formal slot,
/// caller source net)` pairs. See [`Scheduler::split_frame_in_binds`].
type FrameInBinds = (Vec<(u32, Value)>, Vec<(u32, u32)>);

pub(crate) struct Scheduler<'a, 'ir> {
    pub st: &'a mut SimState<'ir>,
    /// Current time's Active/Inactive buckets.
    cur: SlotQueues,
    /// NBA region (applied as a batch when Active+Inactive empty).
    nba: Vec<NbaUpdate>,
    /// The `Lvalue` `apply_nba` lends to a single-chunk update so it can call the
    /// `&Lvalue` write funnel without the update having carried a heap `Vec` here. Its
    /// one-element allocation is made once and reused for the whole run.
    nba_scratch_lhs: Lvalue,
    nba_seq: u64,
    /// Future events keyed by absolute tick.
    wheel: BTreeMap<u64, Vec<(RegionTag, Ready)>>,
    /// Processes blocked on Wait conditions.
    waiters: Vec<Waiter>,
    /// WAITER-POOL p2: running counts of `Expr`/`Level` waiters in `waiters`,
    /// maintained exactly at the two push sites (`suspend_on` Expr, `arm_
    /// sensitivity`/`suspend_on` Level) and the single `propagate_changes`
    /// `retain` removal. When a count is 0 the matching `retain` match-arm
    /// cannot fire, so `propagate_changes` skips building the corresponding
    /// `expr_now`/`level_fire` precompute buffer entirely — an all-`Edge`
    /// design (every flop `always @(posedge clk)`, no `wait(expr)`/`@*`) pays
    /// nothing for those scans. Byte-identical: the skipped buffer was unused.
    n_expr_waiters: usize,
    n_level_waiters: usize,
    /// Activity id currently executing a body (set by `run_body`, the single
    /// dispatch choke) — `disable fork` kills THIS activity's descendants.
    cur_aid: u32,
    /// net → edge-sensitive process resumes.
    /// DIRTY-SETTLE: continuous assigns that must be re-evaluated on EVERY settle
    /// pass because `levelize::ca_deps` could not certify them skippable (a delayed
    /// assign, a multi-driver member, an impure RHS, or a heap-handle dependency whose
    /// contents can move without the handle net changing). Ascending, so the union with
    /// the dirty worklist stays in declaration order.
    /// CONT-ASSIGN NATIVE: one pre-compiled native program per continuous assign, or
    /// `None` where `try_compile` refused. Continuous assigns are the one evaluation the
    /// bytecode backend never touched — they are not process bodies — and on a real
    /// design the settle is the largest block the backend leaves alone (measured on
    /// picorv32 + testbench: bodies 3762 -> 2171 ms under the VM while the settle stayed
    /// at ~1090 ms, 23.4% of the run).
    ///
    /// Compiled in EXACTLY the context `eval_for_lvalue` uses — `lvalue_width(lhs)`
    /// widened by the RHS self-width, signedness from the RHS — which is the same
    /// (context, program) pairing the body path's `Op::EvalNative` uses and the P5 gate
    /// already locks.
    ca_native: Vec<Option<crate::native_eval::NativeProg>>,
    ca_always: Vec<u32>,
    net_to_edge: Vec<Vec<(EdgeKind, Ready)>>,
    /// Per-activity private state. `index == Ready.proc` (activity id). Seeded 1:1
    /// with `ir.processes` at t0; fork appends children (append-only, never reused).
    pub(crate) activities: Vec<Activity>,
    /// Live fork join barriers. `index == JoinBarrier id` (a child's `join_ref`).
    /// Slots are RECYCLED through `free_barriers` once every child has reported
    /// (P3-1) — no live reference can outlast that point, so no ABA.
    barriers: Vec<JoinBarrier>,
    /// P3-1 free lists: completed fork-child activity slots / fully-drained
    /// barrier slots, recycled by the next `exec_fork`. Without these a
    /// `forever fork … join_none` loop grows both arenas O(timesteps)
    /// (~800 MB over 10M cycles). Determinism: the free-list state is a pure
    /// function of the (deterministic) execution, and ids are internal — VCD/
    /// stdout bytes are unchanged (P5 gate + corpus enforce).
    free_activities: Vec<u32>,
    free_barriers: Vec<u32>,
    /// Join-mode side table from elaborate, keyed `(template, join_bb)`.
    fork_modes: ForkModeTable,
    /// Last RHS value seen per cont-assign — only used by DELAYED `assign #d`
    /// (change detection, so a delayed write schedules once per RHS change).
    last_ca: Vec<Option<Value>>,
    /// S1: last value this cont-assign actually DROVE onto the net (the gate's
    /// own output node — a superseded/inertially-cancelled pending write never
    /// updates it). This is the baseline for the per-bit rise/fall/turnoff delay
    /// (IEEE 1364 §7.14: a transition's delay is measured from the value the net
    /// CURRENTLY holds, not the previous RHS — they differ on inertial supersede).
    last_ca_drv: Vec<Option<Value>>,
    /// Per-cont-assign schedule generation — bumped on every new RHS change,
    /// invalidating any pending write carrying an older generation (the
    /// inertial cancel; see `DelayedWrite`).
    ca_gen: Vec<u64>,
    /// MULTI-DRIVER: nets driven by ≥2 cont-assigns that are ALL whole-net,
    /// non-delayed targets → `(net, driver cont-assign indices, resolution kind)`.
    /// kind 0=WIRE (z yields; equal keeps; conflict→x), 1=WAND (wired-AND),
    /// 2=WOR (wired-OR) — z is the identity for all three. Settled by per-bit
    /// 4-state resolution instead of last-write-wins. Empty for every single-driver
    /// design ⇒ byte-identical (multi-driver was E3001-rejected before). Partial/
    /// dynamic/array/delayed overlaps stay E3001 (whole-net buses are the v1 scope).
    md_nets: Vec<(u32, Vec<usize>, u8)>,
    /// MULTI-DRIVER: per-cont-assign flag — this driver is a member of an
    /// `md_nets` group, so the per-driver individual write in `settle_cont_assigns`
    /// is SKIPPED (the net is written once by the resolution instead).
    ca_md: Vec<bool>,
    /// Per-cont-assign flag: a DELAYED driver that is the SOLE cont-assign driver
    /// of every net its lhs touches. Only such a driver gets the initial-X drive
    /// during `[0, d)` (its output register holds x until the first delayed write
    /// lands). A delayed driver sharing a net with ANY other cont-assign is
    /// excluded — the every-delta X-drive would otherwise fight a concurrent
    /// undelayed driver and spin the delta budget (`ca_md` does NOT catch this:
    /// a delayed ci is never a `ca_md` member, and a dynamic/array-element overlap
    /// is a deliberate E3001 blind spot). Non-sole delayed nets keep the pre-fix
    /// undriven-z during the window (safe: byte-identical to before this fix).
    delayed_sole: Vec<bool>,
    /// Pending inertial-delay cont-assign writes, keyed by absolute apply tick.
    delayed_ca: BTreeMap<u64, Vec<DelayedWrite>>,
    /// Transport NBAs (`q <= #d v`, v5 increment A): updates due at a FUTURE
    /// tick's NBA region. Drained into `nba` when time advances to the key;
    /// `apply_nba`'s global-seq sort keeps statement order across both paths.
    delayed_nba: BTreeMap<u64, Vec<NbaUpdate>>,
    delta_count: u64,
    max_deltas: u64,
    time_limit: Option<u64>,
    /// Scratch buffers reused across `propagate_changes` calls (take/restore —
    /// the alternative per-call `Vec::new` allocates on every delta).
    scratch_changed: Vec<u32>,
    /// GLITCH/SELF-RETRIG: per-changed-net `(net, slot_edge_mask, blocking_writer)`.
    /// `mask` = the net's intra-slot bit0 edge summary (`SimState::slot_edge`), so
    /// both the static edge-wake pass (a) and the in-body `Edge` waiter pass (b)
    /// fire on a glitch the endpoint `prev/cur` compare would lose (single write ⇒
    /// mask == `edge_fires(kind, prev, cur)` ⇒ byte-identical). `blocking_writer` =
    /// the activity that authored the change via a BLOCKING write (`u32::MAX`
    /// otherwise); snapshotting it here lets the `retain` closure suppress
    /// re-firing a process on a net it itself wrote without re-borrowing `self.st`.
    scratch_edges: Vec<(u32, u8, u32)>,
    /// N4 / multi-edge dedup: per-`propagate_changes` markers so a process
    /// sensitive to MULTIPLE nets that ALL change in the SAME delta is woken
    /// EXACTLY ONCE (IEEE §9: an `always @(posedge c1 or posedge c2)` ticks once
    /// per slot even when both edges occur together — vita previously pushed it
    /// twice → ran the body twice). `seen[proc]` marks an already-queued proc this
    /// pass; `marked` records the touched indices so cleanup is O(#fired), not
    /// O(#procs). Empty/untouched for single-edge designs ⇒ byte-identical.
    scratch_edge_seen: Vec<bool>,
    scratch_edge_marked: Vec<u32>,
    /// C-FORCE-REEVAL-p2: reused buffer of the force KEYS (target nets) selected
    /// for re-eval this delta (changed-net-triggered ∪ always-reeval), kept
    /// sorted to preserve the BTreeMap re-eval order contract.
    scratch_force_keys: Vec<u32>,
    /// WAITER-POOL: reused `expr_now`/`level_fire` precompute buffers in
    /// `propagate_changes` (was two fresh `Vec<bool>` per non-idle delta).
    scratch_expr_now: Vec<bool>,
    scratch_level_fire: Vec<bool>,
    /// VM-REGPOOL: recycled bytecode-VM register/offset files (was a fresh
    /// `vec![None; n]` pair per `vm_exec` activation). Leased in `vm_run_body`.
    vm_regs_pool: Vec<crate::backend::RegFile>,
    vm_offs_pool: Vec<crate::backend::OffFile>,
    /// Recycled wheel-bucket Vecs: `wheel.remove` would otherwise drop one
    /// bucket allocation per distinct simulation time (O(timesteps) churn).
    bucket_pool: Vec<Vec<(RegionTag, Ready)>>,
    /// Generation of the CURRENTLY-running activity (`activities[cur_aid].gen`),
    /// set alongside `cur_aid`. Keys §16.4 deferred reports so a recycled `aid`
    /// cannot flush a completed prior instance's pending report.
    cur_gen: u32,
}

/// Why the run ended (scheduler precedence order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Finish,
    Stop,
    Quiescent,
    DeltaLimit,
    Error,
}
