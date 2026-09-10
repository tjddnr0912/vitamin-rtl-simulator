# 10 · Glossary

Every term this documentation set uses with a fixed meaning, defined once. Where a term
comes from the language standards, the IEEE clause is given; where it is this project's own
vocabulary, the column reads `—`. A term defined here means the same thing in every other
document in the tree, and a document that needs a different meaning defines it locally
instead of redefining one of these.

Two standards are cited throughout. `IEEE 1364` is IEEE 1364-2005, the Verilog LRM;
`IEEE 1800` is IEEE 1800-2017, the SystemVerilog LRM, which subsumes it.

---

## 1. The pipeline

The stage names below are the whole pipeline, in order:
`preprocess → lex → parse → elaborate → sim-ir → sim-engine → waveform`. Everything up to
and including parse is language-dependent; everything after it is language-neutral.

| Term | Definition | IEEE |
|---|---|---|
| **Compile** | Preprocessing, lexing, parsing and syntax checking, taken together. The product is an AST rooted at `hdl_ast::SourceUnit`. Directive and syntax diagnostics are reported here and nowhere later. | 1800 §22 (directives) |
| **Elaboration** | Turning an AST into a simulatable design: parameter resolution, hierarchy instantiation, type checking, port connectivity, generate expansion and the multi-driver check. The product is `sim_ir::SimIr`. | 1800 §23 |
| **sim-ir** | The language-neutral simulation IR: the output of elaborate and the only input the engine reads. Nine arenas of `Vec`, indexed by `u32` — `instances`, `nets`, `processes`, `cont_assigns`, `funcs`, `exprs`, `stmts`, `blocks`, `consts`. Abstracting Verilog and SystemVerilog semantics at this boundary is what lets a second execution backend be added without a rewrite. | — |
| **Design unit** | One elaborable declaration: a `module`, `interface`, `package` or `class`. Units are the granularity a work library records. | 1800 §3 |
| **Compilation unit** | The set of source files compiled together, sharing one `` `define `` and one file-scope declaration space. | 1800 §3 |
| **Top / root** | The design unit elaboration starts from. Inferred when one candidate is unambiguous, pinned with `--top`, and required in library mode. | 1800 §23 |
| **One-shot flow** | `vita design.sv` — compile, elaborate and simulate in one process, streaming through memory, writing no intermediate file. | — |
| **Staged flow** | `vcmp → velab → vrun` — the same three stages as three processes exchanging files on disk (`.vu`, then `.velab`). It answers the same question as Cadence `xmvlog`/`xmelab`/`xmsim` and Synopsys `vlogan`/`vcs`/`simv`: per-stage rebuild, per-stage debugging, and skipping a stage whose inputs have not changed. Contract: [14-staged-artifacts.md](14-staged-artifacts.md). | — |

---

## 2. Scheduling and time

| Term | Definition | IEEE |
|---|---|---|
| **Event-driven simulation** | Executing a process only when a value it depends on changes, rather than re-evaluating the whole design every clock. The model the engine implements. | 1800 §4 |
| **Region** | One ordered slot inside a single simulation time, so that events at the same time still have a defined order. IEEE 1800 defines seventeen; the engine implements seven — Preponed, Active, Inactive, NBA, Observed, Reactive, Postponed. Constructs that could observe a Pre-, Post- or Re- variant are refused rather than approximated. | 1800 §4 |
| **Stratified event queue** | The region set as a whole, viewed as the queue discipline that makes same-time ordering deterministic. IEEE 1364 defines four strata (Active, Inactive, NBA, Monitor); IEEE 1800 extends them to seventeen. The frozen IR's `RegionTag` carries the four-stratum form, and the engine's own loop supplies the rest. | 1800 §4, 1364 §11 |
| **Delta cycle** | One zero-time iteration of the region cascade within a single simulation time, repeated until values stop moving. A `#0` delay or a chain of combinational logic produces several. `$time` does not advance between them. The standard describes the iteration; "delta cycle" is the industry name for it. | 1800 §4 |
| **NBA** | Non-blocking assignment, `<=`. The right-hand side is evaluated in the Active region and the left-hand side update is committed in the NBA region, which is what makes a register swap at a clock edge behave as hardware does. Also the name of the region that holds those pending updates. | 1800 §10.4.2 |
| **Time wheel** | The engine structure holding future events, keyed by simulation time. Three maps participate — the event wheel, delayed continuous-assign writes, and transport-delayed NBA updates — and time advances to the earliest key present in any of them. | — |
| **Quiescence** | The state in which no event remains in any of the three time-keyed maps. Reaching it ends the run without `$finish`. | — |
| **Finish reason** | Why a run ended: `Finish` (`$finish` or a fatal severity task), `Stop` (`$stop`), `Quiescent`, `DeltaLimit` (the delta budget was exhausted, or a continuous-assign settle would not converge) or `Error`. Printed as `simulation ended ({reason}) at time {n}`; written to the observability rail in lower snake case (`finish`, `stop`, `quiescent`, `delta_limit`, `error`). | — |
| **timescale** | The `` `timescale <unit>/<precision> `` directive. The unit is the real time `#1` denotes; the precision is the smallest representable step. Internally, time is counted in precision ticks. | 1364 §19.2, 1800 §22 |
| **Time unit / time precision** | The two halves of a timescale, tracked per module and folded into a global precision for the run. Contract: [08-timescale-and-timing.md](08-timescale-and-timing.md). | 1800 §3 |
| **`$time`** | The current simulation time, returned as a 64-bit integer scaled to the calling module's time unit. Because it is scaled to the unit and not the precision, it can round. | 1800 §20.3 |
| **`$realtime`** | The current simulation time as a `real`, carrying the sub-unit part `$time` rounds away. | 1800 §20.3 |

