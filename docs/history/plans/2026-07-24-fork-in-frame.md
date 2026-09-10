# fork-in-frame Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Support `fork…join`/`join_any`/`join_none` spawned inside a suspendable task body — currently loud (E3009), the one round-18 report item (C1 part 2) kept correct-or-loud LOUD.

**Architecture:** The scheduler is single-threaded, so concurrent fork children + a parked parent take turns on the shared `frame_stack` — the existing owned-window model already isolates them for arms that don't touch the parent's locals (**Case A**). Only an arm that reads/writes the parent task's automatic locals while the parent is parked (**Case B**) needs a shared window, which routes to an interior-mutable `frame_windows` arena (the `dyn_heap`/`class_heap` pattern), refcounted for `join_none` lifetime. Staged 1→2→3, each independently testable and correct-or-loud.

**Tech Stack:** Rust (17-crate workspace), `crates/sim-engine` (event scheduler + frame executor), `crates/elaborate` (AST→sim-ir). CLI integration tests run `vita` on `.sv` source and assert on stdout/stderr. iverilog 13.0 is the differential oracle.

## Global Constraints

- **No `format_version` bump.** All changes are runtime engine types (`FrameRec`, `frame_stack`, new arena) + elaborate-transient sidecars (`FuncMeta`). SimIr golden root unchanged; no `.velab` regeneration. (Verify: `crates/vita-artifact/src/header.rs::CURRENT_FORMAT_VERSION` stays 23.)
- **correct-or-loud.** Any construct a stage has not reached stays LOUD (E3009), never silent-wrong.
- **File size ≤ 1000 lines.** If a file nears it, split (submodule + `use super::*` + `pub(crate)`).
- **MSRV 1.85**, edition 2021, `--locked` on every cargo command.
- **Determinism.** Arena handle alloc/free must be a pure function of execution order; internal ids only (VCD/stdout unchanged — the `free_activities`/`free_barriers` argument).
- **Verification gate (every task):** `cargo test --workspace --locked` (all green), `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`. Differential-verify each new supported case against iverilog 13.0 (`iverilog -g2012 t.sv && ./a.out`).
- **Commit trailer (every commit):** end the message with
  `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_011teDGcVPTUGkX5ByoG35HV`.
- **Branch:** all work on `fork-in-frame` (already created; `main` untouched). Do NOT commit `CLAUDE.md`, `LOOPROMPT.md`, `target/` (git-ignored dev-meta).

---

## Task 1: Stage 1 — Case A vertical slice (the report repro runs)

Bring `fork <self-contained arms> join` inside a suspendable task from E3009 to running, on the existing owned-window model. This is one vertical slice because the first green requires the elaborate gate AND the engine spawn together.

**Files:**
- Modify: `crates/elaborate/src/frames_classify.rs` (the fork gate at `frame_body_is_leaf_nonsuspending`, ~line 519-549; the lift condition at ~line 315)
- Modify: `crates/sim-engine/src/sched/propagate.rs` (`exec_fork`, ~line 707-828)
- Modify: `crates/sim-engine/src/sched/mod.rs` (`FrameRec`, ~line 53-71 — no field change in Stage 1; confirm `window: Option<Vec<Value>>`)
- Modify: `crates/sim-engine/src/exec/process.rs` (loop-top intercept ~line 51-70; `Terminator::Fork` arm ~line 320-335)
- Modify: `crates/sim-engine/src/state/task_frames.rs` (new `exit_arm_frame` helper near `exit_task_frame`, ~line 395)
- Test: `crates/cli/tests/fork_in_frame.rs` (new)
- Modify: `crates/cli/tests/suspendable_const_repeat.rs` (flip `fork_in_frame_stays_loud` → `fork_of_tasks_join_runs`)

**Interfaces:**
- Produces (elaborate): `fn fork_arms_self_contained(&self, entry: u32, lo: u32, hi: u32) -> ForkAdmit` where `enum ForkAdmit { CaseA, CaseB, Loud }` — walks the reachable blocks of a task body; for each `Terminator::Fork`, classifies its arms. Consumed by the lift condition and (Stage 2) the shared-window flag.
- Produces (engine): `fn exec_fork(&mut self, parent_aid, children, join, resume_bb) -> Option<u32>` gains an in-frame branch; `SimState::exit_arm_frame(&self, callee: u32)` pops the arm's live window off `frame_stack`.

- [ ] **Step 1: Write the failing test (report repro)**

Create `crates/cli/tests/fork_in_frame.rs`:

```rust
//! Stage 1 (Case A): `fork <self-contained arms> join[_any|_none]` inside a
//! suspendable task now runs. The arms are separate task calls / blocks that do
//! NOT reference the enclosing task's automatic locals, so the existing owned-
//! window model isolates them (the single-threaded scheduler + stash/restore).
//! ORACLE: iverilog 13.0 runs fork…join inside a task.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fif_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009")
}

// ── the report's C1 repro: fork of two separate suspendable tasks, join ──
#[test]
fn report_repro_fork_of_tasks_join() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat (2) @(posedge clk); endtask\n\
        task automatic b; repeat (2) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); fork a(); b(); join $display(\"PASS @%0t\", $time);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    // run's @posedge at t=5; a,b each wait 2 posedges (t=15,25) → both done t=25.
    assert!(!is_loud(&o) && o.contains("PASS @25"), "report repro:\n{o}");
}
```

- [ ] **Step 2: Run it, verify E3009 failure**

Run: `cargo test -p cli --test fork_in_frame report_repro_fork_of_tasks_join -- --nocapture`
Expected: FAIL — output contains `E3009` (the current loud reject).

- [ ] **Step 3: Elaborate — add the arm classifier**

In `crates/elaborate/src/frames_classify.rs`, add near `frame_body_is_leaf_nonsuspending`:

```rust
/// Stage-1 fork-in-frame admission. `(lo,hi)` = the enclosing task's frame-local
/// net range `[base_net, base_net+locals_len)`. Walks the task's reachable blocks;
/// for every `Fork`, classifies its arm subtrees:
/// - CaseA: no arm reads/writes a net in `[lo,hi)` → runnable on the owned model.
/// - CaseB: some arm touches a parent frame-local → needs the shared window
///   (Stage 2/3). Loud in Stage 1.
/// - Loud: a NESTED fork, or `wait fork` / `disable fork` in a frame body.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ForkAdmit { CaseA, CaseB, Loud }

pub(crate) fn fork_arms_self_contained(&self, entry: u32, lo: u32, hi: u32) -> ForkAdmit {
    let mut worst = ForkAdmit::CaseA;
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![entry];
    while let Some(bi) = stack.pop() {
        if !seen.insert(bi) { continue; }
        let Some(blk) = self.func_blocks.get(bi as usize) else { continue; };
        if let ir::Terminator::Fork { children, .. } = &blk.term {
            for &arm in children {
                match self.classify_one_arm(arm, lo, hi) {
                    ForkAdmit::Loud => return ForkAdmit::Loud,
                    ForkAdmit::CaseB => worst = ForkAdmit::CaseB,
                    ForkAdmit::CaseA => {}
                }
            }
        }
        // Follow this task's own CFG edges (Call → ret_bb, never into a callee).
        match &blk.term {
            ir::Terminator::Goto { target } => stack.push(*target),
            ir::Terminator::Branch { then_bb, else_bb, .. } => { stack.push(*then_bb); stack.push(*else_bb); }
            ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => stack.push(*resume),
            ir::Terminator::Call { ret_bb, .. } => stack.push(*ret_bb),
            ir::Terminator::Fork { children, join, resume_bb } => {
                stack.extend(children.iter().copied());
                stack.push(*join); stack.push(*resume_bb);
            }
            ir::Terminator::Return => {}
        }
    }
    worst
}

/// One arm subtree (from `arm_entry` up to its `join` sentinel — a fork arm is
/// sealed with `goto(join_bb)`). Returns Loud on a nested fork / wait-fork /
/// disable-fork; CaseB if any stmt or terminator reads/writes a net in `[lo,hi)`;
/// else CaseA. Does NOT descend into called tasks (they have their own frames).
fn classify_one_arm(&self, arm_entry: u32, lo: u32, hi: u32) -> ForkAdmit {
    let in_range = |n: u32| n >= lo && n < hi;
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![arm_entry];
    let mut admit = ForkAdmit::CaseA;
    while let Some(bi) = stack.pop() {
        if !seen.insert(bi) { continue; }
        let Some(blk) = self.func_blocks.get(bi as usize) else { continue; };
        // nested fork / wait-fork / disable-fork → Loud.
        if let ir::Terminator::Fork { .. } = &blk.term { return ForkAdmit::Loud; }
        for &sid in &blk.stmts {
            match &self.stmts[sid as usize] {
                ir::Stmt::Disable { scope_kind: ir::DisableKind::Fork, .. } => return ForkAdmit::Loud,
                ir::Stmt::BlockingAssign { lhs, rhs } => {
                    if lhs.chunks.iter().any(|c| in_range(c.net)) || self.expr_reads_range(*rhs, lo, hi) {
                        admit = ForkAdmit::CaseB;
                    }
                }
                ir::Stmt::NonblockingAssign { lhs, rhs, .. } => {
                    if lhs.chunks.iter().any(|c| in_range(c.net)) || self.expr_reads_range(*rhs, lo, hi) {
                        admit = ForkAdmit::CaseB;
                    }
                }
                ir::Stmt::SysTask { args, .. } => {
                    if args.iter().any(|&a| self.expr_reads_range(a, lo, hi)) { admit = ForkAdmit::CaseB; }
                }
                _ => {}
            }
        }
        match &blk.term {
            ir::Terminator::Goto { target } => stack.push(*target),
            ir::Terminator::Branch { cond, then_bb, else_bb } => {
                if self.expr_reads_range(*cond, lo, hi) { admit = ForkAdmit::CaseB; }
                stack.push(*then_bb); stack.push(*else_bb);
            }
            ir::Terminator::Wait { cond, resume } => {
                if self.wait_cond_reads_frame_local(cond, &in_range) { admit = ForkAdmit::CaseB; }
                stack.push(*resume);
            }
            ir::Terminator::Delay { resume, .. } => stack.push(*resume),
            ir::Terminator::Call { ret_bb, .. } => stack.push(*ret_bb), // args handled at the Call's in_binds; see note
            ir::Terminator::Fork { .. } => return ForkAdmit::Loud,
            ir::Terminator::Return => {}
        }
    }
    admit
}
```

Add a small reader helper if one does not already exist (search first — `expr_reads_range`, `expr_reads_frame_local`, or similar may already be present near `wait_cond_reads_frame_local`):

```rust
/// True if evaluating `e` reads any net in `[lo,hi)` (a parent frame-local).
fn expr_reads_range(&self, e: u32, lo: u32, hi: u32) -> bool {
    // Reuse the existing net-collection walk over `self.exprs` if one exists
    // (e.g. `collect_expr_nets`); otherwise a direct recursion over ir::Expr
    // matching Signal { net, .. } / index / concat children. Return true on the
    // first net in [lo,hi).
    self.expr_nets_any(e, &|n| n >= lo && n < hi)
}
```

> NOTE for the implementer: `fork a(x)` where `x` is a parent local is Case B — the actual `x` is evaluated at the arm's `Call` in_binds, which reference `x`'s net. Confirm whether the `Call` in_binds live in `task_calls_func`/`task_calls_proc` (side tables) vs. the block stmts; if the arg exprs are NOT in the arm's block stmts, extend `classify_one_arm`'s `Call` arm to inspect that call's in_bind exprs for a `[lo,hi)` read. This is the one spot the walk could under-detect Case B → verify against Test in Task 2 (`fork_arg_is_parent_local_*`).

