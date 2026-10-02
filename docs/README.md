# The vitamin documentation map

What lives under `docs/`, what question each part answers, and which parts
describe the simulator as it is now.

Four kinds of document live here. Mixing them up is the one way to be misled by
any of them.

| Kind | Meaning |
|---|---|
| **Current** | describes HEAD and is updated with it |
| **Specification** | the design contract. Changing the behaviour means changing the specification too |
| **Reference** | notes on the language standards themselves, independent of what vitamin implements |
| **History** | a frozen pointer to where records live in git; a slice's record is its commit message. A statement in one is true of its date, not of HEAD |

## Start here

| | Path | Kind | Answers |
|---|---|---|---|
| 1 | [manual/](manual/) | Current | how to use the simulator — install, CLI, supported language, system tasks, limits, error codes. Chapters `000`–`007`, in order |
| 2 | [REMAINING_WORK.md](REMAINING_WORK.md) | Current | one screen: what stands between HEAD and the two goals, and what is queued next |
| 3 | [ROADMAP.md](ROADMAP.md) | Current | every open item, by section — silent-wrong defects, loud-to-supported promotions, performance, observability |
| 4 | [ENGINEERING_RULES.md](ENGINEERING_RULES.md) | Current | how work is done here: the correct-or-loud ladder, the census and review method, and the rule each lesson produced. Read before implementing |
| 5 | [PROBE_CATALOG.md](PROBE_CATALOG.md) | Current | pre-existing defects a review measured outside a slice's fix path, one row each with repro and oracle values |

### The user manual

| Chapter | Answers |
|---|---|
| [000_introduction.md](manual/000_introduction.md) | what vitamin is and who it is for |
| [001_installation.md](manual/001_installation.md) | building and installing the binaries from source |
| [002_quickstart.md](manual/002_quickstart.md) | a first simulation, end to end |
| [003_language-reference.md](manual/003_language-reference.md) | which Verilog and SystemVerilog constructs are supported, per construct |
| [004_cli-reference.md](manual/004_cli-reference.md) | every flag of `vita`, `vcmp`, `velab` and `vrun` |
| [005_system-tasks.md](manual/005_system-tasks.md) | every supported `$` task and function |
| [006_limitations.md](manual/006_limitations.md) | every intentional simplification and every divergence, with a workaround |
| [007_error-codes.md](manual/007_error-codes.md) | reading and looking up a `VITA-####` diagnostic |

## Specifications

[`preview/`](preview/) is the design contract, numbered by topic. The filename
is the subject.

| File | Subject |
|---|---|
| [00-overview.md](preview/00-overview.md) | the vision and the pipeline, opening the numbered set |
| [01-goals-and-scope.md](preview/01-goals-and-scope.md) | goals, non-goals, and the in/out language-feature scope table |
| [02-implementation-language.md](preview/02-implementation-language.md) | the Rust crate-selection contract and the MSRV policy |
| [03-build-and-portability.md](preview/03-build-and-portability.md) | the build, feature and CI portability contract |
| [04-architecture.md](preview/04-architecture.md) | the crate graph, the layering rules, and the executor-backend architecture |
| [05-strategy-and-roadmap.md](preview/05-strategy-and-roadmap.md) | what each phase must deliver before the next opens |
| [06-simulation-engine.md](preview/06-simulation-engine.md) | the event-driven kernel contract: regions, process execution, the frozen suspend state |
| [07-vcd-format.md](preview/07-vcd-format.md) | waveform emission: header, `$var` naming, value encoding, FST dispatch |
| [08-timescale-and-timing.md](preview/08-timescale-and-timing.md) | the timescale and precision contract, and the two-stage rounding rule |
| [09-testing-and-verification.md](preview/09-testing-and-verification.md) | the test-layer contract and what CI must enforce |
| [10-glossary.md](preview/10-glossary.md) | recurring terms, with IEEE clause numbers |
| [11-sources-and-citations.md](preview/11-sources-and-citations.md) | the copyright and citation policy, and the source catalogue |
| [history/research-log/METHODOLOGY.md](history/research-log/METHODOLOGY.md) | how the reference notes were researched; the numbered set has no `12` |
| [13-diagnostics-and-logging.md](preview/13-diagnostics-and-logging.md) | the diagnostic data model, `-Wno-` / `-Werror=` gating, the exit-class contract |
| [14-staged-artifacts.md](preview/14-staged-artifacts.md) | the `.vu` and `.velab` on-disk contract, work libraries, filelists, staleness |
| [15-error-code-reference.md](preview/15-error-code-reference.md) | the canonical catalogue: one entry per diagnostic code |
| [16-schema-hash-spec.md](preview/16-schema-hash-spec.md) | the structural hash that gates `.vu` and `.velab` staleness |
| [17-sim-ir-ir-backbone-freeze.md](preview/17-sim-ir-ir-backbone-freeze.md) | the frozen IR types, and what a `format_version` bump costs |
| [18-acceleration-analysis.md](preview/18-acceleration-analysis.md) | parallel, GPU and compiled-backend feasibility, with the measured basis for each verdict |
| [19-ai-agent-observability.md](preview/19-ai-agent-observability.md) | the observability rail: `run.json`, `results.jsonl`, `trace.jsonl`, `stage.jsonl` |
| [20-cycle-mode-feasibility.md](preview/20-cycle-mode-feasibility.md) | the cycle-based-mode feasibility contract and its verdict |
| [21-tier3-native-backend.md](preview/21-tier3-native-backend.md) | the native backend: what it is, how a design and a body qualify, the arena, refusal and fallback, and the equivalence contract |

