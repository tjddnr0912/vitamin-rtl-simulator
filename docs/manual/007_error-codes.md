# 007 · Error & Warning Codes

Every diagnostic vitamin prints carries a stable code: a number like `VITA-E3009`
and a mnemonic like `E-ELAB-UNSUPPORTED`. This chapter defines the rendered
diagnostic format, the severity levels and how to change them, the process exit
codes, and the complete catalogue of all 68 registered codes.

For installing and running the tools see [Installation](001_installation.md) and
the [CLI Reference](004_cli-reference.md), which covers `vita` and the staged
`vcmp` → `velab` → `vrun` flow.

vitamin runs on Linux and macOS. Path and case behaviour described here assumes a
POSIX filesystem.

---

## How a diagnostic is rendered

Diagnostics go to stderr, one line each, in this shape:

```
<file>:<line>:<col>: <severity>[<VITA-NUMBER>] <MNEMONIC>: <message> [in <instance>] [at time <ticks>]
```

Four parts are optional. Each is omitted entirely — no placeholder, no empty
bracket — when the emitter does not have it. The order never varies: location prefix,
then the head, then the instance, then the simulation time.

| Part | Present when |
|---|---|
| `<file>:<line>:<col>: ` | The emitter resolved a source span. Preprocess, lex, parse and elaborate resolve through the preprocessor's provenance map; runtime diagnostics resolve through the statement-location side table that `velab` writes. |
| `<severity>[<NUMBER>] <MNEMONIC>: <message>` | Always. This head is the one mandatory part. |
| ` [in <instance>]` | The emitter knew an instance path. Elaborate supplies the `%m` prefix of the scope it is lowering; the engine supplies the instance that owns the running process. |
| ` [at time <ticks>]` | The emitter is the simulation engine. Every runtime emitter stamps the current time; front-end and artifact-gate diagnostics do not. |

The instance path matters because a module instantiated N times lowers N copies
of one statement, and `file:line:col` alone cannot tell those N diagnostics
apart. The time matters because a design with many indexed arrays or many
`unique case` sites reports the same line repeatedly, and the time separates the
reset window from steady state.

### One example of each part

Head only, no location, no instance, no time — a preprocess-stage warning about
the whole design:

```
warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
```

Location plus head — a parse error:

```
pe.sv:4:1: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected statement, found keyword 'endmodule'
```

Location, head and instance — an elaborate error:

```
un.sv:3:17: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `top.nope` [in top]
```

All four parts — a runtime `$error` inside an instantiated child:

```
t.sv:6:5: error[VITA-E4003] E-RUN-USER-ERROR: bad [in top.u1] [at time 5]
```

A `note` carries its parent error's code and points at a different span, so the
pair reads as one diagnostic and one `-Wno-` covers both:

```
nt.sv:6:7: error[VITA-E3009] E-ELAB-UNSUPPORTED: block-local `tmp` is referenced outside its `begin…end` block; vita cannot resolve this to the outer `tmp` yet (it would silently read the block-local) — rename the block-local or hoist its declaration to the enclosing scope [in top]
nt.sv:9:9: note[VITA-E3009] E-ELAB-UNSUPPORTED: `tmp` is referenced here, outside the block that declares it [in top]
```

### The second format: command-line usage errors

Errors raised before or during argument handling never reach the diagnostic sink.
They print in a shorter form — no mnemonic, no instance, no time — and are not
counted in the end-of-stage line:

```
error[<VITA-NUMBER>]: <message>
```

```
$ vita vrun missing.velab
error[VITA-E8005]: cannot read 'missing.velab': No such file or directory (os error 2)
errors=0 warnings=0 notes=0
```

Three codes appear this way: `E-CLI-BAD-FLAG`, `E-FLIST-NOT-FOUND` and
`E-FLIST-WRONG-STAGE`. Because they bypass the sink they are also outside the
reach of `-Wno-` and `-Werror`.

### Which stream carries what

| Output | Stream | Suppressed by `-q` |
|---|---|---|
| Diagnostics | stderr | No |
| End-of-stage counts | stderr | No |
| `simulation ended (<reason>) at time <n>` | stdout | Yes |
| RTL text from `$display` / `$write` | stdout, written verbatim | Yes |

`$display` and `$write` output is not a diagnostic, carries no code, and never
appears in the counts.

