# 13 · Diagnostics and Logging

This is the authoritative contract for everything vitamin writes about a run: the diagnostic
data model, the rendered line, the severity rules, the stable message codes and the flags that
override them, the stream discipline, the process exit classes, and the machinery that gives a
diagnostic a source location. The complete per-code catalogue lives in
[15-error-code-reference.md](15-error-code-reference.md); this document defines the system that
catalogue is part of. The user-facing summary is
[../manual/007_error-codes.md](../manual/007_error-codes.md).

---

## 1. Layering

Three crates divide the work along one seam: producers describe a diagnostic, the CLI renders it,
and a thin policy layer sits between them.

| Crate | Role | Depends on |
|---|---|---|
| `diag` | The data model and the sink boundary. `Severity`, `MsgCode`, `Diagnostic`, `Frame`, `SourceLoc`, `TimeStamp`, `LogEvent`, `ProgressEvent`, `RtlText`, `trait LogSink`, `trait SpanResolver`. Pure data plus traits — no IO, no formatting of a diagnostic. | nothing |
| `vita-log` | The gate. `GatePolicy` (`-Wno-` / `-Werror[=CODE]`) and `GatedSink`, an adapter that filters and rewrites the diagnostic stream in front of another sink. | `diag` |
| `cli` | The one concrete production sink, `StderrSink`: stream selection, verbosity, the `--log` tee, the severity counters, the rendered line, the counts epilogue, and the exit-class decision. | everything |

The rules that make the seam hold:

- Every producer — `hdl-preprocess`, `hdl-lexer`, `hdl-parser`, `elaborate`, `sim-engine`,
  `vita-artifact` — takes a `&dyn LogSink` and emits events. No producer selects a stream, opens a
  file, counts anything, or decides an exit code.
- `LogSink` lives in `diag` rather than in `vita-log`, so a producer that reports a diagnostic
  takes on no dependency beyond the leaf crate. The dependency graph stays acyclic:
  `diag` is a leaf, `vita-log → diag`, `cli → everything`.
- Only `cli` constructs a sink and installs it.
- Output policy is never an input to a hash. `GatePolicy`, verbosity and the log path have no field
  in `PreprocInputs` or `ElabInputs`, so `-Wno-`, `-Werror` and `--log` cannot invalidate a `.vu` or
  a `.velab` and cannot reach the SimIr golden. See [14-staged-artifacts.md](14-staged-artifacts.md).

`hdl-builtins` is a one-line stub and takes no part in diagnostics.

### Status at HEAD

| Intent | State |
|---|---|
| Diagnostic rendering is a separate concern from the producers | Holds. The single renderer is `StderrSink::render_diagnostic` in `cli`. |
| The renderer draws a caret-underlined source snippet | Not implemented. Rendering is one line per diagnostic, carrying `file:line:col` but no source excerpt. There is no `miette` or `codespan-reporting` dependency in the workspace. |
| The sink layer is built on `tracing` / `tracing-subscriber` | Not implemented. `StderrSink` writes directly; no crate in the workspace depends on `tracing`. |
| `vita-log` owns the sink, the counters and the exit policy | `vita-log` owns the gate only; the sink, counters and exit policy are in `cli`. |
| `crates/diag/src/fmt.rs` | Not the diagnostic renderer. It is the shared `$display`-family field-width and padding rule set used by both the runtime formatter and elaborate's compile-time `$display` evaluation. |

The production `LogSink` implementations are `StderrSink` (`cli`) and `GatedSink` (`vita-log`).
`sim-engine` also carries a capture sink that keeps `RtlOutput` text in a string, used by the
`simulate_capture` harness entry point.

---

## 2. The event model

One stream carries everything a run says.

```rust
enum LogEvent {
    Diagnostic(Diagnostic),  // a coded, severity-bearing report
    Progress(ProgressEvent), // tool-side narration
    RtlOutput(RtlText),      // $display/$write/$monitor/$strobe — user text, no severity
}

struct Diagnostic {
    severity: Severity,          // Note | Info | Warning | Error | Fatal
    code: MsgCode,               // stable enum, e.g. E-ELAB-MULTIDRIVER
    message: String,
    location: Option<SourceLoc>, // file, line, col, byte_start, byte_end
    context: Vec<Frame>,         // instance / hierarchy path
    sim_time: Option<TimeStamp>, // runtime events only
}
```

| Field | Contract |
|---|---|
| `severity` | Chosen by the emitter (§4). Drives the printed token, the counters and the exit class. |
| `code` | Always present. There is no uncoded diagnostic. |
| `message` | The one-line body. An empty message renders the code's `title()` instead, so a bare `$error;` still prints something meaningful. |
| `location` | `None` when the emitter has no resolvable span, or when no `SpanResolver` is installed. Absent means the line renders without a location prefix, never with a fabricated one. |
| `context` | `Frame { label, location: Option<SourceLoc> }`. Only `context.first()` is rendered, as the instance path. No emitter sets a frame's own `location` and the renderer does not read it. |
| `sim_time` | `TimeStamp { ticks: u64 }`. Stamped by every runtime emitter; `None` for preprocess, lex, parse, elaborate and the artifact gates. Same clock and wording as the `simulation ended (…) at time N` line. |

`ProgressEvent { message }` and `RtlText { text, sim_time }` carry neither severity nor code, are
never counted, and are never gated by `-Wno-`/`-Werror`.

`LogSink::emit(&self, event: LogEvent)` takes `&self`. A sink that accumulates state therefore uses
interior mutability: `StderrSink` holds `Cell<u32>` counters and an `Option<RefCell<Box<dyn Write>>>`
log writer. This is deliberate — an emitter deep in an `&self` evaluator must be able to report
without threading a mutable borrow out of the engine.

`Diagnostic` derives `PartialEq`, `Eq`, `Clone` and `Debug`, and has no `Display`; the rendered form
is the sink's, not the type's.

### Progress events at HEAD

Two producers emit `Progress`:

| Producer | Content |
|---|---|
| `sim-engine` end of run | `simulation ended ({FinishReason}) at time {n}` |
| `cli` `-v` echo | The effective-invocation block (§7) |

