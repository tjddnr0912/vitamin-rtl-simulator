# `dyn_heap → RefCell` — interior-mutable dynamic heap for the `&self` frame executors

**Goal:** unblock the four round-16 *executor-bound* gaps (V2A-dyn non-suspendable task, V2B function output/inout dyn formal, V5 function-body dyn-local + `new[]`, framed-function nested/recursive dyn call) by making the dynamic-storage heap interior-mutable so the synchronous `&self` executors (`run_frame_call`, `run_task`) can perform heap operations — the same capability the `&mut` suspendable path (`run_process`) already has.

**Approach (chosen by user 2026-07-21):** convert `SimState::dyn_heap: Vec<Option<DynObj>>` → `RefCell<Vec<Option<DynObj>>>`, mirroring the existing `class_heap: RefCell<BTreeMap<u32, ClassObj>>` which already lets `class_alloc(&self)` / `class_field_write(&self)` mutate the class heap from the expression read-path. Rejected alternative: making only `run_task` `&mut` + hoisting function calls to statements — it leaves in-expression function heap-ops loud and does not satisfy V5 in expression position.

**Tech stack:** Rust (edition 2021, MSRV 1.85). No new deps. `format_version` stays **22** (dyn_heap is engine runtime state, never serialized into the SimIr golden root — verify golden tests unchanged).

---

## Background: the `&self` / `&mut` heap boundary

Confirmed by investigation (3 agents + a full access-site survey):

- `dyn_heap: Vec<Option<DynObj>>` (state.rs:245) is a plain, NetId-keyed flat Vec → mutating it requires `&mut self`.
- `frame_stack` / `static_store` / `class_heap` are `RefCell` → writable from `&self`. That is why the `&self` executors can write frame slots and even `class_alloc`, but **not** the dyn heap.
- Two executor trees:
  - `run_frame_call` (pure value-returning FUNCTION) is hard-`&self`: it is reached only through `EvalCtx { nets: &'a N }` (expression evaluation). Cannot become `&mut` without rewriting the hot-path evaluator carrier.
  - `run_task` / `run_task_call` (synchronous/subset TASK + output-formal functions) is `&self` only by symmetry; both call sites already sit in `run_process(sched: &mut Scheduler)`.
- The suspendable `run_process` (`&mut`) does all heap ops via its WRITE-phase `apply_effect(&mut K)`, plus the frame-local dyn lifecycle: `frame_dyn_reentry_ok` (reentry fatal-loud), `frame_dyn_free` (free-at-exit), `frame_dyn_snapshot_formals` (pass-by-value formal deep-copy).

Making `dyn_heap` interior-mutable lets **both** executor trees mutate the heap from `&self`, so no evaluator change and no call-hoisting are needed; in-expression function heap-ops become supported.

---

## The conversion contract

### C1. Field type
`state.rs:245`: `pub dyn_heap: Vec<Option<DynObj>>` → `pub dyn_heap: std::cell::RefCell<Vec<Option<DynObj>>>`. Init at `state.rs:711` wraps in `RefCell::new(...)`.

### C2. `dyn_entry` API change (the one architectural HIGH-risk site)
`dyn_entry(&mut self, net, init) -> &mut DynObj` (state.rs:1975) returns a `&mut` **into** the Vec. Under `RefCell` that would keep the `RefMut` guard alive at the call site and panic (`BorrowMutError`) the moment a caller re-touches `dyn_heap` (the queue-push callers immediately call `enforce_queue_bound`). Replace with a **closure-scoped** form that drops the guard before returning:

```rust
fn with_dyn_entry<R>(&self, net: usize, init: impl FnOnce() -> DynObj,
                     f: impl FnOnce(&mut DynObj) -> R) -> R {
    let mut g = self.dyn_heap.borrow_mut();
    if g[net].is_none() { g[net] = Some(init()); }
    f(g[net].as_mut().unwrap())
}   // guard dropped here, BEFORE the caller's next dyn_heap touch
```

Three callers rewrite to compute their result inside the closure, then release, then call `enforce_queue_bound`:
- `dyn_write` queue append (state.rs:2081-2107)
- `dispatch` QPushBack/Front (builtins.rs:298-310)
- `dispatch` QInsert (builtins.rs:373-379)

### C3. Receiver relaxations (dyn_heap-only mutators → `&self`)
After C1, these touch only `dyn_heap` (+ already-interior-mutable state) and become `&self`:
`dyn_write` · `assoc_write` · `assoc_str_write` · `enforce_queue_bound` · `frame_dyn_reentry_ok` · `frame_dyn_free` · `frame_dyn_snapshot_formals`. (`&mut`-context callers still call them fine.) Verify `fatal_run` used by `frame_dyn_reentry_ok` is already `&self` (same latch family as `fatal_frame_heap_write`, which is `&self`).

### C4. Must stay `&mut` (also write the plain net store `self.nets: Vec<NetSlot>`, state.rs:163)
`write_chunk` (writes `&mut self.nets[net]` at state.rs:1287) · `write_lvalue` · `assoc_iter_step` (writes the iteration-key net) · Scheduler-level `dispatch` / `run_queue_slice` / `arr_locator`. **Therefore the `&self` executors must NOT reach dyn writes through `write_lvalue`** — they route through a dyn-only `&self` entry (see C5).