With `-l <FILE>` / `--log <FILE>` every line above — diagnostics, progress, RTL
text and the counts — is teed to that one file in emission order, regardless of
verbosity. `--log-append` appends instead of truncating; `--log -` writes to
stderr. A log that cannot be opened is a usage error:
`error[VITA-E0001]: cannot open log '<path>': <err>`, exit 3.

Verbosity is `-q` / `--quiet` (0), the default (1), `-v` (2), `-vv` (3), or
`--verbosity <0..3>`. Level 2 adds the effective-invocation echo; level 3 renders
the same as level 2 and is reserved surface.

### The end-of-stage counts

Every pipeline run ends with one unsuppressible line on stderr:

```
errors=<n> warnings=<n> notes=<n>
```

- `errors` sums the Error and Fatal buckets into one number.
- `notes` sums the Note and Info buckets into one number.
- The counts reflect the stream as it stands after severity gating: a diagnostic dropped
  by `-Wno-` is not counted, and a warning promoted by `-Werror` counts as an
  error.
- The line is not printed for `--help`, `--version`, `vita explain`, or an
  argument-parsing failure.

---

## Severity levels

| Severity | Printed token | Effect on the run |
|---|---|---|
| Note | `note` | Follow-on detail for the error above it. Carries that error's code. Never affects the exit code. |
| Info | `info` | Reports information. Never affects the exit code. |
| Warning | `warning` | Records and continues. Never affects the exit code on its own; a `-Werror` promotion changes that. |
| Error | `error` | Records and continues. Sets the run's exit class, so the process exits 1. |
| Fatal | `fatal` | Records and stops. The engine latches the finish, so the process exits 1. |

Severity is a property of the diagnostic as emitted, not of the code. A code's
registered default severity is what `vita explain` reports and what the catalogue
below lists; two codes are emitted at a severity other than their default, and
both are marked in the catalogue.

---

## Controlling severity

Two flags change how a diagnostic is handled. Both work identically on `vita`,
`vcmp`, `velab` and `vrun`.

| Flag | Effect |
|---|---|
| `-Wno-<CODE>` | Drop every diagnostic carrying `<CODE>` that is emitted at Warning, Info or Note severity. |
| `-Werror` | Rewrite every Warning-severity diagnostic to Error. |
| `-Werror=all` | The same as `-Werror`. |
| `-Werror=<CODE>` | Rewrite Warning-severity diagnostics carrying `<CODE>` to Error. |

### What can and cannot be changed

| Emitted severity | Under `-Wno-<its code>` | Under `-Werror` / `-Werror=<its code>` |
|---|---|---|
| Warning | Dropped: never printed, never counted | Rewritten to Error |
| Info | Dropped | Unchanged |
| Note | Dropped | Unchanged |
| Error | Unchanged — passes through | Unchanged |
| Fatal | Unchanged — passes through | Unchanged |

Error and Fatal are the always-logged spine. `-Wno-E-ELAB-UNRESOLVED-NAME` is
accepted, because the mnemonic resolves, and has no effect: a suppression flag
can never hide a real failure.

Of the 68 registered codes, 30 default to Warning and 2 to Info, so those 32 are
the suppressible set by default. Promotion applies to the 30 Warning-default
codes. Notes are suppressed through their parent error's code, which drops the
note while the error itself still prints.

A promoted diagnostic keeps its original number. Only the severity token changes,
so a promoted warning prints as an `error` on a `W`-lettered number:

```
$ vita -Werror t.sv
t.sv:5:5: error[VITA-W4007] W-RUN-USER-WARNING: careful [in top.u1] [at time 5]
```

Because the counts reflect the post-gate stream, suppression and promotion both
move them:

```
$ vita t.sv                                              # errors=1 warnings=2 notes=1
$ vita -Wno-VITA-W4007 -Wno-I-RUN-USER-INFO t.sv         # errors=1 warnings=1 notes=0
$ vita -Werror t.sv                                      # errors=3 warnings=0 notes=1
```

A `-Werror`-promoted warning is enough to turn an otherwise-clean run into exit
1. On `velab` the promotion aborts before the `.velab` artifact is written.

### Two paths the gate does not cover

Command-line usage errors print before a gate policy exists, so they are never
suppressed or promoted. Filelist expansion also runs before argument parsing and
therefore before the policy is built; the one filelist warning that reaches the
gated path is `W-FLIST-OVERRIDE`, which is recorded during parsing and replayed
through the gate at pipeline start.

