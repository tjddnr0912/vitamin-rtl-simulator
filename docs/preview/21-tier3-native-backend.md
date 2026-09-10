# doc-21 · The native backend

The design contract for `native`, vita's shipping process-body executor. It covers what the
backend is, what disqualifies a design from running on it, how its net storage is laid out and
why that layout is sound, how a body the compiler declines is executed instead, what it must
report when it declines a whole design, and the equivalence gate that keeps its answers
identical to the other two executors' answers.

The execution record that produced this backend is not part of the contract; it lives in
[history/ROADMAP_ARCHIVE_PHASE_A-D](../history/ROADMAP_ARCHIVE_PHASE_A-D.md). The numbers that
justify a rule are quoted here in the present tense, without the work item that measured them.

---

## 1. What the backend is

`crates/sim-engine/src/native/` — an executor that owns net storage. Flat net values live in a
`NetArena` the backend builds from the frozen `SimIr`, not in the engine's `SimState.nets`, and
resolving one net's metadata happens once at build rather than on every access.

Owning storage is the defining property and it sets the granularity of everything else: an
interpreter body cannot see the arena, so there is no body-level fallback. A design is wholly
eligible or wholly on the engine.

The source calls this backend tier-3, after the three-tier model in
[study/01](../study/01-interpreted-vs-compiled.md): tier-1 is the tree-walking interpreter,
tier-2 the bytecode VM, tier-3 the executor with its own storage. Refusal strings and module
documentation use that name; `--backend` and `run.json` use `native`.

| Executor | `--backend` spelling | Feature gate | Role |
|---|---|---|---|
| `Native` | `native` | always compiled | The default. The shipping executor |
| `Bytecode` | `vm`, `bytecode` | `oracle` | Bytecode VM. A bisection oracle, and the fallback target when a native refusal happens in a build that has one |
| `Interpreter` | `interp`, `interpreter` | `oracle` | Tree walk over `SimIr` — the reference semantics. A test instrument, and permanently excluded from performance work |

`Backend::Native` is `#[default]` on the enum and in `SimOpts::default()`; both spellings of
that default are pinned by `backend_equiv::the_default_backend_is_native`. `backend_name()` has
one spelling shared by the engine and by `cli::stage_args`, so a diagnostic and `run.json`
cannot disagree about what ran.

The interpreter's machinery is not gated behind `oracle` — only the `Backend` variant and the
dispatch that selects it are. The statement semantics (`exec::compute_effect` /
`exec::apply_effect`) and the synchronous frame executor are compiled in every build, and the
native backend calls both.

---

## 2. Why it exists

The limit the backend removes is the runtime representation, not the executor. A nonblocking
assignment on the engine's flat store carries these costs, none of which is the arithmetic:

| | Engine store | Native backend |
|---|---|---|
| Value | `Value`, 72 bytes | Two `u64` plane words for a ≤64-bit net; a word pair per element otherwise |
| Destination | `Lvalue` → chunks → `Offsets` resolved per execution | Slot index resolved at build; offsets cached per `ExprId` |
| Write | Runtime branch over the metadata plus the routing bitmaps | Two stores at a computed word index |
| Wake | `net_to_edge` / dirty / waiter lookups | The same lookups, over the arena's own channel |

Flattening the layout alone yields nothing, and the reason is the read path: a leaf load asks
`is_real` / `array_len` / `width` / `signed` before it can interpret the bits, so shortening the
pointer chase leaves every question standing. The elimination is of the questions.

| | Elimination | Question it removes | Status at HEAD |
|---|---|---|---|
| R1 | Static net allocation | "What is this net's width, element count, signedness, kind?" | Implemented — `native::arena` |
| R2 | Width-specialised operations | "How many words is this value, and must the top word be masked?" | Implemented for the admitted subset — `native::wprog` |
| R3 | Schedule elimination | "Which processes wake when this net changes?" | Not implemented. The run loop answers it at runtime from a wake table, as the engine does |
| R4 | Nonblocking specialisation | "What shape and offset is this update's destination?" | Not implemented. Updates ride the shared queue |

The order is forced. R2 cannot specialise on a width R1 has not fixed, and R3/R4 move nothing
while value access is still generic — a faster wake in front of a generic load leaves the total
where it was.

What the backend is not for:

- It is a speed axis and never an accuracy trade. Four-state semantics, `correct-or-loud`, and
  byte-identical output across platforms hold exactly as they do on the other executors; see
  [ENGINEERING_RULES](../ENGINEERING_RULES.md) for the accuracy ladder the rule comes from.
- Verilator is not the target. Verilator buys its speed with two-state values and a levelised
  schedule, and vita gives up neither ([study/01](../study/01-interpreted-vs-compiled.md)).
- `SimIr` is read-only to this backend. Nothing here enters the frozen IR, so the schema hash
  and `format_version` are unaffected ([16](16-schema-hash-spec.md),
  [17](17-sim-ir-ir-backbone-freeze.md)).

---

## 3. Selecting the backend

### 3.1 Features

| Feature | Crate | Default | Effect |
|---|---|---|---|
| `oracle` | `sim-engine` | on | Compiles the `Interpreter` and `Bytecode` variants, their dispatch, and the native-refusal fallback arm. It gates the choice, not the semantics; nothing is deleted |
| `oracle` | `cli` | on | Forwards `sim-engine/oracle`; makes the four oracle `--backend` spellings accepted |
| `jit` | `sim-engine` | off | Adds the five cranelift crates and the machine-code path inside tier-3 |
| `jit` | `cli` | off | Forwards `sim-engine/jit` |