- [ ] **Step 4: Elaborate — flip the lift gate**

In `frames_classify.rs`, the lift condition (~line 315) currently rejects any task where `frame_body_is_leaf_nonsuspending(fid)` is false (which a `Fork` forces). Change the fork handling: replace the blanket `ir::Terminator::Fork { .. } => return false` in `frame_body_is_leaf_nonsuspending` (~line 530) with a call that admits Case A:

```rust
ir::Terminator::Fork { .. } => {
    // fork-in-frame: admitted iff every arm is self-contained (Case A) in this
    // stage. Case B / nested / wait-fork / disable-fork stay loud.
    match self.fork_arms_self_contained(self.funcs[fid as usize].entry, self.frame_lo(fid), self.frame_hi(fid)) {
        ForkAdmit::CaseA => { /* liftable — fall through, keep walking */ }
        ForkAdmit::CaseB | ForkAdmit::Loud => return false,
    }
}
```

Add `frame_lo`/`frame_hi` helpers (the task's `base_net` / `base_net+locals_len`) if not already reachable here — the `pending` loop at line 313 already has `(fid, name, base_net, locals_len, ...)`, so thread `base_net`/`locals_len` into `frame_body_is_leaf_nonsuspending` instead of recomputing (change its signature to take `lo, hi`). Update its two other call sites (class-method + frame-func) to pass their ranges.

> The whole-body walk in `frame_body_is_leaf_nonsuspending` computes the SAME `fork_arms_self_contained` once — factor so the classifier is called once per task, not per Fork block.

- [ ] **Step 5: Engine — `exit_arm_frame` helper**

In `crates/sim-engine/src/state/task_frames.rs`, after `exit_task_frame`:

```rust
/// Stage-1 fork-in-frame: tear down a completing fork child's ARM frame — pop its
/// live window off the shared `frame_stack` (the arm rode it while running). No
/// out-copy (an arm has no out-binds). `callee` is the enclosing task's FuncId
/// (the arm frame's `callee`); `func_has_auto` gates the pop exactly like
/// `exit_task_frame`. Stage 3 overrides this to also release a shared handle.
pub(crate) fn exit_arm_frame(&self, callee: u32) {
    if self.func_has_auto[callee as usize] {
        self.frame_stack.borrow_mut().pop();
    }
    self.frame_scope.borrow_mut().pop();
    self.call_depth.set(self.call_depth.get().saturating_sub(1));
}
```

> The arm frame is entered by `exec_fork` (Step 7) which pushes onto `frame_scope`/`call_depth` symmetrically — confirm those pushes exist so this pop balances.

- [ ] **Step 6: Engine — in-frame child-completion intercept**

In `crates/sim-engine/src/exec/process.rs`, the loop-top intercept (~line 51) is inside `if !in_frame`. Add an in-frame sibling branch right after it:

```rust
// Stage-1 fork-in-frame: an IN-FRAME fork child completes when its (sole) arm
// frame reaches the barrier's join_bb sentinel. call_stack.len()==1 ensures an
// inner callee frame reaching a same-valued id cannot mis-fire.
if in_frame
    && sched.activity_is_child(pi)
    && sched.activities[pi as usize].call_stack.len() == 1
{
    let arm = sched.activities[pi as usize].call_stack.last().unwrap();
    let arm_bb = arm.bb;
    let arm_callee = arm.callee;
    if let Some(jr) = sched.activity_join_ref(pi) {
        if arm_bb == sched.barrier_join_bb(jr) {
            sched.st.exit_arm_frame(arm_callee); // pop the arm's live window
            sched.activities[pi as usize].call_stack.clear();
            sched.on_child_complete(jr, pi);
            return Step::Done;
        }
    }
}
```

- [ ] **Step 7: Engine — `exec_fork` in-frame branch + Fork terminator arm**

In `crates/sim-engine/src/exec/process.rs` `Terminator::Fork` arm (~line 320), replace the `if in_frame { mark_fatal }` guard so an in-frame parent stashes its window and forks:

```rust
Terminator::Fork { children, join, resume_bb } => {
    if in_frame {
        // Stash the parent frame's window (as Delay/Wait in-frame do) so the
        // children — which take turns on the shared frame_stack — never see it.
        stash_frame_windows(sched, pi);
    }
    match sched.exec_fork(pi, children, *join, *resume_bb) {
        Some(cont) => { set_pos!(cont); }   // join_none / zero children
        None => return Step::Suspended,     // parked on the barrier
    }
}
```

In `crates/sim-engine/src/sched/propagate.rs` `exec_fork` (~line 707), make child spawn frame-aware. Detect the in-frame case via the parent's call_stack, and give each child an arm `FrameRec` instead of an empty `call_stack`:

```rust
let parent_in_frame = !self.activities[parent_aid as usize].call_stack.is_empty();
// The FuncId whose CFG the fork lives in (the arm blocks are its blocks).
let arm_callee = if parent_in_frame {
    self.activities[parent_aid as usize].call_stack.last().unwrap().callee
} else { 0 };
```

In the child-spawn loop, build the child's `call_stack`:

```rust
let child_call_stack = if parent_in_frame {
    // Stage 1: Case A arm — an EMPTY owned window (the arm touches no frame slot).
    // window = Some(..) so run_process's restore pushes it on the child's first run.
    vec![crate::sched::FrameRec {
        callee: arm_callee,
        bb: child_entry,
        ret_bb: join,            // unused: the child dies at join_bb via the intercept
        out_binds: Vec::new(),
        window: Some(Vec::new()),
    }]
} else {
    Vec::new()
};
let child = Activity { call_stack: child_call_stack, /* ...other fields unchanged... */ };
```

And the child's `Ready.block` stays `child_entry` (ignored for an in-frame child, which reads its frame bb — but harmless and correct as the initial frame bb).

> `on_child_complete` re-enqueues the parent with `Ready { block: resume_bb }`. For an in-frame parent that field is IGNORED (run_process reads the frame bb). Step 8 sets the frame bb.

- [ ] **Step 8: Engine — resume an in-frame parent at the frame bb**

In `crates/sim-engine/src/sched/wait_fork.rs` `on_child_complete` (~line 69), where it re-enqueues the parent, set the in-frame parent's frame bb to `resume_bb` first:

```rust
if fire && !b.fired {
    b.fired = true;
    let parent = b.parent;
    let resume_bb = b.resume_bb;
    // Stage-1 fork-in-frame: an in-frame parent resumes at the frame's PC, not the
    // process bb — set it here (the Ready.block below is ignored while in_frame).
    if let Some(f) = self.activities[parent as usize].call_stack.last_mut() {
        f.bb = resume_bb;
    }
    let tie = self.activities[parent as usize].tie;
    push_sorted(&mut self.cur.active, Ready { tie, proc: parent, block: resume_bb });
}
```

- [ ] **Step 9: Run the report-repro test, verify PASS**

Run: `cargo test -p cli --test fork_in_frame report_repro_fork_of_tasks_join -- --nocapture`
Expected: PASS (output contains `PASS @25`).
Differential: `iverilog -g2012 t.sv && ./a.out` on the same source prints `PASS @25`.

- [ ] **Step 10: Flip the existing loud test**

In `crates/cli/tests/suspendable_const_repeat.rs`, rename `fork_in_frame_stays_loud` → `fork_of_tasks_join_runs` and change the assertion from `assert!(is_loud(&o), ...)` to:

```rust
assert!(!is_loud(&o) && o.contains("PASS"), "fork of tasks in a frame task now runs:\n{o}");
```

- [ ] **Step 11: Full gate + commit**

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
git add crates/elaborate/src/frames_classify.rs crates/sim-engine/src/sched/propagate.rs \
        crates/sim-engine/src/exec/process.rs crates/sim-engine/src/state/task_frames.rs \
        crates/sim-engine/src/sched/wait_fork.rs crates/cli/tests/fork_in_frame.rs \
        crates/cli/tests/suspendable_const_repeat.rs
git commit  # message: "fork-in-frame Stage 1: Case A (self-contained arms) join loud→supported"
```

---

## Task 2: Stage 1 — breadth + correct-or-loud

Add join_any / join_none / interleave coverage and lock the correct-or-loud boundaries (Case B, nested fork, wait/disable fork stay LOUD). Pure test additions against the Task 1 engine, plus any narrow fixes they surface.

**Files:**
- Modify: `crates/cli/tests/fork_in_frame.rs`

**Interfaces:**
- Consumes: Task 1's `report_repro_fork_of_tasks_join` harness (`run`, `is_loud`).

- [ ] **Step 1: Add join_none / join_any / interleave tests**

```rust
// ── join_none: parent continues immediately; children run in background ──
#[test]
fn fork_join_none_of_tasks() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat (2) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); fork a(); a(); join_none $display(\"forked @%0t\", $time);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    assert!(!is_loud(&o) && o.contains("forked @5"), "join_none:\n{o}");
}

