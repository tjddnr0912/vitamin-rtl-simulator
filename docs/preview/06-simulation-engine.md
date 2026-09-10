# 06 · Simulation Engine

The kernel contract: which scheduling regions vitamin models and how each one is
represented, the exact order a time step is drained in, the key that makes that order
deterministic, how a process body suspends and resumes, how values and dynamic storage
are represented, and the hardening limits that bound a run. Delay arithmetic and the
timescale model live in [08-timescale-and-timing.md](08-timescale-and-timing.md); the
tier-3 backend's storage and gates in
[21-tier3-native-backend.md](21-tier3-native-backend.md).

The crates are `sim-ir` (the frozen, serialized IR) and `sim-engine` (the scheduler,
the executors, the evaluator, the heaps). `simulate` borrows the IR immutably for the
whole run and never writes to it:

```rust
pub fn simulate(ir: &SimIr, sink: &dyn LogSink, opts: SimOpts) -> SimResult
```

---

## 1. The event model

vitamin is event-driven. A process runs only when a value it depends on changes, and
time jumps straight to the next scheduled event rather than stepping through empty
ticks. A cycle-based simulator evaluates the whole design once per clock edge, which is
simpler to build but cannot express asynchronous logic or sub-cycle timing; the trade
for vitamin is analysed in [20-cycle-mode-feasibility.md](20-cycle-mode-feasibility.md).

A **delta cycle** is a zero-time iteration within one simulation time. The scheduler
re-drains the region cascade until nothing moves; `now` does not advance and a
per-timestep delta counter increments.

```
T=10 : a changes to 1
  delta 1: always @(a) b = a  → b changes  → b's observers queue
  delta 2: always @(b) c = b  → c changes  → c's observers queue
  delta 3: nothing observes c → every queue empty
  → T=10 is stable; time advances to the next scheduled event
```

Combinational feedback (`assign a = ~a;`) never reaches a fixpoint. vitamin bounds it
rather than hanging: `max_deltas` deltas per time step (§17) ends the run with
`F-RUN-NO-CONVERGE` and `FinishReason::DeltaLimit`.

---

## 2. Scheduling regions

IEEE 1364 divides a time slot into four regions; IEEE 1800 refines that into seventeen.
vitamin models **seven**, and the seven are the ones a design outside a `program` block
can observe.

| Region | Modelled | Representation |
|---|---|---|
| Preponed | yes | `SimState::preponed_buf`, snapshotted at time zero and at every time advance, committed at the clocking edge during change propagation |
| Active | yes | `SlotQueues.active`, drained after the continuous-assign settle |
| Inactive | yes | `SlotQueues.inactive`, promoted wholesale into Active |
| NBA | yes | `Scheduler::nba: Vec<NbaUpdate>` plus `delayed_nba: BTreeMap<u64, Vec<NbaUpdate>>` for transport delays |
| Observed | yes | `postponed.deferred_observed` — where `assert #0` matures |
| Reactive | yes | `postponed.deferred_reactive` — where `assert final` matures |
| Postponed | yes | `flush_postponed()` — the `$strobe`/`$fstrobe` FIFO, then the `$monitor`/`$fmonitor` change check |
| Pre-Active, Pre-NBA, Post-NBA, Pre-Observed, Post-Observed, Re-Inactive, Pre-Re-NBA, Re-NBA, Post-Re-NBA, Pre-Postponed | no | see below |

Two constructs can observe the difference between the modelled seven and the full
seventeen, and they are handled differently.

- A **clocking skew** other than `#1step` is refused with `E3009`. A clocking block with
  the default input skew parses, elaborates and runs, and a clocking read returns the
  value sampled immediately before the edge; `#0`, `#N`, `##N` and output skews all
  refuse. Icarus Verilog does not support clocking blocks, so the verdicts here are
  pinned against the standard by hand, with Verilator as a partial second reading.
- A **`program` block** parses into the same module container and runs. Its processes are
  scheduled in the Active region rather than the Reactive region.

> **Status at HEAD.** The Reactive-region scheduling that IEEE 1800 §24 gives `program`
> processes is approximated as Active scheduling. For a standalone testbench the
> observable behaviour matches Icarus Verilog; a design that races a `program` block
> against the device under test is where the approximation shows.

Concurrent SVA does not need the remaining ten regions. A `property` or `sequence` is
lowered by elaborate into a synthesized clocked checker — a shift-register pipeline or a
goto/non-consecutive FSM, plus its pass and fail actions — so it evaluates as ordinary
clocked logic on the regions above. `cover property` becomes a clocked match counter and
a `final` report. Deferred immediate assertions are the constructs that genuinely need
Observed and Reactive, and those two regions exist for them.

A terminating step (`$finish`, `$stop`, `$fatal`) drains the deferred regions and then
flushes Postponed before returning, so a `$strobe` or a matured `assert #0` in the same
slot as the finish is still printed.

### 2.1 Why the NBA region matters

A nonblocking assignment samples its right-hand side in Active and applies its
left-hand side in NBA. Any process reading that signal in between still sees the old
value, which is what makes a flip-flop chain shift by exactly one stage per edge.

```systemverilog
// Deterministic: the NBA region separates sample from update.
always_ff @(posedge clk) begin
  b <= a;  // Active samples a; NBA writes b
  c <= b;  // Active samples the OLD b; NBA writes c
end
// a → b → c, one stage per clock.

// Race-prone: blocking assignment updates in place.
always_ff @(posedge clk) begin
  b = a;   // b is a immediately
  c = b;   // reads the already-updated b, so c is also a
end
// Both registers take a; there is no shift.
```

---

## 3. The time step loop

`Scheduler::run` is two nested loops: the outer one owns a simulation time, the inner
one drains that time to a stable point.

