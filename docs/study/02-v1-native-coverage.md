# study/02 — Terminology, and the native backend's coverage

Defines the three words this repository uses in a non-standard way — *census*, *coverage*
and *mutation* — and then states the tier-3 `native` backend's coverage: what it compiles,
what falls back, how a fallback is reported, and how its equivalence against the other two
executors is gated.

Companion studies: the performance axis in [study/01](01-interpreted-vs-compiled.md); the
workload corpus in [study/03](03-workload-corpus.md).

---

## 1. Terminology

### 1.1 The simulator's vocabulary

| Term | Meaning |
|---|---|
| **RTL** | Register-Transfer Level: hardware described as which signal changes on which clock. Verilog and SystemVerilog are the languages |
| **design** | one source bundle that is simulated. "N designs" means N distinct test inputs |
| **net** | one signal (`reg [7:0] q;` declares the net `q`) |
| **elaborate** | read the source, expand the module hierarchy, fix parameters, and lower to an executable intermediate representation |
| **IR** | `sim_ir::SimIr`, the elaborate output. From here on the data structure is language-neutral |
| **process** | one `initial` or `always` block. In the IR it is a graph of basic blocks |
| **terminator** | the last instruction of a basic block: `Goto`, `Branch`, `Delay` (`#5`), `Wait` (`@(posedge clk)`), `Fork`, `Call`, `Return` |
| **scheduler** | advances time and decides which process wakes when, implementing the IEEE region model (Active / Inactive / NBA / Observed / Reactive / Postponed) |
| **`fork … join`** | splits one process into concurrent branches (IEEE 1800 §9.3) |
| **arm** | one branch of a `fork`. In `fork begin A end begin B end join`, `begin A end` is an arm |
| **join modes** | `join` waits for every arm; `join_any` waits for the first; `join_none` waits for none and the parent continues immediately. The three are observably different and each has its own tests |
| **parent / child** | the process that executed the `fork` is the parent, each arm a child. Internally both are *activities*; a child runs the parent's body with its own program counter |
| **barrier** | the counter that implements a join. Miscounting produces a wrong order, not a crash |
| **`wait fork`** | wait until every child this process spawned has finished (IEEE 1800 §9.6.1). It is not a waiter: there is no signal to register on, and child-completion bookkeeping is what wakes it |
| **`disable fork`** | kill every descendant of the calling process; the caller survives (IEEE 1800 §9.6.3) |

### 1.2 This repository's vocabulary

| Term | Meaning |
|---|---|
| **tier-1 / `interp`** | the tree-walking interpreter. Walks the IR directly. The reference statement of what a construct means, and the oracle for the other two |
| **tier-2 / `vm`** | the bytecode VM. Compiles a body once per process template and runs an op loop |
| **tier-3 / `native`** | the default executor. Keeps net values in a flat `NetArena` and evaluates uniform-width expressions on a specialised evaluator. It generates no machine code — the name refers to native storage |
| **arena** | tier-3's flat array of net values. Tiers 1 and 2 keep values in `SimState::nets`; tier-3 keeps them in its own buffer, and that pair of stores is the root of nearly everything in this document |
| **kernel** | the `Kernel` trait between an executor and a store. Statement meaning lives in shared code generic over `Kernel`; only what differs per store is a kernel method |
| **gate** | the decision "can tier-3 take this design". It has three layers (§4.1) |
| **row** | one reject clause inside a gate, identified by a human-readable string such as `"a task frame that FORKS"` |
| **fallback** | routing a refused design to tier-2. A slower answer, not a wrong one |
| **sidecar** | a table alongside the IR rather than inside it, for example `fork_modes` (which `fork` is `join`/`join_any`/`join_none`) or `task_calls_proc` (argument-to-formal mapping per call site) |
| **correct-or-loud** | refuse loudly rather than answer wrongly in silence |
| **silent-wrong** | exit code 0, no diagnostic, wrong value |
| **accuracy ladder** | `silent-wrong ≪ loud ≪ correct-support`. Movement is upward only; turning something that worked into a refusal is also a regression |