### The three spellings of a code

Both flags, and `vita explain`, accept any of three spellings, case-insensitively
and with surrounding whitespace trimmed:

| Form | Example |
|---|---|
| The mnemonic | `W-ELAB-FEATURE-LIMIT` |
| The printed number | `VITA-W3056` |
| That number without the prefix | `W3056` |

All three resolve through one function, so the string a diagnostic printed always
works in a flag. A spelling that resolves to no code is a hard usage error, not a
silently ignored typo:

```
$ vita -Werror=W-RUN-USER-INFO t.sv
error[VITA-E0001]: unknown diagnostic code 'W-RUN-USER-INFO' in '-Werror=' — a code is its mnemonic (`W-ELAB-FEATURE-LIMIT`), its printed number (`VITA-W3056`), or that number bare (`W3056`); `vita explain <CODE>` describes one
```

That invocation exits 3.

---

## `vita explain`

`vita explain <CODE>` prints the long-form catalogue entry for one code on
stdout and exits 0. It is dispatched before any flag parsing and exists on the
one-shot `vita` applet only; the staged applets have no `explain` subcommand.

```
$ vita explain W3056
### VITA-W3056 · `W-ELAB-FEATURE-LIMIT` (Warning)
…
```

| Input | Result | Exit |
|---|---|---|
| Any of the three spellings of a registered code | That code's entry, verbatim | 0 |
| No argument | `error[VITA-E0001]: 'explain' needs a diagnostic code (mnemonic or VITA-####)` | 3 |
| A string that resolves to no code | `error[VITA-E0001]: unknown diagnostic code '<q>' — <the three forms>` | 3 |

The text it prints is the body of
[`docs/preview/15-error-code-reference.md`](../preview/15-error-code-reference.md),
embedded into the binary at compile time. That file is the long-form catalogue:
one entry per code, each with the cause, a worked example, and the fix, with the
same numbers and default severities as the tables below. Its prose is written in
Korean.

The registry and that file are kept in step by a test that asserts a 1:1
correspondence between the 68 enum entries and the 68 document entries, and that
each entry's number and severity match the registry. Adding a code without a
document entry, or the reverse, fails the build gate.

### Codes are stable identifiers

A code's meaning is fixed for the life of the tool. A number is never renumbered
and never reused; a mnemonic never changes meaning. Both are safe to match on in
scripts, CI greps and issue reports.

---

## Diagnostic caps

Three internal caps limit how many diagnostics one run reports. None of them is
configurable from the command line.

| Cap | Limit | What happens at the limit |
|---|---|---|
| Parser errors | 50 | Further parse errors are not recorded. No diagnostic marks the cap. |
| Elaborate errors | 200 | One `E-ELAB-UNSUPPORTED` at Error severity reads `too many elaborate errors; further diagnostics suppressed (cap 200)`. Every later elaborate error and note is dropped. |
| Runtime index reports | 8, with separate budgets for known-out-of-range and unknown-index | Report 8 replaces its message with `further out-of-range diagnostics suppressed` or `further unknown-index diagnostics suppressed`. Report 9 onward emits nothing. |

---

## Exit codes

| Exit | Meaning |
|---|---|
| `0` | Clean. Parse and elaborate succeeded and the simulation finished with no errors, having reached `$finish`, `$stop`, or quiescence. |
| `1` | User or design error. Lex or parse errors, elaboration failed, a runtime `$fatal`, or a `-Werror`-promoted warning. |
| `2` | Stale or rejected artifact. Magic, `format_version`, `schema_hash`, tool semver-major, or a RULE-V upstream digest mismatch. |
| `3` | Usage error. No source files, an unreadable or unwritable file, an unknown applet or flag, or an unresolvable code in `-Wno-` / `-Werror=`. |

Exit 2 is deliberately distinct from exit 1: it says the artifact is out of date,
so CI should re-run `vcmp` / `velab` rather than start debugging RTL.

```
$ vita vrun bad.velab
error[VITA-E9001] E-ART-FORMAT-MISMATCH: bad or missing velab magic
errors=1 warnings=0 notes=0
$ echo $?
2
```

A single `error`-severity diagnostic anywhere in a run is enough to push the exit
code to 1. A `fatal` stops the current stage immediately. A panic exits 101, the
conventional Rust code, and is not mapped into this table.