```
snapshot_preponed()                        # time 0
loop {                                     # OUTER: one simulation time
  if finished { return finish_kind() }
  delta_count = 0
  loop {                                   # INNER: drain this time
    check_call_fatal()                      -> Error
    match settle_cont_assigns() {           # (a) continuous assigns to fixpoint
      None       => return DeltaLimit,      #     oscillating assign
      Some(true) => propagate_changes(),
      Some(false)=> {}
    }
    check_call_fatal()                      -> Error
    if !active.is_empty()   { take batch; run each body; propagate; delta++; continue }
    if !inactive.is_empty() { active = take(inactive); reset_edge_seen(); delta++; continue }
    if !nba.is_empty()      { reset_edge_seen(); apply_nba(); propagate; delta++; continue }
    if !deferred_observed.is_empty() { mature(Observed); propagate; delta++; continue }
    if !deferred_reactive.is_empty() { mature(Reactive); propagate; delta++; continue }
    if !dirty.is_empty()    { propagate_changes(); delta++; continue }   # late producer
    break                                   # stable
  }
  flush_postponed()                         # POSTPONED
  next = min(wheel.first, delayed_ca.first, delayed_nba.first)   or Quiescent
  if time_limit.is_some_and(|l| next > l) { return Quiescent }
  now = next
  reset_edge_seen_marks()
  snapshot_preponed()
  apply due delayed continuous-assign writes (generation-filtered); propagate if moved
  take_due_delayed(next)                    # transport NBAs join this tick's batch
  drain wheel[next] into the Inactive or Active queue per its RegionTag
}
```

`delta_count` is shared between the continuous-assign fixpoint and the region cascade,
and is reset at the top of every time step. Every `delta++` site immediately compares it
against `max_deltas` and fails the run once, loudly, if it is exceeded.

The final `dirty` arm exists for one producer: the clocking commit, which runs inside
change propagation after that pass has already taken its changed set.

---

## 4. The time wheel

| Structure | Type | Holds |
|---|---|---|
| `wheel` | `BTreeMap<u64, Vec<(RegionTag, Ready)>>` | process resumes scheduled at a future tick |
| `delayed_ca` | `BTreeMap<u64, Vec<DelayedWrite>>` | inertial `assign #d` writes |
| `delayed_nba` | `BTreeMap<u64, Vec<NbaUpdate>>` | transport `q <= #d v` updates |

`DelayedWrite` is `(continuous-assign index, generation, Lvalue, Value, Offsets)`.

Time advances to the **minimum first key of all three maps**. When all three are empty
the run ends `Quiescent`. Drained wheel buckets are recycled through a bucket pool.

`schedule_resume(proc, block, tick, inactive)` short-circuits: when `tick == now` it
pushes straight into the current Active or Inactive queue; otherwise it files into
`wheel[tick]` tagged `RegionTag::Active` or `RegionTag::Inactive`. This tag on a wheel
entry is the only place either of those two tags is ever constructed (§12).

---

## 5. Ordering and determinism

The contract is that one `SimIr` produces byte-identical VCD and byte-identical stdout
on every supported platform. IEEE leaves the order of processes within a region
implementation-defined; vitamin fixes it, because a deterministic answer is worth more
here than the freedom to reorder.

No hash-map iteration ever decides execution order.

| Mechanism | Rule |
|---|---|
| Ready ordering | `Ready { tie, proc, block }`; the insert point is `partition_point(\|x\| x.tie <= r.tie)`, so a queue is sorted by `tie` and equal ties keep insertion order |
| Top-level tie | `tie == template == declaration index`; activities are seeded 1:1 with `ir.processes` |
| Fork-child tie | `compose_child_tie(parent, i) = ((parent + 1) << 16) \| (i & 0xFFFF)` — children sort strictly after their parent, siblings in declaration order |
| Tie-encoding cap | at most 65534 top-level processes and 65536 arms per fork; either overflow is a graceful fatal, never a silent alias |
| Time wheel | `BTreeMap`, drained in insertion order within a tick |
| NBA batch | sorted by `seq`, a single monotonic counter shared by same-tick and transport updates |
| Continuous-assign settle | worklist visited in ascending index, i.e. declaration order |
| Changed-net sweep | the dirty list is sorted ascending before use |
| Forces | `active_forces` is a `BTreeMap`, so re-evaluation order is fixed |
| Deferred reports | keyed `(marker StmtId, activity id, activity generation)` in a `BTreeMap` |
| Monitors | the stdout monitor first, then file monitors in ascending descriptor |
| Postponed order | frozen: every strobe in call order, then the monitor lines |
| Slot recycling | the activity and barrier free lists are a pure function of the (deterministic) execution, and the ids they hand out are internal |

The ready-ordering key is `Ready.tie`, an engine-side field. It is **not** the
`tie_break` field of the IR's `WakeKey`, which nothing reads (§12).

---

## 6. Change propagation and edge detection

`propagate_changes()` runs three passes, and their order is load-bearing.

0. If any force is active and the dirty list is non-empty, re-evaluate the active
   forces to a fixpoint (budget: `active_forces.len() + 2` passes).
1. Take the write funnel's dirty list, deduplicated by a per-net flag, and sort it
   ascending. **Membership in the dirty list alone is the changed set**: an A→B→A round
   trip inside one slot still counts as a change, which is what IEEE asks for and what
   an endpoint comparison would drop.
2. Snapshot `(net, slot_edge[net], last_blocking_writer[net])` for every changed net.
3. **(a) static edge wakes.** Walk the per-net edge-observer list and fire an observer
   when the intra-slot edge mask matches its kind, the target activity is not suspended
   mid-body, and the writer is not the observer itself. A clocking-commit handler is
   diverted here instead of being queued. Multi-edge duplicates are collapsed through
   scratch mark arrays.
4. **(b) in-body waiters.** `Level` waiters fire per the precomputed fire set, `Edge`
   waiters from the intra-slot mask, `Expr` waiters when the predicate is now true. A
   fired waiter is consumed.
5. **(c) refresh `prev`** for every changed net — last, so that every edge observer in
   this pass read the pre-change value.

Edge masks are a `u8` per net: bit 0 records that a posedge occurred, bit 1 a negedge,
bit 2 that the net changed at all. They are maintained only for nets that are actually
edge targets, and reset when a net is first dirtied in a slot.

The "already woken this cluster" marks are **timestep-scoped, not per-delta**. They are
reset at a `#0` Inactive promotion, at an NBA apply, and at time advance — the gated-clock
rule.