### C5. `&self` dyn write/alloc entry for the frame executors
`frame_write_lvalue(&self)` currently *fatals* on a dyn-handle write (`fatal_frame_heap_write`, state.rs:2398). Replace that fatal with a real dyn write by calling the now-`&self` `dyn_write` directly (bypassing `write_chunk`'s plain-net path). Add a `&self` `dyn_new` helper (mirrors builtins.rs:190) so a frame body can execute `loc = new[N]` (SysTaskId::DynNew) without a `&mut Scheduler`. Likewise a `&self` mini-dispatch for the heap SysTasks a function/task body can legitimately contain (DynNew, DynDelete, queue push/pop, assoc delete) — added incrementally as each gap needs it.

### C6. Borrow discipline (mandatory, prevents `BorrowMutError`)
1. **Reads clone-and-release** — never hold a `borrow()` across a nested call/eval (existing `frame_stack` rule; `dyn_read`/`dyn_size`/`dyn_values` already do this).
2. **Writes scope the `borrow_mut()` to the shortest span** — compute any warn/order/decision first, drop the guard, then act. Do NOT hold a guard across `dyn_warn_once_at`, `enforce_queue_bound`, `apply_order`, or any `self.`-call.
3. The MEDIUM survey sites (dyn_write match arms, sort/delete/putc) are safe *because* their nested calls (`dyn_warn_once_at` → `dyn_warned`; `apply_order` = free fn) never touch `dyn_heap` — preserve that invariant; if in doubt, scope-to-single-statement.

---

## Phased plan

### Phase 0 — RefCell plumbing, behavior-preserving (no gap fixed yet)
Convert C1, rewrite `dyn_entry`→`with_dyn_entry` (C2), fix all 47 access sites to borrow-scoped access, relax C3 receivers. **Zero behavior change.** Gate: full `cargo test --workspace --locked` stays **3944 green**, golden/format_version unchanged, clippy+fmt clean. This is the risky-but-mechanical foundation; lock it before building features.

### Phase 1 — synchronous TASK heap ops → V2A-dyn, V2B
In `run_task`: replace the dyn `fatal_frame_heap_write` with a real `&self` dyn write (C5); add frame-dyn lifecycle to the run_task entry/exit (reentry guard + formal snapshot for a dyn `input`/`output` formal + free-at-exit), mirroring `run_process`. Remove the elaborate E3009 gates that rejected a subset-task dyn formal (elaborate/lib.rs:16169-16186) and a function output/inout dyn formal. Adversarial-verify V2A-dyn (non-suspendable task) and V2B (function output/inout dyn).

### Phase 2 — pure FUNCTION heap ops → V5, framed-nested
In `run_frame_call`: allow the heap SysTasks (`new[]`, dyn element write, queue/assoc ops) in a function body — add the `&self` mini-dispatch (C5) and the frame-dyn lifecycle for function-body dyn LOCALS. Remove the elaborate cut that rejects SysTask/dyn-mutation in a function body, and the E3009 gates for V5 / framed-nested / function-in-expression dyn ops. Because dyn_heap is now interior-mutable, this works **in expression position** with no hoisting. Adversarial-verify V5 (`function; int loc[]; loc=new[N]; ... return loc[i];`) and framed nested/recursive dyn calls.

---

## Correct-or-loud boundary (what stays loud after all phases)
- Recursion/concurrency on a frame-local dyn array → `frame_dyn_reentry_ok` fatal (unchanged, never silent).
- Anything genuinely outside the integral/dyn model the executors can express (multi-dim/packed dyn element types, etc.) stays loud as before.
- No silent-wrong is introduced: every heap op the frame executors cannot do is either now supported (this work) or still a loud fatal — verified by keeping the existing "stays-loud" tests and re-pointing only the ones this work promotes.

## Test strategy
- Adversarial 2-lens per CLAUDE.md: **differential** (live iverilog; macOS has no `timeout` → wrap every `iverilog`/`vvp`/`vita` with `perl -e 'alarm N; exec @ARGV' …`) + **soundness** (hand-IEEE). Dyn subroutine formals/locals are largely **no-oracle** in iverilog 13.0 (it rejects unpacked/dyn subroutine ports) → verify by hand-IEEE self-consistency (element-wise reference; `arr[i]` == manual layout) and any iverilog-pinnable subset.
- Per-gap new test files under `crates/cli/tests/`; keep every existing test green (3944 baseline). Re-point (not delete) the tests that currently assert loud for a now-supported gap.

## Risks
- **`BorrowMutError` panic** from a held guard across a re-borrow — mitigated by C6 discipline + the full suite as regression + the survey's explicit HIGH/MEDIUM inventory.
- **Hot-path read overhead** — `dyn_read` now goes through `borrow()`; a cheap refcount check, precedented by `class_heap`. Acceptable.
- **Format stability** — dyn_heap is never serialized; assert golden/format_version 22 unchanged in Phase 0.
