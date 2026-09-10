# 04 — System architecture

The shape of the simulator: the pipeline and where language dependence ends, the crate
graph and the dependency rules that hold it, what each crate owns, the one-shot and staged
execution models, the three executors and how a process body reaches each of them, and the
mechanisms that make all of it enforceable rather than aspirational.

Companion contracts: the build and feature contract in
[03-build-and-portability.md](03-build-and-portability.md); the scheduler in
[06-simulation-engine.md](06-simulation-engine.md); the frozen IR in
[17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md); the native backend in
[21-tier3-native-backend.md](21-tier3-native-backend.md).

---

## 1. The pipeline

```text
source files
  → preprocess   (`define / `ifdef / `include / `timescale)
  → lex          (token stream)
  → parse        (AST; grammar checking happens here)
  → elaborate    (parameters, generate, instances, hierarchy flattening, type and
                  connectivity checks)
  → sim-ir       (nets, processes, sensitivity, subroutine bodies, system-call nodes —
                  language-neutral)
  → sim-engine   (event-driven kernel, time wheel, system-task dispatch)
  → VCD / FST    (written only when the design calls a dump task, or -o names a path)
```

| Stage | Crate | Consumes | Produces | Checks it performs |
|---|---|---|---|---|
| preprocess | `hdl-preprocess` | source text | preprocessed text plus a source map and the timescale regions | directive syntax, include resolution, macro arity |
| lex | `hdl-lexer` | preprocessed text | tokens with spans | a structural lexical error carrying its span and reason |
| parse | `hdl-parser` | tokens | the AST | grammar. Parsing failure stops the run; elaboration is not attempted |
| elaborate | `elaborate` | the AST | `sim_ir::SimIr` plus out-of-band sidecar tables | parameter override evaluation interleaved with generate, recursive instance resolution and flattening, port and type compatibility, unconnected nets, multiple drivers |
| simulate | `sim-engine` | `SimIr` plus `SimOpts` | a transcript, a waveform, an exit class | run-time limits, convergence, index range, and every `$` task |
| waveform | `vcd-writer` | value changes from the engine's write funnel | VCD, or FST by transcode | none; it serializes |

Diagnostics and operational output cross every stage as one event stream. `diag` owns the
data model and the shared `$display` field rules; `vita-log` owns severity gating; `cli` is
the only crate that installs a concrete sink. See
[13-diagnostics-and-logging.md](13-diagnostics-and-logging.md).

### 1.1 Where language dependence ends

Language dependence ends at parse. `elaborate`, `sim-ir`, `sim-engine` and `vcd-writer`
contain no Verilog, SystemVerilog or VHDL knowledge; they operate on the neutral IR. Apart
from the driver, `elaborate` is the only crate that depends on both `hdl-ast` and `sim-ir`,
which makes it the single place where language knowledge is converted into simulation
knowledge.

That boundary is what would let a second front end reuse the whole back half. No such front
end exists: vitamin accepts Verilog and SystemVerilog only, and a VHDL front end is a
conditional roadmap item, not implemented. The reference notes under
[hdl-reference/vhdl/](hdl-reference/vhdl/) document the standard, not vitamin's support.

---

## 2. The crate graph

Seventeen crates in one workspace. Fourteen are publishable; `hdl-builtins`, `vcd-diff` and
`corpus-runner` carry `publish = false`.

```text
        vita-schema ──► vita-artifact-derive (proc-macro)
             │                    │
             └──────┬─────────────┘
                    ▼
              hdl-ast ────► hdl-parser ◄──── hdl-lexer
                 │
                 ▼
  diag ──►   elaborate ──► sim-ir ──► vcd-writer
   │  │          │            │            │
   │  └──────────┴────────────┴────────────┴──► sim-engine
   │
   ├──► hdl-preprocess          vita-artifact ──┐
   └──► vita-log ───────────────────────────────┴──► cli  →  bin `vita`
```

Direct dependencies, exactly as the manifests declare them:

| Crate | Depends on (workspace) | Depends on (external) |
|---|---|---|
| `diag` | — | — |
| `vita-schema` | — | `blake3` |
| `vita-artifact-derive` | — | `syn`, `quote`, `proc-macro2` |
| `hdl-preprocess` | `diag` | — |
| `hdl-lexer` | — | `logos` |
| `hdl-ast` | `vita-schema`, `vita-artifact-derive` | `serde` |
| `hdl-parser` | `hdl-lexer`, `hdl-ast` | — |
| `sim-ir` | `vita-schema`, `vita-artifact-derive` | `serde` |
| `elaborate` | `hdl-ast`, `sim-ir`, `diag` | `serde` |
| `vcd-writer` | `sim-ir` | `fst-writer` |
| `sim-engine` | `sim-ir`, `diag`, `vcd-writer`, `elaborate` | `libm`; `cranelift-*` under `jit` |
| `vita-artifact` | `sim-ir`, `vita-schema`, `diag` | `serde`, `postcard` |
| `vita-log` | `diag` | — |
| `cli` | eleven: `hdl-preprocess`, `hdl-lexer`, `hdl-parser`, `hdl-ast`, `elaborate`, `sim-ir`, `sim-engine`, `diag`, `vita-log`, `vita-artifact`, `vita-schema` (`vcd-writer` and `vita-artifact-derive` arrive transitively) | `postcard`, `blake3`, `serde` |
| `hdl-builtins`, `vcd-diff`, `corpus-runner` | — | — |

`diag`, `vita-schema` and `vita-artifact-derive` are the leaf crates: nothing in the
workspace is below them, so each is testable in complete isolation.

### 2.1 The edges that are deliberately absent

An absent edge is a rule. Each of the following is enforced by the manifests and would be
noticed the moment it changed.

| Non-edge | Rule it encodes |
|---|---|
| `diag` depends on nothing | the diagnostic model stays IO-free and allocation-light, so every producer can depend on it. Rendering and sinks live above it |
| `hdl-lexer` and `hdl-parser` do not depend on `diag` | the front end raises structural errors carrying a span and a reason; mapping those to a `MsgCode` is the driver's job, which keeps the error model separable from rendering |
| `hdl-lexer` does not depend on `hdl-preprocess` | the lexer consumes text, not a preprocessor. `cli` wires the two together |
| `sim-engine` does not depend on `hdl-builtins` | the `$` handlers live inside the engine, in its private `builtins` module. `hdl-builtins` is the reserved extraction target |
| `elaborate` does not depend on `sim-engine` | the one edge between them points the other way, and it exists only so `SimOpts` can name the join-mode sidecar types. There is no cycle |
| `vita-artifact` does not depend on `hdl-ast` | the artifact crate owns the container — header, versioning, staleness gates — and never the body. Body serialization belongs to `cli`, so the container stays independent of what it carries |
| The schema trait is in `vita-schema`, not `vita-artifact` | putting it in `vita-artifact` would create `sim-ir` → `vita-artifact` → `sim-ir`. Splitting the runtime trait into its own leaf is what breaks the cycle ([16-schema-hash-spec.md](16-schema-hash-spec.md)) |
| Nothing depends on `vcd-diff` | it is a one-line stub with no CLI and no caller |

---

## 3. What each crate owns

`hdl-preprocess` isolates everything before language parsing: include search, macro
expansion, conditional compilation, and the `` `timescale `` regions that determine the
design-wide precision. Later stages see only preprocessed text and the source map that
points back into the originals.

`hdl-lexer` owns the token set. Keyword sets differ between the languages, so the variation
is contained here.

`hdl-parser` is a hand-written recursive-descent parser. Grammar checking happens inside it.
It is split from `hdl-ast` because the AST types are referenced by both the parser and
elaborate; keeping the types in their own crate is what prevents a cycle.

`hdl-ast` holds the AST types. They derive `serde` and `SchemaHash`, because the AST is the
root of the `.vu` artifact.

`elaborate` produces two structurally separate things. One is `sim_ir::SimIr`, the golden
serialized IR that `format_version` and the staleness gate protect. The other is a set of
out-of-band sidecar tables — join modes, net names, statement locations, class layouts,
coverage manifests and about sixty more — which never enter `SimIr`, never affect the
schema hash, and reach the engine through `SimOpts`. That split is what lets engine-facing
information be added without touching the frozen golden.

`sim-ir` depends on no HDL and no backend. It also holds the rules that both elaborate and
the engine must agree on — the self-determined width rule, static real-ness, the multi-word
limb kernels, the system-task and system-function name tables — as single definitions, so a
second spelling cannot drift from the first. The narrower its surface, the cheaper an
executor is to add or replace.

`sim-engine` owns the event-driven kernel: the time model, the region cascade, delta
cycles, the net store, the heaps, and the three executors. The `$` task and function
handlers are inlined in its private `builtins` module.

`hdl-builtins` is a one-line stub reserved as the extraction target for those handlers.
Marker comments in the engine identify the boundary along which they would move.

`vcd-writer` owns serialization only: the VCD header, the `$scope` hierarchy, `$var`
declarations, value-change records, and the VCD-to-FST transcode, which delegates the FST
binary encoding to `fst-writer`. It contains no execution logic.

`diag` owns the diagnostic data model — `Diagnostic`, `MsgCode`, `Severity`, `SourceLoc`,
`Frame`, `LogEvent`, `LogSink` — and the `$display` format-field rules shared by the
run-time renderer in the engine and the elaboration-task renderer in elaborate. It performs
no IO.

`vita-artifact` owns the `.vu` and `.velab` container: the magic, the header, the version
and the three staleness gates. See [14-staged-artifacts.md](14-staged-artifacts.md).

`vita-artifact-derive` provides `#[derive(SchemaHash)]` and nothing else. It runs inside
rustc, which is how structural hashing obeys the no-build-script rule.