---

## 3. Values, storage and execution

| Term | Definition | IEEE |
|---|---|---|
| **4-state logic** | The value set `0`, `1`, `X` (unknown) and `Z` (high impedance). The model IEEE simulation is defined in, the model Icarus Verilog implements, and the model this simulator implements. | 1800 §6.3 |
| **2-state logic** | The reduced value set `0` and `1`, with `X`/`Z` collapsed to `0`. Verilator's default. Faster, and blind to the initialisation defects `X` exists to expose. Applies here only to the 2-state SystemVerilog types (`int`, `bit`, and their relatives), never to the whole design. | 1800 §6.3 |
| **Net** | A value with a driver rather than a stored assignment — `wire` and its relatives — and, in this implementation, also the name of the flat storage slot every signal occupies at run time. Each slot holds the current and previous packed values, a width, an array length, and sign and real flags. | 1800 §6.5 |
| **Variable** | A value that holds what was last assigned to it procedurally — `reg`, `logic`, `int` and their relatives. | 1800 §6.5 |
| **Process** | One independently schedulable thread of execution: an `initial`, `final` or `always` block, or a `fork` arm. Elaboration classifies each into a sensitivity kind — `Initial`, `Comb`, `Latch`, `Level` or `Edge` — which decides whether it runs at time zero or waits. | 1800 §9 |
| **Activity** | One runtime instance of a process. Top-level processes map one to one onto the IR's process list; `fork` children are appended. An activity carries its own call stack, its join reference, and the flags that keep an `always` from re-entering itself before it completes. | — |
| **Frame** | One activation record of a subroutine: its local storage window plus, for a suspendable task, the return block and the output bindings the caller is waiting for. Two mechanisms exist — a synchronous window stack for subroutines that cannot suspend, and a per-activity call stack for those that can, whose window is stashed across a suspend and restored on resume. `Frame` is also a frozen IR type. | 1800 §13 |
| **fork/join** | Concurrent execution of several statements, with `join`, `join_any` or `join_none` deciding when the parent resumes. A `join_any` or `join_none` survivor can outlive its parent, so frame windows are reference-counted. | 1800 §9.3 |

---

## 4. Waveform

