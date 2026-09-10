# Design: `fork…join[_any|_none]` inside a suspendable task ("fork-in-frame")

- **Date**: 2026-07-24
- **Status**: approved (brainstorming) → implementation-planning
- **Owner track**: ROADMAP §0 NEXT (the one round-18 report item, C1 part 2, kept correct-or-loud LOUD)
- **Format impact**: none — all changes are runtime engine types + elaborate-transient sidecars. `format_version` stays 23; SimIr golden root unchanged.

## 1. Problem

A `fork … join` / `join_any` / `join_none` spawned **inside a suspendable task body** is loud today (E3009):

```systemverilog
task automatic a; repeat (2) @(posedge clk); endtask
task automatic b; repeat (2) @(posedge clk); endtask
task automatic run;
  @(posedge clk); fork a(); b(); join $display("PASS");
endtask
initial begin run(); $finish; end
```

iverilog runs this (it is a real differential **oracle**). vita rejects it. This was the only round-18 report family not brought to correct-support, because the naive framing ("concurrent children must share the parent's single-owner move-based frame window; `exec_fork` assumes a top-level process") looked like a deep scheduler rewrite with blast radius across the whole frame subsystem.

## 2. Current architecture (what actually exists)

Two findings from reading the engine + elaborate reshape the problem:

### 2.1 Elaborate already lowers fork-in-frame