Self-retrigger suppression: the running activity is recorded as the blocking writer
around a body run, and the last change's author is recorded per net. A sentinel author
(NBA, continuous assign, clocking commit, force) re-fires normally.

---

## 7. Re-arming

When a body returns, whether its sensitivity is re-registered depends on how that
sensitivity is stored. The asymmetry is load-bearing.

| `SensKind` | On `Return` |
|---|---|
| `Edge` | do **not** re-register — edge-observer entries are permanent and are read, not consumed; re-pushing would make the process fire 2^k times on the k-th edge |
| `Initial` | one-shot; the activity is done after its single run |
| `Comb`, `Latch`, `Level` | **must** re-arm — their waiters are consumed when they fire |
| any fork child | never re-arms |

`arm_sensitivity` registers an `Edge` process once per edge term in the per-net observer
list, and registers `Level`, `Comb` and `Latch` with a single level waiter over the
sensitivity read set. A process whose read set is empty (a bare self-timed `always`, or
`always_comb o = 1'b0;`) registers nothing.

A waiter with no arm snapshot is static sensitivity and fires on any change. An in-body
`@(sig)` carries an arm snapshot, so it fires only on a change that happens after it
armed.

---

## 8. Time-zero arming and static initialization

`arm_processes()` seeds the base activities and then runs, in this order:

1. **Fork-mode gate, total or fatal.** Every `Terminator::Fork` in every body must have
   a `(template, join_bb)` entry in the fork-mode table. A miss ends the run with a
   graceful fatal and arms nothing. A mode is never fabricated: turning a `join_none`
   into a `join` would deadlock silently.
2. **Declaration initializers** run to completion, in the sidecar's order, before
   anything is armed. An out-of-range process id is a graceful fatal.
3. **Copy-net repair.** A net whose every continuous driver only moves bits is re-driven
   after the initializers land.
4. **Roll back the dirt** those two steps created, so initialization produces no event —
   `reg clk = 0;` must not hand `always @clk` an X→0 edge — plus a transitive
   copy-net suppression pass.
5. **Arm.** `Initial`, `Comb` and `Latch` are pushed into the time-zero Active queue;
   `Edge` and `Level` only register sensitivity. Processes listed in the init and final
   sidecars are skipped.

Before any of that, `simulate` runs a structural continuous-assign settle. If it cannot
converge, the run ends immediately with `FinishReason::DeltaLimit`.

`run_finals()` runs each `final` process once, in ascending order, after the main loop
ends and whatever the finish reason was. A `$finish` inside a `final` block is absorbed,
and Postponed is flushed after each.

> **Status at HEAD.** A `$finish` reached in the Active region terminates the time step
> without draining the edge-triggered processes still pending in it, so a clocked
> liveness checker can miss an edge that coincides with the finish.

---

## 9. Continuous assignments

`settle_cont_assigns()` is a fixpoint over a dirty worklist: the certified assigns whose
dependency nets moved, union the assigns the dependency certifier refused to certify
(delayed, multi-driver member, impure right-hand side, heap-handle dependency). The
worklist is visited in ascending index. Failure to converge is `DeltaLimit`.

- A **delayed** assign is skipped inside the fixpoint and scheduled after it. While no
  driver value has been produced yet and the assign is the sole driver, the target reads
  **X** — `assign #3 o = a & b;` gives `o == x` over `[0, d)`.
- **Multi-driver** nets carry a driver group tagged wire, `wand` or `wor`. Members are
  skipped individually inside the fixpoint; all drivers are evaluated, folded by the
  group's resolution function, and the net is written once. Z is the identity for all
  three folds. Partial, dynamic, array and delayed overlaps stay an elaborate-time
  `E3001` refusal.
- **Inertial pulse filtering**: a generation counter is bumped on every new right-hand
  side change, and a pending write carrying an older generation is dropped at apply time.
- **Distinct rise / fall / turn-off delays** select the effective delay as the maximum
  over the changed bits of the destination-based delay (→1 rise, →0 fall, →z turn-off,
  →x the minimum of the three). The transition is atomic at that maximum.
- The delayed set is pre-filtered once so the fixpoint does not walk it.

---

## 10. Nonblocking updates

```rust
struct NbaUpdate { seq: u64, lhs: NbaLhs, sampled: Value, offsets: Offsets }
enum  NbaLhs     { One(LvalChunk), Many(Lvalue) }
```

The single-chunk case dominates — measured at 99.5% of updates on a long picorv32 run —
so `One` stores the chunk by value and allocates nothing.

Scheduling samples the **dynamic left-hand index now**, in the Active region, so
`a[i] <= x; i = i + 1;` writes the old `i`. A whole-net scalar left-hand side takes a
compiler-proved specialisation with its offsets baked in. `q <= #d v` files a transport
update at `now + ticks` (saturating) into the delayed map. Applying the batch sorts by
`seq` and writes each update through the shared write funnel.

---

## 11. The process execution model

### 11.1 Bodies are basic blocks; every wait point is a terminator

Elaborate flattens a procedural block (`initial`, `always*`, task, `fork`-`join`) into a
sequence of basic blocks. Each block is a `Vec<Stmt>` plus **exactly one**
`Terminator`, and every suspension point is a terminator, never a statement:

| Terminator | Meaning |
|---|---|
| `Goto { target }` | unconditional edge |
| `Branch { cond, then_bb, else_bb }` | conditional edge; loops are back-edges |
| `Delay { amount, region, resume }` | `#d`; `amount` is an ExprId in the module's time units, evaluated at suspension time |
| `Wait { cond, resume }` | `@(edge)`, `@(level list)`, `wait(expr)`, `@(named_event)`, `wait fork` |
| `Fork { children, join, resume_bb }` | `fork`-`join` / `join_any` / `join_none` |
| `Call { target, ret_bb }` | a call to a subroutine the scheduler drives |
| `Return` | end of body or of a frame |

This is a structured IR, not a bytecode instruction set: the data-growth axis (`Expr`,
`Stmt`) is kept separate from the resume vocabulary (`Terminator`) so that adding
expression shapes does not churn the resume schema. A resume point is a **block index**,
never a native program counter.