---

## Code bands

The number carries both the stage and the default severity: `VITA-` then one
letter — `E` Error, `W` Warning, `I` Info, `F` Fatal — then a four-digit number
whose leading digit is the stage.

| Band | Stage |
|---|---|
| `0xxx` | General and system |
| `1xxx` | Preprocess |
| `2xxx` | Parse |
| `3xxx` | Elaborate |
| `4xxx` | Runtime |
| `5xxx` | Assertion and SVA — reserved, no registered codes |
| `6xxx` | SystemVerilog type system — reserved, no registered codes |
| `7xxx` | VHDL — reserved, no registered codes |
| `8xxx` | Filelist |
| `9xxx` | Artifact and work library |

In the catalogue below, "Reserved" marks a code that is registered, documented
and resolvable by `vita explain`, `-Wno-` and `-Werror=`, but that no code path
in the tool produces. Six of the 68 are in that state.

---

## `0xxx` — general and system

| Number | Mnemonic | Default | Meaning |
|---|---|---|---|
| `VITA-E0001` | `E-CLI-BAD-FLAG` | Error | An unknown or invalid command-line flag or flag value. Printed in the usage-error form and exits 3. |
| `VITA-F0002` | `F-LIMIT-ERRORS` | Fatal | Reserved. An error-limit abort. The two live caps behave as described under Diagnostic caps and do not use this code. |

---

## `1xxx` — preprocess

Emitted while expanding `` `define ``, `` `include ``, `` `ifdef `` and
`` `timescale `` ahead of lexing.

| Number | Mnemonic | Default | Meaning |
|---|---|---|---|
| `VITA-E1001` | `E-PP-INCLUDE-NOT-FOUND` | Error | An `` `include `` file is not on the current directory or any search path. Paths are case-sensitive. |
| `VITA-E1002` | `E-PP-MACRO-ARITY` | Error | A function-like macro is called with the wrong number of arguments. |
| `VITA-W1003` | `W-LINT-UNCLOSED` | Warning | Reserved. An inline lint-off pragma left open at end of file. No such pragma exists in the tool. |
| `VITA-E1004` | `E-PP-RECURSIVE-MACRO` | Error | A text macro re-invokes itself during its own expansion. |
| `VITA-E1005` | `E-PP-RECURSIVE-INCLUDE` | Error | An `` `include `` chain is cyclic: a file includes itself directly or transitively. An include guard breaks it. |
| `VITA-E1013` | `E-PP-BAD-DIRECTIVE` | Error | A malformed directive: an unknown directive, an undefined macro used, a stray backtick, or an unbalanced or duplicated conditional. |
| `VITA-W1007` | `W-PP-MACRO-REDEFINED` | Warning | `` `define `` redefines an existing macro with different text; the new text wins. An identical redefinition is silent. |
| `VITA-W1008` | `W-PP-UNDEF-UNDEFINED` | Warning | `` `undef `` names a macro that was never defined. Harmless. |
| `VITA-W1017` | `W-PP-TIMESCALE-DEFAULT` | Warning | No module in the design declares a `` `timescale ``; the global unit and precision are the `1ns/1ns` base. See the [Language Reference](003_language-reference.md). |
| `VITA-W1018` | `W-PP-TIMESCALE-MIXED` | Warning | Some modules carry a `` `timescale `` and others do not, which IEEE 1800 §3.14.2.2 does not allow. Names up to eight modules, then `(and N more)`. |

---

## `2xxx` — parse

Emitted by the lexer and parser, the last language-dependent stage.

