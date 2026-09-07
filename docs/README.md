# `docs/` — what is here, and which of it is current

Three kinds of document live under `docs/`, and mixing them up is the only way to be
misled by any of them:

- **Current** — describes HEAD and is updated with it. Read these.
- **Spec** — the design contract. Changing the behaviour means changing the spec first.
- **Frozen** — a record of what was done, or of what was believed on a date. Never
  updated; a statement in one is true *of its date*, not of HEAD.

## Start here

| | file | kind | what it answers |
|---|---|---|---|
| 1 | [manual/](manual/) | current | **how to use the simulator** — install, CLI, supported language, system tasks, limits, error codes. The public-facing set (`000`–`007`, in order) |
| 2 | [REMAINING_WORK.md](REMAINING_WORK.md) | current | one screen: what stands between HEAD and the two goals, and what is queued next |
| 3 | [ROADMAP.md](ROADMAP.md) | current | every open item, by section — §2 silent-wrong, §3 loud→supported, §5 performance, §6 observability |
| 4 | [ENGINEERING_RULES.md](ENGINEERING_RULES.md) | current | how work is done here: the correct-or-loud ladder, census and review method, and the lesson behind each rule. Read before implementing |

## Specifications

`preview/` is the design contract, numbered by topic (`ls docs/preview/` — the filename is
the subject). The ones consulted most often:

| file | subject |
|---|---|
| [preview/15-error-code-reference.md](preview/15-error-code-reference.md) | every diagnostic code |
| [preview/16-schema-hash-spec.md](preview/16-schema-hash-spec.md) | the structural hash that gates `.vu` / `.velab` staleness |
| [preview/17-sim-ir-ir-backbone-freeze.md](preview/17-sim-ir-ir-backbone-freeze.md) | the frozen IR, and what a `format_version` bump costs |
| [preview/19-ai-agent-observability.md](preview/19-ai-agent-observability.md) | the G2 observability rail (`run.json`, probes, coverage) |
| [preview/hdl-reference/](preview/hdl-reference/) | language reference notes (Verilog / SystemVerilog / VHDL / system tasks) |

## Measurement studies

Each one is a measurement with its method attached, kept so a later claim can be checked
against how the number was produced.

| file | subject |
|---|---|
| [study/01-interpreted-vs-compiled.md](study/01-interpreted-vs-compiled.md) | the performance axis |
| [study/02-v1-native-coverage.md](study/02-v1-native-coverage.md) | terminology (census · coverage · mutation) and the native backend's coverage |
| [study/03-workload-corpus.md](study/03-workload-corpus.md) | the ten third-party workloads, their oracles, and what they found |

## Frozen — history, not status

| path | contents |
|---|---|
| [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md) | completed slices (§4.5.x), in detail |
| [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md) · [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md) | earlier roadmaps, section numbers preserved |
| [DEVLOG.md](DEVLOG.md) | per-round development log |
| `superpowers/specs/` · `superpowers/plans/` | the original design spec and the implementation plans built from it (2026-05 – 2026-07). Superseded by `preview/` for the contract and by `ROADMAP_ARCHIVE.md` for what shipped |
| `preview/research-log/` | raw research output behind the `preview/` documents, dated |
| [reviews/](reviews/) | external review documents, preserved verbatim; their internal links point at the reviewer's own repository and are dead here |

A user-facing change to the CLI, the supported language, or a diagnostic code must land in
`manual/` in the same slice; `CHANGELOG.md` at the repository root is what a user of the
simulator reads.