A process body is a **private block arena** with process-local indices. Function and
task bodies live in the single global `ir.blocks` arena.

`WaitCause` is the closed wait vocabulary the engine reads: `Edge{net,kind}`,
`Level{nets}`, `Expr{expr}`, `Named{ev}`, and the unit `Fork` for `wait fork`.
`DelayRegion { Active, Inactive }` on a `Delay` terminator is the one scheduling-region
enum in the IR that the engine actually consults.

### 11.2 Process kinds

| Source construct | `SensKind` | Time-zero behaviour |
|---|---|---|
| `initial` | `Initial` | runs at time zero, one-shot |
| `final` | `Initial` shape plus a `final_procs` sidecar entry | never armed; runs once after the loop ends |
| `always_comb` | `Comb` | runs at time zero (IEEE implicit execution) |
| `always_latch` | `Latch` | runs at time zero |
| `always_ff @(…)` | `Edge` | waits for the first edge |
| `always @(posedge/negedge …)` | `Edge` | waits |
| `always @(a or b)`, no edges | `Level` | waits |
| `always @*` / `always @(*)` | `Level` | waits — measured: unlike `always_comb`, it has no implicit time-zero execution |
| bare `always` with in-body `#` or `@` | `Comb`, empty edge set | runs at time zero; nothing would move otherwise |
| bare `always` with no timing at all | `Comb`, empty edge set, plus a warning that it is unschedulable | inert |

`Comb` and `Latch` carry the read set elaborate inferred. The scheduler registers
`Level`, `Comb` and `Latch` with the same level waiter.

### 11.3 Activities

An `Activity` is a runtime process instance. Top-level processes are seeded 1:1 with
`ir.processes`; fork children are appended with ids above that range. The arena only
grows and ids are never reindexed, so a `Ready` stored by value in the wheel, in a
waiter or in an edge-observer list stays valid.

```rust
struct Activity {
  call_stack: Vec<FrameRec>,     // suspendable-task frames; empty in the common case
  template: u32,                 // index into ir.processes (body and sensitivity)
  tie: u32,                      // deterministic ordering key
  join_ref: Option<u32>,         // the barrier a fork child reports to
  is_child: bool,
  reported: bool,                // fire-once completion guard
  dead: bool,                    // killed by `disable fork`
  wait_fork: Option<WaitForkPark>,
  busy: bool,                    // suspended mid-body
  gen: u32,                      // incarnation counter for a recycled slot
}
```

`busy` gates only the static top-level sensitivity re-fire: an `always` is not re-entered
until it completes and re-arms. In-body waiter wakes are unaffected.

`gen` disambiguates a recycled activity id for deferred reports keyed
`(marker, activity, generation)`. Top-level activities never recycle, so their generation
stays 0. Completed child activities and drained barriers go on free lists; without them
a `forever fork … join_none` loop grows both arenas in proportion to the number of time
steps.

### 11.4 `Step` and the `Kernel` seam

One activation returns `Done`, `Suspended`, `Finish`, `Stop` or `Fatal`.

`trait Kernel` is the seam between a process body and the kernel. It has a read phase
(evaluate an expression, resolve left-hand offsets, ask a self width, read a net), a
write phase (write an lvalue, schedule an NBA, force, release, dispatch a system task,
allocate a class object, disable a fork), a control surface (`k_now`, `k_suspend_on`,
`k_schedule_resume`, `k_rearm`, `k_enter_body`, `k_call_fatal`, the delta and time
budgets) and the statement-effect family (queue pop, seeded random, cast, `$value$plusargs`,
the file-read calls, `$sformatf`, associative iteration), each with a predicate that
forwards to one canonical definition.

`compute_effect` and `apply_effect` are **generic over `Kernel`**. They are the statement
semantics, written once and shared by every executor; only body control flow differs
between executors. The shared net-write and VCD choke point stays on the shared side, so
VCD and stdout bytes cannot diverge in an executor-specific way. The other face of that
design: a defect in the shared code is wrong in every executor at once, which is why an
absolute oracle is mandatory rather than optional.

### 11.5 Frames and call stacks

Two independent frame mechanisms exist, because a subroutine that cannot suspend does
not need the scheduler.

**(a) The synchronous frame executor.** A value-returning function, or a task in the
non-suspending subset, runs entirely inside expression evaluation on a `&self` path.
Storage:

| Structure | Role |
|---|---|
| `frame_stack: RefCell<Vec<WindowSlot>>` | LIFO of live windows; `Owned(Vec<Value>)` is the common path, `Shared(u32)` the fork-in-frame path where a parked parent and its arms reference one window by handle |
| `frame_windows` + `frame_window_free` + `frame_window_rc` | the shared-window arena, its free list and its reference counts (a `join_any`/`join_none` survivor can outlive its parent) |
| `static_store` | per-`FuncId` persistent slabs for static slots, X-initialised once and never restored |
| `frame_template` | the per-`FuncId` fresh-window template, computed once from the IR |
| `frame_window_pool` | capacity-capped free list of retired automatic windows |

`frame_slot_default` is the single producer of a fresh slot's default: a `string` slot
starts empty, everything else starts from the net's IR initial value — so 2-state locals
start at 0, not X.

**(b) The scheduler's suspendable-task call stack.** `Activity.call_stack` holds frames
for subroutines that can suspend:

```rust
struct FrameRec { callee: u32, bb: u32, ret_bb: u32,
                  out_binds: Vec<(u32, Lvalue)>,
                  window: Option<WindowSlot>,             // stashed across a suspend
                  dyn_stash:  Vec<(u32, Option<DynObj>)>, // outer activation's frame-local arrays
                  dyn_parked: Vec<(u32, Option<DynObj>)>, // this frame's arrays across a suspend
                  forked: bool, is_arm: bool }
```

While the stack is non-empty the top frame's CFG runs from the **global** block arena,
so a `#delay` or `@(…)` inside a task body suspends the whole activity. On suspend the
frame's automatic window is popped off the shared frame stack and stashed in the record;
on resume it is pushed back. `Return` pops the frame, copies the out-binds to the
caller's lvalues, and resumes the parent at `ret_bb`. `is_arm` distinguishes a fork-arm
frame from a callee frame, because their block numbers live in different numbering
spaces and a numeric collision otherwise kills a child mid-task at exit 0.

