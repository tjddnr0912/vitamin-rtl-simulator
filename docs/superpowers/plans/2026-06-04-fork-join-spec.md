# fork / join / join_any / join_none — Concurrent Implementation Spec

**Status:** implementation-ready. SPEC ahead of code, single source of truth.
**Scope:** `elaborate` (emit a real `Terminator::Fork`) + `sim-engine` (concurrent
child scheduling + a join barrier). **`sim-ir` is FROZEN and is NOT touched** —
`Terminator::Fork`, `JoinState`, `WakeCond`, `ProcFlags`, `SuspendState` are used
verbatim. **No golden-root re-hash. No new diag code (one MUST-mint `ElabUnsupported`
reuse for nested fork; see §5).**

**MVP boundary (hard, enforced — not "warn and proceed"):** exactly **one fork level**.
A `fork` whose child body itself contains a `fork` is a **hard elaborate ERROR** (no
grandchildren are ever spawned). This is the single scope cut that makes child identity
**flat**: every child's `tie` is a one-shot composite of `(top-level-proc-tie, child-idx)`
that can never overflow or alias because the parent tie is always a small dense top-level
index. Nesting is DEFERRED (§6.2) behind a clean, total error — not a silent corruption.

Ground truth verified against the live tree on 2026-06-04:
- `crates/sim-ir/src/lib.rs` — `Terminator::Fork {children, join, resume_bb}` (l.301), `JoinState` (l.74), `ProcFlags(pub u8)` (l.32), `WakeCond::Join {join_ref}` (l.52), `Process {sensitivity, body, entry, suspend}` (l.399), `SimIr.processes: Vec<Process>` (l.439).
- `crates/elaborate/src/lib.rs` — degrade-to-sequential `Stmt::Fork` arm (l.2757), `ProcessBuilder` (l.2395) with `new_block`/`start_block`/`end_block_with`/`goto`/`finish` + `BlockId::raw` (l.2371), the `lower_case` allocate-then-fill template (l.2785).
- `crates/sim-engine/src/sched.rs` — `Ready {tie, proc, block}` (l.18), `Scheduler` (l.45) owns `cur`/`wheel`/`waiters`/`net_to_edge`, `arm_processes` (l.122), `arm_sensitivity` (l.143), `run` loop (l.171), `propagate_changes` (l.389), `schedule_resume` (l.517), `suspend_on` (l.547), `rearm` (l.573), `push_sorted` (l.587).
- `crates/sim-engine/src/exec.rs` — `run_process(sched, pi, bb)` (l.24), Delay (l.70), Wait (l.81), Return (l.98), dead Fork/Call arms (l.104).
- `crates/hdl-ast/src/lib.rs` — `Stmt::Fork {label, decls, stmts, join, span}` (l.360), `JoinKind {Join, JoinAny, JoinNone}` (l.423).
- Parser already produces `Stmt::Fork` with correct `JoinKind` (`hdl-parser/src/lib.rs:2400,2457`), so e2e tests through `build()` exercise this end to end.

---

## 0. The central problem and the chosen solution (one paragraph)

The engine is **stateless except a basic-block PC**: all data lives in the global
net table, and a process's entire per-resume state is `Ready {tie, proc, block}`
(`sched.rs:18`) where `proc == declaration index == tie`. This single overloaded
`proc` axis is the *only* identity, and it is the same value used to (a) key the
wheel/waiters/edge-map, (b) break ties for deterministic ordering, and (c) look up
the process body + sensitivity (`ir.processes[proc]`). Two concurrent fork children
under one `proc` would produce **bit-identical `Ready` records** — indistinguishable
in every keyed table, lost/double-counted in the consumed-on-fire `waiters.retain`.
**Solution (the activity-id indirection, exactly the scheme the frozen
`SuspendState`/`JoinState`/`WakeCond::Join` closure was reserved for):** introduce
an engine-owned `activities: Vec<Activity>` arena. `Ready.proc` becomes a **runtime
activity id**, NOT a declaration index. Activities `0..ir.processes.len()` are the
pre-seeded top-level processes (1:1 with declarations); `fork` **appends** new
activities (ids ≥ `ir.processes.len()`), which the `u32`-typed, value-stored
wheel/waiters/edge-map already accept untouched. The only two sites that dereference
`ir.processes[proc]` (`run_process` body lookup, `rearm` sensitivity lookup) are
re-pointed through `activities[proc].template`. Child private state (PC, join role)
lives in the `Activity`. This fixes the identity collision with **no IR change** and
churn confined to ~6 call sites.

---

## 1. ELABORATE — lower `Stmt::Fork` to a real `Terminator::Fork`

### 1.1 What the three BB-id fields index (confirmed)

`Terminator::Fork {children: Vec<u32>, join: u32, resume_bb: u32}` — **all three are
process-local `BlockId`s into THIS process's `body: Vec<BasicBlock>`**, emitted via
`BlockId::raw()`, exactly like `Branch.then_bb` / `Delay.resume` / `Goto.target`
(`elaborate/src/lib.rs:2371` documents `BlockId` = process-local index, NOT the
top-level `SimIr.blocks` arena, NOT a `ProcId`). Confirmed against every other
terminator in the same builder.

- `children[i]` = entry BB of child `i`'s CFG chain (declaration order `i`).
- `join` = the join convergence BB; every child's tail `Goto`s into it.
- `resume_bb` = the parent's continuation BB (the statement after the join keyword).

### 1.2 The join *mode* is NOT in the terminator — it rides on `JoinState.flags`

The frozen `Fork` terminator has **no mode field** and there is **no `JoinState::All/Any/None`
enum** — `JoinState` is a struct `{parent, children, detached, flags: ProcFlags(u8)}`.
The task's premise "map join_any/join_none to JoinState variants" is corrected here:
the elaborator's terminator emission is **mode-agnostic structure** (CFG is identical for
all three modes), and the mode is threaded out-of-band via the `ForkModeTable` side table
(§1.5), NOT via the IR. The engine populates the runtime role bits below at
Fork-execution time (the elaborator does not populate runtime `JoinState`):

```
ProcFlags bit layout (engine-runtime, written when a Fork executes):
  bit0  IS_FORK_PARENT   — this activity is blocked on a join barrier
  bit1  IS_FORK_CHILD    — this activity is a spawned child (has a parent join_ref)
  bit2  MODE_ALL         — join (wait for all children)   ─┐ mutually exclusive
  bit3  MODE_ANY         — join_any (wait for first child) ─┤ pair; both 0 ⇒
  // join_none never blocks the parent, so it sets neither MODE_ALL nor MODE_ANY
  // and never sets IS_FORK_PARENT — the parent simply falls through to resume_bb.
```

### 1.3 The lowering recipe (mirrors `lower_case`, INV-1/INV-2 honored)

Replace the `Stmt::Fork` arm at `elaborate/src/lib.rs:2757`. Capture `stmts`, `join`
**and** `span` (the `..` currently drops them). The CFG shape is **identical for all
three modes** — the mode is NOT encoded in topology; it travels in the side table (§1.5):

- **`join` (JoinKind::Join, All):** `join_bb` seals with `Goto{resume_bb}`. The engine,
  on each child reaching `join_bb`, decrements; the barrier fires when count hits 0.
- **`join_any` / `join_none`:** **structurally identical CFG** (`join_bb` →
  `Goto{resume_bb}`, every child tail `Goto{join_bb}`). The mode is recovered by the
  engine from `fork_modes[&(template, join_bb)]` (§1.5), which it consults when it
  executes the `Fork` terminator to decide whether/how the parent blocks.

> **Decision (scoped to keep IR frozen):** the frozen `Fork` carries no mode field, so
> the engine cannot recover the mode from the terminator or CFG alone. The mode is
> carried **out-of-band via a parallel, non-IR `ForkModeTable` built during elaborate
> and threaded into the run through `SimOpts` (and a non-golden `.velab` trailer for the
> staged CLI), NOT serialized into `SimIr`** — see §1.5/§1.6. This keeps `SimIr`'s golden
> root byte-identical while giving the engine the mode. The lookup is **total-or-fatal**
> (asserted present, never defaulted — §2.4). The alternative of encoding mode in CFG
> topology (a sentinel extra child) is brittle and rejected.

#### The code (drop-in replacement for lines 2757–2765)