### 1.3 Verification vocabulary

| Term | Meaning |
|---|---|
| **oracle** | an external tool that supplies the expected answer: Icarus Verilog 13.0, with Verilator 5.050 as a second opinion on 2-state arithmetic |
| **hand-IEEE** | an expected value derived by reading the IEEE 1364/1800 text, used where no external tool can arbitrate because the tool rejects the construct. The value is pinned as a literal and the test header records why there is no tool oracle |
| **differential** | running two implementations on the same input and comparing output. Here usually tier-3 against tier-2, or vita against Icarus Verilog |
| **absolute anchor** | a test that pins a concrete expected string rather than comparing two implementations. It catches the case where both implementations are wrong in the same way |
| **census** | counting what the code actually does over a population, rather than arguing about it. §3 and §4.2 are censuses |
| **coverage** | the fraction of the test suite's `simulate()` calls that tier-3 actually ran. Not code coverage. §3 |
| **mutation** | deliberately breaking one place in production code and checking whether the suite notices. §5 |
| **battery** | several mutations run as one sequence, each restore → substitute → build → run → verdict. §5.1 |
| **KILLED / SURVIVED** | the suite caught the mutation / it did not. `SURVIVED` is a question, not a failure. §5.3 |
| **flip run** | running the whole suite with the default backend inverted, to ask whether any result changes. §4.4 |

---

## 2. The three executors

### 2.1 One set of semantics

The three executors are not three simulators. What a statement means — what `q <= d + 1`
does — lives in the shared functions `compute_effect` and `apply_effect`, which are generic
over the `Kernel` trait. The executors differ in the `Kernel` implementation they use.

```
        statement semantics (shared, generic over Kernel)
             compute_effect / apply_effect
                          │
             ┌────────────┴────────────┐
      Kernel for Scheduler       Kernel for NativeKernel
        (interp and vm)                (native)
             │                            │
       SimState::nets                NetArena (flat buffer)
```

Two consequences follow, and they pull in opposite directions. Adding a backend is not
re-implementing the IEEE rules, which is why most coverage work is delegation rather than
construction. And a differential between two executors is structurally blind to a defect in
the shared code: if `compute_effect` is wrong, all three are wrong identically. Anywhere an
executor delegates, an absolute anchor is mandatory (§4.5).

### 2.2 What actually differs

**When the work is decided.** `interp` re-derives "this is an assignment, the lvalue is
here, the right-hand side is a binary Add" on every activation. `vm` lowers a body once to
a `CompiledBody` op stream. `native` reuses that same `CompiledBody` where it can, so this
axis alone does not make tier-3 faster than tier-2.

**Where values live, and in what shape.** This is the difference that shows in a profile.
Tiers 1 and 2 hold net values in `SimState::nets` as `Value`, a 72-byte structure carrying
two 4-state bit planes plus width, signedness and type flags. Tier-3 holds them in a flat
`NetArena` — a single `u32`-indexed word buffer, laid out `words * 2 * max(array_len, 1)`
per net — and evaluates uniform-width expressions of at most 64 bits on `WProg`, which
never constructs a `Value` at all. Measured shares of what disappears: `Value` marshalling
(`one_word_value` 7.3%, `resize` 4.7%, `mask_top` 3.7%); three comparison operators each
building two 72-byte `Value`s for their result (picorv32 0.781 s against 0.709 s); the last
`Value` consumers in `!`, the reductions and `&&`/`||` (0.712 s against 0.633 s); the write
funnel taking the general path for a one-word destination (0.633 s against 0.594 s).

**The granularity of fallback.** Tiers 1 and 2 mix per body inside one design: an
`always_ff` on the VM, a testbench `initial #1 …` on the interpreter. Tier-3 is
**all-or-nothing per design**, because it owns net storage — one process reading a value
from outside the arena would see the time-zero state. That single property is why a gate
exists and why coverage is a subject at all.

### 2.3 Measured