`vita-schema` provides the `SchemaShape` trait, the shape registry, and the blake3
composition that turns a type closure into one hash.

`vita-log` owns severity policy: `-Wno-<CODE>` suppression and `-Werror[=<CODE>]`
promotion, applied between the producers and the sink. Errors and fatals are never
suppressed.

`cli` is the driver. It parses argv and filelists, wires the stages, owns the artifact
bodies, writes the observability rail, installs the only concrete `LogSink`, and produces
the `vita` binary. The whole CLI runs on a spawned worker thread with a 256 MiB stack,
because the parse and elaborate recursion needs far more than an OS default.

---

## 4. Execution model

The pipeline is driven two ways.

One-shot, `vita`: preprocess through waveform in one command, with nothing written to disk
between stages. This is the default entry point.

Staged, `vcmp` / `velab` / `vrun`: the same pipeline split into three independently
invocable stages, each reading what the previous one left on disk.

| Command | Stage | Crates | Input | Output |
|---|---|---|---|---|
| `vcmp` | compile | `hdl-preprocess`, `hdl-lexer`, `hdl-parser` | HDL sources | `<first-source>.vu`, and a work-library entry when `--work` is given |
| `velab` | elaborate | `elaborate` | one `.vu`, or libraries bound with `-L` | `<input>.velab` — the golden IR plus non-golden trailers |
| `vrun` | simulate | `sim-engine`, `vcd-writer` | one `.velab` | transcript, waveform, exit class |

Mapping to the commercial flow, which the three-stage split follows one to one:

| vitamin | Cadence Xcelium | Synopsys VCS |
|---|---|---|
| `vcmp` | `xmvlog` / `xmvhdl` | `vlogan` / `vhdlan` |
| `velab` | `xmelab` | `vcs` (elaborate, then build `simv`) |
| `vrun` | `xmsim` | `simv` |
| `vita` | `xrun` | — |

What the split buys:

- Stage isolation. A failure can be attributed to compile, elaborate or simulation by
  looking at which stage refused.
- Re-running only simulation. Unchanged sources mean the `.vu` and `.velab` are reused and
  only `vrun` repeats.
- Independent binaries for debugging. The dev-only `separate-bins` feature emits standalone
  `vcmp`, `velab` and `vrun` targets over the same code path.

Skipping a stage is only sound if staleness is detected rather than assumed. Every artifact
read checks `format_version`, then `tool_semver_major`, then `schema_hash`, lowest first,
and all three rejections exit 2 naming the stage to re-run. Content freshness against live
sources is checked automatically for library-bound units, and for a `.vu` given to `vrun`
when `--upstream` names it. Timestamps are never used. The authority on all of this,
including how each flag binds into a staleness hash, is
[14-staged-artifacts.md](14-staged-artifacts.md); the governing rule is that a flag is
classified by which stage's output it perturbs, not by which binary parses it.