Two of these have teeth in the build.
[15-error-code-reference.md](preview/15-error-code-reference.md) is compiled into
the binary and gated 1:1 against the `MsgCode` enum, so a code and its entry
cannot drift apart. [16-schema-hash-spec.md](preview/16-schema-hash-spec.md) and
[17-sim-ir-ir-backbone-freeze.md](preview/17-sim-ir-ir-backbone-freeze.md)
describe what a frozen type is and what changing one costs.

## Language reference notes

[`preview/hdl-reference/`](preview/hdl-reference/) documents the standards
themselves — what IEEE requires, not what vitamin implements. Use it to settle
what correct behaviour is; use [manual/003](manual/003_language-reference.md) to
find out whether vitamin does it.

| Area | Contents |
|---|---|
| [00-standards-map.md](preview/hdl-reference/00-standards-map.md) | the IEEE 1800 / 1364 / 1076 / 1164 version map |
| [01-synthesizability-legend.md](preview/hdl-reference/01-synthesizability-legend.md) | the legend shared by every note |
| [verilog/](preview/hdl-reference/verilog/) | IEEE 1364: lexical elements, data types, expressions, hierarchy, behavioural modelling, procedural statements, tasks and functions, gate level, compiler directives, synthesizability |
| [systemverilog/](preview/hdl-reference/systemverilog/) | IEEE 1800: data types, arrays, procedural blocks, interfaces, packages, classes, assertions, subroutines, synthesizability |
| [vhdl/](preview/hdl-reference/vhdl/) | IEEE 1076: lexical elements, types, objects, design units, concurrent and sequential statements, subprograms, packages, synthesizability |
| [system-tasks/](preview/hdl-reference/system-tasks/) | the `$` families: display and I/O, file I/O, memory load, simulation control, time, conversion, bit-vector, math, random, `$dump*`, assertion sampling, introspection |

## Measurement studies

Each study is a measurement with its method attached, kept so a later claim can
be checked against how the number was produced.

| File | Kind | Subject |
|---|---|---|
| [study/01-interpreted-vs-compiled.md](study/01-interpreted-vs-compiled.md) | Current | the performance axis, and where vitamin sits on it |
| [study/02-v1-native-coverage.md](study/02-v1-native-coverage.md) | Current | terminology — census, coverage, mutation — and the native backend's coverage |
| [study/03-workload-corpus.md](study/03-workload-corpus.md) | Current | the ten corpus workloads — eight third-party, two first-party — their oracles, and what they found |

## History

[`history/`](history/) is a frozen pointer: a slice's record is its commit message.
Nothing in it should be read as a statement about HEAD.

| Path | Contents |
|---|---|
| [history/README.md](history/README.md) | the index of everything below |
| [history/ROADMAP_ARCHIVE.md](history/ROADMAP_ARCHIVE.md) | pointer: where completed-slice records (`§4.5.N`, `§5.1-<id>`) are kept in git |
| [history/reviews/](history/reviews/) | review documents received from outside, preserved verbatim. Their internal links point at the reviewer's own repository and do not resolve here |
| [history/research-log/](history/research-log/) | the raw research output behind the reference notes |

## Outside `docs/`

| Path | Kind | Answers |
|---|---|---|
| [../README.md](../README.md) | Current | the public front page: what vitamin is, what it supports, how to run it |
| [../CHANGELOG.md](../CHANGELOG.md) | History | what changed between releases — the history a user of the simulator reads |
| [../CONTRIBUTING.md](../CONTRIBUTING.md) | Current | toolchain, the gate, determinism and frozen-type rules, workflow |
| [../examples/README.md](../examples/README.md) | Current | the four bundled designs and how to run them |
| [../bench/README.md](../bench/README.md) | Current | the workload corpus: what is committed, what is cloned, and the runner commands |

## The standing rule

A user-facing change — to the CLI, the supported language, a system task, or a
diagnostic code — lands in [manual/](manual/) in the same change that makes it,
and in [../CHANGELOG.md](../CHANGELOG.md), which is what a user of the simulator
reads. A behaviour change that the design contract covers lands in
[preview/](preview/) as well.