picorv32, release build, interleaved, best of five. Reproduce with
`cargo build --release -p cli --locked`, then from `bench/picorv32`,
`vita --backend <b> tb.v picorv32.v`; all three executors and Icarus Verilog end at
simulated time `399995000`.

| Executor | Time | vs `native` |
|---|---:|---:|
| `--backend interp` | 1.319 s | 2.57× slower |
| `--backend vm` | 0.838 s | 1.63× slower |
| `--backend native` | **0.513 s** | 1.00× |
| Icarus Verilog 13 | 0.585 s | 1.14× slower |

These ratios belong to this design. A different shape gives different ratios: an
optimisation worth 17% on one corpus row is flat on another. "A shared function got faster,
so every tier gains" is a claim until it is measured.

### 2.4 Why the interpreter stays

It is the definition of the answer. It walks the IR most directly, with no compiled form
and no second store, so when `vm` and `native` disagree there is something to arbitrate
with. It is a test instrument rather than a product surface: the `oracle` Cargo feature
makes that structural, and a build without it does not contain the variant. It is
permanently excluded from performance work, because every specialisation is a second
spelling of a semantic rule and a second spelling drifts silently. It remains load-bearing
inside an oracle build — the VM falls back to it body by body for anything
`is_codegen_able` refuses, and tier-3 delegates frame bodies to it.

---

## 3. Coverage

### 3.1 Definition

> **Coverage** is the fraction of the whole test suite's `simulate()` calls that tier-3
> `native` actually ran.

The denominator is `simulate()` invocations, not designs: one test running one design on
three backends contributes 3. The numerator is the invocations where `native` was requested
and the gate let it run. This is not code coverage and says nothing about how many source
lines a test executed.

`crates/sim-engine/src/lib.rs` records the census that made `Native` the default:
**6 470 / 6 470 = 100.00%**, zero refusals. The denominator is the suite size at the time
that census ran; the suite is 7 352 tests today, so re-running the census re-measures both
numbers. The generated-corpus half of the same question is a committed gate:
`sim-engine::native_gate::p6_corpus_eligibility_is_72_of_72`.

### 3.2 How a census is taken

The census is a method, not committed code. There is no census hook, no environment
variable and no script at HEAD; the only environment variables `sim-engine` and `cli` read
are `VITA_THREADS`, `REGEN_GOLDEN` and — behind the `jit` feature — `VITA_JIT` and
`VITA_JIT_STATS`.

The method: plant a temporary hook where `simulate` computes `native_refusal`, call the
three gate layers **independently**, and append one line per `simulate()` call:

```
D:fork\tS:a task frame that FORKS…\tX:a `wait fork`…\n
```

An empty line means no layer refused, i.e. tier-3 ran. Asking the three layers
independently is the point: production code short-circuits on the first refusal, so the
storage and executor verdicts for a design the design gate rejected are never measured.

Two mechanical hazards, both of which produce a plausible wrong number:

- **Buffering.** Writing with `writeln!` to an unbuffered `File` emits one `write(2)` per
  fragment, and a test runner that forks per test then interleaves them, tearing lines. Emit
  one line in one `write`, in append mode.
- **Two spellings of the default.** Inverting the `Backend` enum's `#[default]` moves only
  the library half if `SimOpts::default()` also hard-codes a variant. Invert both.

Re-run the census before starting any work that targets a gate row. A row's yield is
measured against the gate as it is now, and closing one row moves every other row's number:
a target estimated at +81 designs can measure +1 in isolation because 80 of the 81 are also
held by another row.

---

## 4. What tier-3 accepts, what falls back, and how that is reported

### 4.1 The three gate layers

A refusal has three different characters, and merging them would hide which one is
actually blocking.

| Layer | Function | Question | Example row |
|---|---|---|---|
| **D — design** | `sim_engine::native::design_eligibility` | is this feature inside v1 scope? | a statement-effect right-hand side outside the wired set |
| **S — storage** | `NetArena::buildable` | can this design's values live in the arena? | `"arena exceeds u32 words"` |
| **X — executor** | `sim_engine::native::run::executor_rows` | can today's executor walk every body? | ``a `wait fork`, a `fork`, or a call statement whose callee forks: S3b`` |