| Number | Mnemonic | Default | Meaning |
|---|---|---|---|
| `VITA-E2001` | `E-DUP-UNIT` | Error | A design unit — module, package, class or interface — is defined more than once. |
| `VITA-E2002` | `E-PARSE-UNEXPECTED-TOKEN` | Error | A token no grammar rule can continue: a missing `;`, a stray keyword, an unbalanced `begin`/`end`, a malformed expression. Every lexer error also carries this code. |
| `VITA-W2003` | `W-PARSE-IMPLICIT-NET` | Warning | An undeclared identifier is inferred as an implicit net under `` `default_nettype wire ``. |
| `VITA-W2004` | `W-PARSE-SELECT-BASE` | Warning | A bit or part select on an operand IEEE 1800 §11.5.1 does not allow one on. Other tools reject it. Emitted ahead of the parse-error gate, so a syntax error elsewhere cannot swallow it. |

Constructs the parser does not model surface here. Constructs that parse but
cannot be lowered surface as `E-ELAB-UNSUPPORTED` instead.

---

## `3xxx` — elaborate

Emitted while lowering the parsed design into the simulation IR: flattening,
connectivity, parameter resolution.

| Number | Mnemonic | Default | Meaning |
|---|---|---|---|
| `VITA-E3001` | `E-ELAB-MULTIDRIVER` | Error | Two drivers the engine cannot resolve land on one net or variable. See the detail below. |
| `VITA-E3002` | `E-ELAB-PORT-MISMATCH` | Error | An instance port connection is incompatible with the module's declared ports: a `.foo()` that is not a port, a wrong direction, or too many positional connections. |
| `VITA-E3003` | `E-ELAB-UNRESOLVED-INSTANCE` | Error | An instantiated module name resolves to no compiled design unit. Add the missing source, or fix the name. |
| `VITA-E3004` | `E-ELAB-USER-ERROR` | Error | An elaboration-time `$error`. Records and continues. |
| `VITA-F3005` | `F-ELAB-USER-FATAL` | Fatal | An elaboration-time `$fatal`. Emitted at `error` severity, not `fatal`, so the printed token differs from the registered default. |
| `VITA-I3006` | `I-ELAB-USER-INFO` | Info | An elaboration-time `$info`. |
| `VITA-W3007` | `W-ELAB-USER-WARNING` | Warning | An elaboration-time `$warning`. |
| `VITA-W3008` | `W-ELAB-WIDTH-TRUNC` | Warning | Reserved. Implicit width truncation or extension. The generic simplification channel is `W-ELAB-FEATURE-LIMIT`. |
| `VITA-W3011` | `W-ELAB-CASEZ-APPROX` | Warning | Reserved. A `casez` label bit written as an explicit `x` treated as a don't-care. |
| `VITA-E3009` | `E-ELAB-UNSUPPORTED` | Error | Elaborate cannot lower this construct faithfully, so it refuses rather than approximate. The widest code in the tool: it covers every construct outside the supported subset. See the detail below and [Limitations](006_limitations.md). |
| `VITA-E3010` | `E-ELAB-UNRESOLVED-NAME` | Error | A reference to a net or variable that is not declared in the enclosing scope, in an `assign`, an expression, or an lvalue. |
| `VITA-E3018` | `E-ELAB-LVALUE-KIND` | Error | A continuous `assign` targets a variable, or a procedural assignment targets a net. |
| `VITA-W3056` | `W-ELAB-FEATURE-LIMIT` | Warning | A legal construct is accepted but simplified: an unconnected port, an `inout` approximated as unidirectional, a dropped intra-assignment delay, a skipped system task. The IR is kept. |
| `VITA-W3057` | `W-ELAB-AUTOTOP-AMBIGUOUS` | Warning | The design has several uninstantiated roots and auto-top picked one. Pin the intended root with `--top`. |
| `VITA-W3058` | `W-ELAB-STR-TERNARY` | Warning | A ternary whose arms are string literals is an integral value, so `$display` prints it as a number rather than as text. |
| `VITA-W3059` | `W-ELAB-STR-ESCAPE` | Warning | A string literal uses an escape IEEE 1800 Table 5-1 does not define, and tools read it differently. One line per literal-and-escape pair for the whole run. |

### `E-ELAB-MULTIDRIVER` in detail

Two shapes reach this code.

A **net** driven by more than one continuous assignment is an error only when the
overlap is one the engine does not model. Whole-net, non-delayed continuous
assignments overlap legally: four-state wire resolution settles them, so they
elaborate cleanly. The error fires when the overlapping set contains a delayed
assignment, a multi-chunk lvalue, an array-element write, or a partial or
bit-select driver. Dynamic, non-constant selects are not counted at all, because
a false report on a disjoint dynamic split would reject legal code.

```systemverilog
module top;
  wire [3:0] w;
  assign     w = 4'h0;
  assign #1  w = 4'h1;     // one delayed driver puts the pair outside wire resolution