The `cli` crate declares its `sim-engine` dependency `default-features = false`. That is
load-bearing: without it, feature unification silently re-enables `oracle` in a
`--no-default-features` CLI build, and the product-shape axis tests nothing.

### 3.2 `--backend`

`vita` and `vrun` accept the flag. `vcmp` and `velab` reject it — it is a simulate-side
argument and nothing in the artifact depends on it:

```
error[VITA-E0001]: '--backend {name}' is a simulate-side argument — '{stage}' does not run
process bodies. Pass it to `vita` or `vrun` instead; the choice does not affect the artifact
'{stage}' writes
```

An unrecognised value is a loud CLI error at exit code 3:

```
error[VITA-E0001]: '--backend' takes 'native' (the DEFAULT — it runs every design), or, to
bisect a suspected defect against a second implementation, 'interp' (the readable reference
semantics) or 'vm' (the bytecode VM). Same output whichever you pick — that equivalence is the
gate, so this only moves wall-clock. --obs-dir run.json records which executor actually ran
beside the one requested
```

In a build without `oracle`, `interp` and `vm` are a distinct loud rejection rather than an
unknown value, and never a silent downgrade to `native`:

```
error[VITA-E0001]: '--backend' takes only 'native' in this build — the oracle executors
('vm', 'interp') are compiled out. Rebuild with the `oracle` feature to select them.
```

### 3.3 The two build shapes

| | Default build | `--no-default-features` |
|---|---|---|
| Executors compiled | `interp`, `vm`, `native` | `native` |
| `--backend vm` | accepted | loud CLI error, exit 3 |
| A gate refusal | `warning[VITA-W4030]` plus fallback to `vm`, exit unchanged | `fatal[VITA-F4004]`, run declines to execute, exit class Fatal |
| CI job | `build-native` (ubuntu, macOS), `build-rhel` (RHEL9/UBI) | `build-no-oracle` |

Both shapes are real and answer different questions. The product shape must stay a separate CI
axis rather than an extra step on the workspace jobs, because a `--workspace` build's
dev-dependencies pull `sim-engine` with default features and re-enable `oracle` for every crate.
Details in [03 · build and portability](03-build-and-portability.md).

---

## 4. Eligibility

### 4.1 Three layers, three questions

| Layer | Function | Question |
|---|---|---|
| Design | `native::design_eligibility(ir, opts) -> NativeEligibility` | Is this feature family inside scope? |
| Storage | `NetArena::buildable(ir, opts)` | Can this design's values live in the arena? |
| Executor | `native::run::executor_rows(ir, opts)` | Can the executor that exists run every body? |

`native::runtime_gate` is design ∧ storage. `simulate` asks the design layer once, while `opts`
is still whole — the scheduler consumes `opts.fork_modes` by value further down, and a later
read would see an emptied table and call a forking design eligible. When nothing has refused,
`simulate` then asks `executor_rows` and writes its answer into the same `refused` field, so the
verdict `run.json` publishes and the decision `simulate` executed come from one evaluation.

`NativeEligibility` carries four fields, and `run.json`'s `native` object is their serialisation
([19 · observability](19-ai-agent-observability.md)):

| Field | Meaning |
|---|---|
| `eligible` | The design layer found no disqualifier ⇔ `reject_reasons` is empty |
| `buildable` | The storage layer accepted the design |
| `refused` | The runtime gate's reason, or `null` |
| `reject_reasons` | Reject family → count of offending items. Any non-zero row disqualifies |

`eligible` and `buildable` are reported side by side rather than folded together because they
answer different questions and their answers differ. `refused` carries two vocabularies: when
the design layer refused it is a key of `reject_reasons` (the byte-lexicographically first when
several fired), and when the design layer passed and storage refused it is that refusal's own
prose, which appears in no map. A consumer joining `refused` back to `reject_reasons` must read
a miss as the storage case, not as an error.

### 4.2 The design gate

`design_eligibility` destructures `SimOpts` exhaustively, with no `..` rest pattern. Adding a
sidecar to `SimOpts` without classifying it here is a compile error rather than a silent
eligibility over-claim. The same rule governs the two loops below it: both are `_`-free, so a
new `Stmt` kind or a new `NetKind` must be classified on purpose.

Sidecars that bind to `_` fall into three groups.

| Group | Members |
|---|---|
| Configuration knobs, not design features | `vcd_path_override`, `timescale_unit`, `vcd_date`, `max_deltas`, `max_body_steps`, `max_class_objs`, `time_limit`, `backend`, `threads`, `plusargs` |
| Core sidecars the backend supports | `net_names`, `net_dims`, `net_decl_ranges`, `proc_multipliers`, `proc_prec_mults`, `global_prec_exp`, `proc_scopes`, `proc_inst_scopes`, `ca_delays`, `assign_ranks`, `two_state_nets`, `wired_and_nets`, `wired_or_nets`, `radixes`, `severities`, `init_procs`, `final_procs`, `timeformat_stmts`, `stmt_locs`, `stmt_scopes`, `expr_scopes` |
| Feature families routed rather than refused | `func_table`, `func_names`, `task_calls_proc`, `task_calls_func`, `fork_modes`, `assert_fire`, `assert_ctl`, `queue_slice_stmts`, `queue_bounds`, `coverage_manifest`, `clocking_inputs`, `clocking_commit`, `clocking_outputs`, `defer_marks`, `defer_acts`, `handle_copy_stmts`, `file_directed_stmts`, `real_elem_dyn_nets`, `string_elem_dyn_nets`, the twelve `class_*` / `randomize_with` tables, `probed_nets`, `stage_stmts`, `proc_profile` |