| Term | Definition | IEEE |
|---|---|---|
| **VCD** | Value Change Dump, the ASCII waveform format: a declaration header followed by time-ordered value changes. The default output format here. | 1364 §18, 1800 §21.7 |
| **Identifier code** | The short printable-ASCII key a VCD uses to name a signal in the body, bound to a hierarchical name by a `$var` line in the header. Every tool assigns them differently, so comparing two VCDs means normalising them first. | 1364 §18.2 |
| **FST** | Fast Signal Trace, GTKWave's binary waveform format, read natively by GTKWave and Surfer. Written here when the dump path ends in `.fst`, by transcoding a VCD written to a sidecar. | — |
| **Dump** | The act of writing a waveform. Nothing is written unless the RTL calls `$dumpvars`; `$dumpfile` alone only names the path, and `$dumpon`, `$dumpoff` and `$dumpall` before any `$dumpvars` do nothing. | 1364 §18.1, 1800 §21.7 |

Contract: [07-vcd-format.md](07-vcd-format.md).

---

## 5. Diagnostics

| Term | Definition | IEEE |
|---|---|---|
| **Diagnostic** | One rendered message: severity, message code, `file:line:col`, a title, and a visual underline of the offending span, in the shape the Rust compiler uses. | — |
| **Severity** | One of `Note`, `Info`, `Warning`, `Error`, `Fatal`. Error and Fatal are always logged; Info, Note and Warning pass a gate. | 1800 §20.10 |
| **MsgCode** | The stable identity of a diagnostic: a permanent mnemonic (`E-ELAB-MULTIDRIVER`) plus a printed number (`VITA-E3001`). The enum is exhaustive and holds 68 variants. The mnemonic is the primary key and never changes; the number encodes severity in its letter and stage in its band — `0xxx` general, `1xxx` preprocess, `2xxx` parse, `3xxx` elaborate, `4xxx` runtime, `8xxx` filelist, `9xxx` artifact, with `5xxx`–`7xxx` reserved. Numbers are never reused. | — |
| **Error code reference** | [15-error-code-reference.md](15-error-code-reference.md): one entry per message code, giving cause, example and remedy. It is the source `vita explain <CODE>` prints, and the test suite holds it to a one-to-one correspondence with the `MsgCode` enum, so a new code and its entry cannot drift apart. | — |
| **Suppress / promote** | `-Wno-<CODE>` demotes a diagnostic out of the stream; `-Werror=<CODE>` raises it to Error; `-Werror` raises all of them. A code is accepted as its mnemonic, its printed number, or that number without the `VITA-` prefix. | — |
| **Transcript** | The whole operational output of a run: banner, files read, library resolution, elaboration progress, `$display` output, severity lines and the run summary. Written to the terminal and, with `--log`, tee'd to a file that replays the console faithfully. | — |
| **Exit class** | The meaning of the process exit code. `0` clean; `1` an RTL or user error, including `$fatal`; `2` a stale artifact, meaning the inputs must be rebuilt rather than the RTL inspected; `3` a command-line or usage error. | — |

Contract: [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md). User-facing view:
[../manual/007_error-codes.md](../manual/007_error-codes.md).

---

## 6. Artifacts and determinism