// ── join_any: parent resumes after the FIRST child; surplus runs on ──
#[test]
fn fork_join_any_first_wins() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic fast; @(posedge clk); endtask\n\
        task automatic slow; repeat (3) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); fork fast(); slow(); join_any $display(\"any @%0t\", $time);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // run @posedge t=5; fast → next posedge t=15 (first); join_any resumes t=15.
    assert!(!is_loud(&o) && o.contains("any @15"), "join_any:\n{o}");
}

// ── two forks in sequence in the same task ──
#[test]
fn two_sequential_forks() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; @(posedge clk); endtask\n\
        task automatic run;\n\
          fork a(); a(); join fork a(); a(); join $display(\"DONE @%0t\", $time);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    // 1st fork: both a wait 1 posedge → t=5. 2nd fork: → t=15. DONE @15.
    assert!(!is_loud(&o) && o.contains("DONE @15"), "two forks:\n{o}");
}
```

- [ ] **Step 2: Run them, differential-verify each vs iverilog**

Run: `cargo test -p cli --test fork_in_frame -- --nocapture`
For any mismatch, run `iverilog -g2012 t.sv && ./a.out` on that exact source and reconcile the expected value (iverilog is the oracle). Fix engine bugs the interleave surfaces (likely areas: `stash_frame_windows` for a child mid-suspend; the intercept's `call_stack.len()==1` gate).

- [ ] **Step 3: Add correct-or-loud tests (stay LOUD)**

```rust
// ── Case B (arm writes a parent local): loud in Stage 1 (supported in Stage 2) ──
#[test]
fn case_b_arm_writes_parent_local_stays_loud_stage1() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          int x = 0;\n\
          fork begin @(posedge clk); x = 42; end @(posedge clk); join\n\
          $display(\"x=%0d\", x);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "case B must stay loud in Stage 1:\n{o}");
}

// ── nested fork inside a frame arm: loud (all stages) ──
#[test]
fn nested_fork_in_frame_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; @(posedge clk); endtask\n\
        task automatic run;\n\
          fork begin fork a(); a(); join end join $display(\"X\");\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "nested fork must stay loud:\n{o}");
}

// ── wait fork inside a frame body: loud (all stages) ──
#[test]
fn wait_fork_in_frame_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; @(posedge clk); endtask\n\
        task automatic run;\n\
          fork a(); join_none wait fork; $display(\"X\");\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "wait fork in frame must stay loud:\n{o}");
}

