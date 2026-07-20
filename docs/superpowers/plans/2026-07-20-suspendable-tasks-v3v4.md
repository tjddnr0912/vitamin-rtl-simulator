# Suspendable Tasks (V3/V4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a task body contain `@`/`#delay`/`wait`/`wait fork` (V3) and NBA/`$systask`/`force`/`release`/`fork` (V4) by making a non-subset task call *suspend the calling process*, so the reviewer's four testbenches (KAT drivers = `task drive(...); @(posedge clk); sig<=v;`) elaborate and run.

**Architecture:** Add a per-activity **call-stack** to the scheduler. Classify each task as *subset* (blocking-assign + control-flow only — keeps today's fast synchronous `run_task_call`) or *non-subset* (has timing/NBA/$systask/fork). A non-subset task call pushes a frame record onto the calling activity's call-stack; `run_process` then executes the task's func-arena CFG and suspends on its `Delay`/`Wait` exactly as it does for a process, resuming into the call-stack top. `Return` pops the frame and copies outputs back. Recursion/reentrancy fall out of the existing per-activation automatic-window model. `format_version` stays 22 (the IR already expresses every terminator; the change is elaborate emitting richer task CFGs + an out-of-band sidecar + engine execution).

**Tech Stack:** Rust (edition 2021, MSRV 1.85), `cargo test --workspace --locked`, iverilog 13.0 as the differential oracle (`/opt/homebrew/bin/iverilog`), `postcard`+`blake3` golden gate.

## Global Constraints

- **MSRV 1.85**, vita crates edition 2021; `--locked` required (3-OS reproducibility).
- **correct-or-loud at every commit:** a non-subset shape not yet handled stays **loud** (E3009) with a precise message — never silent-wrong. No commit may ship a task that elaborates but mis-runs.
- **format_version stays 22:** every phase asserts the SimIr root hash is unchanged (golden gate green). If a genuinely new frozen-IR field proves unavoidable, STOP and escalate (bump is a separate decision, not a silent side effect).
- **Determinism:** BTree-only, no `usize`/float in frozen types, 3-OS byte-identical.
- **Gates per commit:** `cargo build -p cli --locked`; `cargo test --workspace --locked`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo fmt --all -- --check`.
- **macOS test harness:** no `timeout` — wrap oracle calls with `perl -e 'alarm 10; exec @ARGV' <cmd>`; every `initial` gets a `#<N> $finish;` watchdog.
- **Commit footer (verbatim):**
  ```
  Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01L3LJdSaS8ctAvKDmfUVqoV
  ```
- **Repo:** standalone PUBLIC (`tjddnr0912/vitamin-rtl-simulator`); `git add <path>` from repo root; branch `feat-<slug>` → ff-merge to main; CLAUDE.md/LOOPROMPT.md are gitignored (never committed). Do NOT push without explicit user confirmation.

**Reference designs:** spec `docs/superpowers/specs/2026-07-20-round14-deferred-items-design.md` §1. Key existing code: `crates/sim-engine/src/exec.rs:224` (`run_process`), `:390` (`Terminator::Call` arm), `crates/sim-engine/src/state.rs:2596` (`run_task`), `crates/sim-engine/src/sched.rs:391` (`struct Activity`), `crates/elaborate/src/lib.rs:17279` (`validate_frame_body`).

---

## Phase 0 — Pin the oracle & lock the reproductions

### Task 0: Capture iverilog golden outputs for the V3/V4 matrix

**Files:**
- Create: `/private/tmp/.../scratchpad/v3v4/*.sv` (scratch, not committed) — the probe matrix.
- Reference: `docs/vita_repros/round12.sv` sections V3A_TASKTIME/V3B_TASKDELAY/V4A_TASKNBA/V4B_TASKSYS.

**Interfaces:**
- Produces: a table `probe → iverilog stdout` used as the expected value in every later test. No code artifact.

- [ ] **Step 1: Write the probe matrix** (one module per construct). Minimum set:

```systemverilog
// p_at.sv  — @ (posedge) wait in a task
module t; logic clk=0; always #5 clk=~clk;
  task automatic wait2(); @(posedge clk); @(posedge clk); endtask
  initial begin wait2(); $display("o=%0t", $time); #1 $finish; end endmodule
// EXPECT iverilog: o=15

// p_delay.sv — #10 in a task
module t; task automatic dly(); #10; endtask
  initial begin dly(); $display("o=%0t",$time); $finish; end endmodule
// EXPECT: o=10

// p_nba.sv — NBA to a module net from a task, then observe
module t; logic clk=0; logic [7:0] d; always #5 clk=~clk;
  task automatic put(input logic [7:0] v); @(posedge clk); d<=v; endtask
  initial begin put(8'hA5); @(posedge clk); $display("o=%0h",d); $finish; end endmodule
// EXPECT: o=a5

// p_sys.sv — $display in a task
module t; task automatic say(input int n); $display("o=%0d",n); endtask
  initial begin say(7); $finish; end endmodule
// EXPECT: o=7

// p_seq.sv — several sequential task calls with timing from one initial (the KAT-driver shape)
module t; logic clk=0; logic [7:0] bus; always #5 clk=~clk;
  task automatic drive(input logic [7:0] v); @(posedge clk); bus<=v; endtask
  initial begin drive(8'h11); @(posedge clk); drive(8'h22); @(posedge clk);
    $display("o=%0h",bus); $finish; end endmodule
// EXPECT: o=22

// p_recur.sv — recursion WITH timing (Phase 4 target; keep for the matrix)
module t; logic clk=0; always #5 clk=~clk;
  task automatic countdown(input int n); if (n>0) begin @(posedge clk); countdown(n-1); end endtask
  initial begin countdown(3); $display("o=%0t",$time); $finish; end endmodule
// EXPECT: o=35

// p_fork.sv — fork inside a task (Phase 4)
module t; logic clk=0; always #5 clk=~clk;
  task automatic par(); fork @(posedge clk); @(posedge clk); join endtask
  initial begin par(); $display("o=%0t",$time); $finish; end endmodule
// EXPECT: o=15
```

- [ ] **Step 2: Run each through iverilog, record the exact stdout.**

Run (per file): `perl -e 'alarm 10; exec @ARGV' iverilog -g2012 -o x.vvp p_at.sv && perl -e 'alarm 10; exec @ARGV' vvp x.vvp`
Expected: matches the `// EXPECT` comment. If any differs, update the EXPECT to iverilog's actual output (iverilog is the oracle of record).

- [ ] **Step 3: No commit** (scratch only). Record the table in the session notes; it seeds every later test's expected value.

---

## Phase 1 — Classifier + sidecar + richer task-body lowering (elaborate)

**Deliverable:** elaborate can tell subset from non-subset tasks, records a `suspendable_tasks` sidecar, and lowers a non-subset task body into the func arena **with real `Delay`/`Wait`/`Fork` terminators and NBA/$systask statements** — while the E3009 reject STAYS in force (Phase 2 lifts it in the same change that wires the engine). Golden root hash unchanged.

### Task 1.1: Split `validate_frame_body` into a classifier

**Files:**
- Modify: `crates/elaborate/src/lib.rs:17279-17365` (`validate_frame_body`).
- Test: `crates/elaborate/src/lib.rs` unit test module (or a new `crates/cli/tests/frame_classifier.rs`).

**Interfaces:**
- Produces: `fn classify_frame_body(&self, …) -> FrameClass` where `enum FrameClass { Subset, NonSubset }`. `validate_frame_body` keeps emitting the loud E3009 for NonSubset (unchanged external behavior this phase); it now delegates the walk to `classify_frame_body`.

- [ ] **Step 1: Write the failing test** — a subset task classifies Subset, a task with `@` classifies NonSubset. (Unit-test the classifier directly, or assert via the existing E3009 message which must be UNCHANGED this phase.)

```rust
// crates/cli/tests/frame_classifier.rs — behavior gate: message unchanged this phase
#[test]
fn nonsubset_task_still_loud_this_phase() {
    // A task with @(posedge) must STILL be E3009 in Phase 1 (engine not wired yet).
    let src = "module t; logic c=0; always #5 c=~c;\n\
        task automatic w(); @(posedge c); endtask\n\
        initial begin w(); $finish; end endmodule";
    let (out, ok) = run(src);            // run() helper as in round14_report_gaps.rs
    assert!(!ok, "must stay loud in Phase 1: {out}");
    assert!(out.contains("frame-call subset"), "{out}");
}
```

- [ ] **Step 2: Run to verify it passes already** (E3009 exists today) — this is a REGRESSION PIN, not a red test. `cargo test -p cli --test frame_classifier -- --nocapture`. Expected: PASS (loud today).

- [ ] **Step 3: Refactor** `validate_frame_body` to compute `FrameClass` via a new `classify_frame_body` that walks the same blocks (`func_blocks[block_base..]`) and returns `NonSubset` on the first timing terminator / NBA / SysTask / force-release / fork, else `Subset`. Keep the E3009 emission for `NonSubset` byte-identical.

- [ ] **Step 4: Run the pin + full suite.** `cargo test --workspace --locked`. Expected: 3710 green (no behavior change). Assert golden root hash unchanged (the suite's determinism gate covers this).

- [ ] **Step 5: Commit.**
```bash
git add crates/elaborate/src/lib.rs crates/cli/tests/frame_classifier.rs
git commit -m "V3/V4 Phase 1a: extract classify_frame_body (behavior unchanged)"  # + footer
```

### Task 1.2: Record the `suspendable_tasks` sidecar

**Files:**
- Modify: `crates/elaborate/src/lib.rs` (elaborator struct: add `suspendable_tasks: Vec<u32>` next to the other engine-facing side tables; populate it when `classify_frame_body == NonSubset` for a TASK, using the task's func-arena template id).
- Modify: the elaborate→engine handoff (where `task_calls_proc`/`func_metas` are handed out-of-band) to carry `suspendable_tasks`.
- Test: `crates/cli/tests/frame_classifier.rs`.

**Interfaces:**
- Consumes: `classify_frame_body` (Task 1.1).
- Produces: `SimOpts`/handoff field `suspendable_tasks: Vec<u32>` (sorted, dedup — BTree discipline). Engine reads it in Phase 2.

- [ ] **Step 1: Write the failing test** — assert the sidecar is populated for a non-subset task and empty for a subset-only design. (Expose via a debug accessor or assert indirectly once Phase 2 consumes it; for now, a unit assertion on the elaborate output struct.)

```rust
#[test]
fn suspendable_sidecar_lists_nonsubset_task() {
    // one non-subset task `w` + one subset task `s`
    let ir = elaborate_for_test("module t; logic c=0; always #5 c=~c;\n\
        task automatic w(); @(posedge c); endtask\n\
        task automatic s(input int n); int x; x=n; endtask\n\
        initial begin s(1); $finish; end endmodule");
    assert_eq!(ir.suspendable_tasks.len(), 1, "only w is suspendable");
}
```

- [ ] **Step 2: Run to verify it fails** (`suspendable_tasks` doesn't exist yet). Expected: compile error / assertion fail.

- [ ] **Step 3: Implement** the field + population (sorted dedup) + handoff wiring. Do NOT change any runtime behavior (engine ignores it this phase).

- [ ] **Step 4: Run full suite + golden gate.** Expected: green; root hash unchanged (sidecar is out-of-band, not in SimIr).

- [ ] **Step 5: Commit.** `V3/V4 Phase 1b: suspendable_tasks sidecar (out-of-band, unused)`

### Task 1.3: Lower non-subset task bodies with real terminators (still loud)

**Files:**
- Modify: `crates/elaborate/src/lib.rs` (task-body lowering path — where a task frame body is lowered to `func_blocks`). Ensure a non-subset task's `@`/`#`/NBA/`$display` lower to real `Terminator::Delay`/`Wait`/`Fork` + the corresponding `Stmt` variants IN THE FUNC ARENA, instead of being suppressed.
- Test: `crates/cli/tests/frame_classifier.rs` (assert the lowered func-arena CFG for `w` contains a `Wait` terminator — via a debug accessor or a golden-CFG check).

**Interfaces:**
- Consumes: classifier + sidecar.
- Produces: func-arena CFGs for non-subset tasks that are *executable by `run_process`* in Phase 2. The E3009 reject remains (Phase 2 removes it).

- [ ] **Step 1: Write the failing test** — the func-arena body of task `w` has a `Wait` terminator (today it doesn't, because timing is rejected before full lowering).

- [ ] **Step 2: Run to verify it fails.**

- [ ] **Step 3: Implement** the richer lowering (emit the terminators/stmts the classifier now permits). Keep the E3009 gate so nothing runs yet. Confirm `func_blocks` type already admits these variants (it does — shared with processes).

- [ ] **Step 4: Full suite + golden gate.** Expected: green; **root hash unchanged** (the func arena is not part of the frozen SimIr schema shape change — verify explicitly). If the hash flips, STOP and escalate (unexpected — investigate before proceeding).

- [ ] **Step 5: Commit.** `V3/V4 Phase 1c: lower non-subset task bodies with real terminators (still loud)`

---

## Phase 2 — Activity call-stack + `run_process` generalization (engine)

**Deliverable:** the scheduler activity carries a call-stack; a non-subset task Call enters a frame; `run_process` executes the task's func-arena CFG. **This phase lifts the E3009 reject** (in the same change) so a non-subset task that does NOT suspend (e.g. `$display`-only, V4B) runs correctly end-to-end. Timing/suspend arrives in Phase 3. Subset tasks are byte-identical.

### Task 2.1: Add the call-stack to `Activity`

**Files:**
- Modify: `crates/sim-engine/src/sched.rs:391` (`struct Activity`) — add `call_stack: Vec<FrameRec>` (empty for the overwhelming majority). Define `struct FrameRec { cfg: FrameCfg, bb: u32, window: FramePtr }` and `enum FrameCfg { Process, Task(u32) }` (task template id).
- Modify: `crates/sim-engine/src/exec.rs:247` — `run_process` reads the *current frame's* CFG: base process body when `call_stack` is empty, else the top frame's task CFG at its `bb`.

**Interfaces:**
- Produces: `FrameRec`, `FrameCfg`; accessor `sched.cur_frame(pi) -> &FrameRec` / block fetch that honors the call-stack top.

- [ ] **Step 1: Write the failing test** — a $display-only task (`p_sys.sv`) must print `o=7`. It is loud today; this whole phase makes it pass.

```rust
// crates/cli/tests/suspendable_tasks.rs
#[test]
fn task_with_display_runs() {   // V4B, no suspension needed
    let src = "module t; task automatic say(input int n); $display(\"o=%0d\",n); endtask\n\
        initial begin say(7); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "{out}"); assert!(out.contains("o=7"), "{out}");
}
```

- [ ] **Step 2: Run to verify it fails** (E3009 today). Expected: FAIL (loud).

- [ ] **Step 3: Implement the struct additions ONLY** (empty call-stack path byte-identical). Add the field, the `FrameRec`/`FrameCfg` types, and make the block-fetch in `run_process` branch on `call_stack.is_empty()` (empty ⇒ exactly today's `ir.processes[tmpl].body[bb]`).

- [ ] **Step 4: Run full suite.** Expected: 3710 green (empty call-stack ⇒ no behavior change). The new test still FAILS (Call not wired yet) — that's expected; mark it `#[ignore]` with a note "un-ignore in Task 2.2".

- [ ] **Step 5: Commit.** `V3/V4 Phase 2a: Activity call-stack scaffold (empty-stack byte-identical)`

### Task 2.2: Suspendable `Terminator::Call` + `Return`-pops-frame + lift the reject

**Files:**
- Modify: `crates/sim-engine/src/exec.rs:390` (`Terminator::Call` arm) — if the callee is in `suspendable_tasks`, PUSH a `FrameRec` (push an automatic window, copy inputs into input-formal slots) and continue the loop against the task CFG; else keep today's synchronous `run_task_call`.
- Modify: `crates/sim-engine/src/exec.rs:365` (`Terminator::Return`) — if `call_stack` is non-empty, POP the top frame (copy `output`/`inout` slots to caller lvalues, release the window) and continue at the caller's `ret_bb`; else today's `rearm`+`Done`.
- Modify: `crates/elaborate/src/lib.rs:17355` — remove the E3009 emission for NonSubset TASKS (keep it for any shape still unsupported; functions never reach here).
- Test: `crates/cli/tests/suspendable_tasks.rs` (un-ignore `task_with_display_runs`).

**Interfaces:**
- Consumes: `suspendable_tasks` sidecar, `FrameRec` (2.1), the input/output slot binding from `task_calls_proc` (`exec.rs:391`).

- [ ] **Step 1: Un-ignore the test** from 2.1 and add a nested-non-subset-call test (task calls a $display task).

- [ ] **Step 2: Run to verify it fails.** Expected: FAIL (Call still synchronous / reject still present).

- [ ] **Step 3: Implement** the push/pop + reject lift. NBA/$systask statements execute via the SAME `compute_effect`/`apply_effect` the process body uses (they run because we're now inside `run_process`). NO suspend yet (a `Delay`/`Wait` in the task body would still be reachable — guard: if a non-subset task hits Delay/Wait in this phase, it's handled in Phase 3; until then keep those specific shapes loud via the classifier splitting "V4-only" from "V3" — OR land Phase 3 immediately after so there is never a shipped gap). **Correct-or-loud:** if Phase 3 is not in the same PR, restrict the sidecar to V4-only (NBA/$systask, no timing) tasks so nothing that suspends is admitted yet.

- [ ] **Step 4: Full suite + oracle.** New tests pass; 3710+ green; `p_sys.sv` matches iverilog (`o=7`). Golden hash unchanged.

- [ ] **Step 5: Commit.** `V3/V4 Phase 2b: suspendable Call/Return for non-suspending (V4) tasks`

---

## Phase 3 — Suspend/resume across a task frame (the core)

**Deliverable:** `@`/`#`/`wait` inside a task suspend the calling process and resume correctly. V3A/V3B and the KAT-driver shape (p_seq) run.

### Task 3.1: `Delay`/`Wait` in a task frame preserve the call-stack

**Files:**
- Modify: `crates/sim-engine/src/exec.rs:318` (`Delay`) and `:332` (`Wait`) — on suspend, the activity's `call_stack` is preserved (it lives on the `Activity`, so `schedule_resume`/`suspend_on` already keep it); ensure `resume` targets the block in the CURRENT FRAME's CFG (not the base process body). `schedule_resume`/`suspend_on` (`sched.rs:2457`/`:2495`) record `(proc, block)` — confirm `block` is interpreted against the call-stack top on resume.
- Modify: `crates/sim-engine/src/sched.rs` window lifetime — the top frame's window must NOT be popped on suspend (only on `Return`).
- Test: `crates/cli/tests/suspendable_tasks.rs`.

**Interfaces:**
- Consumes: Phase 2 Call/Return + call-stack.

- [ ] **Step 1: Write the failing tests** — p_delay (`o=10`), p_at (`o=15`), p_nba (`o=a5`), p_seq (`o=22`). Each asserts vita == the Phase-0 iverilog golden.

```rust
#[test]
fn task_delay_suspends() {   // p_delay
    let src = "module t; task automatic dly(); #10; endtask\n\
        initial begin dly(); $display(\"o=%0t\",$time); $finish; end endmodule";
    let (out, ok) = run(src); assert!(ok,"{out}"); assert!(out.contains("o=10"),"{out}");
}
// + task_at_wait (o=15), task_nba_from_task (o=a5), task_seq_drivers (o=22)
```

- [ ] **Step 2: Run to verify they fail** (Phase 2 admitted only V4-only tasks; timing tasks still loud or mis-run). Expected: FAIL.

- [ ] **Step 3: Implement** — admit timing tasks into the sidecar; make Delay/Wait resume against the call-stack top; fix window lifetime (pop only at Return). This is the highest-risk step — do it minimally and lean on the golden gate.

- [ ] **Step 4: Full suite + oracle matrix** (p_delay/p_at/p_nba/p_seq all byte-match iverilog). 3710+ green. Golden hash unchanged.

- [ ] **Step 5: Commit.** `V3/V4 Phase 3: suspend/resume across a task frame (V3 timing works)`

---

## Phase 4 — Recursion, fork, disable, wait-fork

**Deliverable:** the general cases — recursion-with-timing (p_recur `o=35`), fork-in-task (p_fork `o=15`), `disable` of a suspended task, `wait fork` in a task.

### Task 4.1: Recursion-with-timing

**Files:** Modify: `crates/sim-engine/src/exec.rs` Call arm (per-call window push already supports nesting) + `sched.rs` (call-stack depth guard shares `MAX_CALL_DEPTH`). Test: `suspendable_tasks.rs`.

- [ ] **Step 1: Failing test** p_recur → `o=35`.
- [ ] **Step 2: Verify fail.**
- [ ] **Step 3: Implement** — confirm each recursive Call pushes a distinct window+FrameRec; Return pops one level; a runaway recursion hits `MAX_CALL_DEPTH` loud (not infinite).
- [ ] **Step 4: Suite + oracle** (`o=35`). Golden unchanged.
- [ ] **Step 5: Commit.** `V3/V4 Phase 4a: recursion with timing`

### Task 4.2: fork / disable / wait-fork in a task

**Files:** Modify: `crates/sim-engine/src/exec.rs` Fork arm (`:374`) + disable handling to operate on a call-stack frame. Test: `suspendable_tasks.rs`.

- [ ] **Step 1: Failing tests** p_fork (`o=15`) + a `disable`-a-suspended-task test + a `wait fork` in a task test (each with its iverilog golden from an added Phase-0 probe).
- [ ] **Step 2: Verify fail.**
- [ ] **Step 3: Implement** — a `fork` inside a task frame spawns children whose barrier resumes the task frame; `disable <task-scope>` unwinds the call-stack to that scope; `wait fork` parks the frame. Any shape that genuinely can't be modeled stays **loud**, not silent.
- [ ] **Step 4: Suite + oracle.** Golden unchanged.
- [ ] **Step 5: Commit.** `V3/V4 Phase 4b: fork/disable/wait-fork in a task`

---

## Phase 5 — Adversarial verification + docs + reviewer TBs

### Task 5.1: Adversarial 2-lens

- [ ] **Step 1:** Differential agent — broad probe sweep (20+ diverse task/timing/NBA/fork/recursion/disable combinations) diffed vs iverilog; report any rc=0-both value divergence.
- [ ] **Step 2:** Soundness agent — review the scheduler diff for: window double-pop / early-pop, call-stack not preserved across a specific suspend path, subset fast-path regression, disable-unwind leaks, fork-child call-stack isolation.
- [ ] **Step 3:** Fix every confirmed finding (each is its own micro-commit with a regression test). Re-run both lenses until CLEAN/SOUND.

### Task 5.2: Reviewer testbench smoke

- [ ] **Step 1:** If the reviewer's `tb/*.sv` are reachable, run `vita rtl/... tb/tb_sha3.sv` and confirm it advances past the frame-call wall (may surface the NEXT layer — V2A/V5 — which are separate plans). Record how far it gets.

### Task 5.3: Docs + slice close

- [ ] **Step 1:** ARCHIVE §4.5.x entry (Parts, adversarial outcome, lessons); ROADMAP §3 — mark V3/V4 RESOLVED; DEVLOG bullet; REMAINING_WORK baseline; CLAUDE.md/LOOPROMPT.md (gitignored) status.
- [ ] **Step 2:** Update the spec's §1 status → implemented.
- [ ] **Step 3:** Final gates: `cargo test --workspace --locked` (green), clippy, fmt, golden hash unchanged (or a justified bump).
- [ ] **Step 4:** Commit the whole feature branch, ff-merge to main, **ask the user before pushing** (PUBLIC repo).

---

## Self-Review (author checklist — completed)

1. **Spec coverage:** spec §1 (V3/V4) fully mapped — classifier/sidecar/lowering (Phase 1), call-stack/Call-Return (Phase 2), suspend/resume (Phase 3), recursion/fork/disable (Phase 4), verification+docs (Phase 5). Spec §1.6 format-stability is a phase gate in every phase. Companion items (spec §2-7) are OUT OF SCOPE of this plan — they get their own plans (spec §0 ordering), as the writing-plans scope-check requires.
2. **Placeholder scan:** tests are concrete (real .sv + iverilog-verified expected outputs). Engine-internal steps name real files/functions/lines; where the exact struct code is discovered during implementation, the step specifies the precise change ("add field X to struct Y at file:line", "branch on call_stack.is_empty()") plus the test that pins it — not "TBD".
3. **Type consistency:** `FrameRec { cfg, bb, window }`, `FrameCfg { Process, Task(u32) }`, `FrameClass { Subset, NonSubset }`, `suspendable_tasks: Vec<u32>`, `classify_frame_body` — used consistently across tasks.

**Known plan characteristic (deep-engine feature):** Phases 2-4 modify the scheduler hot path. The concrete *tests* are the fixed contract (iverilog goldens); the exact engine code for the call-stack/window-lifetime is refined against the golden gate during execution (spec §9 open questions #1-2). Each phase gate (full suite + golden hash + oracle) prevents drift. If Phase 3's window-lifetime step reveals the model needs a frozen-IR field, STOP and escalate (do not bump silently).