Three entries in the third group carry a rule worth stating outright, because comments elsewhere
in the tree say the opposite:

- `probed_nets` is core. `--probe` does not disqualify a design. The arena emits probe rows at
  its own store point, and a `--probe`-armed run reports `"backend": "native"` with
  `"native": {"eligible": true, …}`.
- `stage_stmts` is core. `$vita_stage` does not disqualify a design; its argument reads are
  threaded through the alternate net reader, so a native run records the values the design
  actually wrote.
- `proc_profile` is core. `--obs-procs` counters live on `SimState` and both executors bump them
  at their own dispatch seam, so a profiled run is not a different design.

`native_gate::stage_markers_are_core` pins the first two together.

Three of those rows would double-book if they were counted here. `fork_modes` describes a
process-level fork, whose bookkeeping is the scheduler's and whose remaining unhandled shapes
are refused by the storage and executor layers under their own names. `real_elem_dyn_nets` and
`string_elem_dyn_nets` describe heap-kind handle nets the net-kind scan already classifies, so
they carry no axis of their own. A row that names a feature rather than missing machinery
refuses designs the executor gets right, which is a rung down the accuracy ladder.

One family remains counted, `stmt_effect`, and it is a scan of `ir.stmts` rather than a sidecar.
Its criterion is "writes a net from inside the call" — an effect that never passes through
`write_lvalue`:

| Half | Predicate | Carve-out |
|---|---|---|
| A `BlockingAssign` whose RHS is in the statement-effect family | `sim_ir::rhs_is_stmt_effect` — the same function the VM's compile gate consults, never a second spelling | `native::stmt_effect_wired`, which names each wired member through the canonical `exec::kpred` predicate |
| A `SysTask` whose net write is `NetWrite::Flat` | `sim_ir::systask_net_write` | `Sformat`, `ReadmemB`, `ReadmemH`, `Cast` |

The wired set is `value_plusargs_rhs`, `queue_pop_rhs`, `random_seeded_rhs`, `dist_seeded_rhs`,
`cast_rhs`, `assoc_iter_rhs`, `sscanf_rhs`, `fopen_rhs`, `fgetc_rhs`, `feof_rhs`, `ungetc_rhs`,
`fgets_rhs`, `fscanf_rhs`, `fread_rhs`.

Naming a member in that set whose `k_*` still refuses is a silent-wrong rather than a compile
error: the design runs natively and its effect lands in the engine's store. Every addition to
the set therefore ships with both a differential and an absolute anchor — as the backend
delegates more shared code, a differential against the engine goes blind exactly where the
delegation is.

`$sformatf` in its function form is deliberately outside this family. Its only effect is the
rendered value, written through the ordinary funnel — true for an executor that routes
statements through `compute_effect` / `apply_effect`, which the native backend does. The VM
bypasses those and must exclude it; that is the one documented delta between the two gates.

The net-kind scan admits every `NetKind`. `Wire`, `Reg`, `Logic` and `Integer` are the arena's
own ground. `Real` is ordinary word storage plus a flag — the 64 bits of an IEEE-754 double fit
the word plane, and what the kind carries is that reads stamp `is_real` and writes take the
real↔int coercion. `DynArray`, `String`, `Queue`, `Assoc` and `AssocStr` keep their values in
`SimState::dyn_heap`, keyed by net id, which both kernels borrow — so admitting them is routing,
not a second store. The shapes that mutate a heap net from inside a call (`q.pop_front()`,
`aa.first(i)`, `foreach`) are refused under `stmt_effect`, by their own name.

Status at HEAD: no design-gate family is reachable from a design a compiler can produce. The map
comes back empty, and what the gate pins today is that emptiness. The loops and the counted
family are kept so that a new statement kind, net kind or sidecar has to be classified.

### 4.3 The storage gate

`NetArena::buildable` is the same refusals as `build` without the allocation, so `run.json` can
carry the storage verdict on every run at the cost of one scan, and `build` calls it first —
one predicate, not two that can drift. It refuses:

| Refusal | Cause |
|---|---|
| `arena exceeds u32 words` | The accumulated word offset does not fit `u32` |
| `arena exceeds usize` | The accumulated word offset does not fit `usize` |
| `a module body that names a frame-local net` | A process body reads or writes a subroutine's local, whose value lives in the activation window |
| `a call in a delayed continuous assign: S3b` | A delayed continuous assign whose RHS calls a subroutine |
| `a system task the tier-3 kernel refuses, inside a task frame` | A task frame reaches a system task this kernel declines to dispatch |
| `a nonblocking assign to a frame-local net: S3b` | An NBA destination inside the frame window |
| `a nested call with no sidecar entry: S3b` | A call site with no `task_calls_*` entry |
| `a nested call to an unresolved target: S3b` | A call site whose callee is not resolved |
| `a subroutine that WRITES a net outside its own frame: S3b` | The delegation precondition fails: the body names a net outside `[base, base+len)` |
| `a subroutine statement the frame executor drops` | The engine's `&self` frame executor has no arm for a statement in the body |
| `a subroutine body that suspends, forks or calls a task` | Outside the delegable subset |
| `malformed frame sidecar (…)` | Four shape checks over `func_table`: table length, frame window range, return-slot range, block-id range |

Frame refusals sit in the storage layer rather than the executor layer because they are about
where the values live. A subroutine whose frame never needs the module store is delegated whole;
everything outside that subset refuses here, in its own words.

### 4.4 The executor gate

`executor_rows` walks every process and asks two questions:

| Row | Predicate | Refusal |
|---|---|---|
| Can the walk run this body? | `body_is_walkable`, with `frames::call_site_runnable` deciding each call site | ``a `wait fork`, a `fork`, or a call statement whose callee forks: S3b`` |
| Does the body reach only dispatchable system tasks? | `body_dispatch_ok` over `native::kernel::systask_refusal` | `a system task the tier-3 kernel refuses (VCD, $monitor/$strobe, file)` |

`body_is_walkable` is a reachability scan from the process entry. `Goto`, `Branch` and `Return`
are structural; `Delay` and a `Wait` on `Edge`, `Level`, `Expr` or `Fork` push their resume block
and do not disqualify; `Fork` pushes its children and its resume block, because an arm that
reaches a shape the walk cannot run is still a refusal, just not because of the arm; `Call`
consults `call_site_runnable`. A `Wait` on `WaitCause::Named` refuses — nothing fires it, so
parking on one is a hang. That variant is not constructible from source today (elaborate lowers
a named event to a counter net and `@(ev)` to a `Level` wait); the arm is explicit so a future
lowering change is a compile-time question rather than a hang.

`call_site_runnable` is `callee_mode(...).is_some()`, and the mode is what the walk does with
the call:

| Mode | Condition | Execution |
|---|---|---|
| `Synchronous` | The callee is not in `compute_suspendable_tasks` | Delegated whole to the engine's `&self` frame executor |
| `DrivenFrame` | The callee would be driven from a `FrameRec`, but its body reaches no suspending terminator | The walk drives the callee's CFG to `Return` without leaving this activation |

`systask_refusal` returns `None` for every `SysTaskId` — the refused set is empty. The function
and both of its consumers are kept: `k_dispatch_systask` panics on a `Some`, and this gate row
refuses the design so nothing reaches that panic. Written as two separate matches the gate would
go stale the first time an arm is added, and the symptom would be a mid-run panic on a design
the gate called runnable. A new `SysTaskId` that reads a store this seam does not thread has to
land in that one function.

### 4.5 Reachability of the gates

Status at HEAD: no gate row of any layer has an input a compiler can produce. Every refusal
string above is reachable only through a corrupted or truncated artifact sidecar, and that is
deliberately how the rows are tested — `native_gate.rs` pins that each reject family actually
fires, because a gate that never fires is vacuous, and pins the generated-corpus eligibility as
an exact count. Completeness of the classification is not a test; it is the compile-time
exhaustive destructure inside `design_eligibility`.

---

## 5. The arena

### 5.1 The slot descriptor

One dense record per net, fully resolved at build, replacing the per-access metadata questions
and the routing bitmaps in front of them:

| Field | Meaning |
|---|---|
| `off` | Word index of element 0's `val` plane in `buf` |
| `words` | Words per plane per element — `nwords(width).max(1)` |
| `width` | Element width in bits (the declared packed width) |
| `elems` | Element count — `array_len.max(1)`; 1 is a scalar |
| `signed` | Declared signedness |
| `two_state` | A two-state variable: the write funnel coerces X/Z bits to 0 before they land (IEEE 1800 §6.11.3). Resolved at build from `two_state_nets`, so the write path never asks a side table |
| `is_real` | `NetKind::Real`: the stored 64 bits are an IEEE-754 double. Seeded from the net's kind, which cannot change during a run |

### 5.2 Layout and invariants

One flat `u64` buffer. Element `e` of net `n` owns `2 * words` consecutive words at
`off + e * 2 * words` — the `val` plane, then the `unk` plane adjacent. `(val, unk)` is the
four-state encoding shared with the engine: `unk=1` means x when `val=0` and z when `val=1`.

Three invariants make the fast paths sound:

1. Elements are word-aligned by construction. The engine's flat store packs elements
   bit-contiguously and pays a bit-serial fallback on an unaligned base; that path does not
   exist here.
2. Bits above `width` in a top word are zero. Writes mask, so reads may copy words verbatim and
   re-mask only the top word, and a specialised load needs no mask at all.
3. A net whose value is not in this store still owns a slot, so `slots[net]` keeps meaning net
   `net`. That slot is dead, and the arena says so through the bitmaps in §6 rather than by
   giving the net no slot.

### 5.3 Initialisation at time zero

Per net, the width-wide element initial value is extracted once and broadcast to every element.
A heap-kind net is skipped: its declared initialiser describes the packed literal, whose bits
run above the width the dead slot was sized to, and its real initial value is the heap's
(IEEE 1800 §7.5.2). A debug assertion pins that no declared initialiser carries bits above its
own width — the arena keeps those bits zero and the engine's scalar init path word-resizes
without masking, so an initialiser with junk above the width would make the two stores agree on
every read and disagree once, on a whole-net write's `changed` verdict.

### 5.4 The dirty and edge channel

Half of scheduling hangs off the write, not off the loop, and it lives on the arena for the same
reason it lives on `SimState`: the two points where a stored word actually changes are inside
the write funnel, and a channel the funnel could not reach would have to be updated by its
callers.

| Carried | Rule |
|---|---|
| `dirty` | The nets that took a real bit change since the last sweep, in write order. Membership alone is the changed set — an A→B→A round trip inside one slot ends with `cur == prev` and is still a change the observer must see (IEEE 1364 §9 fires the glitch once). An endpoint comparison drops exactly those |
| `last_blocking_writer` | Who authored the change, so a process is not re-fired on a net it blocking-wrote itself |
| `slot_edge` | The intra-slot bit-0 edge summary, OR-accumulated per transition and reset on the net's first dirtying each slot. This recovers the edge kind for a glitch the endpoints lost. Maintained only for `is_edge_target` nets |