`design_eligibility` returns `NativeEligibility { eligible, buildable, refused,
reject_reasons }`, and it asks `NetArena::buildable` itself, so `refused` is the design
gate's first reject family or the storage refusal. `native::runtime_gate` is that
conjunction. The executor layer is asked separately in `simulate` and its answer is written
back into the published verdict, so `run.json` cannot report `refused: null` on a run that
fell back.

`SimOpts` is destructured exhaustively inside `design_eligibility`, with no `..` rest
pattern, and the `NetKind` loop is `_`-free. Adding a sidecar or a net kind without
classifying it is a compile error rather than a silent over-claim of eligibility.
Completeness of the classification is therefore not a test — it is the compile-time
exhaustive destructure.

### 4.2 What the design gate still refuses

**No design-gate reject family is reachable from source a compiler can produce.** One
family is still counted: `"stmt_effect"`, which fires for a `BlockingAssign` whose
right-hand side is in the statement-effect family and is not in `stmt_effect_wired`, or for
a flat-writing `SysTask` that is not `Sformat`, `ReadmemB`, `ReadmemH` or `Cast`. Every
`NetKind` arm is admitted, including every heap-storage kind — `DynArray`, `Queue`,
`String`, `Assoc`, `AssocStr` — whose values live in `SimState::dyn_heap` keyed by net id,
which the tier-3 kernel already borrows.

`stmt_effect_wired` names its members through the canonical `exec::kpred` predicates rather
than re-listing them: `value_plusargs_rhs`, `queue_pop_rhs`, `random_seeded_rhs`,
`dist_seeded_rhs`, `cast_rhs`, `assoc_iter_rhs`, `sscanf_rhs`, `fopen_rhs`, `fgetc_rhs`,
`feof_rhs`, `ungetc_rhs`, `fgets_rhs`, `fscanf_rhs`, `fread_rhs`.

`native::kernel::systask_refusal` returns `None` for every `SysTaskId`: the refused
system-task set is empty. The function and both consumers — the panic in
`k_dispatch_systask` and the `body_dispatch_ok` gate row — are kept so a new store-reading
task has to be classified.

The storage and executor rows that remain are reachable only from a malformed sidecar or a
construct no source can spell. Verbatim, these are the strings `native.refused` reports:

| String | Layer |
|---|---|
| `arena exceeds u32 words` / `arena exceeds usize` | S |
| `malformed frame sidecar (func_table length)` / `(frame window out of range)` / `(return slot out of range)` / `(block id out of range)` | S |
| `a module body that names a frame-local net` | S |
| `a call in a delayed continuous assign: S3b` | S |
| `a system task the tier-3 kernel refuses, inside a task frame` | S |
| `a nonblocking assign to a frame-local net: S3b` | S |
| `a nested call with no sidecar entry: S3b` / `a nested call to an unresolved target: S3b` | S |
| `a subroutine that WRITES a net outside its own frame: S3b` | S |
| `a subroutine statement the frame executor drops` | S |
| `a subroutine body that suspends, forks or calls a task` | S |
| ``a `wait fork`, a `fork`, or a call statement whose callee forks: S3b`` | X |
| `a system task the tier-3 kernel refuses (VCD, $monitor/$strobe, file)` | X |

An empty gate is not a deleted gate. All three functions and their consumers remain, and a
new feature that passes through unclassified is still refused. What is pinned today is that
the sets are empty, asserted with `is_empty()`, so adding a row forces either a design that
exercises it or a written reason why one cannot be built.

### 4.3 What runs where inside tier-3

Tier-3 owns net storage, so there is no body-level fallback: a design runs wholly on tier-3
or wholly on another executor. Inside tier-3, the choice is per body.

- For `act == tmpl`, if `compiled_for(tmpl)` yields a `CompiledBody` — the same tier-2
  `is_codegen_able` plus `compile_body` — the body runs through `crate::backend::vm_exec`
  over the arena.