Per-stage banners, per-file `compiling …` lines, work-library resolution lines, hierarchy-flattening
counts and filelist echoes are not implemented.

---

## 3. The rendered diagnostic line

`StderrSink::render_diagnostic` is the sole renderer. The canonical shape:

```
<file>:<line>:<col>: <severity>[<VITA-NUMBER>] <MNEMONIC>: <message> [in <instance>] [at time <ticks>]
```

Composed as:

```rust
head = format!("{}[{}] {}: {}{whose}{when}",
               d.severity.token(), d.code.code_num(), d.code.mnemonic(), d.message);
line = match &d.location {
    Some(loc) => format!("{}:{}:{}: {}", loc.file, loc.line, loc.col, head),
    None      => head,
};
```

| Part | Present when | Omission rule |
|---|---|---|
| `<file>:<line>:<col>: ` | The emitter resolved a span | Dropped whole, including the trailing `: ` |
| `<severity>[<NUMBER>] <MNEMONIC>: <message>` | Always | The one mandatory part |
| ` [in <instance>]` | `context.first()` exists | Dropped whole, including the leading space |
| ` [at time <ticks>]` | `sim_time` is `Some` | Dropped whole |

An absent part is omitted entirely — never an empty bracket and never filler text. The order is
fixed: location prefix, head, instance, time.

The mnemonic sits between the bracket and the colon on every line. It is the string that works
unambiguously in `-Wno-` and `-Werror=`, and it is the key a reader carries into
[15-error-code-reference.md](15-error-code-reference.md) and `vita explain`.

Both discriminators exist because `file:line:col` alone does not identify a diagnostic. A module
instantiated N times lowers N copies of one statement, so N reports share one source line and only
the instance path separates them; a design with many indexed arrays or many `unique case` sites
reports the same line repeatedly, and only the time separates the reset window from steady state.

Examples, as the tools print them:

```
warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
pe.sv:4:1: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected statement, found keyword 'endmodule'
d.sv:5:5: error[VITA-E4003] E-RUN-USER-ERROR: child says 42 [in top.u1] [at time 5]
```

Under `-Werror`, a promoted warning keeps its own number and mnemonic and changes only the token —
an `error` severity on a `W`-lettered number:

```
t.sv:5:5: error[VITA-W4007] W-RUN-USER-WARNING: careful [in top] [at time 0]
```

The exact line is pinned in-repo by `crates/cli/tests/runtime_diag_location.rs` and, across the
one-shot and staged paths, by `crates/cli/tests/staged_diag_location.rs`.

---

## 4. Severity

```rust
pub enum Severity { Note, Info, Warning, Error, Fatal }
```

| Level | Token | Effect on the run |
|---|---|---|
| `Fatal` | `fatal` | Latches both the failure flag and the finish flag; the stage stops. Produces `ExitClass::Fatal`, hence a nonzero exit. |
| `Error` | `error` | Recorded, the stage continues (error recovery), the failure flag latches. Produces `ExitClass::HadErrors`, hence a nonzero exit at the end of the stage. |
| `Warning` | `warning` | Printed, the run continues. Changes the exit code only after `-Werror` promotion. |
| `Info` | `info` | Printed, the run continues. No effect on the exit code. |
| `Note` | `note` | Context attached to a parent diagnostic. Never sets a failure flag, never counts as an error. |

- `Severity` derives no `Ord`. It is never compared with `<` or `>`; every consumer matches on
  variants. The declaration order above is an ordering only in that sense.
- Continue-versus-abort follows IEEE 1800 for the RTL severity tasks: `$info`, `$warning` and
  `$error` continue; `$fatal` stops.
- A `Note` is emitted only by elaborate's `note_at`, as a follow-on line anchored at a span
  different from its parent error, and it carries **the same `MsgCode` as that parent** so the gate
  routes the pair as one diagnostic.
- Stage drivers return nonzero when any Error or Fatal reached the sink, so a Makefile or a CI job
  stops before the next stage runs.

### Severity is the emitter's choice, not the code's

`MsgCode::default_severity()` records the severity a code is *expected* to carry, and the bijection
gate (§5) holds the catalogue to it. It is not consulted when a diagnostic is emitted: only the
bijection test and the `explain` fallback branch read it. The severity that reaches the gate, the
counters and the exit class is the one the emitter passed.

The consequence is that a code's declared severity is documentation, not a guarantee, and the two
can diverge. At HEAD they diverge in two places:

| Code | Declared | Emitted as | Site |
|---|---|---|---|
| `E-ART-FORMAT-MISMATCH` | Error | `Fatal` | Missing `.velab` sidecar entries, in the engine's propagate path |
| `F-ELAB-USER-FATAL` | Fatal | `Error` | Elaboration-time `$fatal`, routed through elaborate's generic `error(...)` helper |

Because the gate keys on the emitted severity, these divergences also decide what `-Wno-` and
`-Werror` can do to a given line.

---

## 5. Message codes

Every diagnostic carries a stable, namespaced code with two spellings: a grep-friendly number
(`VITA-E3009`) and a self-describing mnemonic (`E-ELAB-UNSUPPORTED`).

`MsgCode` is an exhaustive enum generated by the `msgcodes!` macro from one table of
`Variant => ("MNEMONIC", "VITA-NUMBER", DefaultSeverity, "title")`. Accessors `mnemonic()`,
`code_num()`, `default_severity()`, `title()` and the `ALL` slice are all `const fn`. Being an enum
rather than free strings, a code cannot drift between the site that emits it and the site that
suppresses it.

The enum holds **68** codes at HEAD.

### Number bands

| Band | Category | Variants at HEAD |
|---|---|---|
| `0xxx` | General / system | yes |
| `1xxx` | Preprocess | yes |
| `2xxx` | Parse | yes |
| `3xxx` | Elaborate | yes |
| `4xxx` | Runtime | yes |
| `5xxx` | Assertion / SVA | reserved, none |
| `6xxx` | SystemVerilog type system | reserved, none |
| `7xxx` | VHDL | reserved, none |
| `8xxx` | Filelist | yes |
| `9xxx` | Artifact | yes |

The severity letter inside the number is `E`, `W`, `I` or `F`.

### Governance