`note_change` takes the element word as well as the net, because an unpacked array has one VCD
identifier per element, and the waveform record is emitted at the store point rather than at
sweep time — a sweep-time emitter would collapse an intra-slot A→B→A into one record.

### 5.5 The wake decision

The changed set with its intra-slot masks feeds one question: which processes become ready, in
what order. Three rules are not simplifications and must hold in any reimplementation:

1. Fire from the intra-slot mask, never from an endpoint comparison — an A→B→A clock pulse still
   ticks once.
2. A busy process is skipped. A static edge registration is permanent, but IEEE does not re-enter
   an `always` until it completes and re-arms; a process suspended mid-body is woken through the
   waiter path.
3. A clock handler diverts at the position the engine's `commit_clocking` intercept occupies —
   the edge is consumed and the process is not queued.

---

## 6. Routing — "is this net ours?"

Flat net values are the arena's. Frame windows, the dynamic heap, class objects, the file table,
the RNG and the output sink stay in `SimState`, which the native kernel borrows. The ownership
question is therefore asked in four places, and the contract is that they agree.

| Site | Function | How it answers |
|---|---|---|
| Read funnel | `NativeKernel as NetReader::read_net` | A class field select first (a method's `this` is both a handle and a frame-local, and the field must win); then a frame-local or heap net delegates to `SimState::read_net`, which routes on its own bitmaps; otherwise the arena |
| Write funnel | `NativeKernel::write_routed` | The assoc key lanes, then the frame lane, then the heap lane, then the arena. The split is the one `SimState::write_lvalue` makes, at the same point, delegating to the same methods |
| Specialised evaluator | `wprog::compile` | It resolves a `Signal` to an arena slot at compile time and cannot route at all, so it must decline — the arena's `heap` / `frame` / `class` bitmaps exist on the arena, not only on `SimState`, so that this function can ask |
| Reader wrapper | `state::HeapRouted` | Wraps a foreign reader and answers `dyn_is_handle` and `frame_local` nets from `SimState`. The split is by ownership: `st` owns the heaps, the iteration context and the file table; the wrapped reader owns the flat slots |

Two rules follow, and both come from measurement rather than taste:

- Opening a net class the arena does not own means visiting all four sites before writing code.
  Fixing one makes a different part of one design's output correct, so any stopping point looks
  like success.
- A class handle is the one partly-dead slot: the handle's own value (the object id, `0` for
  null) is in this store, and the object's fields are in `SimState::class_heap` under the same
  net id with a field id in the `word` position. Every consumer must ask
  `class[net] && word.is_some()`. Routing on the bitmap alone sends a bare handle read to the
  heap, where there is nothing to read.

Every entry point below the arena `debug_assert`s on the ownership bitmaps, which turns "audit
sixteen call sites that index `slots` by net id" into "run the suite once and read the panics".
It is a debug assertion because the release path must stay byte-identical.

---

## 7. Executing a body

### 7.1 Two executors inside the backend

`native::run::dispatch_body` is the one place tier-3 chooses:

| Condition | Executor |
|---|---|
| The activity is its own template and `is_codegen_able` accepts the body | `backend::vm_exec` over a `CompiledBody`, run against the arena |
| The same, plus the `jit` feature and `VITA_JIT` set | `jit::run_body_jit` for the entry block |
| Otherwise, and always for a fork child | `native::body::run_body`, a walk over `SimIr` |

A child activity always takes the walk. `vm_exec` carries one process id and uses it for both
roles the walk keeps apart — the body it indexes and the identity it schedules under — so a
child running its parent's compiled body would schedule as its parent. The guard is
structurally unreachable today, because `is_codegen_able` refuses both terminators that can
create a child activity, and it stays because that is a property of today's codegen coverage
rather than of the identity split, and because the failure mode if it changes is silent.

The two are not two semantics. `vm_exec` calls the same `Kernel` methods in the same order that
`compute_effect` / `apply_effect` do, and `compile_body`'s documentation pins that
correspondence. What the compiled form adds is that the statement kind, the lvalue offsets and
the RHS program are decided once per process template rather than on every execution — and it is
also cranelift's input, which is why it is the compiled path rather than a detour around one.

`Op::ends_statement` recovers statement boundaries from the op stream, so `k_call_fatal` and
`k_drain_diags` fire at the same points the walk fires them. Omitting that marker makes a
runaway body end `Quiescent` instead of `Error` — the op stream has no statement boundary of its
own to hang the check on.

### 7.2 Statement meaning is shared

`compute_effect` and `apply_effect` are generic over `K: Kernel`, so statement semantics come
from the same code the engine runs. `Scheduler` and `NativeKernel` are the two implementors.
What the native side must restate is the block loop and the terminator decisions —
`exec::run_process` is `&mut Scheduler`-fixed, and it is the one piece that could not be shared.

Two consequences are contract, not convenience:

- Every `Kernel` predicate (`*_rhs`, `class_new_site`) forwards to the canonical `exec::kpred`.
  A predicate does not produce a value; it decides which statement `compute_effect` builds. A
  refused family stubbed to `false` does not become loud — it becomes a different statement, and
  flows silently down the pure-eval path. So the predicates all answer truthfully and share one
  spelling with the engine; only the workers are loud.
- Any rule the engine already spells (`eval::resolve_offsets`, `eval::delay_ticks_of`,
  `value::coerce_assign`, `eval::binops::*`, `resolve_md_group`, `offset_of_index_value`) is
  called, never re-spelled. Restating a rule from memory drops the clauses that are not visible
  in a value comparison: a re-derived delay conversion that loses its X/Z guard and its
  saturation sentinel makes an infinite delay fire at `t+0`, and no value differential can see
  that.