// ── disable fork inside a frame body: loud (all stages) ──
#[test]
fn disable_fork_in_frame_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat(9) @(posedge clk); endtask\n\
        task automatic run;\n\
          fork a(); join_none disable fork; $display(\"X\");\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "disable fork in frame must stay loud:\n{o}");
}
```

- [ ] **Step 4: Run, confirm all loud**

Run: `cargo test -p cli --test fork_in_frame -- --nocapture`
Expected: the four `*_stays_loud*` tests pass (output contains `E3009`). If Case B or nested fork does NOT go loud, tighten `classify_one_arm` (Case B under-detection) / confirm the `in_fork` elaborate error fires for a fork nested in a frame arm.

- [ ] **Step 5: Full gate + commit**

```bash
cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all -- --check
git add crates/cli/tests/fork_in_frame.rs
git commit  # "fork-in-frame Stage 1: join_any/join_none breadth + correct-or-loud (Case B / nested / wait|disable fork stay loud)"
```

---

## Task 3: Stage 2 — Case B for `join` (shared arena window)

Let a `join` fork's arms read/write the enclosing task's automatic locals. Introduce the interior-mutable window arena and route Case-B tasks to it; `join`-all guarantees children complete before the parent returns, so the parent frees the window on `Return` (no refcount yet).

**Files:**
- Modify: `crates/sim-engine/src/state/mod.rs` (add the arena fields; `frame_stack` element type → `WindowSlot`)
- Modify: `crates/sim-engine/src/value.rs` or `crates/sim-engine/src/state/mod.rs` (define `enum WindowSlot`)
- Modify: `crates/sim-engine/src/state/frame_eval.rs` (`frame_slot_read`/`frame_slot_write` — `WindowSlot` match; ~line 75-107)
- Modify: `crates/sim-engine/src/state/task_frames.rs` (`enter_task_frame` arena alloc for a shared-window callee; `exit_task_frame`; `exit_arm_frame`)
- Modify: `crates/sim-engine/src/state/init_diag.rs` (init the new fields; ~line 187)
- Modify: `crates/sim-engine/src/sched/mod.rs` (`FrameRec.window: Option<WindowSlot>`; ~line 70)
- Modify: `crates/sim-engine/src/exec/process.rs` (`stash`/`restore` handle `WindowSlot`; ~line 13-41)
- Modify: `crates/sim-engine/src/sched/propagate.rs` (`exec_fork`: Case-B arm window = `Shared(parent_handle)`)
- Modify: `crates/elaborate/src/tables.rs` (`FuncMeta.contains_shared_fork: bool`; ~line 130-180)
- Modify: `crates/elaborate/src/frames_classify.rs` (set the flag; admit Case-B `join`)
- Modify: `crates/elaborate/src/frames_reserve.rs` + `crates/elaborate/src/classes.rs` (init the new `FuncMeta` field to `false` at the 3 push sites)
- Modify: `crates/cli/tests/fork_in_frame.rs` (flip `case_b_arm_writes_parent_local_stays_loud_stage1`; add sibling-visibility)

**Interfaces:**
- Produces: `enum WindowSlot { Owned(Vec<Value>), Shared(u32) }`; `SimState::frame_windows: RefCell<Vec<Option<Vec<Value>>>>`, `frame_window_free: RefCell<Vec<u32>>`; `SimState::alloc_frame_window(&self, init: Vec<Value>) -> u32`, `free_frame_window(&self, h: u32)`; `FuncMeta.contains_shared_fork: bool` threaded to `func_table` and read by `enter_task_frame`.

- [ ] **Step 1: Write the failing Case-B test**

Add to `crates/cli/tests/fork_in_frame.rs`:

```rust
// ── Case B: a join arm WRITES a parent automatic local; parent reads it after join ──
#[test]
fn case_b_join_arm_writes_parent_local() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          int x = 0;\n\
          @(posedge clk);\n\
          fork\n\
            begin @(posedge clk); x = 42; end\n\
            begin @(posedge clk); @(posedge clk); end\n\
          join\n\
          $display(\"x=%0d @%0t\", x, $time);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // run @posedge t=5; arm1 writes x=42 at t=15; arm2 done t=25; join resumes t=25.
    assert!(!is_loud(&o) && o.contains("x=42 @25"), "case B join:\n{o}");
}
```

- [ ] **Step 2: Run, verify loud (E3009) — Case B still rejected**

Run: `cargo test -p cli --test fork_in_frame case_b_join_arm_writes_parent_local -- --nocapture`
Expected: FAIL — contains `E3009`.

- [ ] **Step 3: Define `WindowSlot` + arena fields**

In `crates/sim-engine/src/state/mod.rs`, define:

```rust
/// A live frame window on `frame_stack`. `Owned` is the common/Case-A path
/// (byte-identical storage to the pre-arena `Vec<Value>`); `Shared(handle)`
/// indexes `frame_windows` — a Case-B task whose fork arms read/write its locals,
/// so parent + children reference one arena window by handle.
pub enum WindowSlot {
    Owned(Vec<crate::value::Value>),
    Shared(u32),
}
```

Change `frame_stack: RefCell<Vec<Vec<Value>>>` → `RefCell<Vec<WindowSlot>>`. Add:

```rust
/// Case-B fork windows: interior-mutable arena (the dyn_heap/class_heap pattern).
/// `None` = a freed slot (reusable via `frame_window_free`).
pub frame_windows: std::cell::RefCell<Vec<Option<Vec<crate::value::Value>>>>,
pub frame_window_free: std::cell::RefCell<Vec<u32>>,
// Stage 3 adds: pub frame_window_rc: std::cell::RefCell<Vec<u32>>,
```

In `crates/sim-engine/src/state/init_diag.rs` (~line 187) init: `frame_windows: RefCell::new(Vec::new()), frame_window_free: RefCell::new(Vec::new()),`.

- [ ] **Step 4: Arena alloc/free helpers**

In `crates/sim-engine/src/state/task_frames.rs`:

```rust
/// Allocate a Case-B shared window pre-filled with `init`, reusing a freed slot.
pub(crate) fn alloc_frame_window(&self, init: Vec<Value>) -> u32 {
    if let Some(h) = self.frame_window_free.borrow_mut().pop() {
        self.frame_windows.borrow_mut()[h as usize] = Some(init);
        h
    } else {
        let mut g = self.frame_windows.borrow_mut();
        g.push(Some(init));
        (g.len() - 1) as u32
    }
}
/// Free a Case-B window (Stage 2: called on the parent's Return). Stage 3 gates
/// this behind a refcount decrement.
pub(crate) fn free_frame_window(&self, h: u32) {
    self.frame_windows.borrow_mut()[h as usize] = None;
    self.frame_window_free.borrow_mut().push(h);
}
```

- [ ] **Step 5: `frame_slot_read`/`write` through `WindowSlot`**

In `crates/sim-engine/src/state/frame_eval.rs` (~line 75), the automatic arm becomes:

```rust
if automatic {
    let g = self.frame_stack.borrow();
    match g.last().expect("frame read: no active call window") {
        WindowSlot::Owned(w) => w[slot as usize].clone(),
        WindowSlot::Shared(h) => {
            let a = self.frame_windows.borrow();
            a[*h as usize].as_ref().expect("live shared window")[slot as usize].clone()
        }
    }
} else { /* static_store path unchanged */ }
```

Symmetric for `frame_slot_write` (~line 100): match the top `WindowSlot`; for `Shared(h)`, `self.frame_windows.borrow_mut()[h][slot] = v;`. Keep each borrow scoped to the single index op (no eval inside — the existing §borrowDiscipline rule). Audit every other `frame_stack.borrow()`/`borrow_mut()` site (grep) and adapt the pattern-match (`enter_task_frame` push, `exit_task_frame` pop, stash/restore).

- [ ] **Step 6: `enter_task_frame` allocates an arena window for a shared-window task**

In `crates/sim-engine/src/state/task_frames.rs::enter_task_frame` (~line 360), branch on the callee's shared-fork flag (add `func_contains_shared_fork: Vec<bool>` to `SimState`, filled from `func_table`/`func_metas` exactly like `func_has_auto`):

```rust
let shared = self.func_contains_shared_fork[callee as usize];
match (has_auto, has_static, shared) {
    (true, _, true) => {
        let h = self.alloc_frame_window(fresh);          // Case B: arena
        self.frame_stack.borrow_mut().push(WindowSlot::Shared(h));
    }
    (true, true, false) => { self.frame_stack.borrow_mut().push(WindowSlot::Owned(fresh.clone())); self.static_store.borrow_mut().entry(callee).or_insert(fresh); }
    (true, false, false) => self.frame_stack.borrow_mut().push(WindowSlot::Owned(fresh)),
    (false, _, _) => { self.static_store.borrow_mut().entry(callee).or_insert(fresh); }
}
```

In `exit_task_frame` (~line 407), when popping a `WindowSlot::Shared(h)` for a shared callee, free it (Stage 2 — join guarantees children are done):

```rust
if has_auto {
    match self.frame_stack.borrow_mut().pop() {
        Some(WindowSlot::Shared(h)) => self.free_frame_window(h),   // Stage 3: rc-- then free at 0
        _ => {}
    }
}
```

- [ ] **Step 7: `stash`/`restore` + `FrameRec.window` carry `WindowSlot`**

`crates/sim-engine/src/sched/mod.rs`: `FrameRec.window: Option<crate::state::WindowSlot>`.
`crates/sim-engine/src/exec/process.rs` `stash_frame_windows`/`restore_frame_windows`: they already move the popped/pushed value between `frame_stack` and `FrameRec.window` — with `WindowSlot` the moved value is the enum (a `Shared(h)` moves the handle, the arena data stays put). No logic change beyond the type.

- [ ] **Step 8: `exec_fork` — Case-B arm windows reference the parent handle**

In `crates/sim-engine/src/sched/propagate.rs::exec_fork`, when `parent_in_frame`, read the parent frame's current window slot; if it is `Shared(h)`, the arm window is `Shared(h)` (same handle); else `Owned(empty)` (Case A). Because the parent's window was just stashed by the Fork terminator arm (Task 1 Step 7), read it from the parent's `FrameRec.window`:

```rust
let arm_window = match &self.activities[parent_aid as usize].call_stack.last().unwrap().window {
    Some(WindowSlot::Shared(h)) => Some(WindowSlot::Shared(*h)),  // Case B: share the handle
    _ => Some(WindowSlot::Owned(Vec::new())),                     // Case A: empty owned
};
```

Use `arm_window` in the child `FrameRec` built in Task 1 Step 7.

> Stage 2 needs no rc bump (join keeps all children ≤ the parent's lifetime). Verify the parent's stashed window is `Shared(h)` here (the Fork arm stashed it just before calling `exec_fork`).

- [ ] **Step 9: Elaborate — `FuncMeta.contains_shared_fork` + admit Case-B join**

In `crates/elaborate/src/tables.rs` `FuncMeta` (~line 130): add `pub contains_shared_fork: bool,` (with `#[serde(default)]` if the struct derives Serialize — mirror `has_hier_call`). Init `false` at the 3 push sites (`frames_reserve.rs:523`, `:739`, `classes.rs:695`).