endmodule
```
```
error[VITA-E3001] E-ELAB-MULTIDRIVER: net `top.w` driven by multiple overlapping continuous assignments
```

A **variable** that has both a declaration initializer and an `always_comb` write
is two drivers on one variable under IEEE 1800 §9.2.2.2:

```
md3.sv:3:3: error[VITA-E3001] E-ELAB-MULTIDRIVER: variable `v` has a declaration initializer AND is written by `always_comb`, which is two drivers on one variable (IEEE §9.2.2.2) — drop the initializer or the `always_comb` write [in top]
```

`always_ff`, `always_latch`, plain `always` and `initial` are unaffected: a
register's declaration initializer is its power-on value. Both
`logic [7:0] c = 0; always_ff @(posedge clk) c <= c + 1;` and
`logic clk = 0; always #5 clk = ~clk;` are legal.

### The most common `E-ELAB-UNSUPPORTED`: a real value where an integer is required

IEEE 1800 requires an integral constant for a select index, a range bound and a
replication count (§11.5.1, §11.4.12.2). vitamin neither rounds such a value nor
reinterprets its bit pattern, because both produce a wrong answer with no error.
Instead:

- a real whose value is exactly an integer is converted and accepted, so
  `parameter real R = 4;` works anywhere an integer works;
- anything else is rejected.

```systemverilog
module m;
  parameter real R = 1.5;
  logic [7:0] v;
  initial v[R] = 1'b1;
endmodule
```
```
error[VITA-E3009] E-ELAB-UNSUPPORTED: a select index / bound / size must be integral, not real (IEEE §11.5.1)
```

The same code covers a real value used as a part-select bound; the offset or
width of an indexed part-select (`v[i +: n]`); an array index; a `new[N]` size; a
queue or associative-array index, `.exists()` key or `.delete()` key; a string
method argument; the address argument of `$readmemh`, `$readmemb`, `$writememh`,
`$writememb` or `$fread`; and a replication count. It fires for a value returned
by a function declared `real`, not only for a `real` parameter.

A second message names the width case:

```
error[VITA-E3009] E-ELAB-UNSUPPORTED: a real parameter is not an integral constant and cannot be used in a width / range bound (assign it to an integer localparam first)
```

The fix: if the quantity is conceptually an integer, declare it as one
(`parameter int N = 4;`). If it must stay real, convert at the point of use —
`v[$rtoi(R)]` or `v[int'(R)]`. An expression over a real parameter (`v[R+1]`) is
rejected even when the result would be integral; convert the whole expression
(`v[int'(R+1)]`).

A parameter override that folds to an integer is applied normally (`#(.R(3))`,
`#(.R(i+2))`). One that does not fold — a real expression, or a signal — is
rejected rather than leaving the child at its declared default:

```
error[VITA-E3009] E-ELAB-UNSUPPORTED: a parameter override that reads a real parameter is unsupported (a real has no integral constant value)
```

---

## `4xxx` — runtime

Emitted by the simulation engine while running the design. Every one of these
carries `[at time <ticks>]`, and most carry `file:line:col` and `[in <instance>]`
as well.