Two differences between the flows are contract, not accident. The observability rail —
`--obs-dir`, `--obs-procs`, `--probe`, `$vita_stage` — is a one-shot `vita` surface; the
staged applets refuse those flags loudly rather than accepting them and emitting nothing.
And each staged applet refuses the arguments belonging to another stage: `vcmp` and `velab`
refuse `--backend` and runtime plusargs, `velab` and `vrun` refuse preprocessor definitions
and include paths, `vcmp` and `vrun` refuse `-G` parameter overrides.

### 4.1 One binary, four names

The default build emits exactly one binary. `vita` dispatches on the stem of `argv[0]`:
`vcmp`, `velab` and `vrun` select the staged applets, anything else selects the one-shot
applet. Each stage is also reachable explicitly as `vita vcmp …`, `vita velab …`,
`vita vrun …`, where the subcommand must be the first argument.

Dispatch is hand-written rather than delegated to a CLI framework's multicall mode, because
the default applet takes positional source files: a framework that strips `argv[0]` and
reads the next token as a subcommand name cannot express `vita top.sv`.

The installer creates `vcmp`, `velab` and `vrun` as symlinks beside the installed `vita`,
falling back to copies where the filesystem rejects links. Because the explicit
`vita <applet>` form exists, a renamed or copied binary still reaches every stage.

---

## 5. The executor architecture

Three executors exist. One of them is the product.

```text
                     sim-ir  (language-neutral, SchemaHash-frozen)
                                       │
                                       ▼
                     ┌──────────────────────────────────────┐
                     │  statement semantics — one copy       │
                     │  exec::{compute_effect, apply_effect} │
                     │  generic over the `Kernel` trait      │
                     └──────────────────────────────────────┘
                                       │
                   ┌───────────────────┴───────────────────┐
                   ▼                                       ▼
       impl Kernel for Scheduler                impl Kernel for NativeKernel
                   │                                       │
       ┌───────────┴───────────┐                           │
       ▼                       ▼                           ▼
   interp                    vm                        native
   walks the IR tree      compiled op stream       flat arena + a specialised
   on every activation    per body template        evaluator that builds no
                                                    boxed value
       └───────────┬───────────┘                           │
                   ▼                                       ▼
           SimState::nets                              NetArena
           (4-state `Value`)                           (flat word planes)
```

These are not three simulators. The meaning of a statement is written once, in code generic
over the `Kernel` trait, and an executor is an implementation of that trait. Adding an
executor therefore does not mean re-implementing IEEE rules — and, symmetrically, an error
in the shared code is wrong in all three at once, which is why an absolute oracle is
mandatory and agreement between executors is never sufficient on its own.

### 5.1 What actually differs

| Axis | `interp` | `vm` | `native` |
|---|---|---|---|
| When "what to do" is decided | every activation | once per body template | once per body template, reusing the same compiled body |
| Where net values live, and in what shape | `SimState::nets`, as 4-state `Value` | the same | a flat `NetArena`; a uniform-width expression of 64 bits or fewer builds no `Value` at all |
| Bit planes evaluated | always two, value and unknown | two | one first, re-running on two planes on meeting an unknown leaf |
| Granularity of falling back | per body, mixed within a run | per body, to the interpreter | whole design, all or nothing |

The dominant cost difference is the second axis, not the first: giving the native backend
the VM's compiled bodies is on its own a wash, and the gains come from removing value
marshalling. The single-plane lane rests on measurement — every benchmark shape is 100%
definite and picorv32 is 90.1% of runs — and its fallback is the two-plane path itself, so the
fast lane adds no new semantics and no new correctness surface.

`native_eval` is a fourth thing that is not a backend: a compiler for expressions whose
every node evaluates to 64 bits or fewer, used by the VM and by continuous assignments. An
expression outside its subset compiles to nothing and the generic evaluator runs instead.

### 5.2 How a body reaches an executor

| Executor | Route |
|---|---|
| `interp` | `exec::run_process` walks the IR body directly |
| `vm` | `is_codegen_able` accepts the body, `compile_body` lowers it to an op stream cached per process template, and `vm_exec` runs it. A refused body falls back to `run_process`, body by body |
| `native` | the design passes all three gate layers, then per body: the same `is_codegen_able` and compiled body, run by `vm_exec` over the arena; otherwise `native::body::run_body` walks the IR. A fork child always takes the walk |

A native run drives the design from its own loop, which mirrors the scheduler region for
region and shares everything that is not a net value — the output sink, the file table,
simulation time, the random-number state.

