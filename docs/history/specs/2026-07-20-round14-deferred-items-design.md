# Round-14 Deferred Items — Design Spec

**Date:** 2026-07-20 · **Status:** design (pre-implementation) · **Author:** paired session
**Scope:** every item deferred out of the round-14 slice (§4.5.167), designed for implementation so no item needs to be re-requested.

> **Governing principle (user directive, 2026-07-20):** implementation demand is
> **always assumed to exist**. The absence of a differential oracle (iverilog rejecting a
> *valid* LRM construct) is **not** a reason to keep-loud/defer — such items are designed
> to the IEEE 1800 LRM (hand-IEEE) and verified with internal differential. `correct-or-loud`
> keeps *loud* only for genuinely out-of-scope or ill-formed programs, never as a resting
> place for a valid construct. G1 targets vcs/xcelium grade, which support all of these.

## 0. Item inventory & ordering

| # | Item | Kind | Oracle | Depth | Format impact |
|---|---|---|---|---|---|
| **1** | **V3/V4** — `@`/`#`/wait/NBA/$systask in a task body (suspendable tasks) | engine + elaborate | iverilog ✓ | **DEEP (architectural)** | none (22) — see §1.6 |
| 2 | V2A — task `input` dyn-array formal | elaborate | iverilog ✓ | moderate | none expected |
| 3 | V5 — frame-local dyn-array + `new[]` | elaborate + engine | iverilog ✓ | sidecar (TBD) |
| 4 | static-task inline `string` local | elaborate + engine | iverilog ✓ | none |
| 5 | V2B — function `output`/`inout` dyn-array formal | elaborate | **hand-IEEE** (iverilog rejects) | none expected |
| 6 | V6 — queue/unpacked-array of unpacked struct | parser + elaborate + engine | **hand-IEEE** | **likely bump** (TBD) |
| 7 | V8 — task-local unpacked-struct var decl | parser + elaborate | **hand-IEEE** | none expected |

**Dependency / ordering.** The items are largely independent, but V3/V4 is the **gate** for
the reviewer's four testbenches (every KAT driver is `task drive(...); @(posedge clk); sig<=v;`).
Recommended order: **1 (V3/V4) → 2 (V2A) → 3 (V5) → 4 (static-string) → 5 (V2B) → 7 (V8) → 6 (V6)**.
V6 (unpacked-struct containers) is last because it is the widest storage-model change and the
one most likely to bump `format_version`; sequencing it last keeps the golden re-pin isolated.

Each item below is self-contained: **Problem → Root cause (file:line) → Design → Format impact
→ Verification → Risk → Effort**. V3/V4 gets full architectural treatment; the rest are scoped.

---

## 1. V3/V4 — Suspendable tasks (architectural core)

### 1.1 Problem

A task whose body contains a timing/suspend control (`@`, `#delay`, `wait`, `wait fork`),
a non-blocking assign (`sig <= v`), a `$systask` (`$display`, …), a `force/release`, or a
`fork` is loud-rejected:

```
E3009: frame function/task `wait2` body uses a timing/suspend/fork control (…),
       which is outside the frame-call subset
```

iverilog accepts all of these (verified: V3A/V3B/V4A/V4B all PASS). This is the reviewer's
**①** and gates all four testbenches.

### 1.2 Root cause

vita has **two** subroutine executors:

- **`run_process`** (`crates/sim-engine/src/exec.rs:224`) — the `&mut Scheduler` loop for
  processes (`initial`/`always`). It handles **every** terminator, and **suspends** on
  `Delay` (`:318`, `schedule_resume`) and `Wait` (`:332`, `suspend_on`), resuming at the
  terminator's `resume` block. It applies **all** statement effects (NBA, `$display`, …) via
  `compute_effect`/`apply_effect`.
- **`run_task` / `run_frame_call`** (`crates/sim-engine/src/state.rs:2596` / `:2540`) — a
  synchronous `&self` loop over the **func arena** (`self.ir.blocks`). It handles **only**
  `BlockingAssign` statements and `Goto`/`Branch`/`Return` terminators; `Delay`/`Wait`/`Fork`/
  `Call` and any `SysTask`/NBA statement `break` defensively (`state.rs:2570-2573`) because
  `validate_frame_body` (`crates/elaborate/src/lib.rs:17279`) already loud-rejects them at
  elaborate.