In `frames_classify.rs`, when `fork_arms_self_contained(...)` returns `CaseB`, and every Case-B fork in the task is a `join` (mode from the side table `record_fork_mode`/`fork_modes`), set `self.func_metas[fid].contains_shared_fork = true` and admit (return liftable). A Case-B `join_any`/`join_none` stays `false`+loud until Stage 3. Thread `contains_shared_fork` into `func_table` so the engine fills `func_contains_shared_fork` (mirror the `has_hier_call`→`force_suspend` threading at line 304).

> The join mode lives in the elaborate `fork_modes` side table keyed `(cur_proc/template, join_bb)` — for a frame task the key is the func context; confirm how `record_fork_mode` keys a fork lowered in a task body and read the mode there. If the mode is not readily keyable at classify time, carry it on the `Fork` classification by extending `fork_arms_self_contained` to return the per-fork `(admit, mode)` and require `mode == All` for Case B in Stage 2.

- [ ] **Step 10: Run the Case-B test, differential-verify**

Run: `cargo test -p cli --test fork_in_frame case_b_join_arm_writes_parent_local -- --nocapture`
Expected: PASS (`x=42 @25`). Differential: `iverilog -g2012 t.sv && ./a.out` → `x=42 @25`.

- [ ] **Step 11: Flip the Stage-1 loud test + add sibling visibility**