`cur_proc` / nested-fork enforcement (load-bearing — see §1.5 and §5):
- **`self.cur_proc` is set to `self.processes.len() as u32` at the TOP of
  `lower_proc_block`** (before any `lower_stmt`), which is the slot this process WILL
  occupy when the caller pushes it. `lower_proc_block` is non-reentrant — it fully
  builds one `Process` and returns it BEFORE the caller does `self.processes.push(proc)`
  — so `self.processes.len()` is stable across the entire body lowering. There are two
  push sites (l.506 normal, l.2318 generate); **both must set `cur_proc` at the matching
  `lower_proc_block` entry and `debug_assert!(self.processes.len() as u32 == cur_proc)`
  immediately before pushing.** Any `record_fork_mode` recorded during the body is keyed
  by exactly this `cur_proc`, guaranteeing the engine's `(template, join_bb)` lookup hits.
- **`self.in_fork: bool`** guards nesting. It is `false` at the top of `lower_proc_block`,
  set `true` while lowering child bodies, and restored on exit. A `Stmt::Fork` seen while
  `in_fork == true` is the nested case → **hard error, return early, emit no children**.

```rust
ast::Stmt::Fork { stmts, join, decls, span } => {
    // ── HARD MVP BOUNDARY: no nested fork (§6.2). A fork inside a fork child is a
    //    fatal elaborate error — NOT "warn and proceed". This keeps child identity
    //    flat (one shift in compose_child_tie) and forbids tie aliasing/overflow.
    if self.in_fork {
        self.error_unsupported(span, "nested fork is unsupported in v1 \
            (a fork child may not itself contain a fork)"); // reuses ElabUnsupported
        // Emit a well-formed but inert block so INV-1/INV-2 still hold: seal the
        // cursor straight to the continuation. No children, no barrier, no mode entry.
        let cont = b.new_block();
        b.goto(cont);
        b.start_block(cont);
        return;
    }

    // fork-local decls share the enclosing scope in v1 (like begin-block decls);
    // WARN-ignore them, matching the Stmt::Block decl handling.
    if !decls.is_empty() {
        self.warn("fork-local decls ignored (v1 shared-scope); declared in enclosing scope");
    }

    // INV-2: allocate EVERY named block BEFORE building the Fork terminator.
    // Allocation order is deterministic (3-OS golden stability): join, resume,
    // then each child entry in source/declaration order.
    let join_bb = b.new_block();
    let resume_bb = b.new_block();
    let child_entries: Vec<BlockId> =
        (0..stmts.len()).map(|_| b.new_block()).collect();

    // Record the join MODE for this fork into the parallel side table (NOT IR).
    // Keyed by (cur_proc, join_bb) — globally unique because each process body is a
    // distinct arena and join_bb is unique in it. cur_proc == this process's eventual
    // ProcId (set at lower_proc_block entry; see §1.5). The engine's lookup is
    // TOTAL-OR-FATAL (§2.4): a missing entry is an internal error, never defaulted.
    self.record_fork_mode(join, /*join_bb=*/ join_bb.raw());

    // INV-1: seal the parent block with Fork — this CLOSES the cursor.
    b.end_block_with(ir::Terminator::Fork {
        children: child_entries.iter().map(|e| e.raw()).collect(),
        join: join_bb.raw(),
        resume_bb: resume_bb.raw(),
    });

    // Lower each child chain. `lower_stmt` may split blocks (delays/waits/ifs);
    // it returns with the cursor open at the child's single continuation point.
    // `goto(join_bb)` seals that tail so the child's LAST block hands control to
    // the join. Empty `stmts` ⇒ this loop is skipped (valid: Fork{children:[]}).
    // `in_fork` is set so any Fork lowered INSIDE st hits the hard error above.
    let prev_in_fork = self.in_fork;
    self.in_fork = true;
    for (child_entry, st) in child_entries.iter().zip(stmts.iter()) {
        b.start_block(*child_entry);
        self.lower_stmt(b, st);
        b.goto(join_bb);
    }
    self.in_fork = prev_in_fork;

    // Seal the join block: join_bb → resume_bb. IMPORTANT: this Goto is a
    // NEVER-EXECUTED sentinel. The engine intercepts a child the instant its next
    // bb is computed to equal join_bb (§2.5, centralized loop-top check) and routes
    // it to on_child_complete + Step::Done BEFORE the join block's body/terminator
    // is ever fetched. The parent is resumed DIRECTLY at resume_bb by the barrier,
    // never via this Goto. So NO activity (child or parent) ever executes join_bb's
    // Goto{resume_bb}. The block exists only to (a) keep the CFG well-formed/sealed
    // (INV-2) and (b) give join_bb a concrete, unique BlockId used as the barrier's
    // completion sentinel. run_process MUST debug_assert it never *fetches* a block
    // whose id is any live barrier's join_bb (§2.5).
    b.start_block(join_bb);
    b.goto(resume_bb);

    // Open resume_bb as the single continuation. Post-condition for the caller:
    // exactly one open block, at the parent's continuation point.
    b.start_block(resume_bb);
}
```

### 1.4 CFG-sealing correctness (INV-1 / INV-2)

- **INV-2 (no dangling):** `join_bb`, `resume_bb`, and **all** `child_entries` are
  `new_block()`-allocated before any are named in the `Fork` terminator. Every
  allocated block is reached + sealed: each child entry is `start_block`-ed and its
  tail `goto(join_bb)`-sealed; `join_bb` is sealed `goto(resume_bb)`; `resume_bb` is
  left open for the next stmt (or `finish` seals it `Return`). No block keeps only
  its provisional `Return`.
- **INV-1 (one open block):** `end_block_with(Fork)` closes the cursor. Each child
  does exactly `start_block → lower_stmt → goto`, never two open blocks at once. We
  end with `start_block(resume_bb)` → cursor open at exactly one continuation. The
  caller's post-condition holds; `b.finish()` (l.2453) seals the trailing block.
- **Empty children edge case (`fork join`):** `stmts.len()==0` ⇒ `child_entries`
  empty, the `for` loop body never runs, `Fork{children:[],join,resume_bb}` is
  emitted, `join_bb → resume_bb`, `resume_bb` opened. The engine's barrier with 0
  children fires immediately (count 0 / ALL) → parent resumes at `resume_bb` same
  instant. Sound.

### 1.5 / 1.6 Mode side table (parallel, non-IR) — SOURCE-COMPATIBLE threading

Because `SimIr` is frozen, the join mode travels out-of-band. The table:

```rust
// crates/elaborate/src/lib.rs — engine-facing, NOT part of SimIr (no hash impact).
/// Join-mode side table: (template ProcId, join_bb) → JoinMode. Deterministic
/// BTreeMap so 3-OS byte-stable when serialized; never enters the golden root.
pub type ForkModeTable = std::collections::BTreeMap<(u32, u32), JoinMode>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JoinMode { All, Any, None }

// New Elaborator fields: `fork_modes: ForkModeTable`, `cur_proc: u32` (the ProcId of
// the process currently being lowered, set at lower_proc_block entry — §1.3), and
// `in_fork: bool` (nesting guard — §1.3). `record_fork_mode` inserts:
fn record_fork_mode(&mut self, join: ast::JoinKind, join_bb: u32) {
    let mode = match join {
        ast::JoinKind::Join     => JoinMode::All,
        ast::JoinKind::JoinAny  => JoinMode::Any,
        ast::JoinKind::JoinNone => JoinMode::None,
    };
    self.fork_modes.insert((self.cur_proc, join_bb), mode);
}
```

**Signature stability — DO NOT change `elaborate::elaborate`.** The reviewer-confirmed
blast radius (~25 callers in `elaborate/src/tests.rs`, the determinism pair, two CLI
sites, `end_to_end.rs::build`) makes a return-shape change a needless compile-break
sweep. Instead:

```rust
// UNCHANGED — every existing caller keeps compiling verbatim.
pub fn elaborate(unit: &Unit, sink: &dyn LogSink) -> Option<SimIr> {
    let (ir, _modes) = elaborate_with_modes(unit, sink);
    ir
}

// NEW — the simulate/CLI path calls THIS to also obtain the mode table.
pub fn elaborate_with_modes(unit: &Unit, sink: &dyn LogSink)
    -> (Option<SimIr>, ForkModeTable) { /* the real body */ }
```

`ForkModeTable` reaches the `Scheduler` **only via `SimOpts`** — not via any `build()`
or `simulate` return-shape change:

```rust
// crates/sim-engine/src/lib.rs
pub struct SimOpts {
    // ... existing fields ...
    /// Join-mode side table from elaborate_with_modes. EMPTY when the design has no
    /// forks (every existing test passes SimOpts::default() unchanged → empty table).
    pub fork_modes: ForkModeTable,   // #[derive(Default)] gives empty BTreeMap
}
```

So **`SimOpts::default()` keeps an empty table**, `simulate(ir, sink, opts)` signature
is unchanged, and **every existing `end_to_end.rs::build()` / `simulate_capture` caller
compiles untouched.** Fork tests construct `SimOpts { fork_modes, ..Default::default() }`
(see §4's `build_fork` helper). `Scheduler::new` reads `opts.fork_modes`.

> **Total-or-fatal contract (no silent default).** `Scheduler::new` (or `arm_processes`)
> MUST assert that **every `Terminator::Fork` in every process body has a matching
> `(template, join)` entry** in `fork_modes`. On a miss it aborts the run with an
> internal-error diag (`B-INTERNAL` / panic with MsgCode) — it **never** defaults to a
> mode. This converts any keying mismatch (wrong `cur_proc`, lost sidecar, stale table)
> into a loud failure at t0, not a silent join_any/join_none → blocking-join miscompile.
> See §2.4 `fork_mode`.

#### 1.6 Staged CLI (`vcmp` → `velab` → `vrun`) — REQUIRED non-golden sidecar

`elaborate` runs in `velab`; `simulate` runs in a SEPARATE `vrun` process. The in-memory
`ForkModeTable` is LOST across that boundary unless persisted. **This is a REQUIRED
deliverable, not a parenthetical option** — without it, staged-CLI forks silently
miscompile (well: now they ABORT loudly thanks to the total-or-fatal gate, but they
still don't *run*). Persistence:

- `velab` serializes `ForkModeTable` (postcard, the workspace's single encoder) into the
  `.velab` artifact as a **NON-GOLDEN trailer** appended OUTSIDE the `schema_hash`'d
  `SimIr` region. The golden gate hashes the `SimIr` body ONLY, so the golden root stays
  byte-identical; the trailer is a separate length-prefixed section the staleness gate
  ignores for hashing but `vrun` deserializes.
- `vrun` deserializes the trailer (empty/absent ⇒ empty table, fine for fork-free
  designs) and threads it into `SimOpts.fork_modes`.
- A `.velab` produced by an OLD `velab` (no trailer) loaded by a NEW `vrun`: absent
  trailer ⇒ empty table. If that `.velab` contains forks, the total-or-fatal gate aborts
  loudly — never a silent wrong answer.

**Required test (vrun path):** one staged round-trip — `velab` a fork module to `.velab`,
`vrun` it, assert the same output as the in-process `build_fork` path. Proves the sidecar
survives the process boundary. (Listed in §4 as FORK 13.)

> The `(template_proc, join_bb)` key is globally unique: each process body is a
> private BB arena, and `join_bb` is unique within it. At Fork execution the engine
> knows the parent activity's `template` and the terminator's `join` field, so it
> recovers the mode by `fork_modes[&(template, join)]` — asserted present, never defaulted.

### 1.7 Child bodies reuse `lower_stmt` unchanged — confirmed

No change to `lower_stmt` internals. Each child is `self.lower_stmt(b, st)` against
the same builder. Delays (`Stmt::DelayCtrl`→`Terminator::Delay`, splits + leaves
cursor open), event/level waits (`Terminator::Wait`), blocking/NBA assigns
(straight-line), `begin…end` (`Stmt::Block` iterates in one chain), if/case/loops
(self-contained merge, cursor open) — all already satisfy the single invariant the
Fork lowering depends on: `lower_stmt` returns with the cursor open at one
continuation point, which we then `goto(join_bb)`.

---

## 2. ENGINE — child identity, concurrent scheduling, the join barrier

### 2.1 The `Activity` arena (the identity fix)

Add to `Scheduler` (`sched.rs:45`). `Ready.proc` is **redefined** to be an *activity
id*, not a declaration index. `tie` stays the deterministic ordering key but is now
derived per-activity (§2.6).

```rust
/// Per-activity private state. Top-level processes are pre-seeded 1:1 with
/// `ir.processes`; fork children are appended (id >= ir.processes.len()).
pub(crate) struct Activity {
    /// Index into `ir.processes` for the body/sensitivity TEMPLATE this activity
    /// runs. Multiple activities may share a template (a child shares its parent's
    /// process body — it runs a different BB sub-chain of the SAME `body` Vec).
    pub template: u32,
    /// Deterministic ordering key. Top-level: == template. Children: a composite
    /// of the parent tie and the declaration-order child index (§2.6).
    pub tie: u32,
    /// If this activity is a fork child, the barrier id it reports completion to.
    /// `None` for top-level processes and for join_none orphans after detach.
    pub join_ref: Option<u32>,
    /// Role bits (mirrors the would-be ProcFlags semantics; engine-private).
    pub is_child: bool,
    /// Completion-report guard: set true the FIRST time this child reaches its
    /// barrier's join_bb. A second report for the same child is an internal-error
    /// (double-decrement) — see §2.5 on_child_complete. `false` for top-level.
    pub reported: bool,
}

/// One live fork's join barrier.
pub(crate) struct JoinBarrier {
    /// Activity id of the parent that is (or will be) blocked here.
    pub parent: u32,
    /// The join convergence BB (Fork.join field), in the parent's template body.
    /// Used as the child-completion sentinel: when ANY child's next bb equals this,
    /// the child has completed (§2.5). NEVER fetched as a real block.
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
```

> **Monotonic-append invariant (identity stability under fork).** `activities` only
> ever GROWS: `arm_processes` seeds ids `0..nproc`, and `Fork` execution `push`es new
> ids `>= nproc`. Ids are **never reused or reindexed.** Therefore any `Ready{proc:aid}`
> stored *by value* in `wheel` / `waiters` / `net_to_edge` at any earlier instant stays
> valid after later forks append — the `Vec` grow never invalidates a prior index.
> Children are **never** edge/level-sensitive: a child has no `Sensitivity` of its own
> (it runs a sub-chain of its template's `body`), so a child id NEVER enters
> `net_to_edge` via `arm_sensitivity` — only `suspend_on` (consumed waiter) and
> `schedule_resume` (wheel) carry child ids, both identity-agnostic. `net_to_edge` is
> seeded at t0 with top-level activity ids only, and those remain stable forever. This
> invariant is asserted by FORK 14 (§4).

New `Scheduler` fields:

```rust
activities: Vec<Activity>,         // index == Ready.proc (activity id)
barriers: Vec<JoinBarrier>,        // index == join_ref; append-only, never reused
fork_modes: ForkModeTable,         // from elaborate, keyed (template, join_bb)
```

### 2.2 Seeding the activity arena at t0

`arm_processes` (sched.rs:122) seeds activities 1:1 with declarations, then arms as
today but using the activity id everywhere `pi` was used:

```rust
pub fn arm_processes(&mut self) {
    // Pre-seed top-level activities 1:1 with process declarations.
    self.activities = (0..self.st.ir.processes.len() as u32)
        .map(|pi| Activity { template: pi, tie: pi, join_ref: None,
                             is_child: false, reported: false })
        .collect();

    // TOTAL-OR-FATAL mode gate (§1.5): every Fork in every body must have a mode.
    // A miss here means a keying mismatch / lost sidecar — abort loudly, never run
    // with a fabricated default.
    for (proc_id, p) in self.st.ir.processes.iter().enumerate() {
        for blk in &p.body {
            if let Terminator::Fork { join, .. } = &blk.term {
                assert!(self.fork_modes.contains_key(&(proc_id as u32, *join)),
                    "internal error: Fork in process {proc_id} join_bb {join} has no \
                     ForkModeTable entry (lost/stale mode sidecar?)");
            }
        }
    }

    for aid in 0..self.activities.len() as u32 {
        let tmpl = self.activities[aid as usize].template as usize;
        let p = &self.st.ir.processes[tmpl];
        let entry = p.entry;
        let ready = Ready { tie: self.activities[aid as usize].tie, proc: aid, block: entry };
        match p.sensitivity.kind {
            SensKind::Initial | SensKind::Comb | SensKind::Latch =>
                push_sorted(&mut self.cur.active, ready),
            SensKind::Edge | SensKind::Level => self.arm_sensitivity(aid),
        }
    }
}
```

### 2.3 Re-point the two declaration dereferences through the template

This is the entire OOB-safety fix. `run_process` and `rearm` resolve the template:

**`exec.rs:24` — `run_process` signature unchanged (`pi` is now an activity id):**
```rust
pub(crate) fn run_process(sched: &mut Scheduler, pi: u32, mut bb: u32) -> Step {
    let mut guard: u64 = 0;
    loop {
        let tmpl = sched.activities[pi as usize].template as usize; // ← indirection
        let (stmt_ids, term) = {
            let body = &sched.st.ir.processes[tmpl].body;            // ← was [pi]
            let block = &body[bb as usize];
            (block.stmts.clone(), block.term.clone())
        };
        // ... statements unchanged ...
```

**`sched.rs:573` — `rearm` resolves template; only TOP-LEVEL activities re-arm:**
```rust
pub(crate) fn rearm(&mut self, proc: u32) {
    // Fork children NEVER re-arm: a child's Return-to-join is a one-shot
    // completion, routed by run_process's Return arm to on_child_complete.
    if self.activities[proc as usize].is_child {
        return;
    }
    let tmpl = self.activities[proc as usize].template as usize;     // ← indirection
    let kind = self.st.ir.processes[tmpl].sensitivity.kind;
    match kind {
        SensKind::Edge | SensKind::Initial => {}
        SensKind::Comb | SensKind::Latch | SensKind::Level => self.arm_sensitivity(proc),
    }
}
```

`arm_sensitivity(aid)` likewise reads `ir.processes[activities[aid].template]` and
builds `Ready {tie: activities[aid].tie, proc: aid, block: entry}`.

**Every other proc-indexed structure already tolerates activity ids:** `wheel`,
`waiters`, `net_to_edge` store `Ready` *by value* and only compare `u32`s — they do
**not** index `ir.processes`. `schedule_resume`/`suspend_on`/`push_sorted` are
identity-agnostic. The **only** two `ir.processes[proc]` sites were the two patched
above. Confirmed exhaustive by the research grep ("only `.body`, `.entry`,
`.sensitivity.kind` are touched").

### 2.4 The `Fork` terminator arm (replaces the dead fall-through at exec.rs:104)

```rust
Terminator::Fork { children, join, resume_bb } => {
    let parent_aid = pi;
    let parent_tmpl = sched.activities[pi as usize].template;
    let mode = sched.fork_mode(parent_tmpl, join);    // total-or-fatal; never defaults

    // Register the barrier (append-only id space; never reused → no ABA).
    let join_ref = sched.barriers.len() as u32;
    sched.barriers.push(JoinBarrier {
        parent: parent_aid,
        join_bb: join,                     // child-completion sentinel (§2.5)
        resume_bb,
        mode,
        outstanding: children.len() as u32,
        fired: false,
    });

    // Spawn each child as a NEW activity, sharing the parent's TEMPLATE body but
    // entering at its own child-entry BB. Deterministic: declaration order = the
    // order of `children`, and each child's tie composes parent.tie + child idx.
    // NOTE (§6.2): nested fork is an elaborate ERROR, so `parent_tmpl` here is always
    // a TOP-LEVEL process and `parent.tie` is always a small dense top-level index —
    // compose_child_tie can never overflow or alias (one shift only, never chained).
    for (child_idx, &child_entry) in children.iter().enumerate() {
        let child_tie = compose_child_tie(sched.activities[pi as usize].tie, child_idx as u32);
        let child_aid = sched.activities.len() as u32;
        sched.activities.push(Activity {
            template: parent_tmpl,
            tie: child_tie,
            join_ref: Some(join_ref),
            is_child: true,
            reported: false,
        });
        // Make the child runnable NOW (same instant, Active region), in
        // declaration order (push_sorted by the composed tie keeps it stable).
        push_sorted(&mut sched.cur.active, Ready { tie: child_tie, proc: child_aid, block: child_entry });
    }

    match mode {
        JoinMode::None => {
            // join_none: parent does NOT block. Detach all children, continue NOW.
            sched.barriers[join_ref as usize].fired = true; // never resumes via barrier
            bb = resume_bb;                                  // fall through this instant
            // (children already enqueued; they run concurrently with the continuation.)
        }
        JoinMode::All | JoinMode::Any => {
            if children.is_empty() {
                // fork join / fork join_any with zero children: resume immediately.
                sched.barriers[join_ref as usize].fired = true;
                bb = resume_bb;
            } else {
                // Parent blocks on the join. SUSPEND this activation; the barrier
                // will re-enqueue the parent at resume_bb when its condition fires.
                return Step::Suspended;
            }
        }
    }
}
```

`fork_mode` helper — **TOTAL-OR-FATAL, never defaults to a blocking join:**
```rust
pub(crate) fn fork_mode(&self, template: u32, join_bb: u32) -> JoinMode {
    // A miss here is impossible after the arm_processes gate (§2.2), but we re-assert
    // at the point of use. NEVER default to All: a fabricated blocking-join would
    // silently turn a lost-side-channel join_none/join_any into a deadlock with no
    // diagnostic. Panic (internal error) instead — loud and immediate.
    *self.fork_modes.get(&(template, join_bb)).unwrap_or_else(|| panic!(
        "internal error: no ForkModeTable entry for (template={template}, \
         join_bb={join_bb}) — mode sidecar lost/stale?"))
}
```

### 2.5 The child-completion hook (parent re-entry) — terminator-agnostic, centralized

A fork child must **not** execute the parent's continuation — when its execution would
reach `join_bb`, it has completed: it reports to the barrier and dies. The detection
must be **robust against the child's last terminator being anything** — `Goto{join_bb}`
(the common sealed tail), but also a `Branch` whose then/else target equals `join_bb`, a
`Delay.resume` into `join_bb`, or a `Wait.resume` into `join_bb`. Detecting only in the
`Goto` arm is structurally incomplete and fragile.

**Fix: centralize the check at the TOP of the `run_process` loop, AFTER `bb` is updated
by any terminator and BEFORE the next block is fetched.** Every terminator arm writes
the next `bb` (`Goto` sets `bb = target`; `Branch` sets `bb = then/else`; the loop
re-enters after a `Delay`/`Wait` resume with `bb = resume`). The single check covers all
paths, terminator-agnostic:

```rust
pub(crate) fn run_process(sched: &mut Scheduler, pi: u32, mut bb: u32) -> Step {
    let mut guard: u64 = 0;
    loop {
        // ── CENTRALIZED CHILD-COMPLETION INTERCEPT (covers Goto/Branch/Delay/Wait) ──
        // If this activity is a fork child and the NEXT bb to fetch is its barrier's
        // join_bb, the child has completed. Report + die BEFORE the join_bb block is
        // ever fetched (join_bb is a never-executed sentinel — §1.3). This MUST run
        // before the block fetch below, and a child Activity is set terminal in
        // on_child_complete so a second arrival is structurally impossible.
        {
            let act = &sched.activities[pi as usize];
            if act.is_child {
                if let Some(jr) = act.join_ref {
                    if bb == sched.barriers[jr as usize].join_bb {
                        sched.on_child_complete(jr, pi);
                        return Step::Done;   // child dead; rearm skips it (is_child)
                    }
                }
            }
            // Defense-in-depth: a NON-child (parent / top-level) must NEVER fetch a
            // live barrier's join_bb — that would mean the parent walked the sentinel.
            debug_assert!(
                act.is_child ||
                !sched.barriers.iter().any(|b| !b.fired && b.join_bb == bb
                    && sched.activities[b.parent as usize].template
                       == sched.activities[pi as usize].template),
                "parent/top-level activity fetched a live barrier join_bb sentinel");
        }

        let tmpl = sched.activities[pi as usize].template as usize;
        let (stmt_ids, term) = {
            let body = &sched.st.ir.processes[tmpl].body;
            let block = &body[bb as usize];
            (block.stmts.clone(), block.term.clone())
        };
        // ... statements + terminator arms unchanged; each arm sets `bb` or returns ...
        // The Goto arm is now plain: `Terminator::Goto { target } => { bb = target; }`
    }
}
```

The `Goto` arm no longer special-cases children — it is just `bb = target`. The
intercept at loop-top catches the child whether it arrives at `join_bb` via Goto,
Branch, or a resumed Delay/Wait. This is the minimal robust fix and is **load-bearing
and now explicitly specified.**

`on_child_complete` decrements and fires the barrier exactly once. It takes the child
activity id so it can mark the child terminal (`reported`) — a SECOND report for the
same child is an internal error (double-decrement), not a saturating no-op:

```rust
pub(crate) fn on_child_complete(&mut self, join_ref: u32, child_aid: u32) {
    // Per-child fire-once: a child may reach its join_bb at most once. A second
    // report would under-decrement `outstanding` and fire an All-barrier EARLY
    // (parent resumes while a sibling is still live). Enforce, don't mask.
    debug_assert!(!self.activities[child_aid as usize].reported,
        "internal error: child {child_aid} reported completion twice");
    self.activities[child_aid as usize].reported = true;

    let b = &mut self.barriers[join_ref as usize];
    debug_assert!(b.outstanding > 0,
        "internal error: barrier {join_ref} outstanding underflow");
    b.outstanding -= 1;
    let fire = match b.mode {
        JoinMode::All  => b.outstanding == 0,   // last child
        JoinMode::Any  => true,                 // first child (and any later — guarded)
        JoinMode::None => false,                // never (parent already continued)
    };
    if fire && !b.fired {
        b.fired = true;
        let parent = b.parent;
        let resume_bb = b.resume_bb;
        let tie = self.activities[parent as usize].tie;
        // Re-enqueue the parent at resume_bb THIS instant (Active region). Surplus
        // children stay live in `activities` + their own wheel/waiter entries and
        // run to completion; their later on_child_complete sees `fired==true` → no-op.
        push_sorted(&mut self.cur.active, Ready { tie, proc: parent, block: resume_bb });
    }
}
```

**Why a second report is now structurally impossible (not merely guarded):** the
loop-top intercept returns `Step::Done` the instant `bb == join_bb`, ending the
activity; it is never re-enqueued (no rearm for `is_child`). The `reported` flag +
`debug_assert!(outstanding > 0)` turn any hypothetical future regression (e.g. a builder
edge that produced two `join_bb`-targeting predecessors for one child, or an aliased
re-entry) into a **loud panic in debug**, not a silent early-fire. The guard enforces the
invariant rather than hiding its violation.

**`JoinMode::Any` surplus-children correctness:** the first completer sets
`fired=true` and resumes the parent. Every later sibling still calls
`on_child_complete`, computes `fire=true` (Any), but the `!b.fired` guard blocks the
second resume — the parent resumes **exactly once**. Surplus children keep running
(background), their writes land in time order, their final completion is a no-op on
the barrier. This is the IEEE `join_any` "remaining children keep running" semantics.

**`JoinMode::All`:** `outstanding` starts at `children.len()`; each completion
decrements; the **last** (count 0) fires once. `JoinMode::None`: `fired` is preset
true at Fork time, parent already continued, completions are no-ops.

### 2.6 Deterministic child tie (3-OS golden order)

`push_sorted` (sched.rs:587) orders by `tie`. Siblings must get **distinct,
deterministic** ties in declaration order, and must not collide with unrelated
activities. v1 uses a **stable composite** that preserves the parent's global
position and orders children after the parent by declaration index:

```rust
/// Child tie = (parent_tie+1) in the high 16 bits, child declaration index in the low.
/// `parent` here is ALWAYS a top-level process (nested fork is an elaborate ERROR,
/// §6.2), so parent_tie ∈ [0, nproc) is a small dense int and there is NEVER a chained
/// shift. The (parent_tie+1) offset makes children sort STRICTLY AFTER their parent for
/// ALL parent_tie including 0 (closing the parent==child tie-equality corner), while
/// preserving relative parent ordering and declaration order among siblings.
fn compose_child_tie(parent_tie: u32, child_idx: u32) -> u32 {
    ((parent_tie + 1) << 16) | (child_idx & 0xFFFF)
}
```

> **Tie-space note (now bounded by the no-nesting invariant):** the map is top-level
> tie `t` → child base `(t+1)<<16`. Because nesting is a hard elaborate error, this
> shift is applied **exactly once, never chained** — so the catastrophic overflow case
> (`65536<<16` wrapping to 0, aliasing top-level proc 0) is **impossible by
> construction**, not merely "rare". No-overflow precondition: `(nproc) << 16 < 2^32`,
> i.e. `nproc < 65535` (≤ 65534 top-level processes), with ≤ 65535 children per fork.
> Both bounds are far above any MVP testbench and are documented v1 limits, not silent
> hazards — and crucially they hold for the WORST case (max top-level proc, max child
> idx) since depth is capped at 1. The ordering contract `push_sorted` needs is *total
> + deterministic*; with distinct top-level ties and the +1 offset, no child ever
> aliases another child, its own parent, or any top-level tie. **Alternative
> considered:** a `(u32,u32)` / `u64` tie with a monotonic per-spawn secondary key
> (would make depth unbounded); rejected for v1 because nesting is forbidden, so the
> single-shift composite is sufficient, cheaper, and keeps `Ready` `Copy`-cheap. If a
> future v2 allows nesting, that change is the documented migration (§6.2).

### 2.7 How a child reaches `schedule_resume` / `suspend_on` / `rearm` without collision

A child is just an activity id in `Ready.proc`. When its body hits:
- **Delay (`#n`):** `run_process` calls `schedule_resume(pi=child_aid, resume, tick,
  inactive)` → builds `Ready {tie: <stale `tie=proc` in current code>...}`. **Fix
  `schedule_resume`/`suspend_on` to read the activity's tie**, not `proc`:
  ```rust
  pub(crate) fn schedule_resume(&mut self, proc: u32, block: u32, tick: u64, inactive: bool) {
      let tie = self.activities[proc as usize].tie;     // ← was `let tie = proc;`
      let ready = Ready { tie, proc, block };
      // ... wheel/cur push unchanged ...
  }
  pub(crate) fn suspend_on(&mut self, proc: u32, block: u32, cause: WaitCause) {
      let tie = self.activities[proc as usize].tie;     // ← was `tie: proc`
      self.waiters.push(Waiter { cause, ready: Ready { tie, proc, block } });
  }
  ```
  Now a child with `#5` and a sibling with `#3` land in **distinct wheel slots**
  (different absolute tick) with **distinct ties** (different `proc`/composite tie) —
  fully separable. Two siblings waking the same instant resume in composed-tie
  (declaration) order.
- **Wait (`@event` / `wait`):** identical — `suspend_on` pushes a `Waiter` whose
  `Ready` carries the child's distinct `proc` + composed `tie`. `propagate_changes`
  `waiters.retain` wakes by cause; the woken `Ready` re-enters `cur.active` via
  `push_sorted` keyed by the child's tie. Two siblings on the same event are now
  **distinguishable** (different `proc` field) — neither is lost nor double-counted,
  the exact collision the research flagged is dissolved.
- **Completion:** when a child's next `bb` equals its barrier's `join_bb` — via ANY
  terminator (Goto/Branch/Delay-resume/Wait-resume) — the **centralized loop-top
  intercept** (§2.5) routes it to `on_child_complete` and ends the activity with
  `Step::Done`, never through `rearm` (the `is_child` early-return in `rearm` guarantees
  a child is a true one-shot). Detection is terminator-agnostic, so a child whose last
  statement is an `if`/`case`/delay/wait that resumes INTO `join_bb` is caught.

This is the complete answer to "extend the key vs. parallel table": we **keep the
`Ready.proc` key shape** (no struct change → `Copy/Eq` preserved, wheel/waiters
untouched) and reinterpret `proc` as an activity id, with a **parallel `activities`
table** supplying body/sensitivity/tie. Both the encode (append on fork) and decode
(`activities[proc].template`) are shown; every proc-indexed structure is either
identity-agnostic (wheel/waiters/edge-map) or routed through the table (`run_process`,
`rearm`, `schedule_resume`, `suspend_on`, `arm_sensitivity`).

---

## 3. INTEGRATION with `run()`

### 3.1 Where the barrier is checked / parent re-entry

No change to the `run()` loop body itself (sched.rs:171). The barrier integrates
through the existing region machinery:
- **Fork execution** enqueues children into `cur.active` (same instant) and either
  suspends the parent (All/Any) or falls through to `resume_bb` (None) — all inside
  the current Active-drain `for r in batch` loop. The newly-enqueued children are
  picked up because `cur.active` is re-checked at the top of the inner loop (the
  batch was `mem::take`n, then `propagate_changes` + `continue` re-enters with the
  freshly pushed children present).
- **Child completion** (`on_child_complete`) `push_sorted`s the **parent** back into
  `cur.active` at `resume_bb`. The parent re-enters `run_process(self, parent_aid,
  resume_bb)` on the next Active drain — exactly the standard resume path used by
  Delay/Wait. The parent's `template` lookup gives the right body; `resume_bb` is a
  valid BB in that body (elaborate guaranteed it).
- **Determinism of same-instant child ordering:** all children are pushed in
  declaration order with monotonically-increasing composed ties (§2.6); `push_sorted`
  is a stable insert by tie, so child-0 runs before child-1 before child-2 at the
  launch instant and at any later instant where several wake together. Frozen
  convention, 3-OS byte-stable.
- **Mid-batch enqueue invariant (documented, matches FORK 5/7/8):** children spawned by
  a `Fork` during the current Active-drain `for r in batch` loop land in the FRESH
  `cur.active` (the running batch was already `mem::take`n). They are **NOT** processed
  within the current `for r in batch` — they drain on the NEXT inner-loop pass (after
  `propagate_changes` + `continue`), i.e. same simulation instant, next delta. So FORK 7
  yields `c0,c1,c2` (children drain next pass in tie order, each completing via the
  loop-top intercept; the last fires the barrier re-enqueuing the parent) then `parent`.
  This is the intended same-instant-but-next-delta ordering; `max_deltas` bounds it.

### 3.2 Termination — explicit about background children (Quiescent is NOT guaranteed)

- **A blocked join holds no live entries.** A parent suspended on an All/Any barrier
  holds **no** `cur.active`/wheel/waiter entry — it is parked solely by the barrier
  (`barriers[jr].parent`). So a *blocked parent by itself* contributes nothing to the
  delta count and cannot prevent Quiescent.
- **Background (join_none / surplus join_any) children with self-sustaining timing keep
  the wheel LIVE.** This is the honest, load-bearing caveat: a legal monitor pattern
  like `fork begin forever #1 x = ~x; end join_none` enqueues a wheel entry every tick
  forever. The wheel **never empties**, so the run does **NOT** reach `Quiescent` — it
  ends ONLY via `$finish` (in the parent or any process) or the `time_limit` /
  `max_deltas` caps. **Quiescent is not guaranteed in the presence of a self-timed
  background child.** For the target concurrent-monitor pattern this is expected: the
  parent's continuation is responsible for halting (typically `#k $finish`); a
  background monitor that loops forever is meant to be cut off by `$finish`/`time_limit`,
  not to self-terminate.
- **A genuinely quiescent design still ends via Quiescent.** If every child runs to a
  real completion (reaches its `join_bb`) and nothing else is pending, the wheel empties
  and `FinishReason::Quiescent` (sched.rs:247) ends `run()` as before. Children that
  complete are routed by §2.5; surplus children that complete are barrier no-ops.
- **A never-completing All-join blocks the parent forever — correct IEEE behavior.** If
  a child waits on an event that never comes, the parent stays blocked; if no other
  events remain, the sim still terminates via Quiescent (the blocked parent holds no
  entry). If a *background* self-timed child also exists, see the caveat above — then it
  is `time_limit`/`max_deltas`, not Quiescent, that bounds the run.
- **No infinite delta loop:** children that do zero-time work and complete the same
  instant resume the parent the same instant; the existing `max_deltas` guard in the
  inner loop bounds any pathological zero-time fork storm exactly as it bounds
  zero-delay `#0` loops today.
- **A barrier fires exactly once** (`fired` guard + per-child `reported` guard, §2.5);
  no orphaned/double-fired barrier is possible.

FORK 15 (§4) locks this: a `join_none` child `forever #1 ...` plus a parent `#k $finish`
asserts `FinishReason::Finish` at `k` — proving the background child neither blocks the
parent nor prevents `$finish`, and that termination comes from `$finish`, not Quiescent.

---

## 4. TESTS (16 + 1 determinism regression, exact assertions)

Add to `crates/sim-engine/tests/end_to_end.rs` (FORK 1–12, 14, 15), a nested-fork
ERROR test in `crates/elaborate/src/tests.rs` (FORK 16), and a staged-CLI round-trip in
the CLI test crate / `end_to_end.rs` (FORK 13). Every behavioral assertion is chosen to
**FAIL under the old sequential lowering** (noted per test). Output ordering uses the
declaration-order determinism rule. All use `$display` so the capture string is exact.

**Helper (keeps every existing `build()` caller source-compatible):** since `build()`
returns `SimIr` and `SimOpts::default()` carries an EMPTY `fork_modes`, fork tests use a
thin helper that elaborates WITH modes and threads them through `SimOpts`:

```rust
/// Elaborate src and return (ir, opts) where opts carries the fork mode table.
/// Existing non-fork tests keep using build()/SimOpts::default() unchanged.
fn build_fork(src: &str) -> (SimIr, SimOpts) {
    let unit = parse_unit(src);                      // same front-end as build()
    let sink = CollectingSink::default();            // or the existing test sink
    let (ir, fork_modes) = elaborate::elaborate_with_modes(&unit, &sink);
    let ir = ir.expect("elaborate produced SimIr");
    (ir, SimOpts { fork_modes, ..SimOpts::default() })
}
```

Fork tests below call `let (ir, opts) = build_fork(src); simulate_capture(&ir, opts);`
(shown explicitly in FORK 1; the rest follow the same two-line shape).

```rust
// ── FORK 1. concurrent delays interleave: b at 3, a at 5 (NOT a@5 then b@8) ──
#[test]
fn fork_join_concurrent_delays_interleave() {
    let src = "module m; reg a; reg b; \
               initial begin a=0; b=0; \
                 fork #5 a=1; #3 b=1; join \
                 $display(\"%0d %b %b\", $time, a, b); \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Quiescent);
    // join waits for ALL → parent prints at t=5 with both set. Under sequential
    // lowering a@5 then b@8 → print at t=8. The time token 5 FAILS the old path.
    assert_eq!(out, "5 1 1\n");
    assert_eq!(res.sim_time, 5);
}

// ── FORK 2. join waits for the LATER child (monitor each child) ──────────────
#[test]
fn fork_join_waits_for_all_children() {
    let src = "module m; reg a; reg b; \
               initial begin a=0; b=0; \
                 fork #3 begin b=1; $display(\"b@%0d\", $time); end \
                      #5 begin a=1; $display(\"a@%0d\", $time); end join \
                 $display(\"done@%0d\", $time); \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Quiescent);
    // Concurrent: b@3 first, a@5, then parent done@5. Sequential would give
    // b@3,a@8,done@8 (a serialized after b). "done@5" FAILS the old path.
    assert_eq!(out, "b@3\na@5\ndone@5\n");
}

// ── FORK 3. join_any unblocks at the FIRST completer, surplus runs on ────────
#[test]
fn fork_join_any_unblocks_at_first() {
    let src = "module m; reg slow; reg fast; \
               initial begin slow=0; fast=0; \
                 fork #5 slow=1; #3 fast=1; join_any \
                 $display(\"resume@%0d fast=%b slow=%b\", $time, fast, slow); \
                 #10 $display(\"late@%0d slow=%b\", $time, slow); \
                 $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // join_any resumes at t=3 (fast done), slow still 0 then. Background #5 sets
    // slow=1 at t=5, observed by the late print at t=13. Sequential lowering has
    // no join_any concept → "resume@3" FAILS the old path.
    assert_eq!(out, "resume@3 fast=1 slow=0\nlate@13 slow=1\n");
}

// ── FORK 4. join_none continues IMMEDIATELY (zero blocking) ──────────────────
#[test]
fn fork_join_none_continues_immediately() {
    let src = "module m; reg a; reg c; \
               initial begin a=0; c=0; \
                 fork #5 a=1; join_none \
                 c=9; $display(\"cont@%0d c=%0d a=%b\", $time, c, a); \
                 #6 $display(\"after@%0d a=%b\", $time, a); \
                 $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // join_none → c=9 runs at t=0 (no delay), a still 0. Background child sets
    // a=1 at t=5, observed at t=6. Sequential lowering executes #5 a=1 BEFORE
    // c=9 → "cont@5" / a=1. "cont@0 c=9 a=0" FAILS the old path.
    assert_eq!(out, "cont@0 c=9 a=0\nafter@6 a=1\n");
}

// ── FORK 5. two children write DIFFERENT nets, both visible after join ───────
#[test]
fn fork_join_two_children_different_nets() {
    let src = "module m; reg x; reg y; \
               initial begin x=0; y=0; \
                 fork x=1; y=1; join \
                 $display(\"%b %b\", x, y); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Both children zero-delay → complete at t=0; join releases parent at t=0.
    // Passes under sequential too, but locks the shared-scope visibility contract.
    assert_eq!(out, "1 1\n");
    assert_eq!(res.sim_time, 0);
}

// ── FORK 6. nested begin…end inside a fork child (multi-block child chain) ───
#[test]
fn fork_child_with_nested_begin() {
    let src = "module m; reg p; reg q; \
               initial begin p=0; q=0; \
                 fork \
                   begin #2 p=1; #2 p=0; end \
                   #3 q=1; \
                 join \
                 $display(\"%0d %b %b\", $time, p, q); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Child-0 chain: p=1@2, p=0@4 (multi-block, own delays). Child-1: q=1@3.
    // join waits for the later (p=0@4). Parent prints at t=4: p=0,q=1.
    // Sequential serializes the whole begin then #3 → t differs. "4 0 1" FAILS old.
    assert_eq!(out, "4 0 1\n");
}

// ── FORK 7. deterministic same-instant ordering: child-0 before child-1 ──────
#[test]
fn fork_same_instant_declaration_order() {
    let src = "module m; integer z; \
               initial begin z=0; \
                 fork $display(\"c0\"); $display(\"c1\"); $display(\"c2\"); join \
                 $display(\"parent\"); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // All zero-delay, same instant → declaration order c0,c1,c2, then parent.
    // The FROZEN convention (§2.6 composed tie). Locks 3-OS golden order.
    assert_eq!(out, "c0\nc1\nc2\nparent\n");
}

// ── FORK 8. same-net last-writer-in-declaration-order wins (documented race) ─
#[test]
fn fork_same_net_last_writer_wins() {
    let src = "module m; reg w; \
               initial begin w=0; \
                 fork w=0; w=1; join \
                 $display(\"%b\", w); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Declaration order: child-0 w=0 then child-1 w=1, both at t=0 → w==1.
    assert_eq!(out, "1\n");
}

// ── FORK 9. a child blocks on @event, parent join waits for it ───────────────
#[test]
fn fork_child_waits_on_event() {
    let src = "module m; reg clk; reg got; \
               initial begin clk=0; got=0; \
                 fork \
                   begin @(posedge clk) got=1; $display(\"woke@%0d\", $time); end \
                   #4 clk=1; \
                 join \
                 $display(\"join@%0d got=%b\", $time, got); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Child-0 suspends on posedge clk; child-1 drives clk=1 at t=4 → child-0 wakes
    // at t=4, got=1, then join releases parent at t=4. Exercises suspend_on with a
    // CHILD activity id (the collision the scheme fixes). "woke@4"+"join@4 got=1".
    assert_eq!(out, "woke@4\njoin@4 got=1\n");
}

// ── FORK 10. parent continuation after join SEES children's net effects ──────
#[test]
fn fork_parent_sees_children_effects() {
    let src = "module m; integer sum; reg d1; reg d2; \
               initial begin sum=0; d1=0; d2=0; \
                 fork #1 d1=1; #2 d2=1; join \
                 if (d1 && d2) sum=42; \
                 $display(\"%0d\", sum); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // After join (t=2) both d1,d2 are 1 (shared scope) → sum=42. Confirms parent
    // resumes AFTER all children and observes their writes. "42" FAILS any path
    // where join releases before all children (would print 0).
    assert_eq!(out, "42\n");
}

// ── FORK 11. empty fork…join resumes immediately (zero children) ─────────────
#[test]
fn fork_join_empty_resumes_immediately() {
    let src = "module m; reg r; \
               initial begin r=0; \
                 fork join \
                 r=1; $display(\"%0d %b\", $time, r); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Zero children → barrier (count 0, ALL) fires same instant → r=1 at t=0.
    assert_eq!(out, "0 1\n");
}

// ── FORK 12. join_any leaves the parent runnable while a slow child survives ──
#[test]
fn fork_join_any_surplus_child_survives_to_finish() {
    let src = "module m; reg first; reg second; \
               initial begin first=0; second=0; \
                 fork #2 first=1; #7 second=1; join_any \
                 $display(\"unblock@%0d\", $time); \
                 #10 $display(\"final@%0d first=%b second=%b\", $time, first, second); \
                 $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // join_any unblocks at t=2 (first child). Surplus #7 child survives, sets
    // second=1 at t=7. Final print at t=12 sees both. Locks the "exactly once"
    // parent resume + surplus survival. "unblock@2" FAILS the old path.
    assert_eq!(out, "unblock@2\nfinal@12 first=1 second=1\n");
}

// ── FORK 14. monotonic-append identity stability: a top-level edge process keeps
//    firing AFTER a fork appends activities (its net_to_edge Ready{proc} by value
//    stays valid; child ids never enter net_to_edge). ──────────────────────────
#[test]
fn fork_does_not_disturb_toplevel_edge_process() {
    let src = "module m; reg clk; integer ticks; \
               always @(posedge clk) ticks = ticks + 1; \
               initial begin clk=0; ticks=0; \
                 fork #1 clk=1; #2 clk=0; #3 clk=1; join \
                 $display(\"ticks=%0d\", ticks); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // The always-block (a top-level EDGE activity armed at t0 into net_to_edge) must
    // still fire on each posedge driven by the fork CHILDREN (clk 0→1 at t=1, 0→1 at
    // t=3) AFTER the fork appended child activities. Two posedges → ticks=2. Proves
    // fork append never reindexes/invalidates the pre-stored Ready{proc} in net_to_edge.
    assert_eq!(out, "ticks=2\n");
}

// ── FORK 15. background join_none child loops forever; parent $finish halts. ──
//    Termination comes from $finish, NOT Quiescent (the wheel never empties). ───
#[test]
fn fork_join_none_background_child_does_not_block_finish() {
    let src = "module m; reg t; \
               initial begin t=0; \
                 fork begin forever #1 t = ~t; end join_none \
                 #5 $display(\"fin@%0d\", $time); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    // The forever-looping monitor child keeps the wheel live forever → Quiescent is
    // NEVER reached. The parent's `#5 $finish` is what halts the run. Asserting
    // Finish at t=5 proves: (a) join_none did not block the parent, (b) the background
    // child does not prevent $finish, (c) termination is via $finish, not Quiescent.
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(res.sim_time, 5);
    assert_eq!(out, "fin@5\n");
}
```

**FORK 13 — staged-CLI sidecar round-trip (in the CLI/integration test crate).** Compile
+ elaborate a fork module with `velab` to a `.velab` artifact, then `vrun` that artifact
in a SEPARATE invocation; assert the captured output equals the in-process `build_fork`
result for the same source (e.g. reuse FORK 4's join_none source). Proves the
non-golden `ForkModeTable` trailer (§1.6) survives the process boundary and that
`vrun`'s `fork_mode` lookups all hit (no total-or-fatal abort). Also assert the `.velab`
golden `SimIr` region hash is byte-identical to a fork-free baseline's encoding scheme
(the trailer does not perturb the golden root).

**FORK 16 — nested fork is a hard elaborate ERROR (in `elaborate/src/tests.rs`).**
```rust
#[test]
fn nested_fork_is_unsupported_error() {
    let src = "module m; reg a; \
               initial fork begin fork a=1; join end join endmodule";
    let unit = parse_unit(src);
    let sink = CollectingSink::default();
    let (ir, modes) = elaborate::elaborate_with_modes(&unit, &sink);
    // Inner fork (inside a fork child) → ElabUnsupported error. Elaborate still
    // produces a well-formed SimIr (INV-1/INV-2 hold via the inert continuation
    // block), but the diagnostic is emitted and NO grandchildren / inner barrier
    // exist: the inner fork recorded no mode entry.
    assert!(sink.has_error(MsgCode::ElabUnsupported)); // or the chosen reuse code
    // Only the OUTER fork's mode is recorded; the inner one is rejected.
    assert_eq!(modes.len(), 1);
    let _ = ir; // SimIr is produced but the design is rejected by the error above.
}
```

Plus a **determinism regression** (re-run identical, byte-equal output), mirroring the
existing l.289/696 pattern, over FORK 7's source — guarantees the composed-tie order
is reproducible run-to-run.

**Test count: 16 named tests (FORK 1–16) + 1 determinism regression = 17 additions.**
FORK 1–12 + 14 + 15 in `end_to_end.rs`; FORK 13 in the CLI/integration crate; FORK 16 in
`elaborate/src/tests.rs`.

---

## 5. Frozen-IR + diag confirmation

- **NO frozen-IR change.** `Terminator::Fork`, `JoinState`, `ProcFlags`,
  `WakeCond::Join`, `SuspendState`, `SimIr` are used **verbatim**. The activity arena,
  `JoinBarrier`, and `ForkModeTable` are **engine/elaborate-runtime structures that
  never enter `SimIr`** → `schema_hash::<SimIr>()` (the golden root) is byte-identical
  → no `format_version` bump, no `.velab` regeneration. The `body_refs.rs` /
  `SchemaHash` gates are untouched (no new sim-ir cross-type field).
- **One ERROR, reused (no new code minted); bijection untouched.** Nested fork (§6.2)
  is a **hard elaborate error** routed through the EXISTING `ElabUnsupported` code (the
  "construct not supported in this subset" channel) via `self.error_unsupported(span,
  …)`. Reusing an existing code means **no doc-15 MsgCode↔doc bijection promotion** is
  needed — `ElabUnsupported` is already documented and already in the bijection. No NEW
  `W-ELAB-FORK-*` / `E-ELAB-FORK-NESTED` code is minted in v1. (If a future v2 wants a
  fork-specific diagnostic for better UX, mint it then with the bijection promotion;
  **not now** — reuse keeps the gate green.)
- **Fork-local-decls WARN uses the GENERIC warn path (no MsgCode), not W-ELAB-WIDTH-TRUNC.**
  Correction to an earlier draft: `self.warn(msg)` is the generic, *non-coded* warning
  channel (free-text, no `MsgCode`). It does **not** reuse `W-ELAB-WIDTH-TRUNC`
  semantics (which is specifically the width-truncation "lowered with approximation"
  case) — reusing a width code for a decl-scope message would be a semantic mismatch.
  The decl warning carries no `MsgCode`, so it cannot perturb the bijection (which gates
  ERROR codes, not free-text warn strings). The old `"fork/join lowered as sequential"`
  warning is **removed** (concurrency is now modeled).
- **No other fork misuse in the v1 subset is fatal:** empty fork, any join mode, and a
  child containing `begin…end`/if/case/delays/waits all lower cleanly. Only nested fork
  errors. The bijection gate stays as is.

---

## 6. DEFERRED (explicit, with reasons)

1. **Per-thread automatic / local variables.** v1 children share the enclosing scope
   (all data is global nets). Private per-child storage needs a frame model
   (`SuspendState.locals`/`frame_arena` exist but are unused by the engine). Programs
   relying on per-child copies are out of scope.
2. **Nested `fork` inside a fork child — HARD ERROR in v1 (not "warn and proceed").**
   v1 supports **exactly one fork level.** A `fork` lowered while `in_fork == true`
   (i.e. inside a fork child body) is a **fatal `ElabUnsupported` error** (§1.3, §5):
   elaborate emits the diagnostic and emits an inert continuation block (INV-1/INV-2
   preserved) — **no grandchildren are ever spawned, no inner barrier or mode entry is
   created.** This is the deliberate scope cut that makes `compose_child_tie` a SINGLE,
   non-chained shift: because the parent of every spawned child is always a TOP-LEVEL
   process, `(parent_tie+1)<<16` can never overflow or alias (the old "emit
   structurally and warn" path would have produced grandchild tie `child_tie<<16 =
   0x1_0000_0000` wrapping to `0`, aliasing top-level proc 0 and breaking 3-OS golden
   ordering — that nondeterminism is now impossible by construction). *Migration to v2:*
   to allow nesting, replace the 16-bit composite tie with a `u64`/`(u32,u32)` tie
   carrying a monotonic per-spawn secondary key (depth-unbounded, never aliasing), then
   lift the `in_fork` error. FORK 16 locks the v1 error.
3. **`disable` / `disable fork`.** Explicit thread cancellation is deferred;
   `Stmt::Disable` stays a no-op (exec.rs:55). Background children always run to
   completion — correct for the target concurrent-monitor pattern.
4. **`join_any` background-child reaping nuances** beyond "let surplus run to
   completion": no `wait fork`, no requeue limits, no reaping. Only the three
   resumption conditions (All / Any / None) are modeled.
5. **Process handles** (`process::self`, suspend/resume/kill by handle): out of scope.

---

## 7. Touch list (exhaustive, no TODO)

**Signature-stability rule (no compile-break sweep):** `elaborate::elaborate` keeps its
`-> Option<SimIr>` signature (a one-line forwarder to the new fn). `build()` keeps
`-> SimIr`. `simulate`/`simulate_capture` keep their signatures. The mode table travels
**only** through the new `elaborate_with_modes` and `SimOpts.fork_modes` (empty default).
This leaves the ~25 `elaborate(...)` callers in `elaborate/src/tests.rs`, the determinism
pair, and every existing `build()`/`SimOpts::default()` caller **source-compatible —
untouched.**

| File | Change |
|---|---|
| `elaborate/src/lib.rs` | replace `Stmt::Fork` arm (l.2757) with §1.3 code (incl. `in_fork` nested-fork hard ERROR via `ElabUnsupported`); add `Elaborator` fields `fork_modes: ForkModeTable` + `cur_proc: u32` + `in_fork: bool` + `record_fork_mode`; **set `cur_proc = processes.len()` at BOTH `lower_proc_block` entries (l.~506, l.~2318 generate) and `debug_assert` the push-index matches**; add `pub enum JoinMode` + `pub type ForkModeTable` + `pub fn elaborate_with_modes(...) -> (Option<SimIr>, ForkModeTable)`; **keep `pub fn elaborate(...) -> Option<SimIr>` as a forwarder (signature UNCHANGED)** — no existing caller edits. |
| `sim-engine/src/sched.rs` | add `Activity` (with `reported`), `JoinBarrier` (with `join_bb`), `JoinMode`(re-export), fields `activities`/`barriers`/`fork_modes`; rewrite `arm_processes` (§2.2) incl. the **total-or-fatal mode gate**; `arm_sensitivity` (template + activity tie); patch `rearm` (§2.3, `is_child` early-return), `schedule_resume`+`suspend_on` (activity tie, §2.7); add `fork_mode` (total-or-fatal panic, NO default) + `on_child_complete(jr, child_aid)` (per-child `reported` guard + `debug_assert!(outstanding>0)`). |
| `sim-engine/src/exec.rs` | `run_process` template indirection (§2.3); **centralized child-completion intercept at loop-top** (terminator-agnostic, §2.5) + parent-never-fetches-join_bb `debug_assert`; real `Fork` arm (§2.4); `Goto` arm becomes plain `bb = target`. |
| `sim-engine/src/lib.rs` | add `SimOpts.fork_modes: ForkModeTable` (`#[derive(Default)]` ⇒ empty); `Scheduler::new` reads `opts.fork_modes`. `simulate`/`simulate_capture` signatures UNCHANGED. |
| `cli` (`vcmp`/`velab`/`vrun`, two sites l.247 + l.410) | **REQUIRED, not optional:** `velab` serializes `ForkModeTable` (postcard) into the `.velab` as a NON-GOLDEN trailer OUTSIDE the `schema_hash`'d `SimIr` region (golden root byte-identical); `vrun` deserializes it into `SimOpts.fork_modes`. One-shot `vita` threads it in-process. |
| `sim-engine/tests/end_to_end.rs` | add `build_fork` helper (elaborate_with_modes → `SimOpts{fork_modes,..}`); **`build()` UNCHANGED (still returns `SimIr`)**; add FORK 1–12, 14, 15 + the determinism regression. |
| `elaborate/src/tests.rs` | add FORK 16 (nested-fork `ElabUnsupported` error). |
| CLI / integration test crate | add FORK 13 (staged `velab`→`vrun` sidecar round-trip). |
```