Which subroutines the scheduler drives is **recomputed at startup from the IR**, not
read from a sidecar, so the one-shot and staged paths agree by construction. A statement
is a suspend signal unless it is a `disable` marker or a blocking assignment whose
left-hand side stays inside the function's own frame window and whose right-hand side is
not in the statement-effect family.

### 11.6 fork, join, `wait fork`, `disable`

| Join mode | Parent behaviour |
|---|---|
| `join` | blocks until every child reaches the join |
| `join_any` | unblocks at the first child; the rest run on |
| `join_none` | never blocks |

The mode is looked up by `(template, join_bb)` and the lookup is total or fatal (§8).
A fork inside a task body uses a sentinel template, because its join block is a globally
unique id and its mode must not depend on which process ran the task.

`exec_fork_into` registers a barrier (recycling a free slot) and spawns one child
activity per arm with the composed tie of §5; the tie-encoding cap is checked first.

Child completion is caught by a **centralized, terminator-agnostic intercept** at the
top of the body runner: when a child's next block to fetch is its barrier's join block,
it reports and dies before that block is fetched. A child whose last statement is an
`if`, `case`, delay or wait into the join block is therefore handled uniformly. A second
intercept handles an in-frame fork arm.

`wait fork;` parks the parent with an outstanding count instead of a barrier, because the
cumulative child set spans every prior `join_none` and every surplus `join_any` child,
all of which report to their own barriers.

`disable fork` marks every active descendant of the caller dead, transitively, in index
order; the caller lives on. It **unschedules nothing** — resume entries already filed in
the slot queues, the waiters and the wheel arrive later and are dropped at the single
dispatch choke point. Pending deferred reports belonging to the killed activities are
retained out. A plain `disable <named block>` is the break/continue idiom and lowers to a
marker plus a sibling `Goto`.

### 11.7 Automatic and static lifetimes

Per net and per function, elaborate derives whether the net is frame-local, which
`(function, slot)` it routes to, whether its effective lifetime is automatic, and whether
the function has automatic slots, static slots, a shared fork or a dynamic local. A
frame-local net is read and written from the per-call window when its lifetime is
automatic and from the persistent static slab when it is not. A function pushes a
per-call window only when it has automatic slots.

---

## 12. The frozen suspension shape is inert

`sim_ir::SimIr` includes a per-process `SuspendState`, and that shape is frozen: it is
part of the golden schema hash, so changing it invalidates every `.velab` artifact.

```rust
struct SuspendState {
    resume_pc: u32,
    locals: Vec<FourState>,
    join_state: JoinState,          // { parent, children, detached, flags }
    wake_key: WakeKey,              // { cond: WakeCond, region: RegionTag, tie_break: u32 }
    call_stack: Vec<Frame>,         // { return_pc, callee_entry, locals_base, locals_len, is_automatic }
    frame_arena: Vec<FourState>,
}
```

**No engine code reads or writes any of it.** Elaborate emits one fixed constant per
process (`resume_pc = entry`, empty locals, empty call stack, empty frame arena,
`wake_key = { cond: Level { nets: [] }, region: Active, tie_break: 0 }`), and nothing
consumes it. `simulate` holds the IR immutably and cannot write it even in principle.

| IR field | Frozen shape | What actually holds this state |
|---|---|---|
| `SuspendState.resume_pc` | `u32` | the resume block id inside a `Ready` / a `FrameRec.bb` |
| `SuspendState.locals` / `frame_arena` | `Vec<FourState>` | `SimState.frame_stack`, `frame_windows`, `static_store` (§11.5) |
| `SuspendState.call_stack` | `Vec<Frame>` | `Activity.call_stack: Vec<FrameRec>` |
| `SuspendState.join_state` | `JoinState` | `JoinBarrier` plus `Activity.{join_ref, is_child, reported}` |
| `WakeKey.cond` (`WakeCond`, 6 variants) | frozen enum | `WaitCause` on a `Terminator::Wait`, and the engine's waiter records |
| `WakeKey.region` (`RegionTag`) | 4 variants | wheel entries use `Active` and `Inactive`; **`Nba` and `Monitor` are never constructed anywhere** |
| `WakeKey.tie_break` | `u32` | `Ready.tie`, an engine-side field (§5) |

The engine's own region tag on a wheel entry, and `DelayRegion` on a `Delay` terminator,
are the two region enums that are actually consulted. `RegionTag` carries `Nba` and
`Monitor` variants for the NBA and Postponed regions, but nothing ever constructs either, so
neither region has a live `RegionTag` representation.

The shape is kept because it is frozen: removing a field flips the golden root hash and
invalidates every artifact already written, for no behavioural gain. It is documented here
so nobody reads the IR expecting to find live suspension state, and so nobody adds a
consumer without first deciding what the constant means.

---

## 13. Delays

`Terminator::Delay.amount` is an ExprId whose value is in the **declaring module's time
units**, evaluated at suspension time. A constant `#5` and a runtime `#d`, `#(d*2)` or
`#r` therefore take one path; the constant is simply folded. Conversion to global ticks
is the single shared function `delay_ticks_of(value, M, S)`:

| Input | Result |
|---|---|
| real | round at the module's own precision, then scale: `r = round(v · M/S)`; `r < 0` gives `u64::MAX` (never fires); otherwise `r · S`, saturating |
| any X or Z | **0 ticks** |
| integral | `v · M`, saturating; a negative integer yields `u64::MAX` and never fires |

Region selection at the terminator is `inactive = (region == Inactive) || ticks == 0`,
so **any** zero-length delay lands in the Inactive queue, not only a syntactic `#0`.

The full timescale model — how `M` and `S` are resolved, why the rounding has two stages,
and how `$time`, `$realtime` and `%t` render — is
[08-timescale-and-timing.md](08-timescale-and-timing.md).

---

## 14. System task and function dispatch