In `crates/cli/tests/fork_in_frame.rs`, replace `case_b_arm_writes_parent_local_stays_loud_stage1` with a supported assertion (rename → `case_b_join_arm_writes_parent_local_2` or fold into Step 1's test), and add:

```rust
// ── Case B: sibling sees another arm's write to the shared parent local ──
#[test]
fn case_b_sibling_visibility() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          int x = 0;\n\
          fork\n\
            x = 7;\n\
            begin #1; if (x == 7) $display(\"sib sees x=%0d\", x); end\n\
          join\n\
        endtask\n\
        initial begin run(); #50 $finish; end\n\
        endmodule\n");
    // arm0 (lower tie) sets x=7 at t=0; arm1 waits #1 → t=1 → reads x=7.
    assert!(!is_loud(&o) && o.contains("sib sees x=7"), "sibling visibility:\n{o}");
}
```

- [ ] **Step 12: Full gate + commit**

```bash
cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all -- --check
# confirm format stays 23:
grep -n CURRENT_FORMAT_VERSION crates/vita-artifact/src/header.rs
git add -A crates/sim-engine crates/elaborate crates/cli/tests/fork_in_frame.rs
git commit  # "fork-in-frame Stage 2: Case B for join via interior-mutable window arena (WindowSlot + contains_shared_fork)"
```

---

## Task 4: Stage 3 — Case B for `join_any` / `join_none` (refcount)

A surviving child (join_any surplus, or a join_none child) may outlive the parent while still referencing its shared window. Refcount the arena window so it is freed only when the last referencing `FrameRec` (parent or child) drops.

**Files:**
- Modify: `crates/sim-engine/src/state/mod.rs` (`frame_window_rc`)
- Modify: `crates/sim-engine/src/state/task_frames.rs` (`alloc_frame_window` sets rc=1; `retain_frame_window`/`release_frame_window`; `exit_task_frame` + `exit_arm_frame` release)
- Modify: `crates/sim-engine/src/sched/propagate.rs` (`exec_fork` Case-B arm: `retain_frame_window(h)` per child)
- Modify: `crates/elaborate/src/frames_classify.rs` (admit Case-B `join_any`/`join_none`)
- Modify: `crates/cli/tests/fork_in_frame.rs`

**Interfaces:**
- Produces: `SimState::frame_window_rc: RefCell<Vec<u32>>`; `retain_frame_window(&self, h)`, `release_frame_window(&self, h)` (dec; free at 0). `alloc_frame_window` seeds rc=1 (the parent).

- [ ] **Step 1: Write the failing Stage-3 test**

```rust
// ── Case B join_any: the SURPLUS (slow) child references x AFTER the parent
//    returned — the shared window must outlive the parent (refcount). ──
#[test]
fn case_b_join_any_surplus_outlives_parent() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          automatic int x = 9;\n\
          fork\n\
            @(posedge clk);\n\
            begin repeat (3) @(posedge clk); if (x == 9) $display(\"surplus sees x=%0d @%0t\", x, $time); end\n\
          join_any\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // fast: posedge t=5 → join_any resumes, run RETURNS at t=5. surplus: 3 posedges
    // → t=25, still reads x=9 (window kept alive by refcount).
    assert!(!is_loud(&o) && o.contains("surplus sees x=9 @25"), "join_any surplus lifetime:\n{o}");
}
```

- [ ] **Step 2: Run, verify loud (Case-B join_any rejected in Stage 2)**

Run: `cargo test -p cli --test fork_in_frame case_b_join_any_surplus_outlives_parent -- --nocapture`
Expected: FAIL — `E3009`.

- [ ] **Step 3: Add the refcount**

`crates/sim-engine/src/state/mod.rs`: `pub frame_window_rc: std::cell::RefCell<Vec<u32>>,`; init empty in `init_diag.rs`.

`task_frames.rs`:

```rust
// alloc_frame_window: seed rc=1 (the parent frame owns the initial reference).
// On the free-list reuse path AND the push path, set frame_window_rc[h] = 1.
pub(crate) fn retain_frame_window(&self, h: u32) {
    self.frame_window_rc.borrow_mut()[h as usize] += 1;
}
/// Decrement; free the slot at zero. Replaces the direct free in exit_task_frame /
/// exit_arm_frame for a Shared window.
pub(crate) fn release_frame_window(&self, h: u32) {
    let mut rc = self.frame_window_rc.borrow_mut();
    debug_assert!(rc[h as usize] > 0, "shared frame window rc underflow");
    rc[h as usize] -= 1;
    if rc[h as usize] == 0 {
        drop(rc);
        self.free_frame_window(h);
    }
}
```

Update `alloc_frame_window` to set rc=1 for the returned handle (both the reuse and push paths — resize `frame_window_rc` alongside `frame_windows`).

`exit_task_frame` and `exit_arm_frame`: for a popped `WindowSlot::Shared(h)`, call `self.release_frame_window(h)` instead of `free_frame_window(h)`.

- [ ] **Step 4: `exec_fork` retains per Case-B child**

In `propagate.rs::exec_fork`, when the arm window is `Shared(h)`, `self.st.retain_frame_window(h);` once per spawned child (the child's `FrameRec` holds a reference). The parent's own reference (rc=1 from alloc) is released at the parent's `Return`; each child's is released when it completes (`exit_arm_frame`).

- [ ] **Step 5: Elaborate — admit Case-B join_any/join_none**

In `frames_classify.rs`, drop the `mode == All` restriction added in Task 3 Step 9: a Case-B fork of ANY join mode now sets `contains_shared_fork = true` and is admitted. Nested fork / wait fork / disable fork stay `Loud` (unchanged).

- [ ] **Step 6: Run the Stage-3 test + differential**

Run: `cargo test -p cli --test fork_in_frame case_b_join_any_surplus_outlives_parent -- --nocapture`
Expected: PASS (`surplus sees x=9 @25`). Differential vs iverilog. Add a `join_none` twin (a lone `join_none` child that reads a parent local at `#20` after the parent returns) and verify likewise.

- [ ] **Step 7: Adversarial — rc soundness**

Add tests that would expose a use-after-free or double-free: (a) two surplus children both referencing x after the parent returns (rc must reach 0 only after BOTH complete); (b) a Case-B `join_none` inside a loop (each iteration allocs+frees a distinct window — no leak, no reuse-while-live). Run under `cargo test` (debug asserts on rc active). Any `rc underflow`/`live shared window` panic → fix before proceeding; if a construct's lifetime cannot be proven safe, keep it LOUD (correct-or-loud) rather than guess.

- [ ] **Step 8: Full gate + commit**

```bash
cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all -- --check
git add -A crates/sim-engine crates/elaborate crates/cli/tests/fork_in_frame.rs
git commit  # "fork-in-frame Stage 3: Case B join_any/join_none via refcounted shared window"
```

---

## Task 5: Documentation + final gate

Record the slice and refresh the forward docs. No code change.

**Files:**
- Modify: `docs/ROADMAP.md` (baseline line + retire the §0 NEXT fork-in-frame item)
- Modify: `docs/ROADMAP_ARCHIVE.md` (new §4.5.x entry — full narrative + lessons)
- Modify: `CLAUDE.md` (git-ignored local — status line + latest-slice bullet)
- Modify: `/Users/seongwookjang/.claude/projects/-Users-seongwookjang-project-git-vitamin-rtl-simulator/memory/round16-executor-bound-gaps.md` (append the fork-in-frame resolution — the "single-owner window was over-cautious; single-threaded scheduler + stash/restore already isolate Case A" lesson)

- [ ] **Step 1: Update ROADMAP + ARCHIVE**

Bump the ROADMAP baseline test count to the new green total; move the §0 NEXT fork-in-frame item to "RESOLVED" with a pointer to the new archive §. Write the archive entry: the Case A/B split, the two-model design, the 3 stages, the correct-or-loud residuals (nested fork / wait|disable fork stay loud), and the lessons (single-threaded scheduler makes Case A free on the owned model; arena+refcount for Case B lifetime; the "single-owner window" framing was over-cautious).

- [ ] **Step 2: Update CLAUDE.md latest-slice bullet + memory**

Add the new slice bullet at the top of the CLAUDE.md "최신 슬라이스" list (keep it git-ignored — do NOT `git add` it). Update the memory file's description + append the fork-in-frame confirmation.

- [ ] **Step 3: Final full-suite gate**

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
grep -n CURRENT_FORMAT_VERSION crates/vita-artifact/src/header.rs   # must still be 23
```
Expected: all green; format 23.

- [ ] **Step 4: Commit docs**

```bash
git add docs/ROADMAP.md docs/ROADMAP_ARCHIVE.md
git commit  # "docs: record fork-in-frame (§4.5.x) — Case A/B, 3 stages, correct-or-loud residuals"
```

- [ ] **Step 5: Report to the user (do NOT push)**

Summarize: stages delivered, test delta, format 23 unchanged, correct-or-loud residuals (nested fork / wait|disable fork). Ask whether to merge `fork-in-frame` → `main` and push (the user drives the push, per the round-18 flow).

---

## Self-Review

**Spec coverage:**
- §3 Case A/B boundary → Task 1 Step 3 (`classify_one_arm`).
- §5 staging (1/2/3) → Tasks 1-2 / 3 / 4.
- §6.1 `WindowSlot` + arena → Task 3 Steps 3-5.
- §6.2 exec_fork frame-aware → Task 1 Step 7 + Task 3 Step 8.
- §6.3 in-frame intercept (`call_stack.len()==1`) → Task 1 Step 6.
- §6.4 parent resume + free (Stage 2 direct / Stage 3 refcount) → Task 1 Step 8, Task 3 Step 6, Task 4 Steps 3-4.
- §6.5 Fork terminator arm → Task 1 Step 7.
- §7 elaborate gate + `FuncMeta` → Task 1 Steps 3-4, Task 3 Step 9, Task 4 Step 5.
- §8 correct-or-loud → Task 2 Step 3, Task 4 Step 7.
- §9 determinism / no format bump → Global Constraints + Task 3 Step 12 / Task 5 Step 3.
- §10 verification → every task's gate + differential steps.
- §11 risks (rc underflow, intercept mis-fire, WindowSlot cost, Case-B under-detection) → Task 4 Step 7, Task 1 Step 6, Task 3, Task 1 Step 3 NOTE + Task 2 Step 4.

**Placeholder scan:** The two NOTE blocks (Task 1 Step 3 `Call` in_bind Case-B detection; Task 3 Step 9 join-mode keying) are explicit verification points, not vague placeholders — each states the exact thing to confirm and the fallback. No "TBD"/"add error handling"/"similar to Task N".

**Type consistency:** `ForkAdmit{CaseA,CaseB,Loud}`, `WindowSlot{Owned,Shared}`, `fork_arms_self_contained`, `classify_one_arm`, `exit_arm_frame`, `alloc_frame_window`/`free_frame_window`/`retain_frame_window`/`release_frame_window`, `FuncMeta.contains_shared_fork`, `func_contains_shared_fork` used consistently across tasks.