`stmt_main.rs::Fork` lowers a fork inside a task body into a proper `Terminator::Fork` in the **global `func_blocks` arena** (a frame task's blocks live there), with global block ids for each arm entry, the `join` sentinel, and `resume_bb`. Arms are lowered via `lower_stmt` into global blocks. Fork-local decls "share the enclosing scope" (`stmt_main.rs:777`) — folded into the enclosing task's frame-local net range.

The lowering is complete. It is gated loud by exactly two places:

1. **Elaborate**: `frame_body_is_leaf_nonsuspending` (`frames_classify.rs:530`) returns `false` on **any** `Fork` terminator → the task fails the lift condition (`frames_classify.rs:315`) → E3009.
2. **Engine**: `Terminator::Fork` with `in_frame` → `mark_fatal` (`exec/process.rs:325`). A defense-in-depth backstop; never reached because elaborate rejects first.

### 2.2 The frame-window model, and why it is not the blocker it looked like

Automatic windows live on a **shared** `frame_stack: RefCell<Vec<Vec<Value>>>`; the top is the current frame. `frame_slot_read/write(func, automatic, slot)` read/write `frame_stack.last()[slot]`. On suspend, `stash_frame_windows(pi)` pops **this activity's** windows off the shared stack into their `FrameRec.window`; on resume, `restore_frame_windows(pi)` pushes them back.

The scheduler is **single-threaded**: exactly one activity executes at any instant, and the stash/restore discipline guarantees only the *running* activity's windows sit on `frame_stack`. Therefore concurrent fork children + a parked parent **never co-reside** on the stack — they take turns. The "single-owner move-based window" is only a problem when a child must reach the parent's window **while the parent is parked** (its window stashed in a *different* activity's `FrameRec`).

### 2.3 Each task activation has exactly one window

`reserve_frame_local_decl` reserves **all** of a frame task's storage — declared locals, begin-block locals, and fork-arm locals (which fold into the enclosing scope) — into that task's single frame-window net range `[base_net, base_net + locals_len)`. So "share the parent window" means sharing one flat `Vec<Value>`, not a tree of scopes.

## 3. The Case A / Case B split

- **Case A** — every fork arm is *self-contained*: it does not read or write any net in the enclosing task's frame-local range `[base_net, base_net+locals_len)`. (The report's `fork a(); b(); join` — no args, separate tasks — is Case A.) Works on the **existing owned-window model**: the parent's window sits stashed in its own `FrameRec` while children run; children never touch it; the parent restores it when the join resumes.
- **Case B** — at least one arm reads/writes a parent frame-local. The arm needs the parent's window while the parent is parked → the window must move to a **shared, interior-mutable arena** that parent and children reference by handle.

The boundary is decidable at elaborate: walk each arm's reachable blocks (not descending into called tasks — those have their own frames) and check for a net in `[base_net, base_net+locals_len)`.

## 4. Window-model approach (3 considered)

**① Two-model split + interior-mutable window arena — CHOSEN.** Keep today's owned `Vec<Value>` windows byte-identical for the common/Case-A path. Route only Case-B tasks to a `RefCell`-backed window **arena** — the exact interior-mutable-heap pattern already used for `dyn_heap` (§4.5.194) and `class_heap`. Refcount the arena slot for `join_none`/`join_any` lifetime. A per-func `FuncMeta` flag (threaded to the engine like `has_hier_call`) selects the model.

**② Uniform `Rc<RefCell<Vec<Value>>>` windows — rejected.** Sharing and refcounting come free, but every task/function pays `Rc` clone+drop on every enter/exit/stash/restore — a hot-path regression for a rare feature, touching the whole frame subsystem uniformly.

**③ Per-activity frame chains (windows on each `Activity`, off the shared stack) — rejected.** `SimState`'s frame accessors do not know `cur_aid`, so this re-plumbs every frame read/write; Case B still needs sharing on top. More disruption, no upside over ①.

## 5. Staged delivery

Each stage is independently verifiable, and anything a stage has not yet reached stays **LOUD** (never silent) — so partial delivery is always correct-or-loud.

| Stage | Scope | Window model | Lifetime rule |
|---|---|---|---|
| **1** | Case A: `join`/`join_any`/`join_none` of self-contained arms | owned (unchanged) | parent owns; window stashed in its `FrameRec` while parked |
| **2** | Case B for **`join`** (join-all) | shared arena window | all children complete before the parent resumes → parent frees on `Return` |
| **3** | Case B for **`join_any`/`join_none`** (a surviving child may outlive the parent) | shared arena + **refcount** | window freed when the last referencing `FrameRec` (parent or child) drops |

Stage 1 alone fully closes the reported C1 repro.

## 6. Engine changes (runtime types only)

### 6.1 Storage
- `frame_stack: RefCell<Vec<WindowSlot>>` where `enum WindowSlot { Owned(Vec<Value>), Shared(u32) }`. Common path = `Owned` (storage byte-identical; one predictable match arm added to `frame_slot_read`/`frame_slot_write`).
- New `frame_windows: RefCell<Vec<Option<Vec<Value>>>>` arena, `frame_window_rc: Vec<u32>` refcounts, and a `frame_window_free: Vec<u32>` free-list (all `Shared(handle)` indexes the arena).
- `frame_slot_read(func, true, slot)` becomes: `match frame_stack.last() { Owned(v) => v[slot].clone(), Shared(h) => frame_windows[h].as_ref()[slot].clone() }`. Symmetric for write. Borrow discipline: the arena `borrow()`/`borrow_mut()` is scoped to the single index op, never held across a nested eval (same rule the current code follows for `frame_stack`).

### 6.2 `exec_fork` becomes frame-aware
- When the parent is `in_frame`: stash the parent's windows (as `Delay`/`Wait` in-frame already do), then park it on the barrier whose `join_bb`/`resume_bb` are the fork's **global** block ids.
- Spawn each child as an **in-frame** activity: its `call_stack` starts with one synthetic **arm-`FrameRec`** (`callee` = parent FuncId, `bb` = arm entry, `ret_bb` = the `join_bb` sentinel, `out_binds` = []). Window:
  - Case A → `Owned(empty)` (the arm touches no frame slot).
  - Case B → `Shared(parent_handle)`, and `frame_window_rc[parent_handle] += 1`.
- Child tie: the parent activity running the frame is a **top-level** activity (its `tie` is the dense top-level declaration index < 0xFFFF), so `compose_child_tie` is unchanged. A fork nested inside another fork's frame keeps the existing `in_fork` elaborate error (Stage-independent LOUD).

### 6.3 Child-completion intercept (in-frame)
`run_process`'s loop-top intercept (`exec/process.rs:59`) is `!in_frame`-gated today. Extend it: an in-frame child completes when its (single) arm-`FrameRec`'s `bb` reaches the barrier's `join_bb` sentinel (gated on `call_stack.len() == 1` so an inner callee frame reaching a same-valued id cannot mis-fire). On completion, for a `Shared` arm-window (Case B): **Stage 3** decrements `frame_window_rc[handle]` and frees the arena slot at zero; **Stage 2** does nothing here (join-all guarantees every child has completed before the parent resumes, so the parent's free-on-`Return` is safe on its own).

### 6.4 Parent resume (in-frame)
`on_child_complete`: when the parent is in-frame, set the parent frame's `bb = resume_bb` and re-enqueue. On resume, `run_process` restores the parent's stashed window (`Owned` for Case A, `Shared(handle)` for Case B) and continues in-frame. Case-B window free:
- **Stage 2 (`join`)**: all children have already completed, so the parent frees its arena window directly when its frame `Return`s.
- **Stage 3 (`join_any`/`join_none`)**: the free is refcounted — the parent's `Return` decrements `frame_window_rc[handle]`, and the slot is freed only at zero, correctly deferring past any still-live surviving child.