- With the `jit` feature compiled in *and* `VITA_JIT` set in the environment, the entry
  block may instead run through `crate::jit::run_body_jit`. Both are off by default.
- Otherwise `native::body::run_body` walks the IR.
- A fork child always takes the walk.

Expression evaluation has its own lane. `native_eval::try_compile` returns a program only
when the whole tree is in its subset and every node's context-determined width is at most
64 bits: constants (non-real), scalar signals, `+ - * / %`, the four bitwise operators, all
eight comparisons, `<< >> >>>`, `&& ||`, unary `~ + - !`, the six reductions, `?:`,
`Select` with a dynamic offset (an X/Z or out-of-range offset yields X), `Concat`, and
`Replicate` with a constant count. Anything else — `**`, a system function, a `Call`, a real
constant, an array-indexed signal, more than 64 bits — makes the whole expression decline to
the kernel's tree-walking `eval_ctx`. Continuous assigns get their own pre-compiled programs
in `ca_native`, compiled in exactly the context `eval_for_lvalue` builds.

Tier-3's run loop mirrors `Scheduler::run` region for region: t0 structural settle, `arm_t0`,
`snapshot_preponed`, then [settle → Active → Inactive → NBA], Observed, Reactive,
`propagate` with a re-drain if anything woke, Postponed, and time advance as the minimum
over the wheel, the delayed NBA queue and the next delayed continuous assign. Everything
that is not a net value — the output sink, the file table, `now`, the RNG — is still the
scheduler's, and `NativeKernel` borrows it.

### 4.4 How a fallback is reported

| Build | Behaviour on a gate refusal |
|---|---|
| default (`oracle` feature on) | falls back to `Backend::Bytecode`, and emits a **Warning**, `W-RUN-BACKEND-FALLBACK` / `VITA-W4030`: *"requested backend `{req}` cannot run this design ({reason}); ran on `{eff}` instead — the result is unaffected, the speed is"* |
| `--no-default-features` | there is no fallback target, so `st.fatal_run` reports *"backend `native` cannot run this design ({row}), and this build carries no other executor — the `oracle` backends are compiled out"*, latching `had_fatal`/`finished`, and the run declines to execute |

The warning rather than an error in the default build is an accuracy-ladder decision: the
fallback is a slower answer, not a wrong one, since byte-identity across executors is a
gate. Making it non-zero exit would trade correct-support for loud, a rung down. In the
build where the fallback target is not compiled at all, the choice is loud-or-wrong instead,
so it is fatal.

The verdict is also published rather than only said. `run.json` carries:

```json
"backend": "native",
"backend_requested": "native",
"codegen": {"able": 4, "total": 4, "frame_bodies": 0, "reject_reasons": {}},
"native": {"eligible": true, "buildable": true, "refused": null, "reject_reasons": {}}
```

`backend` is the executor that ran; `backend_requested` is what was asked for; `refused`
names the layer and is `null` when nothing refuses. `codegen.able`/`total` count process
bodies the tier-2 gate admits, and `frame_bodies` counts subroutine bodies — `able == total`
with `frame_bodies > 0` means full process coverage and none of the run time on a compiled
path, so the two must be read together.

The population of the fallback path is **zero today**: every gate row a compiler can produce
input for is closed. The path is written fail-closed so a newly added row reports itself,
and its teeth in the suite are a deliberately corrupted sidecar
(`sim-engine::native_gate::b4a_a_backend_fall_back_emits_a_warning_naming_the_row`).

A fallback is quiet enough to be missed in a test. An anchor test that does not assert
`"backend": "native"` cannot distinguish a native run from a fallback that produced the same
bytes, which is a mistake this project has made and now pins against: 14 `cli` test targets
assert the field.

### 4.5 How equivalence is gated

Selecting a backend must not change one output byte. The invariant that makes that
enforceable is stated in `crates/sim-engine/src/lib.rs`: the shared net-write and VCD choke
point (`state.rs::write_lvalue` / `emit_vcd_change`) stays on the shared side across
backends, so only process-body control flow differs and VCD or stdout bytes cannot diverge
in a backend-specific way.