### 7.3 The specialised expression evaluator

`native::wprog` compiles an expression tree into a program over `W = (val, unk)`, one plane word
each, loading directly from compile-time-resolved buffer indices.

Admission is the correctness argument. A program is compiled only when every node is one of:

- `Const` (numeric) whose self width and sign equal the context's;
- `Signal` reading a whole ≤64-bit integral net, a constant in-bounds element of a ≤64-bit-element
  array, or a runtime element whose index expression is itself admitted;
- `BitNot`, `BitAnd`, `BitOr`, `BitXor`, `Add`, `Sub`;
- `Shl`, `Shr`, `AShr` by a two-state constant amount, except a signed `AShr`, whose sign fill is
  the one shift whose bits depend on the sign;
- the eight comparisons, whose operands are mutually context-determined at `max(self-width)` with
  their pair signedness (IEEE 1800 §11.8.1 — the comparison's own context is one unsigned bit and
  does not inherit the enclosing one);
- `LogAnd` / `LogOr`, whose operands are self-determined;
- `LogNot` and the six reductions over a self-determined operand;
- `Concat` and `Replicate`, whose parts are self-determined and tile the result exactly;
- `Select` over a self-determined base at a constant, provably in-range offset;
- `Ternary` whose branches cannot report — the one admission about evaluation order rather than
  width.

The soundness claim is that every conversion is the generic path's own. The only one this module
emits is a sign extension calling `value::resize_word`, the single spelling `Value::resize`'s
≤64-bit arm uses, emitted under exactly `resize_keep_sign`'s condition. Truncation exists
nowhere: the compiler declines instead. Which nodes may be converted is decided against the LRM's
sizing rules, because converting a context-determined operator instead of computing it at the
context width is a wrong answer — `v[8:11] + 4'd1` is 16 at eight bits and 0 at four.

A program can hold several widths at once (a comparison's operands are `ow` bits wide while its
result is one), so the mask rides the op rather than the program, and every stack value stays
masked to its own width.

Per-op four-state bit semantics are pinned by an exhaustive per-bit-state differential against
the generic evaluator plus a corpus mirror sweep. Nothing is restated: the comparisons call
`eval::binops::{relational, log_eq, case_eq}`, `&&` and `||` call `eval::binops::log_bin_tri`,
and `!` and the reductions call `eval::unary_self_of` — the same functions the generic evaluator
reaches.

### 7.4 The two-state lane

Most evaluated values are definite. The four-state work is therefore the metadata, not the
arithmetic, and the program is run on one plane first: `run_2s` executes the same op sequence
over the `val` plane alone and returns `None` on the first unknown, at which point the canonical
four-state loop runs from the start.

The accuracy surface of this is zero, and that is the design rule: the fallback is the canonical
implementation itself, not an approximation of it. A definite result is returned only when every
leaf was definite, and on definite leaves every admitted op is definite-preserving — measured by
the battery, not assumed. The index of an element load is checked separately from the element:
an index arriving from definite ops does not make the element definite.

### 7.5 The partition with `native_eval`

`wprog` takes uniform width at ≤64 bits. Everything it declines would otherwise fall all the way
to the generic tree walk, which is measurably slower than the backend being replaced on wide
arithmetic and wide select/concat. So the two specialised evaluators partition rather than
exclude each other: `wprog` keeps every RHS it accepts, and the compile context's
`NativesWhen::OnlyWhereWprogDeclines` hands the rest to `native_eval`.

The boundary asks `wprog::compile` itself, through a closure, rather than re-deriving what it
accepts. Asking only its width refusal is a necessary condition and not a sufficient one — the
compiler declines ≤64-bit trees for other reasons, a runtime-offset part-select being the common
one, and each of those would route to neither evaluator. The cost is one extra compile per RHS
per template; the runtime cache builds its own regardless.

The `native_eval` stacks are leased from the kernel behind a `RefCell`, not built per call.
`NativeScratch` is two fixed arrays totalling 1,280 bytes, so constructing one per call is a
memset on the hot path. Reuse is sound for the reason the engine's copy is: the run drives a
stack pointer from zero and every read is of something the same call pushed.

### 7.6 Frames

A subroutine call is core, not refused. Which mechanism runs it is `callee_mode`'s answer (§4.4),
and both mechanisms are the engine's:

- A synchronous call is delegated whole to the engine's `&self` frame executor, reached through
  the kernel's composite reader; `HeapRouted` splits the store so a frame slot is answered from
  `SimState` and a module net from the arena.
- A driven frame's CFG is walked here, to `Return`, without leaving the activation.

The delegation precondition is one question with two halves, and a caller that asks only the
first delegates a body that reads the module store: does the body use only what the `&self`
executor runs, and does it name only nets inside its own window `[base, base + len)`?

### 7.7 The run loop

`native::run::run` mirrors `Scheduler::run` region for region, because region-for-region
identity is what the byte gate compares against. An outer loop over timesteps, an inner loop
draining the current time through the region cascade:

```
t0 structural settle → arm_t0
  → snapshot_preponed
  → [ settle continuous assigns → Active → Inactive → NBA ]   (to a stable point)
  → Observed → Reactive
  → propagate (re-drain if anything woke)
  → Postponed
  → advance time: min over the wheel, delayed NBA, next delayed continuous assign
```

A design that cannot converge in the t0 settle is stopped rather than run on a divergent t0. The
loop shares the scheduler for everything that is not a net value — output sink, file table,
`now`, RNG — so `Scheduler` is still constructed and `NativeKernel` borrows it.