- The **mnemonic is the primary stable key**: permanent, never renamed, never renumbered. The number
  is secondary and equally permanent once assigned; numbers are never reused.
- The macro table is grouped by band but is not sorted by number, and a band's section comment does
  not bind a row's number — two elaborate string warnings are declared inside the preprocess section.
  Ordering carries no meaning; the number does.
- Adding a code requires a catalogue entry in the same change, and vice versa. This is enforced, not
  asked for: `crates/diag/tests/bijection.rs` fails the build otherwise.

### The bijection gate

`crates/diag/tests/bijection.rs` embeds [15-error-code-reference.md](15-error-code-reference.md) and
runs three assertions:

| Test | Asserts |
|---|---|
| `msgcode_matches_doc15_body_one_to_one` | `MsgCode::ALL.len() == 68`, and the sorted mnemonic set of the enum equals that of the catalogue body |
| `doc15_severity_and_number_match_enum` | Each catalogue entry's `VITA-####` and parenthesised severity equal `code_num()` and `default_severity().token()`, case-folded |
| `mnemonics_and_numbers_are_unique` | No duplicate mnemonic, no duplicate number |

The gate parses catalogue headers of the form ``### VITA-W3056 · `W-ELAB-FEATURE-LIMIT` (Warning)``,
taking the number as the first whitespace-delimited token, the mnemonic from the first backtick pair
and the severity from the parenthesised token. Only the body is gated; the catalogue's reserved-code
appendix is excluded, because those codes have no enum variant.

### Default-severity distribution

| Default severity | Codes |
|---|---|
| Error | 30 |
| Warning | 30 |
| Fatal | 6 |
| Info | 2 |
| Note | 0 |

`Note` has no default-severity row: notes borrow their parent's code.

### The three accepted spellings, and the one resolver

`MsgCode::resolve` is the only function that turns a typed string into a code. It trims the input and
compares case-insensitively:

| Form | Example |
|---|---|
| Mnemonic | `W-ELAB-FEATURE-LIMIT` |
| Printed number | `VITA-W3056` |
| Number without the `VITA-` prefix | `W3056` |

Every consumer routes through it: `vita explain`, and `vita-log`'s `intern_mnemonic`, which projects
the answer back onto the interned `&'static str` mnemonic used as the gate's policy key. One
resolver is a contract clause, not an implementation detail: two spellings of "resolve a code" that
answer differently is exactly the defect that made `explain` accept a number the gate flags rejected.

Every rejection appends the same verbatim hint, `MsgCode::ACCEPTED_FORMS`:

```
a code is its mnemonic (`W-ELAB-FEATURE-LIMIT`), its printed number (`VITA-W3056`), or that number bare (`W3056`); `vita explain <CODE>` describes one
```

### What codes are for

1. Per-code suppression and promotion (§6).
2. `vita explain <CODE>` (§8).
3. Test and corpus assertions on the **code**, never on message substrings. See
   [09-testing-and-verification.md](09-testing-and-verification.md).

### Registered codes with no emitter at HEAD

Six codes are in the enum, in the catalogue, and resolvable by `explain`, `-Wno-` and `-Werror=`,
and nothing in the tree produces them:

| Code | Number | Why nothing emits it |
|---|---|---|
| `F-LIMIT-ERRORS` | `VITA-F0002` | There is no `--error-limit` flag; the internal caps report differently (§9) |
| `W-LINT-UNCLOSED` | `VITA-W1003` | The inline lint pragma is not implemented (§6) |
| `W-ELAB-WIDTH-TRUNC` | `VITA-W3008` | The generic elaborate warning channel reports `W-ELAB-FEATURE-LIMIT` |
| `W-ELAB-CASEZ-APPROX` | `VITA-W3011` | No `casez` approximation warning is raised |
| `E-RUN-ASSERT-FAIL` | `VITA-E4001` | Reserved for an assertion failure with no action block |
| `W-RUN-NO-LOCATIONS` | `VITA-W4006` | Nothing strips the location side-table from a snapshot: `velab` always writes it and `vrun` always threads it back (§13) |

---

## 6. The gate: `-Wno-` and `-Werror`

`GatePolicy` is built from argv, and `GatedSink` applies it between the producers and `StderrSink`.

### Grammar

| Argument | Effect | On an unknown code |
|---|---|---|
| `-Wno-<CODE>` | Add the code's mnemonic to the suppress set | CLI usage error |
| `-Werror` | Promote every warning | — |
| `-Werror=all` | Promote every warning (`all` is case-insensitive) | — |
| `-Werror=<CODE>` | Add the code's mnemonic to the promote set | CLI usage error |
| anything else | Not a gate flag; the caller keeps parsing | — |

`<CODE>` is parsed by `MsgCode::resolve`, so all three spellings work in both flags. The policy sets
are `BTreeSet<&'static str>` keyed on the interned mnemonic, so three spellings of one code collapse
to one key.

An unknown code is loud, never a silent no-op:

```
$ vita -Werror=W-RUN-USER-INFO t.sv
error[VITA-E0001]: unknown diagnostic code 'W-RUN-USER-INFO' in '-Werror=' — a code is its mnemonic (`W-ELAB-FEATURE-LIMIT`), its printed number (`VITA-W3056`), or that number bare (`W3056`); `vita explain <CODE>` describes one
EXIT=3
```

Gate flags are recognised in the catch-all `-…` arm of `parse_io_args`, after every named flag has
had its chance. That one function serves all four applets, so the flags behave identically on
`vita`, `vcmp`, `velab` and `vrun`.

### Semantics

| Emitted severity | Under `-Wno-<that code>` | Under `-Werror` or `-Werror=<that code>` |
|---|---|---|
| `Error` | Passes through untouched | Ignored |
| `Fatal` | Passes through untouched | Ignored |
| `Warning` | Dropped before the inner sink; never counted | Rewritten to `Error` |
| `Info` | Dropped | Not promoted |
| `Note` | Dropped | Not promoted |

- **Error and Fatal are the always-logged spine.** A `-Wno-E-ELAB-UNRESOLVED-NAME` is *accepted* —
  the mnemonic resolves — and has no effect. A suppression flag can never hide a real failure.