Even when a task is called *from a process*, the scheduler's `Terminator::Call` arm
(`exec.rs:390`) evaluates inputs then calls the **synchronous** `run_task_call` and writes the
outputs back — it never yields to the scheduler. **A task call runs to completion within one
instant.** That is the entire reason `@`/`#`/NBA/`$systask` are illegal in a task body.

### 1.3 Design — suspendable frame call via an activity call-stack

**Chosen approach (of three).**

| Approach | Verdict |
|---|---|
| **A. CFG splice** — inline the task's blocks into the caller process's CFG at elaborate time | Rejected: cannot express recursion (infinite splice); duplicates blocks. Fails the "fully general" requirement. |
| **B. Suspendable frame call** — the scheduler activity carries a call-stack; `run_process` executes the callee frame's CFG and suspends normally | **Chosen.** Reuses the frame model (automatic windows already handle recursion/reentrancy) *and* the suspend machinery. Subsumes A. |
| C. Extend the synchronous executor to schedule/suspend | Rejected: a synchronous `&self` loop fundamentally cannot yield to the `&mut` scheduler. |

**Core idea.** Classify each task as **subset** (blocking-assign + control-flow + nested
subset-calls only — today's frame-call subset) or **non-subset** (contains timing / NBA /
`$systask` / force-release / fork). Subset tasks keep the fast synchronous `run_task_call`
(zero change). A **non-subset** task is executed by `run_process` as a *sub-context of the
calling process*, so its `Delay`/`Wait` suspend the caller and its NBA/`$systask` run natively.

**Because `run_process` already handles every terminator and statement, routing a non-subset
task's CFG through it solves V3 (timing) and V4 (NBA/$systask) with one mechanism.**

**Mechanism — the activity call-stack.** Today a process activity is essentially
`(template, bb)`. Generalize it to carry a **call-stack** of frame records:

```
FrameRec { cfg: FrameCfgRef,   // which CFG is executing (process body OR func-arena task body)
           bb: u32,            // current block in that CFG
           window: FramePtr }  // this call's automatic-window / slot base
```

`run_process` operates on the **top** of the stack. New/changed behaviour:

1. **Enter a non-subset task (`Terminator::Call`).** Push a `FrameRec` for the callee task
   body (its func-arena entry block + a freshly-pushed automatic window), copy inputs into the
   window's input-formal slots, and continue the `run_process` loop against the callee CFG.
   (Subset tasks keep the old synchronous `run_task_call` — the sidecar tells them apart.)