| Gate | Where | What it compares |
|---|---|---|
| Backend differential | `crates/sim-engine/tests/backend_equiv.rs` (28 tests, no skip, hard on every CI leg) | `corpus(0x5EED_F00D, 72)` — 72 generated designs built once into a `SimIr`, then run on both oracle backends concurrently via `thread::scope`, asserting byte-identical stdout, byte-identical VCD, and the `SimResult` summary (`sim_time`, `finish_reason`, `exit_class`). Plus hand-written shapes the generator cannot emit, a mixed-backend run, the timescale prologue, runaway-loop fatality, blocking-index sampling, and the native-eval arithmetic, XZ-poison, signed, bitwise, mixed-width, select/concat/replicate, wide-lane and indexed-read paths |
| Anti-vacuity on that gate | same file | `gate_actually_compares_vcd_bytes` asserts the VCD bytes are non-trivial; `the_default_backend_is_native` pins both spellings of the default |
| Tier-3 differential | `crates/sim-engine/src/native/run_tests.rs`, `#[cfg(all(test, feature = "oracle"))]` | `agree(src, name)` checks `runnable()` first so a refusal is counted rather than silently passed, runs the same IR on `Bytecode` and `Native` with per-design VCD targets, and merges `RtlOutput` and `Diagnostic` rows through a `MergedSink`. Anti-vacuity first: it asserts `r_nat.backend == Backend::Native`, so a fallback cannot make every later assertion compare the VM with itself |
| Gate teeth | `crates/sim-engine/tests/native_gate.rs` (23 tests) | every reject family actually fires, `the_runtime_gate_is_exactly_design_and_storage`, `every_stmt_effect_family_member_is_wired`, and the pinned generated-corpus eligibility count |
| External differential | `crates/sim-engine/tests/differential.rs` (27 tests) | vita against `iverilog -g2012` plus `vvp`, comparing `$display` stdout. Skips gracefully when the tools are absent — the design still runs through vita, so a vita-side crash is still caught. CI has no Icarus Verilog, so this is a developer-machine gate |
| Thread invariance | `crates/sim-engine/tests/threads.rs` | `--threads N` changes wall clock only; VCD bytes, stdout and the run summary are identical for every N |

**The flip run.** The strongest of these is not in the table, because it is a procedure
rather than a test: invert the default backend and run the whole workspace suite. The
generated 72-design corpus is a far weaker instrument than several thousand real tests, and
the flip run is what has found defects a green corpus differential did not. Run it in both
directions while two executors exist — `native → vm` asks whether the oracle still agrees,
without which the suite silently becomes native-only and the oracle stops being tested. The
expected outcome is that the only tests that change verdict are the ones asserting which
backend is the default; anything else is a real divergence. Invert both spellings of the
default (§3.2), or only half the suite moves.

**Absolute anchors are mandatory wherever tier-3 delegates.** A native-versus-VM
differential is blind to shared code by construction: as tier-3 delegates more, the two move
together and the differential sees nothing. Measured instances of exactly that: a shared
comparison rule, a shared dispatch, and a case where both backends produced the same wrong
value and agreed perfectly.

---

## 5. Mutation

### 5.1 The procedure

A green suite proves that the tests pass, not that they would notice a defect. Mutation
measures the difference. Break one place in production code, run the whole suite, and record
whether anything failed.

```
for case in [A, B, C, …]:
    1. restore     git checkout -- <files the case touches>
    2. substitute  replace one exact string, requiring an exact match count
    3. build+run   cargo build --tests   then   cargo nextest run --workspace
    4. verdict     a failing test name  => KILLED;  none => SURVIVED
```

A case names the file, the exact text to replace, the replacement, and the number of
matches required:

```python
("A_no_child_intercept", "crates/sim-engine/src/native/body.rs",
 """        if frames.is_empty() {
            if let Some(jbb) = k.k_child_join_bb(act) {
                if bb == jbb { k.k_body_done(act, tmpl); return Step::Done; }
            }
        }
""", "", 1),   # replacement "" = deletion; trailing 1 = must match exactly once
```

There is no committed battery script, no `xtask`, no `scripts/` directory and no
`cargo-mutants` configuration. The battery is assembled per unit of work, typically four to
eleven cases, and its cost is dominated by the build: a workspace relink is roughly eight
minutes against about thirty seconds of test execution.

Rules that make the verdicts trustworthy:

| Rule | Reason |
|---|---|
| Restore with `git checkout --`, which requires a snapshot commit first | restoring from file copies loses uncommitted edits |
| Check the exit code of the substitution and of the build | a substitution that matched zero places leaves an unmutated tree, which then goes green and is recorded as `SURVIVED` |
| Keep `SUBST-FAIL` and `BUILD-FAIL` as verdicts distinct from `SURVIVED` | same reason |
| Run `cargo nextest run --workspace` | a narrow filter manufactures fake `SURVIVED`s and cannot manufacture a fake `KILLED`, so a narrowed run's survivors must be re-checked at `--workspace`. `-p A -p B --test X` is not a fix: cargo applies `--test` to every package and runs a small fraction of the suite |
| Detect `FAIL`, `TRY 1 FAIL`, `TIMEOUT`, `SIGSEGV`, `SIGABRT`, `ABORT` and `LEAK-FAIL` | a mutation that turns a design into an infinite loop is reported by nextest as `TIMEOUT`, and a parser counting only lines starting with `FAIL` records it as `SURVIVED` |
| Take a mutation that can hang or run away out of the battery and run it once by hand | §5.4 |

### 5.2 Why it is necessary

Three measured reasons, each of which is a way for a green suite to prove nothing:

- **A gate can be blind to itself.** A `panic!` placed at the top of a settle function let
  the whole suite pass, because that code never executed.
- **A differential is blind to shared code.** See §4.5.
- **Mutation is what enforces the anchor obligation.** In one fork-related unit of work,
  the killer for five of the cases was a single anchor test written in that same unit, and
  no other test in the suite caught any of them — because the gate had been refusing `fork`,
  so no test in the repository ran a `fork` design on tier-3.

### 5.3 `SURVIVED` is a question

A surviving mutation is one of three things, and which one must be established.

| Reading | Meaning | Required action |
|---|---|---|
| **blind axis** | the tests do not exercise that axis | build a discriminating design and kill it |
| **equivalent** | the change makes no observable difference | measure why, and record the reason in the code |
| **unreachable** | no input reaches that code | re-measure with a `panic!` probe showing zero hits, and leave it fail-closed |

Each reading has a characteristic shape. A *blind axis* often needs a design with two of
something: changing a sibling-arm ordering key from the tie-break to the activity id passes
every single-`fork` anchor, because a lone `fork` allocates children in declaration order
and the two keys coincide; a second `fork` discriminates, since a finished child's slot
returns to a LIFO free list, so the second `fork`'s arms take ids set by the order the first
`fork`'s children finished. An *equivalent* mutation usually turns on one index: removing a
depth guard is equivalent in a walk that reads `frames[0]` and load-bearing in the twin that
reads `call_stack.last()`. An *unreachable* mutation is usually made unreachable by an
`_`-free match elsewhere: the terminators that can make `act != tmpl` are `Fork` and `Call`,
and both are in `is_codegen_able`'s reject set.

### 5.4 Mutation can be dangerous

A mutation that changes a descending loop's step to `+1` never reaches its sentinel and
appends without bound. Two `vita` test subprocesses reached about 33 GB each on a 32 GB
machine, and the kernel panicked on the userspace watchdog; the session died with the
mutation still in the tree.

Two standing consequences. `.config/nextest.toml` sets a per-test hard cap:

```toml
[profile.default]
slow-timeout = { period = "60s", terminate-after = 4 }
```