Continuous assigns run their full model here: the zero-delay fixpoint, the delayed wheel, and
multi-driver and `wand`/`wor` resolution through the shared `resolve_md_group` fold over the
scheduler's `md_groups`. What elaborate rejects for both executors alike (`E3001` for partial,
dynamic or delayed overlaps) never reaches this gate.

### 7.8 The `jit` path

Behind the `jit` feature and additionally gated at runtime by the `VITA_JIT` environment
variable; `VITA_JIT_STATS` prints per-run codegen statistics. It compiles a `CompiledBody`'s
entry block to machine code through cranelift, caching per template in `Scheduler::jit_bodies`,
where a `None` entry means "tried and refused" and is remembered. Ops whose Verilog semantics
cranelift IR does not reproduce exactly are refused, and the program runs on the VM instead.

Status at HEAD: it builds, it is wired, it is measured, and it is correct — the whole suite runs
green under `VITA_JIT=1`, and `examples/`, keccak and picorv32 are byte-identical. It is off by
default and it is slower than the path it replaces. The arithmetic is the reason and it is not a
tuning matter: op dispatch is a tenth of the run, while the boundary between generated code and
Rust is roughly two-fifths of it — every leaf load is a call back into Rust and a 72-byte `Value`
is reconstructed at the boundary. Turning it on trades a larger cost for a smaller one. The one
condition under which it is worth revisiting is inlining leaf loads and two-state arithmetic into
the generated code, so the boundary is not crossed at all.

Determinism note carried in the module: cranelift IR masks shift counts itself, so
`ushr(x, 64) == ushr(x, 0)` on both aarch64 and x86-64. This is measured; a host ISA divergence
here is not a hazard the feature carries.

---

## 8. Refusal and fallback

The contract is that a refusal is never silent and never a wrong answer.

| | Default build | `--no-default-features` |
|---|---|---|
| Outcome | Falls back to `Bytecode` | `st.fatal_run(...)`, and the run declines to execute |
| Diagnostic | `warning[VITA-W4030]` (`W-RUN-BACKEND-FALLBACK`) | `fatal[VITA-F4004]` (`F-RUN-FATAL`), latching `had_fatal` and `finished` |
| Exit | Unchanged by the fallback itself | Non-zero exit class (Fatal) |

The warning text:

```
requested backend `{req}` cannot run this design ({reason}); ran on `{eff}` instead — the
result is unaffected, the speed is
```

The fatal text:

```
backend `native` cannot run this design ({row}), and this build carries no other executor —
the `oracle` backends are compiled out
```

Why the severities differ is the accuracy ladder rather than politeness. Byte identity across
the executors is a gate, so the VM's answer is the native answer; a fallback is a slower answer,
not a wrong one, and making it `exit != 0` in a build that has a fallback would trade
correct-support for loud, which is a rung down. In the build where no fallback target is
compiled, the choice is loud-or-wrong instead of loud-or-correct, so the same event is fatal.
The fatal form is graceful — it latches rather than panicking in `NetArena::build`, which is what
a storage refusal would otherwise reach.

Publication is separate from saying. `SimResult.backend` reports what actually ran; `run.json`
carries `backend_requested` beside `backend`, and `native.refused` names the layer. Publication
alone is not enough: a fallback a reader has to go looking for is one nobody looks for. This is
why an anchor test that does not assert `"backend": "native"` cannot tell a native run from a
fallback — a design that has fallen back matches an external oracle exactly, and reads as
agreement.

The path is written fail-closed so that a newly added gate row reports itself without anyone
remembering to. Its teeth are a corrupted sidecar.

---

## 9. The equivalence contract

Selecting a backend must never change one output byte. One structural invariant and five gates
hold that.

The invariant: the shared net-write and VCD choke points (`state.rs::write_lvalue`,
`emit_vcd_change`) stay on the shared side across all three executors, so only process-body
control flow differs. Output bytes cannot diverge in a backend-specific way.

| Gate | Where | What it compares |
|---|---|---|
| Backend differential | `sim-engine/tests/backend_equiv.rs` | 72 generated corpus designs built once into a `SimIr`, then run on two backends concurrently; identical stdout, VCD bytes, `sim_time`, `finish_reason`, `exit_class`. Plus hand-written shapes the generator cannot emit. A plain `#[test]`, no skip, so it is a hard gate on every CI leg |
| Tier-3 differential | `native/run_tests.rs` | `agree(src, name)` runs the same IR on `Bytecode` and `Native` with per-design VCD targets and a merged output/diagnostic sink |
| Design gate | `sim-engine/tests/native_gate.rs` | Each reject family actually fires; corpus eligibility is an exact pinned count |
| iverilog differential | `sim-engine/tests/differential.rs` | vita against `iverilog` + `vvp`; skips gracefully when the tools are absent, and the design still runs through vita |
| The whole suite with this default | The default build | The gate that finds what a corpus differential cannot: 7,352 tests, all backends' shared paths exercised by real designs |

Three anti-vacuity rules are part of the gate, not decoration:

- `agree` asserts `r_nat.backend == Backend::Native` before anything else. Without it a fallback
  makes every later assertion compare the VM against itself.
- `agree` calls `runnable()` first and counts a refusal, rather than passing silently.
- `backend_equiv::gate_actually_compares_vcd_bytes` asserts the compared VCD bytes are
  non-trivial.

A corpus differential is far weaker than the full suite, and the obligation to run the whole
suite on a backend belongs to whichever backend is the default. Keep running it in both
directions while two executors exist.