### 5.3 The default is `native`

`Backend::Native` carries `#[default]`, `SimOpts::default()` sets it, and
`crates/sim-engine/tests/backend_equiv.rs::the_default_backend_is_native` asserts both
spellings so they cannot disagree. That is the contract. Some rustdoc comments in the
engine name a different default; they are stale, and the enum, the constructor and the test
are what govern.

`--backend` is a debugging control, not a product surface, and the help text says so. It is
accepted by `vita` and `vrun` and refused by `vcmp` and `velab`, because nothing in an
artifact depends on it.

Measured on picorv32, release builds, interleaved best of five:

| `--backend interp` | `--backend vm` | `--backend native` | Icarus Verilog 13 |
|---:|---:|---:|---:|
| 1.319 s | 0.838 s | 0.513 s | 0.585 s |

A single design cannot judge a backend. The shape-by-shape harness in
`crates/sim-engine/tests/perf_baseline.rs` is the authority, and its method is in
[study/01](../study/01-interpreted-vs-compiled.md).

### 5.4 The gate — three layers, asked independently

Because the native backend owns net storage, a process reading a value from outside the
arena would see the value as of time zero. Body-level fallback is therefore impossible and
eligibility has to be decided for the whole design.

```text
  design ─► ① design_eligibility ─► ② NetArena::buildable ─► ③ executor_rows ─► native run
              (scope)                  (storage)               (executor)
                │                          │                       │
                └──────────────────────────┴───────────────────────┘
                          any refusal → fall back, or fatal, per build shape
```

| Layer | Question |
|---|---|
| `design_eligibility` | do any feature families in this design put it out of scope |
| `NetArena::buildable` | can the arena hold this design's storage |
| `executor_rows` | can the executor that exists run every body |

Production code short-circuits on the first refusal, so a census that wants all three
answers must ask them separately; a design refused by the first layer produces no data
about the other two. The third layer's answer is published in the run result rather than
merely consumed, so a report can name which layer refused.

No reject family in the first layer is reachable at HEAD: the corpus census is
6 470 of 6 470 designs eligible, with zero refusals, and the refusal set of the engine's
system-task classifier is empty. The checks are not removed. The classifying matches carry
no catch-all arm, so a new statement kind or net kind cannot compile until it is
classified, and the tests pin emptiness as a present state rather than as an assumption.

### 5.5 What a refusal costs, per build shape

| | Build with `oracle` | Build without it |
|---|---|---|
| A gate refusal | falls back to the VM and emits `W-RUN-BACKEND-FALLBACK` | a graceful fatal naming the refusing layer, exit 1 |
| Why | a fallback is a slower answer, not a wrong one, so a non-zero exit would be a regression on the accuracy ladder | with no fallback compiled, the only choices left are loud and wrong |

The fallback warning says what was requested, what ran, and that the result is unaffected
while the speed is. The run result reports the executor that actually ran, beside the one
requested.

### 5.6 Why the slowest executor stays

`interp` interprets the IR most directly, which makes it the readable statement of what the
semantics are. The VM and the native backend compute the same meaning by other means, and
when two of them disagree something has to decide which is wrong.

Two rules follow, and both are contract:

- It is a test instrument, not a product surface. In a release-shaped build
  `Backend::Interpreter` does not exist at all.
- It is permanently excluded from performance work. Every specialisation is a second
  spelling of a rule, and a second spelling is this codebase's defect class. When a profile
  points at the interpreter's body loop, the answer is that the design should not be
  running there, not that the loop should be optimized.

It is not dead code even so: in an oracle build the VM falls back into it per refused body,
and the native backend delegates subroutine-frame bodies to it. "Not a product surface" is
a statement about the flag.

Machine-code generation exists behind the off-by-default `jit` feature and is rejected on
measurement; see [03-build-and-portability.md](03-build-and-portability.md) §3.3 and
[18-acceleration-analysis.md](18-acceleration-analysis.md).

---

## 6. IR design principles

The IR is language-independent. It knows no Verilog syntax, no SystemVerilog type
declaration and no VHDL entity. It carries nets with width and 4-state initial values,
processes with a sensitivity and a body, continuous assignments, hierarchy instances,
subroutine definitions, and system-call nodes. Everything language-specific is digested
during elaboration.