| Number | Mnemonic | Default | Meaning |
|---|---|---|---|
| `VITA-E4001` | `E-RUN-ASSERT-FAIL` | Error | Reserved. An `assert` with no action block failing as an implicit `$error`. |
| `VITA-E4002` | `E-RUN-RANGE` | Error | A runtime array index or bit/part-select is a known value outside the declared range. Per IEEE the read yields `x` and the write is dropped — the run does not crash — but the corruption is surfaced. Validate or clamp the index, or size the array correctly. |
| `VITA-E4003` | `E-RUN-USER-ERROR` | Error | A runtime `$error`. Prints and continues. |
| `VITA-F4004` | `F-RUN-FATAL` | Fatal | A runtime `$fatal`, which implies `$finish`. Also an engine capability limit reached mid-run: a `$fgets` or `$fscanf` inside a framed subroutine body, or a `$finish` or `$stop` actually reached inside a function or task body, where ending the run is preferred over choosing what the calling expression receives. See [Limitations](006_limitations.md). |
| `VITA-I4005` | `I-RUN-USER-INFO` | Info | A runtime `$info`. |
| `VITA-W4006` | `W-RUN-NO-LOCATIONS` | Warning | Reserved. A loaded snapshot carrying no location side table. |
| `VITA-W4007` | `W-RUN-USER-WARNING` | Warning | A runtime `$warning`. |
| `VITA-F4016` | `F-RUN-NO-CONVERGE` | Fatal | The delta-cycle limit is reached: a zero-delay loop or a combinational oscillation. |
| `VITA-W4018` | `W-RUN-VCD-OPEN-FAIL` | Warning | The waveform dump file cannot be opened. |
| `VITA-W4019` | `W-RUN-VCD-WRITE-FAIL` | Warning | A waveform write or flush failed, including the VCD-to-FST transcode. |
| `VITA-W4020` | `W-RUN-DYN-DEGRADE` | Warning | A dynamic-storage operation is degraded. |
| `VITA-W4021` | `W-RUN-DUMP-MULTI` | Warning | An extra `$dumpvars` call is ignored; the first call wins. |
| `VITA-W4022` | `W-RUN-BAD-FD` | Warning | A file operation names an invalid or closed descriptor and is ignored. |
| `VITA-W4023` | `W-RUN-READMEM` | Warning | A `$readmemb` or `$readmemh` problem — a missing file, or a word count that does not match the target — leaves the memory partially loaded. |
| `VITA-F4024` | `F-RUN-CLASS-LIMIT` | Fatal | The class-object budget is exceeded. The class heap is not garbage-collected, so an unbounded `new()` hits this instead of exhausting memory. |
| `VITA-W4025` | `W-RUN-WIDE-ARITH` | Warning | Multi-word arithmetic exceeds the width cap; the result is poisoned to X rather than silently wrong. |
| `VITA-W4026` | `W-RUN-VCD-PKGVAR-SKIP` | Warning | A package variable has no waveform surface and is excluded from the dump. |
| `VITA-F4027` | `F-RUN-BODY-STEP-LIMIT` | Fatal | One process ran past the body-step budget without suspending: an unbounded loop, or a genuinely long computation. |
| `VITA-W4028` | `W-RUN-PLUSARGS-INVALID` | Warning | A matched plusarg carries a value that will not parse; the target variable is written all-X. |
| `VITA-W4029` | `W-RUN-RANGE-UNKNOWN` | Warning | A runtime index or select is unknown (`x`/`z`) rather than a known value past the end. The recovery is the same — read `x`, drop the write — but IEEE 1364 §5.2.1 prescribes exactly that, so it is legal behaviour and not an error. Reading an array with a register that is still X before the first clock edge is the common case. |
| `VITA-W4030` | `W-RUN-BACKEND-FALLBACK` | Warning | The requested execution backend cannot run this design and a different one ran it. The answer is unaffected; the speed is. |
| `VITA-W4031` | `W-RUN-UNIQUE-VIOLATION` | Warning | A `unique` or `priority` `case` or `if` matched no branch, which IEEE 1800 §12.4.2 and §12.5.3 require to be reported. It has its own code so `-Wno-` and `-Werror=` can separate it from an RTL `$warning`. |

An empty-message `$error`, `$warning`, `$info` or `$fatal` prints the code's
registered title as its message.

---

## `8xxx` — filelist

Emitted while expanding `-f` and `-F` filelists. See the
[CLI Reference](004_cli-reference.md) for filelist syntax.

| Number | Mnemonic | Default | Meaning |
|---|---|---|---|
| `VITA-E8001` | `E-FLIST-CYCLE` | Error | A filelist includes itself directly or transitively. |
| `VITA-E8002` | `E-FLIST-DEPTH` | Error | Filelist nesting exceeds the depth cap. |
| `VITA-E8003` | `E-FLIST-DUP-CTX-CONFLICT` | Error | The same source appears twice under differing sticky context, such as different defines or include paths. |
| `VITA-E8004` | `E-FLIST-GLOB` | Error | A wildcard appears in a filelist path. Filelists name files, not patterns. |
| `VITA-E8005` | `E-FLIST-NOT-FOUND` | Error | A filelist, a path it references, or an artifact handed to a staged applet cannot be read. Paths are case-sensitive on Linux and macOS. Exits 3. |
| `VITA-E8006` | `E-FLIST-UNDEF-ENV` | Error | A filelist references an environment variable that is not set. |
| `VITA-E8007` | `E-FLIST-WRONG-STAGE` | Error | A filelist directive is not valid for the applet that was invoked. Exits 3. |
| `VITA-W8008` | `W-FLIST-MIXED-BASE` | Warning | A `-f` inside a `-F` frame re-anchors relative paths to the current working directory rather than to the frame. |
| `VITA-W8009` | `W-FLIST-OVERRIDE` | Warning | A single-value knob is set more than once; the last setting wins. Recorded during parsing and replayed through the severity gate at pipeline start. |
| `VITA-E8010` | `E-FLIST-UNTERMINATED-COMMENT` | Error | A `/*` in a filelist never closes, so every entry after it would be swallowed. The message names the filelist and the opening line. A `/*` inside a `//` comment is text and does not open a block. Exits 3. |