- Suppressible in practice = whatever is emitted at `Warning`, `Info` or `Note`. Because severity is
  the emitter's choice (§4), this is a property of the diagnostic, not of the code.
- Promotable = `Warning` only.
- A note is suppressed by naming its parent error's code; the error itself still prints.
- **A promoted diagnostic keeps its original number.** Only the severity token changes, so the exit
  class changes while the stable identity of the report does not.
- Suppressed diagnostics never reach `StderrSink`, so its counters — and therefore the epilogue —
  report the **post-gate** stream. A promoted warning counts as an error there, which is what drives
  the exit code.
- `GatePolicy::is_empty()` reports a pure pass-through gate.

### Where the gate is installed

Every pipeline driver builds the sink and wraps it:

```rust
let inner = StderrSink::with_output(opts.verbosity.unwrap_or(1), log);
let sink  = vita_log::GatedSink::new(&inner, opts.gate.clone());
```

Six install sites: one-shot `vita`, `vcmp`, `velab`, `velab -L`, `vrun`, and `--dump-filelist`.

### Two bypasses of the gate

1. **CLI usage errors** (§11) print with a raw `eprintln!` and never reach a sink.
2. **Filelist expansion** runs before argv is parsed, so no `GatePolicy` exists yet; it emits through
   a fresh ungated sink at verbosity 1 with no log. `W-FLIST-OVERRIDE` is the one filelist
   diagnostic moved onto the gated path: expansion only *records* each `(knob, old, new)` override,
   and the pipeline replays them through the gated sink at start-up, so
   `-Werror=W-FLIST-OVERRIDE` can promote them and the epilogue counts them.

### One gate for tool diagnostics and RTL severity tasks

Compile-time diagnostics and the runtime severity tasks pass through the same `GatedSink`, so
`-Wno-` and `-Werror=` apply uniformly to both. `-Werror=W-RUN-USER-WARNING` turns every `$warning`
in a design into a CI failure with no edit to the RTL. There is one code path and no special case
for "tool message" versus "RTL message"; this is why the two are one subsystem rather than two.

### Inline lint pragma

The design for source-local suppression is a comment pragma, in Verilator's spelling:

```systemverilog
// vitamin lint_off W-ELAB-WIDTH-TRUNC
…
// vitamin lint_on
```