Four minutes, against a whole workspace of about 31 s of run-phase wall clock and a slowest
single test of about 18 s, so a test that reaches the cap is hung rather than slow. CI runs
`cargo test`, which does not read this file, so the cap protects local runs — which is where
batteries run. And the product fix, not the mutation, is what removes the hazard: making the
loop count-based (`abs_diff + 1`) makes the same mutation die on a value in under a second.

---

## 6. Operating rules this axis produces

1. **Re-run the census before starting work on a gate row.** A row's yield is measured
   against the gate as it stands, and every closed row moves the other numbers.
2. **Close the set that blocks a design, not a row.** A row's value is not the number of
   designs it fires on: two rows can read the same predicate and always fire together, and a
   gate that returns only its first `Err` hides a third row behind them.
3. **Re-read a reject row's stated reason.** A reason of the form "cannot do X" or "not
   until Y" is a claim about a moment; the next stage of the pipeline may already have made
   it false. A predicate stated over elaborate is not a predicate over `simulate`.
4. **Absolute anchors are mandatory where an executor delegates**, because the differential
   is blind there (§4.5).
5. **An anchor asserts `"backend": "native"`**, or it cannot tell a native run from a
   fallback.
6. **Suspect the harness before the engine when two backends agree perfectly.** A test
   helper that builds `SimOpts` by hand and omits one sidecar makes both backends agree
   about work neither performed.
7. **Do not create a second spelling of a semantic rule.** Extract and share it, and let the
   canonical site delegate. Two spellings drift silently.
8. **Batteries run `--workspace`; detection includes `TIMEOUT`** (§5.1).

---

## 7. Current state

| | |
|---|---|
| Coverage | `6 470 / 6 470 = 100.00%`, zero refusals, as recorded in `crates/sim-engine/src/lib.rs` |
| Generated-corpus eligibility | 72 / 72, pinned by `native_gate::p6_corpus_eligibility_is_72_of_72` |
| Design gate | no reject family reachable from compilable source; one counted family, `stmt_effect` |
| Storage gate | refuses only a malformed sidecar or an arena that exceeds `u32` words |
| Executor gate | refuses only shapes no source can construct |
| Refused system tasks | none — `systask_refusal` returns `None` for every `SysTaskId` |
| Fallback population | zero; the path is fail-closed and its teeth are a corrupted sidecar |
| Default executor | `Backend::Native`, pinned by `backend_equiv::the_default_backend_is_native` |
| Suite | 7 352 tests, 7 352 passing, 15 skipped (all `#[ignore]`d perf probes) |

The default build carries three executors with `native` as the default; the product build
(`--no-default-features`) carries `native` alone, and the oracle spellings of `--backend` are
rejected loudly there rather than silently ignored — a CI job asserts that rejection.

---

## 8. Commands

```bash
# Full local gate
cargo nextest run --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check

# One design on each executor, and what actually ran
vita --backend interp t.sv
vita --backend vm     t.sv
vita --backend native --obs-dir obs t.sv && grep '"backend"' obs/run.json

# The oracle
iverilog -g2012 -o t.vvp t.sv && vvp t.vvp

# The equivalence gates
cargo test -p sim-engine --test backend_equiv
cargo test -p sim-engine --test native_gate
cargo test -p sim-engine --test differential      # needs iverilog + vvp on PATH
```

---

## 9. Related documents

- [study/01 — the performance axis](01-interpreted-vs-compiled.md) — why tier-3 exists, and
  the standing verdict on every acceleration evaluated.
- [study/03 — the workload corpus](03-workload-corpus.md) — third-party RTL, its oracles,
  and the grading of a refusal.
- [preview/04 — architecture](../preview/04-architecture.md) — the execution-backend
  structure.
- [preview/21 — tier-3 native backend](../preview/21-tier3-native-backend.md) — what a
  machine-code backend would require.
- [preview/09 — testing and verification](../preview/09-testing-and-verification.md).
- [ENGINEERING_RULES](../ENGINEERING_RULES.md) — the accuracy ladder and the review method.
- [ROADMAP](../ROADMAP.md) — the open queues.
