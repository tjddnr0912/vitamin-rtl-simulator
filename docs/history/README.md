# The vitamin history archive

`docs/history/` holds the frozen record: the per-slice development log, the completed-work
archives, the original design specification and the implementation plans built from it, the
research behind the language reference notes, the review documents received from outside, and the
engineering case studies. Nothing here is updated after it is written, and nothing here describes
HEAD.

## The reading rule

A statement in a frozen document is true of its date, not of HEAD. That is the whole safety rule,
and it has teeth: a verdict recorded in one of these files can be reversed by later work recorded a
few entries away in the same file. Clocking blocks are recorded as a conditional no-go on
2026-06-18, on the grounds that the scheduler had no Preponed region; the Preponed-region sampler is
recorded as built and shipped on 2026-06-25 — one week later, in
[DEVLOG.md](DEVLOG.md), which carries both entries and puts the refusal first. The same pair appears
in [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md), whose own header table lists the
verdicts it records that later work overturned.

Three consequences follow.

- A search hit inside `docs/history/` answers "what was believed on that date", never "what the
  simulator does". For the second question use the documents in
  [Where the current answers are](#where-the-current-answers-are).
- When a history file states a refusal, a limitation or a no-go, read forward in the same file
  before quoting it.
- Section numbers are preserved exactly as they were when each file was split off, because commit
  messages, code comments and lessons cite them. They do not track the section numbering of
  [../ROADMAP.md](../ROADMAP.md) as it stands.

## How the project got here

### Design and research — 2026-05-26 to 2026-05-28

The whole-project design specification came first, in
[specs/2026-05-26-vitamin-rtl-simulator-design.md](specs/2026-05-26-vitamin-rtl-simulator-design.md):
the goal of an open-source RTL simulator in Rust, the `preprocess → lex → parse → elaborate →
sim-ir → sim-engine → VCD` pipeline, and the Verilog-2005 synthesizable subset as the first target.
Two days later, twenty-two research rounds against primary sources produced
[research-log/](research-log/) — the IEEE standard map, scheduling regions, VCD, timescale
semantics, the `$` task families, the SystemVerilog type system, and the observed behaviour of
Icarus Verilog and Verilator. The plan that turned that research into the numbered specification set
now published as [../preview/](../preview/) is
[plans/2026-05-28-vitamin-docs-preview-set.md](plans/2026-05-28-vitamin-docs-preview-set.md).

### The pipeline — June 2026

The implementation plans dated 2026-06-03 to 2026-06-06 in [plans/](plans/) take the pipeline stage
by stage: the preprocessor, the parser AST, parser statements, width inference, elaborate v2, module
instance hierarchy, generate and genvar, functions and tasks, fork/join, the `real` domain, the
staged `vcmp → velab → vrun` flow, `$strobe`/`$monitor`, the artifact format, and the Stage C
bytecode VM. The `[Phase-1 MVP]` section of [../../CHANGELOG.md](../../CHANGELOG.md) records what
that produced: both flows running end to end, an event-driven IEEE 1364 scheduler over 4-state
values, hierarchical VCD, a SchemaHash-frozen `sim-ir` golden root giving byte-identical output on
three operating systems, and a differential harness against Icarus Verilog.

Late June went into SystemVerilog depth, logged per slice in [DEVLOG.md](DEVLOG.md) under
2026-06-25: `casting_type'(expr)` casts, clocking blocks with the Preponed-region sampler, the
twenty-one real-math functions and the non-uniform `$dist_*` family on a vendored pure-Rust `libm`,
and SVA empty-match repetition. The public repository's first commit is dated 2026-06-29.

### The second goal — 2026-07-02

A requirements document arrived from a hash-IP verification team using the simulator, preserved
verbatim as
[reviews/2026-07-02-ai-sim-observability.md](reviews/2026-07-02-ai-sim-observability.md). It asked
what a simulator must leave behind for a language model to diagnose a failure without a round trip.
That document is the origin of the second project goal, G2, and of the observability rail specified
in [../preview/19-ai-agent-observability.md](../preview/19-ai-agent-observability.md).

### Audit, waveforms, and the first release — July 2026

July carried three strands, summarised in [DEVLOG.md](DEVLOG.md) and detailed in
[ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md). FST waveform output landed on top of `fst-writer`, which
set the minimum Rust version at 1.85. A parallel eight-team audit on 2026-07-17 confirmed or refuted
every finding against live Icarus Verilog and produced the two-stage `#delay` conversion, among
other corrections. The rest of the month went into subroutine semantics: definite assignment for
`automatic` block-locals, calls that write an `output` actual, and fork inside a call frame — the
plans and design documents for which are the 2026-07-20 and 2026-07-24 entries in
[plans/](plans/) and [specs/](specs/).

The roadmap was snapshotted on 2026-07-16 and split into
[ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md) twelve days later. Version 0.1.0 was
tagged on 2026-07-31.

### The compiled backend becomes the product — 2026-08-10 to 2026-08-17

[ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md) is the unabridged record of four
phases run in eight days, and its own result table states what each established.

| Phase | What it did | Result |
|---|---|---|
| A | Finish native-backend coverage | Coverage reaches 100.00% — 6,470 of the 6,470 `simulate()` calls the test suite makes actually run on the native backend — with zero refusals and zero divergences |
| B | Split the build | `native` becomes the default and the only executor a default-features build needs; the interpreter and the bytecode VM move behind the `oracle` feature; substitution is reported as `VITA-W4030`; with `oracle` compiled out there is nothing to fall back to, so a refusal is fatal |
| C | Demote the interpreter | The interpreter is a test instrument rather than a product surface, and is excluded from performance work by rule |
| D | Machine-code generation | Native beats the VM on all ten benchmark workloads; a Cranelift code generator was built, wired and measured, and rejected on that measurement |

The block was moved out of the live roadmap on 2026-08-18, keeping its `§5.1-x` numbering so that
citations elsewhere still resolve.

### Since — 2026-08-26 onward

Version 0.2.0 was tagged on 2026-08-26 with the compiled backend as the product and all three
executors producing byte-identical output over the whole corpus. Work after it is recorded slice
by slice in [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md) and, for the incidents worth keeping, in
[lessons.md](lessons.md): silent-wrong corrections found by adversarial
two-lens review against live Icarus Verilog and Verilator, loud refusals promoted to real support,
and the observability rail.

The counts below are quoted from the section of the changelog or log that recorded each; the HEAD
row is `cargo nextest run --workspace --locked`.

| Point | Tests | `format_version` |
|---|---|---|
| Phase-1 MVP | 419 | 3 |
| 2026-07-17 audit | 3,591 | 22 |
| 0.1.0 — 2026-07-31 | 5,009 | 26 |
| 0.2.0 — 2026-08-26 | 6,169 | 29 |
| HEAD | 7,352 | 31 |

## What is here

| Path | Contains | Period | Language | Answers |
|---|---|---|---|---|
| [lessons.md](lessons.md) | Engineering case studies: the design that was run, the numbers that came back, what went wrong, and the rule the incident bought. 25 dated sections plus one for undated slices | 2026-08-08 to 2026-09-09 | English | Why a rule in [../ENGINEERING_RULES.md](../ENGINEERING_RULES.md) exists, and what evidence it rests on |
| [DEVLOG.md](DEVLOG.md) | The per-slice development narrative, with cumulative status snapshots; each entry pins a commit, a green-test count and the review outcome | Stage C to 2026-09-02 | Korean | What a given slice did, in order, and what it measured |
| [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md) | An index of every completed slice, newest first, over the whole range `§4.5.1`–`§4.5.467`; full entries for 303 of them, numbered between `§4.5.135` and `§4.5.467` (numbering is not contiguous — gaps are merged or cancelled work). Counted by heading | 2026-07-16 onward | Korean; entry titles are English from early September | Where a `ROADMAP §4.5.N` reference in a commit message, code comment or lesson resolves |
| [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md) | The Phase A–D execution record: 59 entries numbered `§5.1-x`, unabridged, plus the result table reproduced above | 2026-08-10 to 2026-08-17 | Korean | Where a `ROADMAP §5.1-x` reference resolves; what the native-backend push and the code-generation rejection actually measured |
| [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md) | The roadmap text as frozen on 2026-07-16, original section numbers `§0`–`§7` preserved and the text unaltered; carries the 133 slice entries `§4.5.2`–`§4.5.134`. Its header table lists the verdicts inside it that later work overturned | Up to 2026-07-16 | Korean | Where an old `§0`–`§7` reference resolves, and what the earliest slices did |
| [specs/](specs/) | Four design documents: the 2026-05-26 whole-project specification plus three later subsystem designs | 2026-05-26 to 2026-07-24 | Korean (the first) and English | What the original design said, before [../preview/](../preview/) became the contract |
| [plans/](plans/) | 23 dated implementation plans, one per subsystem or work batch, written before the code | 2026-05-28 to 2026-07-24 | English, with three Korean documents | How a subsystem was planned, and which alternatives were considered and dropped |
| [research-log/](research-log/) | 24 files: 22 primary-source research rounds behind the language reference notes, a README stating the naming and front-matter convention, and `METHODOLOGY.md`, the multi-round research method itself, which [../README.md](../README.md) cites in place of a numbered `preview/12`. Each note carries its queries and the URLs fetched | 2026-05-28 | Korean and mixed | Which sources a claim in [../preview/hdl-reference/](../preview/hdl-reference/) rests on |
| [reviews/](reviews/) | One review document received from outside, preserved verbatim with its request context. Its internal relative links point at the reviewer's own repository and do not resolve here | 2026-07-02 | Korean | What was asked for, in the requester's own words, before it became a specification |

## Where the current answers are

| Question | Document |
|---|---|
| What is under `docs/`, and which parts describe HEAD | [../README.md](../README.md) |
| How to install and run the simulator | [../manual/001_installation.md](../manual/001_installation.md), [../manual/002_quickstart.md](../manual/002_quickstart.md) |
| Which constructs are supported | [../manual/003_language-reference.md](../manual/003_language-reference.md) |
| What every CLI flag does | [../manual/004_cli-reference.md](../manual/004_cli-reference.md) |
| Which `$` tasks and functions exist | [../manual/005_system-tasks.md](../manual/005_system-tasks.md) |
| What the simulator deliberately does not do | [../manual/006_limitations.md](../manual/006_limitations.md) |
| What a `VITA-####` diagnostic means | [../manual/007_error-codes.md](../manual/007_error-codes.md), catalogue in [../preview/15-error-code-reference.md](../preview/15-error-code-reference.md) |
| What is still open, and what is queued next | [../ROADMAP.md](../ROADMAP.md), one-screen snapshot in [../REMAINING_WORK.md](../REMAINING_WORK.md) |
| How work is done here — the correct-or-loud ladder, the census and review method | [../ENGINEERING_RULES.md](../ENGINEERING_RULES.md) |
| The design contract for a subsystem | [../preview/](../preview/) |
| What IEEE requires, independent of what vitamin implements | [../preview/hdl-reference/](../preview/hdl-reference/) |
| A measurement and the method that produced it | [../study/](../study/) |
| What vitamin is, for a first-time reader | [../../README.md](../../README.md) |
| How to contribute | [../../CONTRIBUTING.md](../../CONTRIBUTING.md) |

## The release history

[../../CHANGELOG.md](../../CHANGELOG.md) stays at the repository root. It is the user-facing release
history — what changed between versions, phrased for someone who runs the simulator rather than
works on it — and it is the one history document that is appended to as work lands. The archives
here carry the engineering detail underneath its entries.