`hdl-builtins` is a one-line stub. Every handler is inlined in `sim-engine/src/builtins/`
and reached through a `SysTaskId` / `SysFuncId` match, both of which are closed enums in
the frozen IR.

| Family | Members | Handling |
|---|---|---|
| Output | `$display`, `$write`, `$sformat`, `$swrite`, `$sformatf` and their radix suffixes | rendered through one format engine and written to the deterministic sink |
| Deferred output | `$strobe`, `$monitor` and the `$f…` and radix variants | captured now, rendered in Postponed |
| Time | `$time`, `$realtime`, `$stime`, `$timeformat` | see [08](08-timescale-and-timing.md) |
| Waveform | `$dumpfile`, `$dumpvars`, `$dumpflush`, `$dumplimit` | routed to the writer, see [07-vcd-format.md](07-vcd-format.md) |
| Control | `$finish`, `$stop`, `$fatal`, `$error`, `$warning`, `$info` | terminate or report; see [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md) |
| Random | `$random`, `$urandom`, `$urandom_range`, the seven `$dist_*` | §15.4 |
| File | `$fopen`, `$fclose`, `$fdisplay`, `$fwrite` (including the multi-channel form), `$fgetc`, `$ungetc`, `$feof`, `$fgets`, `$fscanf`, `$sscanf`, `$fread`, `$readmem*`, `$writemem*` | descriptor table in `SimState`; both the write and the read families are implemented |
| Conversion and math | `$signed`, `$unsigned`, `$clog2`, `$rtoi`, `$itor`, `$realtobits`, `$bitstoreal`, `$cast`, the 21 real-math functions | §15.5 |
| Queries | `$countones`, `$onehot`, `$onehot0`, `$isunknown`, `$test$plusargs`, `$value$plusargs` | — |
| Heap and container | the dynamic-array, queue, associative-array and string methods, array reductions, sort and locator methods | §16 |

### 14.1 `$strobe` and `$monitor`

`$strobe` does not print when it executes. It pushes a capture onto the postponed strobe
FIFO, which is rendered with settled values at the end of the time step, in call order,
and then cleared. Each call is one-shot.

`$monitor` is standing. Its contract:

- **One monitor per destination.** A `$monitor` replaces the standing stdout monitor; an
  `$fmonitor` replaces the monitor for its own descriptor and leaves the stdout monitor
  alone.
- **Change detection is on values, not on text.** At the postponed flush the argument
  expressions are evaluated to a value list and compared bit-plane-exactly against the
  stored baseline. A line is printed only when the list differs, or when there is no
  baseline yet (establishment, or a fresh install).
- **At most one line per destination per time step**, emitted in Postponed after every
  strobe, so the values printed are the settled ones.
- `$monitoroff` sets a **global**, simulation-wide disable flag; it works before any
  `$monitor` is installed and survives a re-install. `$monitoron` clears it and clears
  every baseline, forcing a reprint at the next flush.
- Each capture snapshots its registering module's time multiplier and scope, and the
  flush drives `%t` and `%m` from that snapshot per render.

Because a monitor argument is re-rendered on every later change, a system function with a
statement-level side effect is refused in that position rather than being evaluated an
unpredictable number of times.

---

## 15. Value representation

### 15.1 Four-state encoding

Every value carries two bit planes, `(v, u)`, per bit:

| `(v, u)` | Bit |
|---|---|
| `(0, 0)` | 0 |
| `(1, 0)` | 1 |
| `(0, 1)` | X |
| `(1, 1)` | Z |

Word 0 bit 0 is the LSB. This is the same encoding as `sim_ir::BitPacked` and as the VCD
writer, so a runtime value round-trips into the IR and out to the waveform without a
conversion step.

**Z is treated as X in every operator except `===` and `!==`** (and in net storage): the
per-bit primitives are defined over `{0, 1, X}`, and a Z input folds to X because its
unknown plane is set.

### 15.2 `Words` and `Value`

```rust
enum Words { Inline { w: [u64; 2], len: usize },   // ≤128 bits, no allocation
             Heap(Vec<u64>) }                       // >128 bits

struct Value {
    val: Words, unk: Words,
    width: u32, signed: bool,
    is_real: bool,   // val[0] is f64::to_bits(x); 64-bit, 2-state
    is_str: bool,    // width = 8·len packed ASCII, MSB-first; bypasses context resize
}
```

`Words` dereferences to `[u64]` and compares **by slice contents, never by variant**,
because values are compared for `$monitor` change detection. The inline arm exists
because once bit-serial net I/O and shifts were word-ized, value allocation became the
single dominant runtime cost.

`Value` is runtime-only: it is never serialized and never appears in `SimIr` or in the
schema hash.

### 15.3 Arithmetic lanes

| Condition | Lane |
|---|---|
| either operand is real | f64 promotion. An X/Z integer entering a mixed real operation decays to `0.0`. Division follows f64 semantics (`x/0 → ±inf`, `0/0 → NaN`), not X. `%` and `**` on a real are refused at elaborate time |
| any operand has X or Z (integral) | the whole result is poisoned to X |
| signed wider than 64 bits, or unsigned wider than 128 | exact multi-word arithmetic on the word grid |
| otherwise | a 128-bit lane, wrapping, masked to the result width |

Multi-word arithmetic is exact: school multiplication, short or restoring long division,
square-and-multiply power. Each operand extends under its own sign; everything is modulo
2^w two's complement. A quotient truncates toward zero and a remainder takes the
dividend's sign. Division by zero gives X.

`**` is the one operator whose exponent is self-determined, so the sign decision carries
three separate booleans (left, right, result) rather than one.

**`WIDE_ARITH_CAP = 1 << 20` bits.** Above it, `*`, `/`, `%` and `**` poison to X,
because the quadratic kernels would stall — a replication can push an *operand* to 16M
bits, measured at 34 s for one multiply and 163 s for one divide. `+` and `-` are linear
and stay exact at any width. The cap equals `elaborate::MAX_NET_WIDTH`, so any operand
within the declarable width regime is always computed exactly. A run that contains such
an expression emits exactly one `W-RUN-WIDE-ARITH` warning, from a single static scan of
the expression arena.