---

## `9xxx` — artifact and work library

Emitted by the staged pipeline when a `.vu` or `.velab` artifact does not match
the tool trying to consume it. These are the codes a stale downstream stage hits
after the source changes. Policy is refuse-and-rebuild; there is no silent
migration. Every one of them exits 2.

| Number | Mnemonic | Default | Meaning |
|---|---|---|---|
| `VITA-E9001` | `E-ART-FORMAT-MISMATCH` | Error | The artifact's magic bytes or `format_version` do not match this build — a foreign, truncated, or older-format file — or its header will not decode. Also covers a `.velab` trailer sidecar the IR requires but the file lacks, where the engine emits it at `fatal` severity rather than the registered `error`. Regenerate with `vcmp` / `velab`. |
| `VITA-E9002` | `E-ART-SCHEMA-MISMATCH` | Error | The artifact's structural `schema_hash` differs from this tool's: it was built against a different IR shape. Re-run the producing stage. |
| `VITA-E9003` | `E-ART-STALE-UPSTREAM` | Error | RULE V. An upstream input hashed differently than it did when the snapshot was taken — either the source named by `vrun --upstream`, or a work-library manifest, compilation-unit blob or raw source under the automatic work-library gate. Re-run `velab`, or drop `--upstream`. |
| `VITA-E9004` | `E-ART-VERSION-GATE` | Error | The producing tool's semver-major recorded in the artifact is incompatible with the consuming tool. Rebuild with a matching version. |
| `VITA-E9005` | `E-WORK-MANIFEST` | Error | A work-library manifest is invalid or unreadable. |

The header gate checks in order: format, then tool semver-major, then schema
hash.

---

## Codes with no emitter

Six of the 68 are registered and documented but produced by no code path. They
resolve in `vita explain`, `-Wno-` and `-Werror=`, and their numbers are
permanently reserved.

| Number | Mnemonic | Band |
|---|---|---|
| `VITA-F0002` | `F-LIMIT-ERRORS` | General |
| `VITA-W1003` | `W-LINT-UNCLOSED` | Preprocess |
| `VITA-W3008` | `W-ELAB-WIDTH-TRUNC` | Elaborate |
| `VITA-W3011` | `W-ELAB-CASEZ-APPROX` | Elaborate |
| `VITA-E4001` | `E-RUN-ASSERT-FAIL` | Runtime |
| `VITA-W4006` | `W-RUN-NO-LOCATIONS` | Runtime |

## Codes emitted at a severity other than their default

| Number | Mnemonic | Registered default | Emitted as |
|---|---|---|---|
| `VITA-F3005` | `F-ELAB-USER-FATAL` | Fatal | `error` |
| `VITA-E9001` | `E-ART-FORMAT-MISMATCH` | Error | `error` from the artifact header gate; `fatal` from the engine's missing-sidecar checks |

---

## See also

- [Installation](001_installation.md) — getting the tools onto Linux or macOS.
- [CLI Reference](004_cli-reference.md) — flags, filelists, and the staged
  `vcmp` → `velab` → `vrun` pipeline that produces the `9xxx` codes.
- [Language Reference](003_language-reference.md) — the `` `timescale ``
  section behind `W-PP-TIMESCALE-DEFAULT` and `W-PP-TIMESCALE-MIXED`.
- [System Tasks & Functions](005_system-tasks.md) — the severity tasks
  `$info`, `$warning`, `$error` and `$fatal` that raise the `*-USER-*` codes.
- [Limitations](006_limitations.md) — the fail-safe behaviours behind
  `E-ELAB-UNSUPPORTED`, `F-RUN-FATAL` and the runtime caps.
- [`docs/preview/15-error-code-reference.md`](../preview/15-error-code-reference.md)
  — the long-form catalogue `vita explain` prints: one full entry per code, with
  a worked example and a fix.
- [`docs/preview/13-diagnostics-and-logging.md`](../preview/13-diagnostics-and-logging.md)
  — the diagnostic and logging specification.