The comment form is chosen over a SystemVerilog attribute because other tools ignore a comment, so a
file carrying one stays portable; because it works in Verilog-2005, where `(* … *)` does not; and
because an attribute cannot bracket a region. An unclosed `lint_off` at end of file reports
`W-LINT-UNCLOSED`. Being source text, a pragma changes the preprocessed bytes and is therefore
hashed into the compile-stage inputs like any other source change — consistent with, not an
exception to, the rule that flags are output policy and source is input. Its scope is the textually
inlined `` `include `` closure only; it does not cross into a library unit compiled separately,
whose bytes do not contain the pragma.

**Status at HEAD: not implemented.** No pragma is recognised anywhere in the tree, and
`W-LINT-UNCLOSED` has no emitter.

---

## 7. Streams, verbosity and the log tee

### Stream discipline

| Event | Stream | Written when |
|---|---|---|
| `Diagnostic` | stderr | Always |
| Counts epilogue | stderr | Always |
| `Progress` | stdout | `verbosity >= 1` |
| `RtlOutput` | stdout | `verbosity >= 1` |

Diagnostics go to stderr and RTL output goes to stdout, so a pipeline can consume a design's output
without a diagnostic contaminating it, and a redirect of stdout still leaves the failure visible.
`RtlOutput` is written **verbatim**, with no newline appended — `$write` must be able to build a line
in pieces. `ProgressEvent` gets a newline.

Both stdout and stderr writes drop their `io::Error`. `print!` and `println!` panic on `EPIPE`, and
on macOS a worker thread can observe `EPIPE` while its thread-directed `SIGPIPE` is masked; the
process still dies through the pending signal, and a panic message would reach stderr first. Dropping
the error is the conventional producer behaviour: stop quietly. This also swallows a non-pipe write
failure such as `ENOSPC` on a redirected stdout, which yields a truncated transcript rather than a
loud panic.

### Verbosity

| Flag | Verbosity | Meaning |
|---|---|---|
| `-q`, `--quiet` | 0 | Suppress the terminal copy of `Progress` and `RtlOutput`. Diagnostics, the epilogue and the log copy are unaffected. |
| (default) | 1 | Diagnostics, RTL output, progress |
| `-v` | 2 | Adds the effective-invocation echo |
| `-vv` | 3 | Reserved surface; renders the same as 2 |
| `--verbosity <0..3>` | numeric | The same scale, spelled explicitly |

`-q --log run.log` is the intended combination for a quiet console with a complete file.

### The log tee

| Flag | Behaviour |
|---|---|
| `-l <FILE>`, `--log <FILE>` | Tee every emitted line to `<FILE>`. `-` means stderr. |
| `--log-append` | Append instead of the default truncate |

The tee is a **single writer**. Diagnostics, progress lines, RTL text and the epilogue all pass
through it in emission order, regardless of verbosity, so the file is a faithful replay of the
console and the two can never drift apart. Nothing writes to a second, independent file handle.
Interleaving is preserved by construction: there is one ordering point, and every event passes
through it.

A `--log` path that cannot be opened is a CLI usage error, exit 3 — never a silently unlogged run.

### Not implemented

| Surface | State |
|---|---|
| Automatic per-stage log file (`vcmp.log`, `velab.log`, `vrun.log`, `vita.log`) | Not implemented. Logging happens only under `--log`. |
| `--log-dir <dir>`, `--no-log` | Not implemented; they exist to control automatic logging. |
| `--color=auto\|always\|never`, `--no-color`, `NO_COLOR` | Not implemented. Output carries no ANSI escapes, so the terminal and file copies are already byte-identical. |
| `--diagnostics-json <file>` | Not implemented. The stable `MsgCode` on every diagnostic is the key that would join a structured channel to the text one. |

### The `-v` effective-invocation echo

Under a Makefile or a wrapper script, the arguments a human reads and the arguments the process
receives are different texts, and only the second decided the run: the shell expanded `$(…)` before
`vita` started, the filelist expander spliced the `-f` frames away, and an environment knob such as
`VITA_THREADS` never appears in argv at all. A failing CI transcript must be able to answer "which
`W` was compiled in?" on its own.

`-v` therefore prints the resolved answer as a block at the head of the transcript, one `label: value`
row per knob:

| Row | Content |
|---|---|
| `invocation` | The original argv, shell-quoted |
| `cwd` | Working directory |
| `filelists` | Every filelist consulted, nested ones included |
| `sources` | The post-expansion inputs, in command order |
| `incdirs`, `defines` | The effective compile-stage surface |
| `plusargs` | Runtime plusargs |
| `params` | `-G` top-level parameter overrides — the one knob whose effect is a *different design*, so it gets a row of its own rather than hiding inside the raw argv line |
| `tops` | Elaboration roots |
| `output`, `obs-dir`, `obs-procs`, `probes`, `log` | Output surfaces, including which profiling flag was used |
| `timeout` | Tick budget |
| `threads` | Value **and source** — `--threads`, `VITA_THREADS`, or `auto` |
| `env` | The vitamin environment variables that are set |
| (applet-specific) | `-L` libraries, `--work`, `--upstream` |

Rules: an empty row is omitted entirely rather than printed as an empty value; long value lists wrap
to the value column, and a single long value is never split across lines, because a broken path is
worse than a long line; a blank line brackets the block so it reads as one unit in an otherwise flat
line stream. The block is emitted as ordinary `Progress` events, so the `--log` tee captures it
through the same writer in the same order. Each applet prints its own stage's surface. It is pure
reporting: it changes no compilation, no simulation and no output, and is never hashed into an
artifact.

---

## 8. `vita explain`

`vita explain <CODE>` prints the catalogue entry for a diagnostic. It is dispatched before any flag
parsing and exists only on the one-shot `vita` applet; the staged applets have no `explain`.

| Input | Output | Exit |
|---|---|---|
| No argument | `error[VITA-E0001]: 'explain' needs a diagnostic code (mnemonic or VITA-####)` on stderr | 3 |
| Unresolvable code | `error[VITA-E0001]: unknown diagnostic code '<q>' — <ACCEPTED_FORMS>` on stderr | 3 |
| Any of the three spellings of a real code | The catalogue entry, verbatim, on **stdout** | 0 |
| A code with no catalogue entry | ``<NUMBER> · `<MNEMONIC>` (<severity>)`` and the code's `title()` | 0 |

The last row is unreachable: the bijection gate forbids a code without an entry. It exists so
`explain` is total.

The catalogue is embedded at compile time with `include_str!` of
[15-error-code-reference.md](15-error-code-reference.md) — cargo-only, no `build.rs`. Extraction
finds the literal header `### <VITA-NUMBER> ·` and cuts at whichever comes first, the next `\n### `
or the next `\n---`.

The extraction imposes the shape of a catalogue entry: a `### ` header, then the prose, then a
worked example, then the fix — everything up to the next entry header or the next horizontal rule is
what `explain` prints. Two consumers parse that header line, `explain` for the number and the
bijection gate for the number, mnemonic and severity, so its grammar is a contract, not formatting.

---

## 9. Error caps

There is no `--error-limit` flag. Three internal caps stop a broken input from producing thousands
of lines:

| Cap | Value | On reaching it |
|---|---|---|
| Parser error cap | 50 | Further parse errors are not recorded. No diagnostic marks the cap. |
| Elaborate error cap | 200 | Error 201 emits one `E-ELAB-UNSUPPORTED` at `Severity::Error` reading `too many elaborate errors; further diagnostics suppressed (cap 200)`. Every later error is dropped, and so is every `note_at`. |
| Runtime index-report cap | 8, with separate budgets for known and unknown indices | Report 8 replaces its message with `further out-of-range diagnostics suppressed` or `further unknown-index diagnostics suppressed`. Reports 9 and beyond emit nothing. |

**Status at HEAD:** the intent is that reaching a cap is itself a report — a Fatal `F-LIMIT-ERRORS`
that aborts the stage. The elaborate cap reports through `E-ELAB-UNSUPPORTED` instead, and the
parser cap reports nothing at all, so a design that trips it produces a truncated error list with no
line saying so.

---

## 10. The counts epilogue

Every pipeline run ends with one line on stderr:

```
errors=<E> warnings=<W> notes=<N>
```

| Field | Counts |
|---|---|
| `errors` | Error **plus** Fatal. A run that hit a `$fatal` definitely failed; one bucket says so. |
| `warnings` | Warning |
| `notes` | Note **plus** Info |

Properties:

- **Unsuppressible.** No flag removes it. Together with Error and Fatal it is the spine a CI job can
  grep for.
- **Post-gate.** Counters live in `StderrSink`, behind `GatedSink`, so suppressed diagnostics are
  absent and promoted warnings are counted as errors.
- Teed to `--log` like every other line.
- Printed by all five pipeline drivers: `vita`, `vcmp`, `velab`, `velab -L`, `vrun`.
- **Not** printed for `--help`, `--version`, `explain`, or an argv-parse failure — none of which ran
  a stage.

`vrun` and `vita` also print the engine's own end-of-run line, `simulation ended ({reason}) at time
{n}`, as a `Progress` event on **stdout** and therefore subject to verbosity like every progress
line. It and the epilogue are two separate lines on two separate streams; nothing prints them as
one, and only the epilogue is unsuppressible.

---

## 11. CLI usage errors — the second rendered shape

Argument validation happens before a sink exists, and 67 sites across
`crates/cli/src/{stage_args,frontend,pipeline}.rs` report it with a raw `eprintln!` in a different
shape:

```
error[<VITA-NUMBER>]: <message>
```

Compared with a sink-rendered diagnostic, such a line has **no mnemonic**, no instance, no time, is
**not counted** in the epilogue, and is **not subject** to `-Wno-` or `-Werror`. The codes used this
way are `E-CLI-BAD-FLAG` (the great majority), `E-FLIST-NOT-FOUND` and `E-FLIST-WRONG-STAGE`. All of
them exit 3.

```
$ vita vrun missing.velab
error[VITA-E8005]: cannot read 'missing.velab': No such file or directory (os error 2)
errors=0 warnings=0 notes=0
EXIT=3
```

The epilogue reads zero because the error bypassed the sink.

This class covers three kinds of refusal, all of which exit 3 rather than guess:

| Refusal | Example |
|---|---|
| Malformed or incomplete flag | `'--verbosity' needs 0..=3`; `'-G{}' needs NAME=VALUE` |
| Flag offered to the wrong stage | `-L is a velab flag — '<stage>' does not read libraries`; `-G` on `vcmp` or `vrun`; compile-stage `+define+`/`-I` on a run-stage applet |
| Observability rail on a staged applet | `'--obs-dir <D>' is a one-shot vita argument — '<stage>' …`; `--probe`/`--probe-file` and `--obs-procs` likewise. The staged flow refuses the observability rail rather than accepting the flag and dropping it. |

`--probe` without `--obs-dir`, and `--obs-procs` without `--obs-dir`, are also refused: instrumenting
a run and then discarding the numbers is the failure mode the refusal exists to prevent. See
[19-ai-agent-observability.md](19-ai-agent-observability.md).

**Status at HEAD:** the intent is that no producer prints a diagnostic directly, and that every
report is coded, counted and gated. Argv validation is the exception, and its lines are the one
diagnostic shape a log consumer must parse separately.

---

## 12. Exit classes

The exit code is a class, so CI can branch on it without reading the transcript.

| Code | Class | Meaning |
|---|---|---|
| 0 | `EXIT_OK` | Clean: parse and elaborate succeeded, the simulation finished, no Error or Fatal reached the sink |
| 1 | `EXIT_USER_ERROR` | User or design failure: lex/parse errors, elaborate produced no IR, runtime `$fatal`, or a `-Werror`-promoted warning |
| 2 | `EXIT_STALE` | Artifact gate rejection: magic, `format_version`, schema hash, tool semver-major, or a RULE-V upstream mismatch |
| 3 | `EXIT_CLI_ERROR` | CLI or usage error: no sources, unreadable or unwritable file, unknown applet, unknown flag, unknown code in a gate flag |
| 101 | (Rust convention) | Panic. Re-raised on the main thread rather than mapped to a vita class, so a differential runner can never mistake a vitamin crash for an RTL failure. |

**Class 2 is separate from class 1 on purpose.** A stale or incompatible artifact is not an RTL
problem; the correct response is to re-run `vcmp`/`velab`, not to debug the design. Silent reuse of
a mismatched artifact never happens.

```
$ vita vrun bad.velab
error[VITA-E9001] E-ART-FORMAT-MISMATCH: bad or missing velab magic
errors=1 warnings=0 notes=0
EXIT=2
```

`emit_artifact_error` is the single funnel: it renders the artifact error as a location-less
`Severity::Error` diagnostic through the sink and returns `EXIT_STALE`.

### How the simulation exit code is decided

`ExitClass` is `Ok`, `HadErrors` or `Fatal`. `FinishReason` is `Finish`, `Stop`, `Quiescent`,
`DeltaLimit` or `Error`. The rule: `ExitClass::Ok` **and** a finish reason in
`{Finish, Quiescent, Stop}` gives 0; anything else gives 1.

`$finish` exits 0. `$fatal(n, …)` exits 1. The leading `n` is the IEEE finish_number — not a shell
exit code — which selects end-of-run statistics verbosity in
[hdl-reference/system-tasks/04-simulation-control.md](hdl-reference/system-tasks/04-simulation-control.md).
`elaborate` consumes and discards that literal, and vita prints no end-of-run statistics at any
level.

### The promoted-warning path

Each driver applies the same rule at the end:

```rust
if code == EXIT_OK && inner.had_error_or_fatal() { EXIT_USER_ERROR } else { code }
```

`vcmp` and `velab` apply it **before writing the artifact**, so a `-Werror` promotion fails the stage
and leaves no output file — matching what a real compile error does.

### Distinguishing failures inside class 1

Exit 1 covers both "did not compile" and "ran and was wrong". The discriminator is the **MsgCode** on
the always-logged spine, not the exit code and never a text grep. The corpus runner classifies
stale-versus-crash and compile-versus-run this way; see
[09-testing-and-verification.md](09-testing-and-verification.md).

### Not implemented

| Flag | Intent |
|---|---|
| `-n` / `-N` | Treat `$stop` as a clean `$finish` (exit 0), or as a failure (exit 1), for headless CI |
| `--error-exit` | Make a `$error` — which IEEE says does not terminate — exit nonzero |
| `--quiet-exit` | Keep the exit code, suppress the trailing "exiting due to errors" line |
| `--finish-at` | A forced cut-off reported as distinct from a self-`$finish`, so a testbench that never finishes fails instead of passing silently |

The implemented cut-off is `--timeout <ticks>`, which ends the run cleanly with exit 0. Neither `-q`
nor a `$fatal` `n` argument ever changes the exit class: verbosity knobs move text, not classes.

---

## 13. Source locations

### The types

```rust
pub struct SourceLoc { pub file: String, pub line: u32, pub col: u32,
                       pub byte_start: u32, pub byte_end: u32 }

pub trait SpanResolver { fn resolve(&self, lo: u32, hi: u32) -> SourceLoc; }
```

Line and column are 1-based, and the column counts Unicode scalar values from the last newline.

### `SourceMap` — the preprocessor's provenance map

Diagnostics are reported against spans in the **expanded** buffer, and every one must be mapped back
to a byte of an **original** file. `SourceMap` holds:

- `files`: for each file, the display name used in diagnostics and the **original** text — never the
  expanded buffer.
- `segments`: sorted, non-overlapping, covering the whole expanded buffer with no gaps. Each is
  `{ exp_start, exp_end, file, orig_start, collapsed }`.
  - `collapsed == false` (verbatim run): expanded byte `b` maps to original byte
    `orig_start + (b - exp_start)`.
  - `collapsed == true` (macro expansion, substituted body, include boundary): **every** byte of the
    run maps to the single origin byte `orig_start` — the directive or macro-use site. A diagnostic
    inside an expansion points at the use, which is the line the reader can edit.
- A per-file line index, built lazily on the first resolve and deliberately **not** part of the `.vu`
  wire form: it is derivable, so carrying it would be a second source of truth.

`resolve` binary-searches for the segment containing the byte and converts within that file's
original text. An empty map answers `("", 1, 1, 0)`. Out-of-range input is clamped, never a panic,
and the verbatim delta is clamped to the segment width. `resolve_span(lo, hi)` takes file, line and
column from `lo`, and `byte_end` from `hi` only when `hi` resolves into the same file.

### Per-stage acquisition

| Stage | Mechanism |
|---|---|
| Preprocess | Each diagnostic carries an expanded-byte offset; the driver resolves it through the map |
| Lex | `LexError { kind, span }`. The message is `lex error: <kind> (<mnemonic>)`, and all six kinds hint `E-PARSE-UNEXPECTED-TOKEN` |
| Parse | `ParseError { span, expected, found, found_span }` renders `expected {expected}, found {found}`, or just `expected {expected}` when the found token's text cannot be recovered — the tail is **dropped rather than guessed** |
| Parse warnings | Emitted **before** the parse-error gate, so a syntax error elsewhere in the file cannot swallow a portability warning |
| Elaborate | The elaborator holds an optional `&dyn SpanResolver` and a current span. `error_at` / `warn_code_at` / `note_at` temporarily override the current span with an explicit one; `warn_code_at` falls back to the current location when the explicit span resolves to nothing |
| Runtime | The engine's IR is **span-free**. Elaborate pre-resolves every relevant statement into a statement-location table, and the engine's `stmt_diag_meta(sid)` is the only source of a runtime `file:line:col`. A missing entry yields no location and no instance |

The engine works on span-free IR by design; see
[17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md). Locations ride as an independent
overlay so the golden IR shape stays clean under [16-schema-hash-spec.md](16-schema-hash-spec.md).

### The instance context

The statement-location record is `{ file, line, col, byte_start, byte_end, instance }`. The
`instance` field exists because a module instantiated N times lowers N copies of one statement and
`file:line:col` alone cannot separate them.

- Elaborate builds one frame from its `%m` display prefix, empty at top level. A singleton generate
  scope drops its `[0]` index, matching the `%m` spelling.
- The engine builds the frame from the recorded instance. When that string begins with `.` — the
  class-method marker `.<class>.<method>` — the label is completed with the calling process's
  instance scope, so a method reports under the object's owner rather than under a bare class name.

### One-shot and staged parity

One-shot `vita` holds the live `SourceMap` and installs a resolver over it directly.

The staged flow has to carry the same information across process boundaries:

1. `vcmp` appends a **source-map tail** to the `.vu`, after the timescale tail: the file display
   names with their original text, and the segment tuples. The canonical path and directory of each
   file are deliberately **not** carried — machine-local absolute paths would break 3-OS byte
   identity — so the rebuilt entries have neither.
2. `velab` decodes that tail and installs the **same resolver type** the one-shot path uses. An
   absent or undecodable tail is **loud**: `E-ART-FORMAT-MISMATCH` with `undecodable .vu source-map
   trailer: {e}`, exit 2. Tolerating it would silently resurrect location-less staged diagnostics,
   and a resolver over an empty map would stamp every diagnostic `:1:1` — a wrong location is worse
   than none.
3. `velab` writes the elaborate-time-resolved statement locations into a `.velab` trailer, and `vrun`
   threads them back into the simulation options. The same trailer carries the statement, expression
   and process instance scopes used for the `%m` and context labels.
4. Parity is pinned by `crates/cli/tests/staged_diag_location.rs`, which compares the **whole
   diagnostic line** between the two paths *and* pins the location absolutely — parity alone would
   pass two equally location-less lines.

Trailer versions, against `CURRENT_FORMAT_VERSION = 31`: the source-map tail is v28, statement
locations v29, statement and expression scopes v30, process instance scopes v31.

Where no resolver is installed at all — elaborate unit tests, AST-only callers, engine harnesses —
locations are simply absent and diagnostics render without the prefix.

---

## 14. Artifact gate messages

The artifact gates are the producers of exit class 2. Their messages are fixed text:

| Condition | Code | Message |
|---|---|---|
| Bad or missing magic | `E-ART-FORMAT-MISMATCH` | `bad or missing {velab\|vu} magic` |
| Header will not decode | `E-ART-FORMAT-MISMATCH` | `undecodable {label} header: {e}` |
| `format_version` mismatch | `E-ART-FORMAT-MISMATCH` | ``format_version={h} but this tool expects {t}; regenerate with `velab` `` |
| Tool semver-major mismatch | `E-ART-VERSION-GATE` | `produced by vitamin {h}.x, this tool is {t}.x; regenerate or install a matching vitamin` |
| `schema_hash` mismatch | `E-ART-SCHEMA-MISMATCH` | ``sim-ir type shape changed between builds; rerun `velab` `` |
| `vrun --upstream` digest drift | `E-ART-STALE-UPSTREAM` | `{up}: digest changed since the .velab snapshot (rerun velab, or drop --upstream)` |
| Work-library RULE-V auto-gate | `E-ART-STALE-UPSTREAM` | ``work library `{name}`: {path} changed since the .velab snapshot (re-run velab)`` |

Gate order is lowest to highest: format, then tool semver-major, then schema hash. The policy is
version-*gate* — refuse and rebuild — never silent migration. See
[14-staged-artifacts.md](14-staged-artifacts.md).

---

## 15. RTL severity task integration

`sim-engine` and `elaborate` emit `LogEvent`s rather than printing, so `$display` and `$error`
interleave in one stream in simulation-time order.

| Construct | Severity | Code | Behaviour |
|---|---|---|---|
| `$info` | Info | `I-RUN-USER-INFO` | Continues; no effect on the exit code |
| `$warning` | Warning | `W-RUN-USER-WARNING` | Continues; `-Werror=` can fail CI without an RTL edit |
| `$error` | Error | `E-RUN-USER-ERROR` | Continues (IEEE: does not terminate); latches the failure flag |
| `$fatal(n, …)` | Fatal | `F-RUN-FATAL` | Stops with an implicit `$finish`; the leading `n` is consumed and discarded; exit 1 |
| `$display`, `$write`, `$monitor`, `$strobe` | — | — | `RtlOutput`: no severity, no code, same tee |
| `unique` / `priority` case or if matching no branch | Warning | `W-RUN-UNIQUE-VIOLATION` | IEEE 1800 §12.4.2 / §12.5.3 violation report |
| Assertion failure with no action block | Error | `E-RUN-ASSERT-FAIL` | Code reserved; no emitter at HEAD |

Every runtime severity line carries the severity token, `file:line:col` from the statement-location
table, the hierarchical scope, and the simulation time — the four pieces IEEE 1800 §20.10 and §20.11
require a tool to add. An empty message renders the code's `title()` in its place.

The elaborate-side class mapping (`Info`, `Warning`, `Error`, `Fatal`, `UniqueViolation`) is the
single spelling of "what does a statement of this class report", shared by both the statement path
and the frame path so the two cannot disagree. Its `UniqueViolation` variant is declared last on
purpose, so postcard's discriminant numbering keeps older `.velab` files decoding.

### Elaboration-time severity tasks

`$fatal`, `$error`, `$warning` and `$info` fire in **two** contexts under IEEE 1800 — elaboration and
runtime. The elaboration-time forms are emitted by `elaborate` as the same `LogEvent::Diagnostic`,
with **no** `sim_time`, with the span taken directly from the AST rather than through the runtime
location table, and with the partial instance or generate path as the frame. Their codes are
`F-ELAB-USER-FATAL`, `E-ELAB-USER-ERROR`, `W-ELAB-USER-WARNING`, `I-ELAB-USER-INFO`. An
elaboration-time `$fatal` aborts the stage in exit class 1 — a design failure, distinct from a stale
artifact. See [hdl-reference/system-tasks/13-misc.md](hdl-reference/system-tasks/13-misc.md).

**Status at HEAD:** all four are live. The `$fatal` arm is routed through elaborate's generic error
helper, so it is emitted at `Severity::Error` despite `F-ELAB-USER-FATAL` being declared Fatal; the
run still fails, and the printed token reads `error`.

---

## 16. Refusals — where the system chooses loud over quiet

The contract everywhere above is one rule: report, or refuse; never guess. The enumerated refusals:

| Refusal | What it declines to guess |
|---|---|
| `E-ELAB-UNSUPPORTED` | Any construct elaborate cannot lower faithfully. It errors instead of approximating. |
| `W-ELAB-FEATURE-LIMIT` | The opposite lever: a legal construct that *is* simplified degrades with a warning and keeps its IR, instead of discarding the module |
| Artifact header gate | Reusing an artifact from a different format, tool major version, or IR shape |
| Missing `.vu` source-map tail | Falling back to location-less staged diagnostics |
| `E-RUN-RANGE` vs `W-RUN-RANGE-UNKNOWN` | Conflating a design bug with IEEE 1364 §5.2.1-mandated x/z index behaviour. The access is recovered — read X, drop the write — and reported either way |
| `F-RUN-BODY-STEP-LIMIT` | Letting one process run forever without suspending |
| `F-RUN-NO-CONVERGE` | A zero-delay loop or a combinational oscillation |
| `F-RUN-CLASS-LIMIT` | Running out of memory on an unbounded `new()`; the class heap is not garbage-collected |
| `W-RUN-WIDE-ARITH` | A stalled multi-word computation. The result is poisoned to X and the line says so |
| `$finish` / `$stop` reached inside a function body | What the calling expression receives. The reference tools disagree, so the run ends with `F-RUN-FATAL` rather than picking one |
| `W-RUN-BACKEND-FALLBACK` | Running a different backend than the one asked for. The answer is unaffected; the speed is |
| `W-PP-TIMESCALE-MIXED` | Shipping a design that IEEE 1800 §3.14.2.2 makes illegal and other tools reject. Names up to eight modules, then `(and N more)` |
| `W-ELAB-STR-ESCAPE` | A string escape IEEE 1800 Table 5-1 does not define, which tools read differently. One line per literal-and-escape pair per run |
| Unknown mnemonic in `-Wno-`/`-Werror=` | A silently ignored typo in a gate flag |
| Error and Fatal are unsuppressible | Letting a `-Wno-` hide a real failure |
| Output would overwrite an input | Clobbering a source file |
| `--backend vm`/`interp` in a build without them | Silently substituting the default backend |

---

## Related specifications

- [03-build-and-portability.md](03-build-and-portability.md) — cargo-only build; `panic=unwind` and
  the caught panic behind exit 101
- [04-architecture.md](04-architecture.md) — pipeline stages and the crate table
- [09-testing-and-verification.md](09-testing-and-verification.md) — corpus runner, code-based
  assertions, exit classification
- [14-staged-artifacts.md](14-staged-artifacts.md) — artifact gates, trailers, the location overlay,
  and the rule that output policy never enters a hash
- [15-error-code-reference.md](15-error-code-reference.md) — the per-code catalogue and its
  governance
- [16-schema-hash-spec.md](16-schema-hash-spec.md), [17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md)
  — why the engine IR is span-free
- [19-ai-agent-observability.md](19-ai-agent-observability.md) — the observability rail the staged
  applets refuse
- [hdl-reference/system-tasks/13-misc.md](hdl-reference/system-tasks/13-misc.md),
  [hdl-reference/system-tasks/04-simulation-control.md](hdl-reference/system-tasks/04-simulation-control.md),
  [hdl-reference/system-tasks/01-display-io.md](hdl-reference/system-tasks/01-display-io.md),
  [hdl-reference/systemverilog/07-assertions-sva.md](hdl-reference/systemverilog/07-assertions-sva.md)
  — the language-side definitions
- [11-sources-and-citations.md](11-sources-and-citations.md) — the tool precedents this design draws
  on: coded diagnostics with an `explain` command, `-Wno-`/`-Werror`, a log tee flag, and
  warning-to-error promotion for RTL severity tasks
- [../manual/007_error-codes.md](../manual/007_error-codes.md),
  [../manual/004_cli-reference.md](../manual/004_cli-reference.md) — the user-facing form