### 15.4 Randomness

| Generator | Algorithm |
|---|---|
| `$random` | the IEEE 1364 Annex N 32-bit LCG `s' = 69069·s + 1`, with a zero seed substituting `259_341_593` on the first draw. Pure double multiply and add, no libm, bit-exact on any IEEE-754 platform |
| `$urandom`, `$urandom_range` | implementation-defined by IEEE 1800; vitamin pins splitmix64 from initial state 0, taking the top 32 bits, global. This pin is part of the reproducibility contract |
| `$dist_uniform` | the Annex algorithm, one seed advance, pure f64 |
| the six non-uniform `$dist_*` | the seed stream is the Annex pure-integer one; the result comes from the vendored libm and is a vitamin pin, deterministic across platforms |
| `randomize()` | its own seed stream, separate from `$random` and `$urandom` |

Every evaluation of `$random` or `$urandom` is a new draw, so a re-rendered `$monitor`
line re-rolls. The seed cells are interior-mutable so the `&self` read path can draw.

### 15.5 Reals and strings

- A real net is a flat 64-bit net whose initial value stores `f64::to_bits`. **No `f64`
  field exists anywhere in the frozen IR** — the derive guard sees only `u64` inside
  `BitPacked`, which is what keeps the schema hash portable. `realtime` maps to the same
  net kind. The net's real flag drives the real/integer coercion in the write funnel and
  the flag on reads.
- A string net is handle-style: the bytes live in the dynamic heap, the net itself has
  width 0. A read materialises packed ASCII, MSB-first; a write strips leading NUL bytes.
  A string value bypasses context resizing so a dynamic-length string is never truncated
  by a static width.
- The 21 real-math functions and the non-uniform `$dist_*` run on a **vendored pure-Rust
  libm** (`third_party/libm`, MIT), built with no hardware intrinsics so the results are
  byte-identical across platforms. It is a path dependency excluded from the workspace, so
  workspace-wide clippy does not lint third-party code.

---

## 16. Heap and dynamic storage

### 16.1 The dynamic heap

`SimState.dyn_heap` is a flat `Vec<Option<DynObj>>` indexed by net id and sized to the
net arena. A `None` entry is the lazy empty object. Ordering is never observed — every
access is a point operation on one handle net — so the flat layout is byte-identical to a
map. It is interior-mutable because the synchronous `&self` frame executors mutate it.

```rust
enum DynObj {
    DynArray { elems: Vec<Value> },                 // int d[]
    Queue    { elems: VecDeque<Value> },            // int q[$]
    Assoc    { map: BTreeMap<i64, Value> },         // int a[longint]
    Str      { bytes: Vec<u8> },                    // string s
    AssocStr { map: BTreeMap<Vec<u8>, Value> },     // int a[string]
}
```

The byte order of the string-keyed map is IEEE lexicographic string comparison, so
`.first()` and `.next()` iterate in the order IEEE specifies without extra work.

`MAX_DYN_ELEMS = 1 << 24`. **There are no silent caps**: every clamp or drop emits
`W-RUN-DYN-DEGRADE`, once per handle net. A bounded queue (`[$:N]`, capacity `N+1`)
truncates the tail and warns.

Two side bitmaps accelerate the hot path: whether a net is a handle at all, and whether a
dynamic array's elements are string handles.

### 16.2 Class objects

`SimState.class_heap` is a `BTreeMap<u32, ClassObj>` keyed by a **monotonic allocation
id**, never by a net id, because several handles can alias one object and a handle can be
re-`new`ed. Ids start at 1 so that 0 stays reserved for `null`, and are never recycled, so
no use-after-free aliasing is possible.

`ClassObj { class_id, fields: Vec<Value> }`. The class id is the **dynamic** type, set at
`new` and never changed by a handle copy; it drives virtual dispatch. In a class layout,
**base-class fields come first**, so a derived object up-cast to its base reads the same
field ids. Field defaults follow IEEE: a folded declaration initializer wins, otherwise
4-state fields start at X and 2-state fields at 0.

A handle net is an ordinary logic slot holding the object id. Virtual dispatch indexes a
per-class vtable by slot; each call site records its own `(vslot, static target)` pair.

Constrained random rides sidecars: the random field set with its bounds, the constraint
programs in postfix form (solved by rejection sampling: the first pass enforces every
predicate, and only if that finds no solution does a second pass drop the soft ones and
retry with the hard predicates alone), the weighted `dist` entries, the
`randc` permutation state per instance, and the per-call `with { … }` overrides, which are
intersected into the class domain and whose predicates are ANDed.

**The class heap is never garbage-collected.** `max_class_objs` bounds it at the single
allocation choke point, and exceeding it is a graceful fatal with an implicit `$finish`
rather than an out-of-memory abort.

### 16.3 The net store

```rust
struct NetSlot { cur: BitPacked, prev: BitPacked, width: u32, array_len: u32,
                 signed: bool, is_real: bool,
                 vcd_id: Option<IdCode>, vcd_word_ids: Vec<Option<IdCode>> }
```

The current value occupies `array_len × width` bits, with word `w` at
`[w·width, w·width + width)`. Per-net side bitmaps carry the dirty flag, the forced flag,
2-state-ness, the edge mask and whether the net is an edge target, the last blocking
writer, the handle flags, the frame-local and automatic-lifetime flags, whether the net is
a plain scalar, and a lazily memoized tier-3 eligibility flag.

Reads go through a `NetReader` trait so that a backend owning its own storage can answer
them. The trait carries the leaf fast path `read_scalar_words(net, word, ctx_signed)`,
which avoids constructing two values for a leaf read — measured, leaf reads and their
resizes accounted for roughly 30% of a real design's profile. It also carries a deferred
out-of-range report drain, so a replayed diagnostic names the right array and lands in
the right place relative to the `$error` it is inside.

A wrapper splits ownership when a backend owns the flat slots: that backend answers plain
net reads, while `SimState` still answers handle nets, frame-local nets, the iteration
context and the file table.

---

## 17. Resource limits and hardening

Every limit below is fail-closed and loud. None of them truncates a design silently.

