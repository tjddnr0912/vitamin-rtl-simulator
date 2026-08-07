# 7 · Error & Warning Code Reference

Every diagnostic vitamin prints carries a **stable code** — a number like
`VITA-E3009` and a mnemonic like `E-ELAB-UNSUPPORTED`. This chapter is the
user-facing catalogue of the codes you are likely to hit, grouped by the stage
that emits them, with a one-line "what to do" for each.

For installing and running the tools, see [Installation](001_installation.md)
and the [CLI Reference](004_cli-reference.md) (which covers `vita` and the staged
`vcmp` → `velab` → `vrun` flow).

> Platform note: vitamin currently supports **Linux and macOS only**. Windows is
> not supported, and path/case behaviour described below assumes a POSIX
> filesystem.

---

## How a diagnostic looks

vitamin writes diagnostics to **stderr** in this shape:

```
<severity>[<CODE>]: <message>
```

…optionally prefixed with `file:line:col` when a source location is available:

```
m.sv:3:1: error[VITA-E2002]: unexpected token 'endmodule', expected expression
```

```
error[VITA-E3009]: a dynamic-storage handle has no whole-value surface (read elements or call methods)
```

- **Severity** is one of `note`, `info`, `warning`, `error`, `fatal`.
- The **code** shown in brackets is the grep-friendly number (`VITA-E3009`).
  Each number also has an equivalent **mnemonic** (`E-ELAB-UNSUPPORTED`) listed in
  the tables below. The two are interchangeable identifiers for the same
  diagnostic.

### Codes are stable identifiers

A code's meaning is fixed for the life of the tool (the spec calls this the
*doc-15 bijection*): a number is never renumbered or reused, and a mnemonic
never changes meaning. You can safely match on either in scripts, CI greps, or
issue reports — `VITA-E4002` will always mean "runtime index out of range".

### Exit codes

The numeric process exit code tells CI what kind of failure occurred:

| exit | meaning |
|------|---------|
| `0`  | clean — parse + elaborate succeeded, simulation finished with no errors |
| `1`  | user/design error — lex/parse errors, elaboration failed, runtime `$fatal` |
| `3`  | CLI/usage error — no source files, file not found, unknown applet/flag |

A single `error`-severity diagnostic anywhere in the run is enough to push the
exit code to `1`; `fatal` aborts the current stage immediately.

---

## Preprocess — `1xxx`

Emitted while expanding ``` `define ```/``` `include ```/``` `ifdef ```/
``` `timescale ``` before lexing.