### 6.5 Fork terminator arm
Replace the `in_frame → mark_fatal` guard (`exec/process.rs:325`) with the frame-aware `exec_fork`. On `JoinMode::None`/zero children → set the frame `bb = resume_bb` and continue; else park (return `Step::Suspended`).

## 7. Elaborate changes

- Replace `frame_body_is_leaf_nonsuspending`'s blanket `Fork => return false` (`frames_classify.rs:530`) with a fork classifier:
  - **admit** a fork whose arms are all self-contained (Case A) — every stage;
  - **admit** a Case-B fork per the stage gate (Stage 2: `join` only; Stage 3: also `join_any`/`join_none`);
  - **reject (LOUD)**: nested fork (already the `in_fork` error), `wait fork` / `disable fork` in a frame body (a documented Phase-4 follow-on), and any Case-B/join-mode combination the current stage has not reached.
- Per-fork arm analysis (§3): walk each arm's reachable blocks (stop at `Call` `ret_bb`, do not descend into callees) for a net in `[base_net, base_net+locals_len)`. Any hit → Case B.
- `FuncMeta.contains_shared_fork: bool` (default false), set when a task contains a Case-B fork; threaded to the engine verbatim like `has_hier_call` so `enter_task_frame` allocates an arena window for that task. Because it rides `FuncMeta` (out-of-band sidecar, staged trailer), it does not touch the SimIr golden.

## 8. Correct-or-loud boundaries (stay LOUD, never silent)

- Nested fork (fork inside a fork arm) — existing `in_fork` elaborate error.
- `wait fork` / `disable fork` inside a frame body — Phase-4 follow-on; the engine's `WaitCause::Fork` in-frame guard (`exec/process.rs:249`) already fatals, and elaborate rejects at the classifier.
- Case-B `join_any`/`join_none` before Stage 3.
- Any arm construct already loud in a non-fork frame body (frame-local unpacked array read via 1-bit select, NBA to a frame-local, etc.) — the existing `frame_task_has_unsafe_construct` walk applies to arm blocks unchanged.

## 9. Determinism & golden stability

- Arena handle alloc/free is a pure function of the (deterministic) execution order; handles are internal ids. VCD/stdout bytes are unchanged — the same argument that already licenses `free_activities` / `free_barriers` recycling.
- No serialized type changes: `WindowSlot`, `frame_windows`, refcounts are runtime engine state; `FuncMeta.contains_shared_fork` is an elaborate sidecar. `format_version` stays 23; no `.velab` regeneration.

## 10. Verification

- **Differential vs iverilog (oracle)**: all three join modes; timing interleave (arms suspend on different edges); `join_any` surplus children draining as background; `join_none` background completion; **Case-B shared-variable visibility** (an arm's write to a parent local is seen by the parent after join and by siblings). Parent continuation after join runs exactly once.
- **Correct-or-loud adversarial**: nested fork, `wait fork`/`disable fork` in-frame, and each not-yet-reached stage form assert E3009.
- **Regression**: full `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check` per stage. The owned-window path must stay byte-identical (existing frame/task/function tests unchanged).
- **Isolation**: all work in a dedicated `git worktree` so the mainline checkout is untouched during the multi-stage change.

## 11. Risks & mitigations

- **Refcount use-after-free (Stage 3)** — the sharpest silent-wrong risk. Mitigation: debug-assert `rc > 0` on every arena access and `rc == 0` before free; if any construct's lifetime cannot be proven safe, keep it LOUD rather than guess. Stages 1–2 carry no refcount.
- **Intercept mis-fire for a deeply nested in-frame child** — a child whose arm called a task must only complete when the *arm* frame (not an inner callee frame) reaches `join_bb`. Mitigation: gate the in-frame intercept on `call_stack.len() == 1` (arm frame is the sole frame) AND `bb == join_bb`.
- **Hot-path `WindowSlot` match cost** — one predictable branch on `Owned`; measured against the existing frame benchmark before/after Stage 1.
- **Block-local `contains_shared_fork` under-detection** — if the arm-analysis misses a parent-local reference, a Case-B fork would run on the owned model and the child would read a stale/empty window (silent-wrong). Mitigation: the analysis is conservative (any net in range, read or write, counts); cross-check with an adversarial test where an arm reads a parent local and the owned model would visibly diverge.