---

## 10. Limits

| | Present state |
|---|---|
| Body-level fallback | Does not exist and cannot. Storage ownership makes eligibility a whole-design property |
| R3 schedule elimination | Not implemented. Wake is decided at runtime from the changed set |
| R4 nonblocking specialisation | Not implemented. Updates ride the shared queue |
| Machine-code generation | Behind `jit`, off by default, slower than the compiled-op path (§7.8) |
| Frame bodies | Delegated to the engine's `&self` frame executor, or walked in place; the backend has no frame store of its own |
| `unsafe` | None in `native/`. The arena is safe Rust. The workspace's only `unsafe` is the `jit` call boundary and `cli/src/frontend.rs`'s `signal(2)`, each in a designated module with a `// SAFETY:` note |
| Toolchain | rustc 1.85.0 minimum; cranelift pinned at 0.120, the newest line that builds on 1.85 (0.134 needs 1.94) |
| Platforms | Linux and macOS, x86-64 and aarch64. `.github/workflows/ci.yml` runs ubuntu-latest, macos-latest and a RHEL9/UBI container |

Determinism is a gate rather than an aspiration on this axis: cross-platform byte identity is
asserted by the same suites that assert backend equivalence, and the vendored libm carries bit
pins so a real-valued design does not diverge by host libm.

---

## 11. Measurements the rules rest on

Numbers a design rule cites, all from this repository, release builds, interleaved, first round
discarded ([09 · testing and verification](09-testing-and-verification.md) states the A/B method
a performance claim must follow).

| Measurement | Value | Rule it grounds |
|---|---|---|
| Coverage of `simulate()` calls the native backend runs | 6,470 / 6,470 = 100.00%, zero refusals | Making it the default routes nobody silently to another executor |
| picorv32, best-of-5 | interp 1.319 s · vm 0.838 s · native 0.513 s (iverilog 13: 0.585 s) | The ranking of the three executors |
| `size_of::<Value>()` | 72 bytes | R2 is representation removal, not arithmetic tuning |
| Flat-layout probe with the metadata questions left standing | 0% | The elimination is of the questions, not the pointer chase |
| Definite values among evaluated leaves | 100% on every benchmark shape; 90.1% of runs and 91.1% of ops on picorv32 | The one-plane lane with a canonical fallback, and no static X-freedom proof |
| `NativeScratch` per call | 1,280 bytes, memset per evaluation | Leasing the stacks rather than constructing them |
| Declines at the `wprog` admission gate, execution-weighted on picorv32 | 90.4% of generic-path evaluations, 47.6% for sign alone | Widening admission on the sign axis measures 1.00×, so it is not taken. A first-failure histogram overstates a fix — removing one gate helps only a tree that fails nowhere else |
| Front end on real designs | ≥99% of every corpus row is simulation | An executor change is what moves these workloads; a front-end regression is arithmetically invisible in them |

---

## 12. Extending the backend

The gates are built so that a change which would widen the backend's reach cannot land silently.
The forcing functions, and the obligation each one leaves to the author:

| Change | What forces the question | Obligation |
|---|---|---|
| A new `SimOpts` sidecar | The exhaustive destructure in `design_eligibility` fails to compile | Classify it as a knob, a core family, or a counted reject family |
| A new `sim_ir::Stmt` kind | The `_`-free statement loop fails to compile | Decide whether it writes a net from inside the call |
| A new `NetKind` | The `_`-free net loop, `arena::kind_is_heap`, and a `format_version` bump | Decide which store owns it, then check all four routing sites in §6 |
| A new `SysTaskId` that reads a store this seam does not thread | `systask_refusal`, whose two consumers are the dispatch panic and the executor gate row | Add an arm naming what it reads, or thread the read |
| Wiring one more `stmt_effect` member | Nothing — this one is a silent-wrong if it is wrong | Ship a differential and an absolute anchor together; the differential alone goes blind as delegation grows |
| A new refusal row anywhere | The fallback path is fail-closed and reports itself | Give the row a reason that names the predicate that decides, not the feature |

Two rules apply to the reasons themselves. A refusal row does not know when its own reason goes
stale, so a row is worded in terms of the predicate that decides it rather than in terms of the
construct that currently trips it. And a row phrased as a limitation ("cannot", "until") is a
claim to re-measure before it is relied on: the slice that makes it false does not update it.

---

## Related documents

- [00 · overview](00-overview.md) and [04 · architecture](04-architecture.md) — where the backend
  sits in the pipeline.
- [06 · simulation engine](06-simulation-engine.md) — the scheduler this loop mirrors.
- [03 · build and portability](03-build-and-portability.md) — the feature matrix and the two
  build shapes.
- [09 · testing and verification](09-testing-and-verification.md) — the suites named in §9.
- [13 · diagnostics and logging](13-diagnostics-and-logging.md) and
  [15 · error-code reference](15-error-code-reference.md) — `VITA-W4030` and `VITA-F4004`.
- [18 · acceleration analysis](18-acceleration-analysis.md) and
  [20 · cycle-mode feasibility](20-cycle-mode-feasibility.md) — the other speed axes and why they
  are not taken.
- [19 · AI-agent observability](19-ai-agent-observability.md) — `run.json`'s `native` and
  `codegen` objects.
- [study/01](../study/01-interpreted-vs-compiled.md), [study/02](../study/02-v1-native-coverage.md)
  and [study/03](../study/03-workload-corpus.md) — the tier model, the coverage report, and the
  workload corpus.
- [manual/004 · CLI reference](../manual/004_cli-reference.md) — the user-facing `--backend`
  entry.