| Code | Mnemonic | Sev | What triggers it | What to do |
|------|----------|-----|------------------|------------|
| `VITA-E1001` | `E-PP-INCLUDE-NOT-FOUND` | error | ``` `include "x.svh" ``` not found in the current dir or any include search path. | Add the header's directory to the search path, or fix the path/filename (paths are case-sensitive). |
| `VITA-E1002` | `E-PP-MACRO-ARITY` | error | A function-like macro ``` `define M(a,b) ``` called with the wrong number of arguments. | Match the call to the formal arg count, or fix the ``` `define ```. |
| `VITA-E1004` | `E-PP-RECURSIVE-MACRO` | error | A text macro re-invokes itself during its own expansion (infinite expansion). | Remove the self-reference from the macro body. |
| `VITA-E1005` | `E-PP-RECURSIVE-INCLUDE` | error | An ``` `include ``` chain is cyclic — a file includes itself directly or transitively. | Break the cycle, or use an include guard (``` `ifndef ``` / ``` `define ``` / ``` `endif ```). |
| `VITA-E1013` | `E-PP-BAD-DIRECTIVE` | error | A malformed preprocessor directive: unknown directive, use of an undefined macro, stray backtick, or an unbalanced/duplicate conditional. | Check the directive spelling, ``` `define ``` the macro first, or balance the ``` `ifdef ```/``` `endif ``` pair. |
| `VITA-W1007` | `W-PP-MACRO-REDEFINED` | warning | ``` `define ``` redefines an existing macro with **different** text. The new definition wins. | If unintended, rename the macro or ``` `undef ``` before redefining. (Identical re-definition is silent.) |
| `VITA-W1008` | `W-PP-UNDEF-UNDEFINED` | warning | ``` `undef ``` targets a name that was never defined. Harmless. | Check the macro name spelling, or only ``` `undef ``` after a ``` `define ```. |
| `VITA-W1017` | `W-PP-TIMESCALE-DEFAULT` | warning | No module in the design declares a ``` `timescale ```. vitamin locks the global time unit/precision to the base `1ns/1ns`. | Declare an explicit ``` `timescale 1ns/1ps ``` at the top of the file if you need different units. See the [Language Reference](003_language-reference.md). |

---

## Parse — `2xxx`

Emitted by the lexer/parser, the last language-dependent stage.

| Code | Mnemonic | Sev | What triggers it | What to do |
|------|----------|-----|------------------|------------|
| `VITA-E2002` | `E-PARSE-UNEXPECTED-TOKEN` | error | A token that no grammar rule can continue: a missing `;`, a stray keyword, unbalanced `begin`/`end`, a malformed expression. | Fix the syntax at the caret. A `2005`-vs-SystemVerilog dialect mismatch can also make a valid token "unexpected". |

A clean Verilog/SystemVerilog RTL subset is accepted; constructs the parser
does not yet model surface here or, if they parse but cannot be elaborated, as
`E-ELAB-UNSUPPORTED` below.

---

## Elaborate — `3xxx`

Emitted while lowering the parsed design into the simulation IR (flattening,
connectivity, parameter resolution).

| Code | Mnemonic | Sev | What triggers it | What to do |
|------|----------|-----|------------------|------------|
| `VITA-E3001` | `E-ELAB-MULTIDRIVER` | error | A net (or bit range) is driven by more than one structural driver — multiple `assign`s, output ports, or gates onto the same wire. | Reduce to a single driver, or make the intended wired-logic explicit. |
| `VITA-E3002` | `E-ELAB-PORT-MISMATCH` | error | An instance port connection is incompatible with the module's declared ports — a `.foo()` that isn't a port, or too many positional connections. | Fix the connection to a declared port (name / positional count / direction). Leave a port unconnected with `.z()`. |
| `VITA-E3003` | `E-ELAB-UNRESOLVED-INSTANCE` | error | An instantiated module cannot be resolved to a compiled design unit. | Add the missing module's source to the [`vcmp`](004_cli-reference.md) input, or fix a module-name typo. |
| `VITA-E3010` | `E-ELAB-UNRESOLVED-NAME` | error | A reference to a net/variable that is not declared in scope (in an `assign`, an expression, or an lvalue). | Add the missing `wire`/`reg`/`logic` declaration, or fix the typo. |
| `VITA-E3009` | `E-ELAB-UNSUPPORTED` | error | A construct the elaborator does not support is encountered; elaboration stops. Most often a **real value where the language requires an integer** — see below. | Make the value integral (declare it `int`, or convert with `$rtoi()`/`int'()`), or rework the construct. |
| `VITA-W3008` | `W-ELAB-WIDTH-TRUNC` | warning | A width mismatch is resolved by implicit truncation/extension (e.g. assigning an 8-bit value to a 4-bit target loses the top bits). | If the truncation is intended, make it explicit (`wide[3:0]`); otherwise match the widths. |
| `VITA-W3011` | `W-ELAB-CASEZ-APPROX` | warning | A `casez` label contains an explicit `x` bit. vitamin's v1 matcher treats that `x` as a don't-care like `z`, which is looser than strict `casez`. | If you mean don't-care, write `?`/`z` (no warning). Only `casez` labels with explicit `x` trigger this; `casex` is exact by definition. |

### The most common `E3009`: a real value where an integer is required

IEEE 1800 requires an **integral constant** for a select index, a range bound, and
a replication count (§11.5.1, §11.4.12.2). vitamin will not quietly round such a
value or reinterpret its bit pattern as an integer, because both produce a wrong
answer with no error. Instead:

- a real whose value is **exactly an integer** is converted and accepted, so
  `parameter real R = 4;` works anywhere an integer works;
- anything else is rejected with `E3009`.

```systemverilog
module m;
  parameter real R = 1.5;
  logic [7:0] v;
  initial v[R] = 1'b1;
endmodule
```
```
error[VITA-E3009]: a select index / bound / size must be integral, not real (IEEE §11.5.1)
```

The same code appears for a real value used as: a part-select bound; the offset or
width of an indexed part-select (`v[i +: n]`); an array index; a `new[N]` size; a
queue or associative-array index, `.exists()` key or `.delete()` key; a string
method argument; the address argument of `$readmemh`/`$readmemb`/`$writememh`/
`$writememb`/`$fread`; or a replication count. It also fires when the value comes
from a **function declared to return `real`**, not just from a `real` parameter.

A related message names the width case specifically:

```
error[VITA-E3009]: a real parameter is not an integral constant and cannot be used
in a width / range bound (assign it to an integer localparam first)
```

**What to do.** If the quantity is conceptually an integer, declare it as one
(`parameter int N = 4;`). If it must stay real, convert explicitly at the point of
use — `v[$rtoi(R)]` or `v[int'(R)]`. Note that an *expression* over a real
parameter (`v[R+1]`) is rejected even when the result would be integral; convert
the whole expression instead (`v[int'(R+1)]`).

**Overriding a `real` parameter.** An override that folds to an integer is applied
normally (`#(.R(3))`, `#(.R(i+2))`). One that does not fold — a real expression, or
a signal — is rejected rather than silently leaving the child at its declared
default:

```
error[VITA-E3009]: a parameter override that reads a real parameter is unsupported
(a real has no integral constant value)
```

### Elaboration-time `$error` / `$fatal` / `$warning` / `$info`

These codes exist as stable identifiers but are **not yet emitted** in the
current release (elaboration-time severity tasks are a later milestone). They
are listed so the identifiers are reserved and recognisable.

| Code | Mnemonic | Sev | Meaning |
|------|----------|-----|---------|
| `VITA-E3004` | `E-ELAB-USER-ERROR` | error | An elaboration-time `$error` fired (records and continues). |
| `VITA-F3005` | `F-ELAB-USER-FATAL` | fatal | An elaboration-time `$fatal` fired (aborts, no snapshot). |
| `VITA-W3007` | `W-ELAB-USER-WARNING` | warning | An elaboration-time `$warning` fired. |
| `VITA-I3006` | `I-ELAB-USER-INFO` | info | An elaboration-time `$info` reported information. |

---

## Runtime — `4xxx`

Emitted by the simulation engine while running the design.

| Code | Mnemonic | Sev | What triggers it | What to do |
|------|----------|-----|------------------|------------|
| `VITA-E4002` | `E-RUN-RANGE` | error | A runtime array index or bit/part-select is out of the declared range. Per IEEE, the read yields `x` and the write is ignored — the simulator does **not** crash — but the corruption is surfaced as this error. | Validate/clamp the index before the select, or size the array correctly. If the location reads `(source location unavailable)`, see `W-RUN-NO-LOCATIONS` below. |
| `VITA-W4029` | `W-RUN-RANGE-UNKNOWN` | warning | The runtime index was UNKNOWN (x/z) rather than a known value past the end. Same recovery (read `x`, write dropped) — but IEEE 1364 §5.2.1 prescribes exactly that, so it is legal behaviour and not an error. Reading an array with a register that is still X before the first clock edge is the common case. | Nothing, if the reset window is expected. Promote it with `-Werror=W-RUN-RANGE-UNKNOWN`, or suppress with `-Wno-W-RUN-RANGE-UNKNOWN`. |

### Runtime severity tasks & assertions (reserved)

The following runtime codes have stable identifiers but are **not yet emitted**
in the current release. They are reserved for the RTL severity-task and
assertion work:

| Code | Mnemonic | Sev | Meaning |
|------|----------|-----|---------|
| `VITA-E4001` | `E-RUN-ASSERT-FAIL` | error | An `assert` with no action block failed (implicit `$error`). |
| `VITA-E4003` | `E-RUN-USER-ERROR` | error | A runtime `$error` fired (prints and continues). |
| `VITA-F4004` | `F-RUN-FATAL` | fatal | A runtime `$fatal` fired — implicit `$finish`, exit `1`. Also an engine capability limit reached while running (e.g. a `$fgets`/`$fscanf` inside a framed subroutine body): vitamin stops rather than return a quiet 0. |
| `VITA-W4007` | `W-RUN-USER-WARNING` | warning | A runtime `$warning` fired. |
| `VITA-I4005` | `I-RUN-USER-INFO` | info | A runtime `$info` reported information. |
| `VITA-W4006` | `W-RUN-NO-LOCATIONS` | warning | The loaded snapshot has no location side-table, so runtime diagnostics omit `file:line`. Re-elaborate with locations to restore them. |

(Plain `$display`/`$write` output is not a diagnostic and carries no code — it
goes straight to stdout.)

---

## Artifact / staleness — `9xxx`

Emitted by the staged pipeline (`vcmp` → `velab` → `vrun`; see the
[CLI Reference](004_cli-reference.md)) when a `.vu`/`.velab` artifact does not match the tool
trying to consume it. These are the codes you hit after changing the source and
re-running a *downstream* stage against a *stale* upstream artifact.

| Code | Mnemonic | Sev | What triggers it | What to do |
|------|----------|-----|------------------|------------|
| `VITA-E9001` | `E-ART-FORMAT-MISMATCH` | error | The artifact's magic bytes or `format_version` don't match this build — a foreign, corrupt, or older-format file. | Regenerate it with the current tool (`vcmp` / `velab`). Artifacts are always reproducible; there is no silent migration. |
| `VITA-E9002` | `E-ART-SCHEMA-MISMATCH` | error | The artifact's structural `schema_hash` differs from this tool's — it was built by an incompatible IR shape. | Re-run the producing stage (`velab`, after `vcmp` if needed) with the current tool. |
| `VITA-E9004` | `E-ART-VERSION-GATE` | error | The producing tool's semver-major recorded in the artifact is incompatible with the consuming tool. | Rebuild the artifact with a matching tool version. |

> `VITA-E9003` / `E-ART-STALE-UPSTREAM` — the live source re-hash gate (RULE V),
> which rejects a snapshot whose upstream `.sv` changed on disk — is a reserved
> identifier and **not yet wired** in the current release. Today, the
> `schema_hash` + `format_version` gate above (`E-ART-*-MISMATCH`) is what
> catches the common staleness cases.

---

## CLI / system — `0xxx` and filelist — `8xxx`

These come from the driver and argument handling rather than your RTL.

| Code | Mnemonic | Sev | What triggers it | What to do |
|------|----------|-----|------------------|------------|
| `VITA-E0001` | `E-CLI-BAD-FLAG` | error | An unknown or invalid command-line flag/value (e.g. a typo'd flag). Exit `3`. | Fix the spelling/value per the CLI chapter for the applet you ran. |
| `VITA-E8005` | `E-FLIST-NOT-FOUND` | error | A filelist (`-f`) or a path it references does not exist (case-sensitive). Exit `3`. | Fix the path or its base directory; paths are case-sensitive on Linux and macOS. |

A broader set of filelist diagnostics (`E-FLIST-CYCLE`, `E-FLIST-GLOB`,
`E-FLIST-DUP-CTX-CONFLICT`, `E-FLIST-UNDEF-ENV`, `E-FLIST-WRONG-STAGE`,
`W-FLIST-MIXED-BASE`, `W-FLIST-OVERRIDE`) and the error-limit fatal
(`F-LIMIT-ERRORS`, `VITA-F0002`) share the `0xxx`/`8xxx` bands. They become
reachable as the corresponding filelist features land; their identifiers are
already reserved and stable.

---

## See also

- [Installation](001_installation.md) — getting the tools onto Linux/macOS.
- [CLI Reference](004_cli-reference.md) — the staged `vcmp` → `velab` → `vrun`
  pipeline that produces the `9xxx` artifact codes.
- [Language Reference](003_language-reference.md) — the `` `timescale `` section,
  context for `W-PP-TIMESCALE-DEFAULT`.
- [Limitations](006_limitations.md) — fail-safe behaviours behind some warnings.