4-state is the primary representation. A value is two bit planes, `val` and `unk`,
encoding 0, 1, X and Z, and the same encoding is used by the IR, the engine and the
waveform writer, so a value crosses those boundaries without conversion. Initialization to
X, multi-driver Z resolution and X propagation all rest on it. Two-state optimization is an
internal fast path, never a change of representation.

System calls are IR node kinds. `$display(...)` or `$dumpvars` parses to a language-level
call node and elaborates to an IR node carrying a closed-enum id, checked arguments and a
result type. A name the IR has no id for, or an argument mismatch, is a loud
`E-ELAB-UNSUPPORTED` rejection at elaboration, never a silent no-op.

The dump family is routed, not special-cased at the leaf. `$dumpfile`, `$dumpvars`,
`$dumpon`, `$dumpoff`, `$dumpall`, `$dumpflush` and `$dumplimit` reach the waveform writer
through the engine's dispatch. Without a call on that path, no waveform file is created,
and that is not an error.

The IR surface stays narrow. The fewer types and traits it exposes, the simpler the
contract every executor has to satisfy, which is the same discipline Yosys applies by
requiring every front end to produce RTLIL.

The IR is span-free. Source positions are a front-end concern and do not belong in
language-neutral nodes. A run-time diagnostic that has to name a source line reads an
out-of-band table keyed by statement id, holding file, line, column, byte range and the
instance path the statement was elaborated under — the instance path because a module
instantiated N times lowers N copies of one statement and `file:line:col` alone cannot tell
them apart. The table is resolved once during elaboration, which is also what makes
one-shot and staged diagnostics identical by construction. Keeping spans out of the IR is
what lets the schema hash cover only the neutral core.

Status at HEAD: the frozen root also contains a `SuspendState` per process, with a wake key
and a region tag. No engine code reads or writes it; elaboration emits one constant value
and live suspension state is held engine-side. Two of the four region-tag variants are
never constructed anywhere in the workspace. The shape is part of the frozen root and
cannot be removed without a `format_version` bump, so it stands as reserved space rather
than as a description of how scheduling works. The scheduler's real region model is in
[06-simulation-engine.md](06-simulation-engine.md).

---

## 7. System-task dispatch

How one `$` call crosses the pipeline:

1. Parse. `$name(args)` becomes a language-level call node holding the name and the parsed
   argument expressions.
2. Elaborate. The node becomes an IR node carrying a `SysTaskId` or `SysFuncId` — closed
   enums defined in `sim-ir` — with argument count and types checked and the result type
   fixed. An unknown name is rejected here.
3. Dispatch. The engine's `builtins` module maps the id to a handler. Handlers are grouped
   by category: display and stream IO, file IO, simulation control, time, conversion,
   bit-vector queries, real math, random and distributions, the dump family, assertion
   sampling, introspection, string and array methods, and the class and constrained-random
   surface.
4. Execute. A handler receives the simulation context it needs — current time, net state,
   the file table, the heaps — through the `Kernel` seam, so a handler behaves identically
   under every executor.
5. Route. Dump-family handlers call `vcd-writer`.

Status at HEAD: this dispatch lives inside `sim-engine`. `hdl-builtins` is the crate
reserved to receive it as a separate dispatch table, and is a one-line stub. Only step 3's
location changes if that extraction happens; the ids, the checks and the dump routing are
unaffected.

The standards reference for each family is under
[hdl-reference/system-tasks/](hdl-reference/system-tasks/); what vitamin actually supports
is in [../manual/005_system-tasks.md](../manual/005_system-tasks.md).

---

## 8. What keeps the layering enforceable

A layering rule that only exists in prose decays. Each rule below has a mechanism.