| Guard | Constant | Failure |
|---|---|---|
| Delta cycles per time step | `max_deltas = 1_000_000` | `F-RUN-NO-CONVERGE`, once per run; `FinishReason::DeltaLimit` |
| Block steps in one activation | `max_body_steps = 100_000_000` | `F-RUN-BODY-STEP-LIMIT`; `FinishReason::Error`. A separate budget from `max_deltas` on purpose: it answers "has one activation run this long without suspending", not "did the scheduler reach a fixpoint", so a long straight-line loop in an `initial` block is not mis-reported as combinational oscillation |
| Live class objects | `max_class_objs = 1_000_000` (≈160 MiB) | `F-RUN-CLASS-LIMIT`, a graceful fatal with an implicit `$finish` |
| Dynamic-storage elements | `MAX_DYN_ELEMS = 1 << 24` | clamp or drop plus `W-RUN-DYN-DEGRADE`, once per handle net |
| Recursion depth | `MAX_CALL_DEPTH = 8192` | latches a fatal (the evaluator is on a `&self` path and cannot return a `Step`); the in-flight evaluation finishes with X and the scheduler consumes the latch at the next region seam. Sized so that this many re-entries fit the 256 MiB worker stack on the platform with the fattest frames, and still four times the deepest legal-recursion test. Raising it requires raising both worker stacks in lockstep |
| Worker stack | `STACK_SIZE = 256 MiB` | the whole CLI runs on one spawned worker thread; address space is lazily committed, and a panic there is re-raised so the process exits 101 rather than through a vita exit class |
| Wide arithmetic | `WIDE_ARITH_CAP = 1 << 20` bits | `*`, `/`, `%`, `**` poison to X plus one `W-RUN-WIDE-ARITH` per run |
| Out-of-range index reports | 8 per kind, **two separate budgets** | a known index past the end is `E-RUN-RANGE`; an X/Z index is `W-RUN-RANGE-UNKNOWN`. The access is recovered either way — read X, drop the write — so the run finishes. The budgets are separate so that a reset window full of unknown indices cannot starve the genuine out-of-range report. Both messages name the array and the indexing site |
| Fork tie encoding | ≤65534 top-level processes, ≤65536 arms per fork | graceful fatal, never a silent tie collision |
| Bad file descriptor | once-per-descriptor latch | `W-RUN-BAD-FD` |
| `$dumpvars` re-call | once-per-run latch | `W-RUN-DUMP-MULTI` |
| `$dumplimit(bytes)` | RTL-controlled | one `$comment` in the waveform, then records are dropped |
| Simulation-time limit | `time_limit`, CLI `--timeout <ticks>` | when the next scheduled tick exceeds the limit the run ends cleanly as `Quiescent`, which bounds an `always #1;` hang |

Elaborate-side caps that bound the input to all of this: `MAX_NET_WIDTH = 1<<20`,
`MAX_TOTAL_NETS = 1<<17`, `MAX_ARRAY_LEN = 1<<24`, `REPEAT_UNROLL_CAP = 1024`,
`GENERATE_UNROLL_CAP = 4096`, `GENERATE_DEPTH_CAP = 32`, `MAX_ELAB_ERRORS = 200`.

The executing statement id is carried in a cell so that a `&self` primitive can locate
its diagnostic. It **fails to none, never to stale**: it is cleared before every
terminator and on every early return, because a block's terminator condition reads nets
after the last statement. The residue is that a diagnostic raised by a terminator
condition carries no source location.

`unsafe` outside the optional `jit` feature is exactly one call — `signal(2)` in the CLI
front end.

---

## 18. Executors

Three executors share the statement semantics of §11.4 and differ only in how they walk
a body.

| `--backend` | Variant | Compiled in | Role |
|---|---|---|---|
| `native` | `Backend::Native` | always | the default; owns a flat arena net store and runs a compiled op-stream for the bodies that qualify, walking the IR for the rest — and a fork child always takes the walk |
| `vm`, `bytecode` | `Backend::Bytecode` | `oracle` feature (on by default) | a bytecode VM over the shared net store, falling back to the interpreter body by body |
| `interp`, `interpreter` | `Backend::Interpreter` | `oracle` feature | the readable reference semantics — a test instrument, permanently excluded from performance work |

The default is `Native` in both spellings that can express it (the enum's derive and the
`SimOpts` literal), and a test pins both. `SimResult.backend` reports what actually ran;
when a design forces a fall-back, a `W-RUN-BACKEND-FALLBACK` warning says so, because the
fall-back is a slower answer rather than a wrong one. In a build without the `oracle`
feature there is nothing to fall back to, so a refusal is a graceful fatal instead.

Because all three share the write funnel and the VCD choke point, they are usable as
bisection oracles against each other, and the equivalence is itself a gate: corpus
designs are built once and run on two executors with their stdout, VCD bytes, simulation
time, finish reason and exit class compared. See
[09-testing-and-verification.md](09-testing-and-verification.md) for the gate inventory,
[21-tier3-native-backend.md](21-tier3-native-backend.md) for the tier-3 storage and its
three refusal layers, and
[study/02](../study/02-v1-native-coverage.md) for the coverage argument.

The simulation itself is single-threaded. `--threads` moves waveform **file writes** onto
a writer thread behind an order-preserving bounded queue; the byte stream is the one the
single-threaded path would have produced, so output is byte-identical for every thread
count and only wall-clock changes.

---

## 19. Related documents

- [04-architecture.md](04-architecture.md) — the pipeline and where the executor seam sits.
- [08-timescale-and-timing.md](08-timescale-and-timing.md) — delay conversion, precision, time formatting.
- [07-vcd-format.md](07-vcd-format.md) — the waveform writer the funnel feeds.
- [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md) and [15-error-code-reference.md](15-error-code-reference.md) — the runtime diagnostic surface.
- [14-staged-artifacts.md](14-staged-artifacts.md) and [17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md) — the frozen IR and the artifact gate.
- [21-tier3-native-backend.md](21-tier3-native-backend.md) — the default backend in detail.
- [../manual/006_limitations.md](../manual/006_limitations.md) — the user-facing statement of the same limits.