| Term | Definition | IEEE |
|---|---|---|
| **`.vu`** | The `vcmp` output: one compilation unit's parsed AST, serialised, with a timescale tail and a source-map tail appended. Framed as an eight-byte magic, a self-describing header, then the body. | — |
| **`.velab`** | The `velab` output: a fully elaborated, language-neutral `sim_ir::SimIr`, serialised as one self-contained file — the golden frame plus append-only trailer segments. It occupies the position `.vvp` does in `iverilog → .vvp → vvp`: `vrun` simulates from it and needs nothing else. | — |
| **Work library** | A `vcmp` output directory that records compiled units instead of a single file: a machine-written `lib.toml` manifest plus content-addressed unit blobs under `units/`. Units are addressed by a logical `library:unit` key, so several libraries can coexist, as with `cds.lib` and `synopsys_sim.setup`. `velab -L` loads the instantiation closure of the requested tops and never promotes an unrelated library unit to a root. | — |
| **Filelist (`.f`)** | A command file aggregating sources for a large project. `-f` resolves relative paths against the invocation directory, `-F` against the filelist's own directory, and a filelist may nest others to any depth. Expansion order is deterministic, and a cycle is refused as `E-FLIST-CYCLE`. | — |
| **Frozen type** | A serialised type whose shape is part of the on-disk contract. Adding, removing or reordering a field flips the golden root hash and invalidates every existing artifact, so a frozen type is changed only deliberately, together with a `format_version` bump. Frozen types stay at their crate root, because moving one to a submodule flips the hash on its own. Catalogue: [17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md). | — |
| **Schema hash** | A structural blake3 digest of a serialised type's shape — its fields, its variants and the serde attributes that affect the wire form — computed at compile time by `#[derive(SchemaHash)]`. It makes a layout change a version error instead of a silent misparse. Contract: [16-schema-hash-spec.md](16-schema-hash-spec.md). | — |
| **Golden root** | The type whose schema hash gates a whole artifact class: `sim_ir::SimIr` for `.velab`, `hdl_ast::SourceUnit` for `.vu`. | — |
| **`format_version`** | The monotonic integer stamped into every artifact header, currently `31`. It is bumped when the golden shape changes, and also when a staged trailer segment is added even though the golden frame is unchanged. | — |
| **Staleness gate** | The ordered check a staged input passes before its body is decoded: magic, then `format_version`, then the tool's semver major, then the schema hash. Each failure has its own code in the `9xxx` band and its own remedy line, and all of them exit `2`. Beyond the header, `vrun` re-hashes the whole upstream chain against the live sources rather than trusting timestamps. | — |
| **Byte-identical determinism** | The contract that the same inputs produce the same artifact bytes on every supported platform. It is held by ordered containers only, no `usize` or float in a serialised shape, and a span-free IR. | — |

---

## 7. The observability rail

The machine-readable output surface, aimed at a program rather than a person. Nothing on it
changes the simulation, stdout, the waveform or the exit code. Contract:
[19-ai-agent-observability.md](19-ai-agent-observability.md).

| Term | Definition |
|---|---|
| **Observability rail** | The whole surface: `--obs-dir` and the files under it, `--probe`, `$vita_stage`, `--hier-tree` and `--inst-paths`. |
| **`--obs-dir <DIR>`** | The switch that turns the rail on and the directory it writes into. |
| **`run.json`** | One object per run: tool identity, versions, the resolved invocation, the finish reason, counts, and the code-generation and backend summaries. Two subroutine objects live here and do not join — `subroutines` is the static route census and is written unconditionally; `subroutine_calls` is the per-instance runtime profile and needs `--obs-procs`. They are keyed differently, so their columns are not additive. |
| **`results.jsonl`** | The per-record ledger: one JSON object per line. |
| **`coverage.json`** | Functional coverage, when the design defines any. |
| **Probe** | A net named with `--probe` (or listed in a `--probe-file`) whose every value change is streamed to `trace.jsonl`. |
| **`trace.jsonl`** | The probe change stream, one JSON object per change. |
| **`$vita_stage`** | A vendor system task the RTL calls to mark a labelled point in the run, with optional values. It writes `stage.jsonl`, and needs both the `+STAGE_TRACE` plusarg and `--obs-dir`. |
| **`--hier-tree` / `--inst-paths`** | Plain-text dumps of the instance tree and of the flat instance-path list. |

Status at HEAD: the rail is one-shot only. `--obs-dir`, `--probe`, `--probe-file`,
`--obs-procs` and `--obs-procs-time` are refused on `vcmp`, `velab` and `vrun`; a design
that calls `$vita_stage` is refused by `velab`, the stage that elaborates it.
`--hier-tree` and `--inst-paths` are the exception: the staged applets accept them,
exit `0`, and write no file.