| Rule | Mechanism |
|---|---|
| The dependency graph is acyclic and the leaves stay leaves | Cargo refuses a cycle; the manifests in §2 are the statement |
| A frozen IR type cannot change shape unnoticed | the structural `SchemaHash` root is pinned by `crates/sim-ir/tests/schema_hash.rs`; changing a field flips it |
| A frozen type stays portable | `crates/sim-ir/tests/no_float_usize.rs` bans `usize`, `isize`, `f32`, `f64`; `no_serde_attrs.rs` bans serde attributes; collections are `BTree`-only and types are span-free |
| Cross-type IR fields keep one spelling | `crates/sim-ir/tests/body_refs.rs` rejects a bare reference; the fully-qualified `sim_ir::Foo` form is required |
| A `SchemaHash` type does not move between modules | the canonical key embeds `module_path!()`. This one has no test: it is why `hdl-ast`'s and `sim-ir`'s serialized types live at their crate roots |
| A new sidecar cannot slip past the native gate | the eligibility check destructures `SimOpts` exhaustively with no rest pattern, so an unclassified field fails to compile |
| A new statement kind or net kind cannot slip past a classifier | the classifying matches carry no catch-all arm |
| The executors cannot diverge in output | net writes and waveform emission go through one shared choke point, so only body control flow differs between executors; `crates/sim-engine/tests/backend_equiv.rs` asserts byte-identical stdout and waveform |
| A diagnostic code and its documentation cannot drift | `crates/diag/tests/bijection.rs` gates the `MsgCode` enum against [15-error-code-reference.md](15-error-code-reference.md), 68 variants, one to one |
| A source file does not grow past readability | the standing limit is about 1000 lines, split by adding a submodule with a `use super::*` prelude and re-exporting from the crate root, keeping the types at the root so a child module can reach private fields, and never splitting a single `trait impl`. Status at HEAD: 46 non-test source files exceed the limit |

---

## 9. Position relative to the reference implementations

### Icarus Verilog

Icarus splits into `iverilog`, which lexes, parses into a decorated parse tree, elaborates
to a netlist form and emits a text bytecode through a target API, and `vvp`, which runs it.
The `vvp` runtime is two-layered: a structural layer of functor nets encoding 4-state as
two bits with truth tables for combinational logic, and a behavioural layer of threads
corresponding to `initial` and `always` blocks, interacting with the nets through an
instruction set and a skip-list event queue.

Shared with vitamin: the front end and run time are separable, 4-state is a first-class
representation, and execution is event-driven. Icarus is also vitamin's live differential
oracle for every construct it accepts.

### Verilator

Verilator translates to C++ and lets a general compiler optimize the result: parse, roughly
twenty AST passes, a static ordering pass, then C++ emission. The emitted evaluation
function executes regions top to bottom with no dynamic event queue, and multithreading
comes from statically partitioned macro-tasks. Its own internals document states that
dynamic scheduling is rejected in favour of the static assignment.

That choice buys speed and gives up complete IEEE 1800 stratified event scheduling, which
is exactly the property vitamin has to hold from the start, together with timescale
precision and waveform accuracy.

### Where vitamin sits

vitamin is event-driven and 4-state, in the same class as Icarus, and its default executor
is compiled: the native backend runs the whole corpus byte-exactly against the interpreter
and is the only executor in a release-shaped build. What is not adopted is the C++ or
machine-code generation half of Verilator's approach — cranelift was built, wired and
measured, and rejected on the numbers.

The `sim-ir` boundary is what made replacing the executor possible without touching the
front end, which is the load-bearing claim of this whole architecture.

---

## Related documents

- [03-build-and-portability.md](03-build-and-portability.md) — the workspace, the features
  and the CI axes referred to above
- [06-simulation-engine.md](06-simulation-engine.md) — the scheduler, the regions and the
  process model
- [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md) — the diagnostic model and
  the exit-class contract
- [14-staged-artifacts.md](14-staged-artifacts.md) — the on-disk artifacts, work libraries
  and staleness rules
- [16-schema-hash-spec.md](16-schema-hash-spec.md),
  [17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md) — the structural hash
  and the frozen types
- [21-tier3-native-backend.md](21-tier3-native-backend.md) — the native backend's design, its
  eligibility gates, its refusal and fallback path and its equivalence contract
- [study/02](../study/02-v1-native-coverage.md) — the coverage terminology and the native
  backend's measured coverage
- [../history/research-log/eda-architectures-2026-05-28.md](../history/research-log/eda-architectures-2026-05-28.md)
  — the source notes behind the comparison above
- Icarus Verilog developer guide: https://steveicarus.github.io/iverilog/developer/guide/index.html
- Verilator internals: https://github.com/verilator/verilator/blob/master/docs/internals.rst
- Yosys RTLIL: https://yosyshq.readthedocs.io/projects/yosys/en/stable/yosys_internals/formats/rtlil_rep.html