2. **`Delay`/`Wait` inside a task frame.** Handled by the *existing* `run_process` arms — the
   whole activity (with its call-stack) is suspended and `schedule_resume`/`suspend_on` fire.
   On resume, `run_process` re-enters at the call-stack top; the window persists (see #4).
3. **`Return` from a task frame.** Pop the `FrameRec`, copy `output`/`inout` slots to the
   caller-frame lvalues, release the window, and continue at the caller's `ret_bb`.
4. **Window lifetime.** For the synchronous path a window is pushed at entry and popped at end
   of run. For the suspendable path the window's lifetime is **the call-stack record's
   lifetime** — pushed at Call, popped at Return, *persisting across suspends*. `frame_stack`
   already stores automatic windows; the change is *who* pops and *when*.
5. **Frame-slot access.** `read_net`/`write_*` already route frame-local nets through
   `frame_slot_read`/`frame_slot_write` (`state.rs:3076`) using the active window — so once the
   call-stack top's window is installed as current during `run_process`, all frame-slot reads/
   writes in the task body work unchanged.

**Recursion & reentrancy fall out for free.** A recursive Call pushes another `FrameRec` +
window (automatic windows are already per-activation). Concurrent reentrancy via `fork` gives
each child its own activity → its own call-stack → its own windows. `disable` unwinds the
call-stack to the named scope. No special cases.

**Functions are excluded.** IEEE 1800 forbids timing controls in functions, so functions stay
on the synchronous path — narrowing the change to tasks only.

### 1.4 Components

**Elaborate (`crates/elaborate/src/lib.rs`):**
- `validate_frame_body` (`:17279`) — split into a *classifier* returning `Subset | NonSubset`
  instead of an unconditional reject. Non-subset ⇒ still lower the task body into the func
  arena, but now **emit the real `Delay`/`Wait`/`Fork` terminators and NBA/`$systask`
  statements** (the func arena's block type already admits every `Terminator`/`Stmt` variant —
  see §1.6). Loud stays only for genuinely unsupported shapes (e.g. a construct neither executor
  can model).
- A **`suspendable_tasks` sidecar** (set of task template ids) computed here and handed to the
  engine out-of-band (same pattern as `task_calls_proc`, `func_metas`).

**Engine (`crates/sim-engine/`):**
- `Activity`/scheduler state (`exec.rs`, `state.rs`) — add the call-stack.
- `run_process` (`exec.rs:224`) — generalize the block fetch to read the *current frame's* CFG
  (process body or func-arena task body); make the `Terminator::Call` arm (`:390`) push a frame
  for a suspendable callee instead of calling `run_task_call`; add a `Return`-pops-frame path
  for in-task returns; ensure `Delay`/`Wait`/`Fork` preserve the call-stack on suspend.
- Window lifetime tied to `FrameRec` push/pop.

### 1.5 Subset fast-path (performance guard)

The vast majority of activities never enter a suspendable task; their call-stack stays empty
and `run_process` behaves byte-identically (one predictable branch on "is the current frame the
base process?"). No-timing tasks keep the synchronous `run_task_call`. This confines all new
cost to non-subset task calls.

### 1.6 Format-version impact — **none (stays 22)**

`Terminator` already has `Delay`/`Wait`/`Fork`/`Call`/`Return`; the func-arena block pool
(`self.ir.blocks`) already admits every variant. The change is (a) elaborate *emitting* richer
task-body CFGs it previously refused to, (b) an out-of-band `suspendable_tasks` sidecar, (c)
engine execution. **SimIr shape is unchanged ⇒ SchemaHash does not flip ⇒ zero golden churn.**
(This must be re-confirmed empirically in Phase 1: run the golden gate and assert the root hash
is stable. If a genuinely new IR field proves unavoidable, bump per the standard trailer rule.)

### 1.7 Verification

- **Differential (primary):** iverilog 13.0 is a real oracle here (V3/V4 all PASS). Drive the
  round12.sv V3A/V3B/V4A/V4B repros + a matrix: task with `@`/`#`/`wait`/`wait fork`; NBA to a
  module net vs a frame local; `$display`/`$monitor` in a task; nested non-subset task calls;
  **recursion with timing**; `fork` inside a task; `disable` of a suspended task; a task that
  both waits *and* returns a value via output; multiple sequential calls from one `always`.
  Every rc=0-both case must be byte-identical.
- **Adversarial 2-lens:** differential agent (broad probe sweep) + soundness agent (scheduler
  call-stack correctness: window lifetime across suspend, no double-pop, disable-unwind, fork
  child isolation, no regression on the subset fast-path).
- **Regression:** full suite green; the golden root hash unchanged (§1.6).

### 1.8 Risks & mitigations

| Risk | Mitigation |
|---|---|
| `run_process` is the hottest, most correctness-critical code | Subset tasks keep the *old* path untouched; empty call-stack ⇒ byte-identical behaviour. Land the call-stack plumbing as an isolated phase with the golden gate green before any suspend logic. |
| Window lifetime across suspend (early-pop = use-after-free of a slot; late-pop = leak) | Dedicated phase; model the window as owned by the `FrameRec`; assert-guard push/pop balance in debug. |
| `disable`/`fork`/`wait fork` interaction with an in-task suspend | Explicit test matrix; unwind the call-stack to the disabled scope. |
| Golden churn if an IR field sneaks in | Phase-1 gate asserts root hash stability before proceeding. |

### 1.9 Phasing

1. **Classifier + sidecar + richer task-body lowering** (elaborate) — the classifier and the
   `suspendable_tasks` sidecar land, and task-with-timing lowers to a func-arena CFG with real
   terminators; golden root hash asserted stable. **Correct-or-loud gate:** the existing E3009
   reject stays in force for non-subset tasks until Phase 2 can actually execute them — this
   phase never ships a task that would reach the synchronous executor and mis-run. The reject is
   lifted in the *same* change that wires the engine path (Phase 2), so there is no intermediate
   state where a non-subset task elaborates but runs wrong.
2. **Activity call-stack + `run_process` generalization** (engine) — subset behaviour
   byte-identical; suspendable Call enters a frame.
3. **Delay/Wait/Return-in-frame + window lifetime** — the actual suspend/resume across a task.
4. **Recursion / fork / disable / wait-fork** — the general cases.
5. **Adversarial verification + docs.**

---

## 2. V2A — Task `input` dynamic-array formal

**Problem.** `task automatic consume(input byte b[]); … b.size(); endtask` → E3009
"task `consume` has an unpacked-array formal — unsupported". iverilog ✓.

**Root cause.** The task-call path blanket-rejects any unpacked-array formal at
`crates/elaborate/src/lib.rs:17773` (`task.ports.iter().any(|p| !p.unpacked.is_empty())`),
*before* frame/inline dispatch. R11 built read-only input dyn-array formal support for
**functions** — `is_input_dyn_array_formal` (`:19039`) + the `dyn_subst` alias table (`:3527`,
bound in the inline input loop at `:17476-17482`) that maps the formal name to the caller's
`DynArray` NetId so body reads (`b[i]`, `b.size()`) resolve to the caller's heap array.

**Design.** Relax the `:17773` reject to permit an **`input`** dyn-array formal (keep
`output`/`inout` unpacked-array task formals loud → covered conceptually by V2B's copy-out for
the dyn-array case). Extend the `dyn_subst` binding used by the function inline path to the
**task** frame and inline paths (`emit_frame_task_call` and the inline task binding), so a task
body's read of `b` aliases the caller's `DynArray` handle exactly as a function's does.

**Format impact:** none expected (reuses the existing `dyn_subst`/DynArray machinery; no new IR).
**Verification:** iverilog differential (`consume`/`.size()`/element read/re-forward to a nested
task) + soundness (a stray *whole-array* write of `b` must stay loud, per the existing R2 note).
**Risk:** low-moderate — mirrors a proven function path. **Effort:** moderate.

---

## 3. V5 — Frame-local dynamic array + `new[]`

**Problem.** `function automatic int mk(input int n); byte loc[]; loc = new[n]; return loc.size();`
→ E3009 "`new[n]` assigns only to a dynamic-ARRAY handle". iverilog ✓.

**Root cause.** `new[n]` requires the LHS to resolve to a `DynArray` handle net via
`dyn_handle(name)` (`crates/elaborate/src/lib.rs:20006-20018`). A frame-local `byte loc[]` is
reserved as a 1-element unpacked-array frame slot (`frame_array_local`), **not** a `DynArray`
heap handle, so `dyn_handle` returns `None` → loud.

**Design.** Give a frame-local dynamic array (`[]` with no bound) a real `DynArray` handle
stored in its frame slot: at frame-local reservation, when the decl is a dynamic array, reserve
the slot as a `NetKind::DynArray` handle (like a module-scope dyn array) rather than a fixed
1-elem net; register it so `dyn_handle`/`new[]`/`.size()`/element access route to the heap. The
handle id lives in the frame slot; per-activation allocation follows the automatic-window model
(a fresh heap handle per call, freed at frame exit). This composes with V3/V4's window lifetime.

**Format impact:** likely an out-of-band **sidecar** (frame dyn-array slot ids), TBD at plan
stage; SimIr shape unchanged if so. **Verification:** iverilog differential (`new[n]`, `.size()`,
element read/write, resize) + soundness (heap freed per activation; no cross-call leak).
**Risk:** moderate (heap handle in a frame slot). **Effort:** moderate.

---

## 4. Static (non-`automatic`) task inline `string` local

**Problem.** `task g(...); string s; s = "..."; r = s.len();` in a **static** task →
E3018 (`t.$itask$g$L.s`). Static *functions* and *automatic* tasks already work (§4.5.167);
only the static-task inline path is left loud.

**Root cause.** `hoist_inline_task_locals` (`crates/elaborate/src/lib.rs:18119`) still uses
`map_net_kind_or_wire(d.kind)` (string → Wire). This is the **inline** path — its locals are
hoisted nets, **not** `frame_local`, so the §4.5.167 fix (`str_bytes` frame-branch keyed on
`frame_local`) does **not** cover them. Naively switching `:18119` to `frame_local_net_kind`
would make the local `NetKind::String` but **not** frame-local ⇒ `str_bytes` reads empty
`dyn_heap[net]` ⇒ **silent 0** — the exact loud→silent trap §4.5.167 already dodged elsewhere.

**Design.** Give inline-task hoisted `string` locals real heap storage that the *non-frame*
read path can see. Two candidate mechanisms (decide at plan stage):
(a) allocate a module-scope `NetKind::String` heap slot for the hoisted local (its value lives
in `dyn_heap[net]`, which `str_bytes` already reads authoritatively), or
(b) route static-task inline calls through the same frame machinery automatic tasks use (drop
the special inline string path). (a) is smaller and local; (b) unifies but is broader.
**Whichever is chosen, the acceptance test is: no method-on-static-task-string returns a silent
empty string — it either computes correctly or stays loud.**

**Format impact:** none (elaborate + eval only, like §4.5.167).
**Verification:** iverilog differential (STATTASK probe → o=3, matching AUTOTASK/STATFUNC) +
soundness (no silent-0 path; embedded-NUL parity with the module path).
**Risk:** low-moderate. **Effort:** small-moderate.

---

## 5. V2B — Function `output`/`inout` dynamic-array formal (hand-IEEE)

**Problem.** `function automatic void fill(input int n, output byte b[]); b = new[n]; …`
→ E3009. **iverilog rejects this too** ("Function arguments must be input ports") — so there is
**no differential oracle**; per the governing principle this is designed hand-IEEE (IEEE 1800
§13.4.2 explicitly permits `output`/`inout`/`ref` function args).

**Root cause.** The function formal path materializes `input` dyn-array formals only (V2A's
`dyn_subst` is read-only); an `output`/`inout` dyn-array formal has no copy-out of the produced
heap array back to the caller's handle.

**Design.** Extend the subroutine-formal handling so an `output`/`inout` dyn-array formal:
- binds a frame-local `DynArray` handle (reusing the V5 frame dyn-array slot),
- on `Return`, **copies the produced heap array out** to the caller's `DynArray` lvalue (mirrors
  the `output string` copy-out that already exists via `formal_net_kind` → `NetKind::String`).
Depends on **V5** (frame dyn-array slot) and shares copy-out with the string-output path.

**Format impact:** none expected. **Verification:** **hand-IEEE** — golden computed from the
LRM (`fill(4,v)` ⇒ `v=='{0,1,2,3}`), plus internal differential (staged velab→vrun vs one-shot).
**Risk:** moderate (no external oracle → rely on hand-IEEE + internal differential). **Effort:**
moderate. **Sequencing:** after V5.

---

## 6. V6 — Queue / unpacked-array of an unpacked struct (hand-IEEE)

**Problem.** `typedef struct { logic[31:0] addr; logic[7:0] len; } pkt_t; pkt_t q[$];`
→ E2002 (parser, `crates/hdl-parser/src/lib.rs:8141` "an array of unpacked structs (record
array) is unsupported in v1 — scalar record only"). **iverilog rejects unpacked structs too**
("Unpacked structs not supported") ⇒ hand-IEEE (unpacked structs and queues thereof are valid
SV, §7.2/§7.10; vcs/xcelium support them).

**Root cause.** vita models a "scalar record" (packed struct) but has no storage for an
**unpacked** struct as a *container element* (queue/dyn-array/unpacked-array element). The
parser refuses the declaration outright.

**Design (staged, plan-stage grounding required).** Reuse the existing **member-wise struct
desugar** (already used for unpacked-struct tf-ports — round-11 R5 — "one member formal per
field"). Model a container-of-unpacked-struct as a **struct-of-containers**: `pkt_t q[$]`
becomes one queue per member (`q.addr[$]`, `q.len[$]`) sharing an index, with `q.push_back(p)`
pushing each member and `q[i].len` selecting member `len` at index `i`. Parser: lift the
`:8141` reject for the record-container forms; elaborate: expand to per-member containers;
engine: index them in lock-step.

**Format impact:** **likely a bump** (new storage/sidecar for member-container groups) — TBD;
sequence this item **last** so the golden re-pin is isolated. **Verification:** hand-IEEE golden
(`q[0].len==4` after push) + internal differential (staged vs one-shot; member-group index
lock-step). **Risk:** **high** (widest storage change, no external oracle). **Effort:** large.
This is the one item that may warrant its **own** spec+plan cycle if it proves larger than a
member-desugar reuse.

---

## 7. V8 — Task-local unpacked-struct variable declaration (hand-IEEE)

**Problem.** Inside a task, `rec_t r;` (a package-typedef struct) → E2002
"expected '=' or '<=' after lvalue, found Word(Ident)". iverilog rejects unpacked structs ⇒
hand-IEEE.

**Root cause.** The task/block body statement parser (`crates/hdl-parser/src/lib.rs` ~`:9914`)
does not recognize an imported/typedef struct name (`rec_t`) as the start of a **local var
declaration**, so it tries to parse `rec_t r` as an assignment lvalue and fails. (Note: this is
distinct from the body-local *typedef-definition* reject at `:6400`, which is a separate policy.)

**Design.** Extend the task/function/block body statement parser to recognize a known
type-name (imported package typedef or module-scope typedef) followed by an identifier as a
**local variable declaration**, then reuse the existing member-wise struct desugar to reserve a
frame-local member net per field (composing with §4.5.167's `frame_local_net_kind` for string
members and V3/V4's window model for automatic tasks). Field access `r.a` selects the member
net.

**Format impact:** none expected (parser + frame-local reservation; reuses member desugar).
**Verification:** hand-IEEE golden (`r.a == 8` after `r.a = fd+1`) + internal differential.
**Risk:** moderate (parser + frame member storage). **Effort:** moderate.

---

## 8. Cross-cutting verification & rollout

- **Every item** lands as its own slice with: reproduce → root-cause → implement → adversarial
  2-lens (differential where iverilog is an oracle; hand-IEEE + internal differential where it is
  not) → full-suite green → docs (ARCHIVE §4.5.x, ROADMAP §3 resolve, DEVLOG, REMAINING_WORK) →
  commit → push-confirm. This is the established §4.5.x cadence.
- **Golden discipline:** items 1,2,4,5,7 are expected format-stable — assert the SimIr root hash
  is unchanged as a phase gate. Items 3,6 may need a sidecar/bump; if so, follow the trailer-
  addition rule (v20/21/22 precedent) and re-pin only the affected fixtures.
- **`correct-or-loud` invariant:** at no point does an item ship a silent-wrong. Where a sub-case
  is not yet handled (e.g. recursion+timing before Phase 4, or an unpacked-struct member type not
  yet modeled), it stays **loud** with a precise message, never silent.

## 9. Open questions (resolve at plan stage)

1. **V3/V4 §1.6:** confirm empirically that emitting timing terminators into the func arena
   leaves the SimIr root hash unchanged (expected yes). If not, choose the trailer-addition form.
2. **V3/V4 window lifetime:** exact ownership model for the automatic window across a suspend —
   `FrameRec`-owned vs `frame_stack`-indexed. Prototype both in Phase 3.
3. **V5/V2B:** frame dyn-array heap handle allocation/free point under the automatic-window model
   (per-activation alloc, freed at Return) — verify no leak under recursion.
4. **V6:** member-of-container vs container-of-member representation, and whether it bumps
   `format_version`. If the change is larger than a member-desugar reuse, split V6 into its own
   spec+plan cycle.
5. **static-string (§4):** pick mechanism (a) module-scope heap slot vs (b) unify onto the frame
   path.

---

*Next step: `writing-plans` skill → a phased implementation plan (starting with V3/V4 Phase 1),
tracked in `docs/superpowers/plans/`.*