---

## 8. Method vocabulary

The words the engineering method uses. Canonical definitions and the rules built on them:
[../ENGINEERING_RULES.md](../ENGINEERING_RULES.md).

| Term | Definition |
|---|---|
| **Accuracy ladder** | The single scale every change is scored against: silent-wrong is worst, honest-loud is always safe, correct support is best. Changes climb it and never descend, and one silent-wrong is never traded for another. |
| **Silent-wrong** | A wrong answer with no diagnostic and a success exit. The outcome this project exists to prevent. A machine-readable output that misdescribes itself is a silent-wrong of the same kind, because its audience cannot check it. |
| **Honest-loud** | A refusal or diagnostic that names what the tool cannot do. Always safe, never as good as support. Making a construct that worked loud is a regression, not a tidy-up. |
| **Correct support** | The construct runs and its value matches the oracle. |
| **Correct-or-loud** | The design goal the ladder serves: produce the right value, or say plainly that this one cannot be produced. Never a third thing. |
| **Oracle** | An independent tool or standard text that decides what a case should produce. |
| **Live oracle** | `iverilog` plus `vvp`, invoked by the differential harnesses. `verilator` is a second opinion, taken by hand on 2-state arithmetic, width and sign. |
| **hand-IEEE** | An expected value derived by reading the IEEE text, used where no external tool can arbitrate — SystemVerilog assertions, classes, constrained randomisation, parameterised classes and virtual interfaces, none of which the live oracle runs. |
| **Anchor** | An expected value fixed independently of any implementation, so that changing shared code cannot move it. |
| **Census** | An enumeration, taken from the source, of every site that can reach a question, with each cell measured rather than argued. The unit of work: a slice opens with one and closes with one. Four kinds recur and none substitutes for another — a producer census asks who writes a value, a routing census where it goes, an ordering census when it arrives, and a consumer census who reads it and what each reader does with it. |
| **Cell** | One design plus one measured output, inside a census or a sweep. |
| **Control twin** | A second cell identical to the first except for the axis under test, which is what makes a result attributable to that axis. |
| **Lens** | One adversarial reviewer with a fixed attack method. Two are mandatory on every design or implementation change, and a design that changes during review is reviewed again. |
| **Differential lens** | The lens that reproduces behaviour against a live oracle. It reports the oracle's raw output text per divergence, the probe resolution it used, and a four-way classification of each divergence: real gap, no-oracle, vita-ahead, or harness format. |
| **Soundness lens** | The lens that argues from the source and the standard, with censuses as its premises rather than prose. It is commissioned explicitly, against a task list: all-sites and variant enumeration, disjointness, same-name collision, guard traversal completeness, and an audit of how every consumed map is populated. |
| **Conflict rule** | When the soundness lens and the differential lens disagree, the differential wins. Measurement outranks argument. |
| **PRE / POST** | The binary built from the tree before a change, extracted and built separately, and the binary built with the change applied. Performance and regression claims name both. |
| **Mutation battery** | Deliberately breaking one line of the implementation to check that some test notices. A surviving mutant means the suite is not measuring that line. Run across the whole workspace, because a narrow filter manufactures survivors that are not real. |
| **Gate** | The command whose verdict decides whether work is green: `cargo nextest run --workspace --locked`. Also, in code, a predicate that admits or refuses a construct. |
| **Workload corpus** | Ten third-party and first-party designs run end to end, each pinned to an upstream commit and to one accumulated digest that an oracle produced first. The RTL is not redistributed; the runner clones it at the pinned revision. Details: [../study/03-workload-corpus.md](../study/03-workload-corpus.md). |
| **Grade** | One workload's verdict from the corpus runner. Eight exist — `ok`, `known-gap`, `REGRESSION`, `PROMOTED`, `ruled-split`, `DRIFTED`, `absent`, `ORACLE-DRIFT` — of which exactly three are failures: `REGRESSION`, `DRIFTED` and `ORACLE-DRIFT`. `PROMOTED` is upper case and is not a failure: it means a refused workload now runs and its manifest row must move. |

---

## 9. Names in the tree

| Name | What it is | Status at HEAD |
|---|---|---|
| **vitamin** | The project: a memory-safe, portable, precise RTL simulator written in Rust. | — |
| **vita** | The one-shot driver binary: compile, elaborate and simulate in a single invocation. | — |
| **vcmp / velab / vrun** | The staged driver applets, one per stage, each consuming the stage before it. The default build is a single multicall binary, so they are reached as `vita vcmp`, `vita velab` and `vita vrun`; the `separate-bins` feature additionally builds them as three standalone binaries. | — |
| `hdl-preprocess`, `hdl-lexer`, `hdl-parser`, `hdl-ast` | The language-dependent front end and the AST it produces. | — |
| `elaborate` | AST to `sim_ir::SimIr`. | — |
| `sim-ir` | The frozen IR and the analyses computed from it at startup. | — |
| `sim-engine` | The scheduler, the value representation, the execution backends, and the `$` system task and function handlers. | — |
| `vcd-writer` | Waveform emission, VCD and the FST transcode. Active only once the RTL calls `$dumpvars`; there is no automatic whole-design dump. | — |
| `diag` | Renders one diagnostic, and owns the `Severity`, `MsgCode`, `Frame`, `Diagnostic` and `LogEvent` data model plus the `LogSink` trait. It has no I/O or tracing dependency, so it stays a leaf. | — |
| `vita-log` | The operational logging and transcript subsystem: it tees one `LogEvent` stream to terminal and log file, and owns severity routing, the message-code registry, the suppress and promote gates, the counts, the exit-code computation, the banner and progress lines, and runtime location recovery. `$info`, `$warning`, `$error` and `$fatal` pass the same gate. | — |
| `vita-artifact` | The `.vu` and `.velab` container: header framing, versioning and the staleness gates. The one-shot path streams through memory and never calls it. | — |
| `vita-artifact-derive` | The `#[derive(SchemaHash)]` proc macro. A build-graph leaf. | — |
| `vita-schema` | The runtime shape registry and the blake3 hash over it. Depends on nothing but `blake3`. | — |
| `cli` | Argument parsing, filelist expansion, work libraries, the staged applets and the observability writers. | — |
| `corpus-runner` | The workload-corpus harness: `list`, `fetch` and `run`. It prints a fixed-width table — parse it by column offset, not by whitespace — and exits `0` clean, `1` on a failing grade, `2` when nothing is present locally, `3` on usage. It has no external dependencies and is not run by CI, because the corpus RTL is not in the repository. | — |
| `hdl-builtins` | Named for the `$` system tasks and functions. | Empty. The handlers live in `sim-engine`; nothing depends on this crate for behaviour. |
| `vcd-diff` | Named for comparing two VCD files under normalisation — identifier-code remapping, `Z` handling, hierarchical name mapping. | Empty. No comparison tool ships: the crate exports nothing, has no binary target, and has no callers. |

---

## Sources

- IEEE 1800-2017 (SystemVerilog LRM): §3 (building blocks), §4 (scheduling semantics), §6 (data types), §9 (processes), §10.4.2 (non-blocking assignments), §13 (tasks and functions), §20–21 (system tasks and functions), §21.7 (VCD), §22 (compiler directives), §23 (modules and hierarchy).
- IEEE 1364-2005 (Verilog LRM): §11 (scheduling semantics), §18 (VCD), §19.2 (`` `timescale ``).
- Icarus Verilog documentation: https://steveicarus.github.io/iverilog/
- Verilator documentation: https://verilator.org/guide/latest/
- GTKWave and the FST format: https://gtkwave.sourceforge.net/
- Project-internal definitions are taken from the source at HEAD; the method vocabulary is canonical in [../ENGINEERING_RULES.md](../ENGINEERING_RULES.md).
- Citation policy for everything above: [11-sources-and-citations.md](11-sources-and-citations.md).
