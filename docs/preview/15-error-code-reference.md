# 15 · Error Code Reference

The single authoritative catalogue of every diagnostic vitamin can print: cause, worked
example and fix for each code. This file is also a deliverable of the implementation —
`crates/cli/src/lib.rs` embeds it with `include_str!`, and `vita explain <CODE>` prints one
entry from it verbatim. Severity semantics, gating and exit classes are specified in
[13-diagnostics-and-logging.md](13-diagnostics-and-logging.md); this file is the catalogue.

## Governance

- This document is not generated from code. It is a hand-written explanation of causes,
  kept 1:1 with the exhaustive `MsgCode` enum in the `diag` crate.
- CI sync gate: `crates/diag/tests/bijection.rs` asserts that every `MsgCode` variant has an
  entry here and that every code here exists in the enum, with matching number and declared
  severity. Adding a diagnostic without adding its entry fails the build. That gate is the
  mechanism that keeps this catalogue synchronised.
  - The gate covers exactly the codes registered in the enum, which is the set of full
    entries in the body sections below. The reserved codes in Appendix A are not enum
    variants and are excluded. Promoting a reserved code means adding the enum variant and
    the body entry in the same change; promotion is what puts it under the gate.
- The mnemonic is the primary stable identifier (`E-ELAB-MULTIDRIVER`). It is fixed by
  meaning and never renumbered. CLI flags (`-Wno-`/`-Werror=`), the corpus (`expect_codes`)
  and prose all refer to codes by mnemonic, not by number.
- The `VITA-<S>####` number is a secondary, grep-friendly handle. A number, once assigned, is
  permanent: gaps stay gaps, and numbers are never reused or renumbered (the rustc `E0001`
  convention). The severity letter `S` is `E`=Error, `W`=Warning, `I`=Info, `F`=Fatal. New
  codes take the next free number in their band; the band is not re-sorted alphabetically,
  so neither the enum table nor this file is in numeric order overall.
- `MsgCode::resolve` accepts three spellings of a code, case-insensitively, everywhere a code
  is typed — `vita explain`, `-Wno-`, `-Werror=`:

  | form | example |
  |---|---|
  | mnemonic | `W-ELAB-FEATURE-LIMIT` |
  | printed number | `VITA-W3056` |
  | number without the prefix | `W3056` |

  An unresolvable spelling is a hard CLI usage error (`VITA-E0001`, exit 3), never a silently
  ignored flag.
- Suppression and promotion: `-Wno-<CODE>` drops a Warning, Info or Note before it reaches
  the sink; `-Werror` promotes every Warning and `-Werror=<CODE>` promotes one. Error and
  Fatal are the always-logged spine — `-Wno-` on them resolves and has no effect. A promoted
  diagnostic keeps its original code number and only changes its severity token, so
  `error[VITA-W4007] W-RUN-USER-WARNING: …` is the expected shape under `-Werror`. Info and
  Note are suppressible but not promotable.

## Number bands

| Band | Category | Stage |
|---|---|---|
| `0xxx` | GENERAL / SYSTEM | CLI and usage |
| `1xxx` | PREPROCESS | `` `include ``, macros, directives |
| `2xxx` | PARSE | lexing, syntax, design units |
| `3xxx` | ELABORATE | parameters, hierarchy, connectivity, elaboration severity tasks |
| `4xxx` | RUNTIME | simulation, RTL severity tasks, engine limits |
| `5xxx` | ASSERTION / SVA | reserved band, no enum variants (see below) |
| `6xxx` | SV-TYPE | reserved band, no enum variants (see below) |
| `7xxx` | VHDL | reserved band, no enum variants |
| `8xxx` | FILELIST | `.f` expansion (`-f` / `-F`) |
| `9xxx` | ARTIFACT | artifact staleness and version gates |

Two of the reserved bands describe conditions that are already handled under other codes:

- `5xxx` — SVA is implemented (sequences, property operators, `cover property`, deferred
  `assert #0` / `assert final`, multi-clock, named property and sequence). A failing
  assertion reports through the synthesised checker's `$error`, which is `VITA-E4003`. What
  is unassigned is the 5xxx *code*, not the feature.
- `6xxx` — the SV data-type features are implemented: `enum` / `typedef` / packed `struct`,
  dynamic arrays, queues, associative arrays, `string`, and class/OOP with inheritance and
  virtual dispatch, CRV and `$cast`. Type-rule violations report through `E-ELAB-UNSUPPORTED`
  or the runtime degrade codes. Again the *code* is unassigned, not the feature.

## Status at HEAD

Registration in this catalogue guarantees that a code resolves, explains, and can be gated.
It does not guarantee that anything emits it. Three sets of facts are stated per entry as
well, and collected here:

Codes with no emitter anywhere in the tree:

| Code | Mnemonic | Why nothing raises it |
|---|---|---|
| `VITA-F0002` | `F-LIMIT-ERRORS` | there is no `--error-limit` flag; the three internal caps behave differently (see the entry) |
| `VITA-W1003` | `W-LINT-UNCLOSED` | no `lint_off` pragma exists; suppression is `-Wno-` only |
| `VITA-W3008` | `W-ELAB-WIDTH-TRUNC` | the generic elaborate simplification channel reports `W-ELAB-FEATURE-LIMIT` |
| `VITA-W3011` | `W-ELAB-CASEZ-APPROX` | `casez`/`casex` lower to exact `CasezEq`/`CasexEq`; nothing approximates |
| `VITA-E4001` | `E-RUN-ASSERT-FAIL` | a failing assertion reports through its implicit `$error`, i.e. `VITA-E4003` |
| `VITA-W4006` | `W-RUN-NO-LOCATIONS` | nothing strips the location side-table from a snapshot |

Codes emitted at a severity other than their declared default:

| Code | Declared | Emitted | Where |
|---|---|---|---|
| `VITA-F3005` | Fatal | Error | the elaboration `$fatal` arm routes through the shared error path |
| `VITA-E9001` | Error | Fatal | the two runtime `.velab` sidecar guards raise it as Fatal |

A code's severity is chosen by its emitter, not by the code. `default_severity()` is read in
exactly two places: this file's bijection gate, and the `explain` fallback headline.

Codes whose emitter exists but which no current design reaches: `W-RUN-BACKEND-FALLBACK`
(`VITA-W4030`) — see its entry.

---

## 0xxx · GENERAL / SYSTEM

### VITA-E0001 · `E-CLI-BAD-FLAG` (Error)
**An unknown or invalid command-line flag or value.** The argument parser met a flag it does
not know, a value of the wrong form or range, or a flag the invoked stage does not accept.
A misspelled flag (`--timescal`) that silently did nothing would produce a subtly wrong
simulation, so the run fails loudly before compiling anything.
```
$ vita --bogusflag design.sv
error[VITA-E0001]: unknown flag '--bogusflag'
```
This code is also the carrier for every usage error printed straight to stderr — a missing
flag argument, an unreadable input, an unknown diagnostic code in `-Wno-`/`-Werror=`, an
output path that would overwrite an input. Those lines are raw: they print
`error[VITA-E0001]: <message>` with no mnemonic, they bypass the gate, and they are not
counted in the epilogue, because argv has not been parsed yet when they fire.

**Fix:** correct the spelling or the value, or pass the flag to the stage that owns it
(see [../manual/004_cli-reference.md](../manual/004_cli-reference.md)). Not suppressible;
exit class 3.

### VITA-F0002 · `F-LIMIT-ERRORS` (Fatal)
**An error-count limit was reached and the stage aborted.** Reserved for a
user-configurable error budget: past the budget, a badly broken file stops cascading
thousands of follow-on lines and the stage ends.

**Status at HEAD: no emitter.** There is no `--error-limit` flag. Three internal caps exist
instead, and none of them reports through this code:

| Cap | Value | Behaviour on reaching it |
|---|---|---|
| parser error cap | 50 | further parse errors are not recorded; no diagnostic marks the cap |
| elaborate error cap | 200 | one `Severity::Error` `VITA-E3009` with `too many elaborate errors; further diagnostics suppressed (cap 200)`; later errors and their notes are dropped |
| runtime index-report cap | 8, with separate budgets for known and unknown indices | report 8 becomes `further out-of-range diagnostics suppressed` / `further unknown-index diagnostics suppressed`; reports 9 and later print nothing |

**Fix:** fix the earliest error first — the rest are usually cascade. Not suppressible;
exit class 1.

---

## 1xxx · PREPROCESS

### VITA-E1001 · `E-PP-INCLUDE-NOT-FOUND` (Error)
**An `` `include `` target was not found on the search path.** The preprocessor looked in the
including file's directory and in every `+incdir+` / `-I` directory and found nothing. Text
substitution happens at preprocess time, so a missing target means there are no bytes to
splice in (IEEE 1364-2005 §19.5, IEEE 1800-2017 §22.4). The include stack is attached as
frames so the origin is visible.
```
$ vita design.sv
design.sv:1:19: error[VITA-E1001] E-PP-INCLUDE-NOT-FOUND: `include "nope.svh" not found on search path
```
**Fix:** add the header directory with `+incdir+<dir>`, or correct the path or file name
(paths are case-sensitive on every platform vita supports). Inside a filelist tree,
`--dump-filelist` shows what actually reached the stage. Not suppressible.

### VITA-E1002 · `E-PP-MACRO-ARITY` (Error)
**A function-like macro was called with the wrong number of arguments.** A
`` `define NAME(a,b,…) `` macro was expanded with an actual count that does not match the
formals and cannot be filled from defaults (IEEE 1364-2005 §19.3.1, IEEE 1800-2017 §22.5.1).
Both the call site and the definition site are attached.
```
`define MAX(a,b) ((a)>(b)?(a):(b))
module m; wire y = `MAX(1); endmodule
->  m.sv:3:20: error[VITA-E1002] E-PP-MACRO-ARITY: macro `MAX expects 2 argument(s), got 1
    — formal `b` has no default
```
**Fix:** pass the formals the macro declares, or change the `` `define ``. A `(` after an
object-like macro is literal text, not a call, and is not arity-checked. Not suppressible.

### VITA-W1003 · `W-LINT-UNCLOSED` (Warning)
**An inline `lint_off` pragma reached end of file without its matching `lint_on`.** An
unclosed suppression region would silently swallow diagnostics for the whole remainder of a
file, which is nearly always an editing mistake rather than an intent.

**Status at HEAD: no emitter.** No inline lint pragma exists anywhere in the tree; the only
suppression surface is `-Wno-<CODE>` and `-Werror[=<CODE>]` on the command line, which is
scoped to the whole run and cannot be left unclosed.

**Fix:** suppress with `-Wno-<CODE>` rather than an inline region. Suppressible with
`-Wno-W-LINT-UNCLOSED` if it ever fires.

### VITA-E1004 · `E-PP-RECURSIVE-MACRO` (Error)
**A text macro re-entered its own expansion.** The preprocessor tracks the set of
macros currently expanding; a use of a macro already in that set would expand forever, so the
use is left as literal text and this error is reported.
```
`define A `A
module m; wire y = `A; endmodule
->  m.sv:3:20: error[VITA-E1004] E-PP-RECURSIVE-MACRO: recursive expansion of macro `A
```
**Fix:** remove the self-reference from the macro body, or write the fully expanded form
instead of a recursive one. Not suppressible.

### VITA-E1005 · `E-PP-RECURSIVE-INCLUDE` (Error)
**An `` `include `` chain is cyclic.** A file tried to include one that is already open on
the include stack. Detection is on the canonical path stack, and the re-inclusion is
skipped rather than followed.
```
// a.svh:  `include "b.svh"
// b.svh:  `include "a.svh"
->  top.sv:1:19: error[VITA-E1005] E-PP-RECURSIVE-INCLUDE: cyclic `include of "a.svh"
```
**Fix:** break the cycle, or guard the headers with `` `ifndef ``/`` `define ``/`` `endif ``.
Not suppressible.

### VITA-W1007 · `W-PP-MACRO-REDEFINED` (Warning)
**A `` `define `` replaced an existing macro with different text or different parameters.**
The new definition takes effect and the run continues. Redefining a macro with identical
text is silent.
```
`define W 1
`define W 2
->  m.sv:1:19: warning[VITA-W1007] W-PP-MACRO-REDEFINED: macro `W redefined with different text
```
**Fix:** if the redefinition was not intended, rename one of the macros, or `` `undef `` the
name before redefining it. Suppress with `-Wno-W-PP-MACRO-REDEFINED`, promote with
`-Werror=`.

### VITA-W1008 · `W-PP-UNDEF-UNDEFINED` (Warning)
**An `` `undef `` names a macro that is not currently defined.** The operation is harmless;
the warning exists because the usual cause is a typo in the macro name.
```
`undef NEVER
->  m.sv:1:19: warning[VITA-W1008] W-PP-UNDEF-UNDEFINED: `undef of macro `NEVER that was never defined
```
**Fix:** check the spelling, or `` `undef `` only where the macro is known to be defined.
Suppress with `-Wno-W-PP-UNDEF-UNDEFINED`, promote with `-Werror=`.

### VITA-E1013 · `E-PP-BAD-DIRECTIVE` (Error)
**A malformed or unrecognised preprocessor construct.** This is the general preprocess-form
error: an unknown compiler directive, use of an undefined macro, a stray backtick, a
directive in a position where it is not allowed, an unbalanced or duplicated conditional, a
non-literal `` `include `` argument, and `` `undef `` of a directive name.
```
`frobnicate
->  m.sv:1:19: error[VITA-E1013] E-PP-BAD-DIRECTIVE: undefined macro use `frobnicate
```
```
`endif             // no conditional is open
`UNDEFINED_MACRO   // never `defined
```
**Fix:** check the directive spelling, `` `define `` the macro before using it, or balance
the conditional block. Not suppressible.

### VITA-W1017 · `W-PP-TIMESCALE-DEFAULT` (Warning)
**No module in the design declares a `` `timescale `` and no `--timescale` was given.** The
global time unit and precision lock to the `1ns/1ns` base and the run continues (see
[08-timescale-and-timing.md](08-timescale-and-timing.md)). The base is a constant, independent
of OS and compile order, so determinism is unaffected.
```
module top;             // no `timescale anywhere
  initial #2.5 $finish; // 2.5 ns rounded to the 1 ns grid = 3
endmodule
->  warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
```
**Fix:** declare the unit and precision you mean at the top of the file, for example
`` `timescale 1ns/1ps ``. Suppress with `-Wno-W-PP-TIMESCALE-DEFAULT`, promote with
`-Werror=`.

### VITA-W1018 · `W-PP-TIMESCALE-MIXED` (Warning)
**Some modules in the design carry a `` `timescale `` and others do not.** IEEE 1800-2017
§3.14.2.2 requires all or none. The message names the ungoverned modules — up to eight, then
`(and N more)` — because "somewhere in ten files" is not actionable.
```
$ vita leaf.sv top.sv         # only top.sv has a `timescale
warning[VITA-W1018] W-PP-TIMESCALE-MIXED: some modules have a `timescale and these do not:
  leaf — IEEE 1800 §3.14.2.2 requires all or none, and other tools refuse to elaborate the
  mixed form (they take the 1ns/1ns base here)
```
vita and iverilog both run the mixed design, with the ungoverned modules on the `1ns/1ns`
base. Xcelium refuses to elaborate it (`*F,CUMSTS: Timescale directive missing on one or more
modules`) and Verilator reports `Error-TIMESCALEMOD`, so a design that is green here can fail
at sign-off. It is a warning rather than an error because the simulation itself is correct.

**Fix:** give every module a `` `timescale ``, or none. Promote with
`-Werror=W-PP-TIMESCALE-MIXED` in CI; suppress with `-Wno-`.

---

## 2xxx · PARSE

### VITA-E2001 · `E-DUP-UNIT` (Error)
**A design unit (module or package) is defined more than once.** The same unit name appears
twice in the analysed sources — a source file listed twice in a filelist, or two files each
declaring `module m`. A logical library cannot hold two units under one `library:unit` key.
Sources are not deduplicated by default: sticky directive inheritance means two occurrences
of the same path can carry different context, so silent dedup would drop a genuinely
different input.
```
module adder; endmodule
module adder; endmodule
->  error[VITA-E2001] E-DUP-UNIT: module `adder` declared 2 times
```
**Fix:** remove the duplicate source entry or rename one unit. `--dump-filelist` shows the
flattened order. The same canonical path listed twice under *differing* sticky context is a
different code, `E-FLIST-DUP-CTX-CONFLICT`. Not suppressible.

### VITA-E2002 · `E-PARSE-UNEXPECTED-TOKEN` (Error)
**A token that no valid grammar production can continue.** A missing `;`, a wrong keyword,
unbalanced `begin`/`end`, a malformed expression. Parse is the last language-dependent stage,
and every token carries file, line and column, so the diagnostic points at the exact source
position. The lexer's own errors — unterminated string, unterminated block comment, bad
number literal and the rest — also report under this code.
```
module m;
  assign y = a &        // missing operand and ';'
endmodule
->  m.sv:4:1: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected expression, found keyword 'endmodule'
    m.sv:4:1: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected ';', found keyword 'endmodule'
```
When the token text cannot be recovered, the message is just `expected <what>` — the tail is
dropped rather than guessed.

**Fix:** correct the syntax at that position. Check that the dialect matches the file: a
2005-versus-SystemVerilog mismatch can make a valid SystemVerilog token unexpected. Not
suppressible.

### VITA-W2003 · `W-PARSE-IMPLICIT-NET` (Warning)
**An undeclared identifier was inferred as an implicit net under `` `default_nettype wire ``.**
IEEE 1364-2005 §3.5 creates a 1-bit net for an undeclared identifier in specific positions.
The classic bug is a typo (`enabel`) quietly becoming a new wire, so the inference is
reported. The effective `` `default_nettype `` is sticky and inherited across files in
compile order, so whether this fires can depend on compile order.
```
assign enabel = a & b;   // 'enabel' was never declared
->  m.sv:1:8: warning[VITA-W2003] W-PARSE-IMPLICIT-NET: implicit net `m.enabel` inferred as a
    1-bit wire (IEEE 1364-2005 §3.5); declare it explicitly, or use ``default_nettype none``
    to make this an error
```
Only two positions infer a net. Everywhere else an undeclared name is `VITA-E3010`:

| Position | Behaviour |
|---|---|
| gate or module instance terminal list | implicit 1-bit wire, `VITA-W2003` |
| continuous-assign LHS | implicit 1-bit wire, `VITA-W2003` |
| ordinary RHS (`assign y = TYPO`) | `VITA-E3010` |
| procedural lvalue (`initial TYPO = 1`) | `VITA-E3010` |
| any position under `` `default_nettype none `` | `VITA-E3010` |
| `.name` shorthand (IEEE 1800-2017 §23.3.2.2) | `VITA-E3010` — it requires a declared object |
| an interface instance used as an actual | no diagnostic; it is a declaration, not a net |

The inferred net is scalar, as §3.5 mandates, so a wider driver keeps only bit 0. Every
simulator does that silently; vita additionally reports the discarded width as
`VITA-W3056`.

**Fix:** declare the net, or fix the typo. Production RTL should use
`` `default_nettype none ``. Suppress with `-Wno-W-PARSE-IMPLICIT-NET`; restore the strict
behaviour with `-Werror=W-PARSE-IMPLICIT-NET`.

### VITA-W2004 · `W-PARSE-SELECT-BASE` (Warning)
**A bit or part select applies to an expression rather than to a net or variable.**
IEEE 1800-2017 §11.5.1 allows a select only on a variable reference — a name, possibly
indexed or member-selected. vita accepts a select on any primary, and that acceptance is a
vita extension. The value is not changed; only this warning is added, because changing the
value where the oracles disagree would quietly break a design that runs today.

| Form | vita | iverilog 13 | Verilator 5.050 |
|---|---|---|---|
| `((a^b)>>8)[7:0]` | runs | rejects | rejects |
| `16'hABCD[7:0]` | runs | rejects | rejects |
| `f(a)[7:0]` | runs | rejects | runs, same value |
| `{a,b}[7:0]` | runs | rejects | runs, same value |

```
assign o = ((a ^ b) >> 8)[7:0];
->  m.sv:2:14: warning[VITA-W2004] W-PARSE-SELECT-BASE: a bit/part select here applies to an
    expression, not to a net or variable — IEEE 1800-2017 §11.5.1 allows one only on a
    variable reference (a name, possibly indexed or member-selected). vita accepts it;
    iverilog rejects every form of it and Verilator rejects a select on a parenthesised
    expression or on a literal. Assign the value to a variable first, then select from that
```
Not warned, because both oracles accept all of them: `a[7:0]`; `pk::W[7:0]`, a package-scoped
name; `p.hi[3:0]`, a packed-struct member, which the parser desugars to a part-select so the
AST cannot distinguish it — the discriminator is provenance, whether the chain started at a
name; and `m[1][3:0]`, an array element. One form is accepted without a warning that iverilog
rejects: `a[7:0][3:0]`, slice-of-slice, which starts at a name and so passes this check.

**Fix:** assign the value to a variable first and select from that
(`logic [15:0] t = (a^b)>>8; o = t[7:0];`). Suppress with `-Wno-W-PARSE-SELECT-BASE`, promote
with `-Werror=`.

---

## 3xxx · ELABORATE

### VITA-E3001 · `E-ELAB-MULTIDRIVER` (Error)
**Two drivers the engine cannot resolve.** Two distinct conditions share this code.

**Overlapping continuous-assign bit ranges.** Part-select continuous assignments to one net
overlap and the overlap cannot be resolved. Whole-net multiple drivers
(`assign w = a; assign w = b;`) are legal 4-state wired-logic resolution under IEEE 1800-2017
§6.6 and do *not* raise this code — the value is always resolved. A net keeps this check
when any of its drivers is delayed, multi-chunk, an array element, or otherwise outside the
resolved set. Dynamic (non-constant) offsets are not counted, so a disjoint dynamic split is
not falsely rejected.
```
wire [7:0] w;  assign w[3:0] = a;  assign w[5:2] = b;    // [3:2] overlaps
->  error[VITA-E3001] E-ELAB-MULTIDRIVER: net `m.w` driven by multiple overlapping continuous assignments
```

**A declaration initializer on a variable that `always_comb` writes.** IEEE 1800-2017
§9.2.2.2 says no other process may drive a variable an `always_comb` drives, and a
declaration initializer is such a driver.
```
logic rdy = 1'b1;                    // driver 1
always_comb rdy = (cnt < 8'd128);    // driver 2
->  m.sv:2:3: error[VITA-E3001] E-ELAB-MULTIDRIVER: variable `rdy` has a declaration
    initializer AND is written by `always_comb`, which is two drivers on one variable
    (IEEE §9.2.2.2) — drop the initializer or the `always_comb` write
```
The rule is scoped to `always_comb` only. Measured against Verilator 5.050 with
`--lint-only -Wall`, one file per block kind:

| Declaration initializer plus | Verilator |
|---|---|
| `always_comb` | `MULTIDRIVEN`, citing IEEE 1800-2023 §9.2.2.2 |
| `always_ff` | `PROCASSINIT` only, a style note |
| `always_latch` | `PROCASSINIT` only |

iverilog is silent on all three. The split is the purpose of the clause, not a Verilator
omission: `always_comb` models combinational logic, so its output must always be a function
of its inputs and any other write destroys that property, whereas `always_ff` models a
register and the declaration initializer is that register's power-on value —
`logic [7:0] c = 0; always_ff @(posedge clk) c <= c + 1;` is the standard FPGA idiom.
A plain `always` is excluded for a different reason: `logic clk = 0; always #5 clk = ~clk;`
is a testbench idiom every tool accepts and the clause reaches only inferring procedures.
`initial` and `final` are excluded likewise.

The `always_comb` decision procedure over-approximates, so one guard is attached: the
definite-assignment walk is name-based and treats an unresolved call as a write, which
false-positives when a block-local shadow redeclares the same name inside the `always_comb`.
A procedure that declares a local of that name is skipped whole. A write through a task
`inout` actual is still a driver.

**Fix:** separate the overlapping part-select ranges or restructure to a single driver; or
drop either the initializer or the `always_comb` write. Not suppressible; there is no policy
flag to demote it to a warning.

### VITA-E3002 · `E-ELAB-PORT-MISMATCH` (Error)
**An instance port binding is incompatible with the module's port declarations.** A named
connection to a port the module does not have, more positional connections than ports, or a
direction or kind that makes the binding meaningless (IEEE 1800-2017 §23.3.2). This is
distinct from a recoverable width mismatch: the instance cannot be formed at all. The
hierarchical path and the AST span are attached.
```
module child(input a, output y); endmodule
module top; wire y; child u0(.a(1'b0), .z(y)); endmodule
->  top.sv:3:27: error[VITA-E3002] E-ELAB-PORT-MISMATCH: connection `.z(...)` names no port
    of module `child` [in top.u0]
```
**Fix:** connect the ports the module declares, by name or by position, with matching
directions. Leave a port unconnected explicitly with `.z()`. Not suppressible.

### VITA-E3003 · `E-ELAB-UNRESOLVED-INSTANCE` (Error)
**An instantiated module cannot be resolved to a compiled design unit.** While flattening the
hierarchy, the instance's target module was not found in the work library, in any `-L`
library, or by library search. `vcmp` compiles units in isolation — parse is the last
language-dependent stage — so a missing reference only surfaces at elaboration.
```
module t; alu u_alu(.a(1),.b(2)); endmodule
->  t.sv:2:8: error[VITA-E3003] E-ELAB-UNRESOLVED-INSTANCE: unknown module `alu` instantiated [in t]
```
**Fix:** add the missing unit's source to the `vcmp` inputs, make it discoverable with
`-L <lib>`, or correct the module name. Library resolution is a hash input to the artifact,
so a change there invalidates downstream snapshots. Not suppressible.

### VITA-E3004 · `E-ELAB-USER-ERROR` (Error)
**An elaboration-time `$error`.** A `$error` evaluated outside a procedural block — at module
level or inside a generate — failed a design-time check (IEEE 1800-2017 §20.11). The
`elaborate` crate emits it through the same event path as runtime diagnostics but with no
simulation time; the span comes straight from the AST. Per IEEE it records and continues.
```
parameter DEPTH = 0;
if (DEPTH <= 0) begin : g $error("DEPTH=%0d must be positive", DEPTH); end
->  m.sv:4:29: error[VITA-E3004] E-ELAB-USER-ERROR: DEPTH=0 must be positive [in m.g]
```
Elaboration continues, but the run ends with no artifact and exit 1, because any elaborate
Error discards the IR.

**Fix:** correct the parameter or its override (`-G DEPTH=16`). Use `$fatal` in the RTL if
the check should stop elaboration at that point. Suppress with `-Wno-E-ELAB-USER-ERROR`
resolves but has no effect — Error is part of the always-logged spine.

### VITA-F3005 · `F-ELAB-USER-FATAL` (Fatal)
**An elaboration-time `$fatal`.** A `$fatal(n, …)` evaluated at module level or inside a
generate failed a design-time check (IEEE 1800-2017 §20.11). The leading `n` is the IEEE
finish_number, not a shell code; it is consumed and discarded. No snapshot is written and the
stage ends with exit class 1, which is deliberately distinct from the staleness class 2.
```
if (IN_W < 1 || IN_W > 64) $fatal(1, "IN_W=%0d out of range", IN_W);
$ velab -G IN_W=128 top.vu
->  error[VITA-F3005] F-ELAB-USER-FATAL: IN_W=128 out of range
    no .velab is written, exit 1
```
**Status at HEAD: declared Fatal, emitted at Error severity.** The elaboration `$fatal` arm
routes through the shared error path, which hardcodes `Severity::Error`. The printed token is
therefore `error`, elaboration records and continues rather than stopping at that statement,
and the run still ends with no artifact and exit 1 because an elaborate Error discards the IR.

**Fix:** supply valid parameters (`-G IN_W=32`) or correct the guard. Not suppressible.

### VITA-I3006 · `I-ELAB-USER-INFO` (Info)
**An elaboration-time `$info`.** An `$info` at module level or inside a generate printing
information — resolved parameters, which generate configuration was selected. Info severity,
no simulation time, no effect on the exit code.
```
$info("elaborating dcache with WAYS=%0d", WAYS);
->  info[VITA-I3006] I-ELAB-USER-INFO: elaborating dcache with WAYS=4
```
**Fix:** nothing to do. Silence it with `-q` or `-Wno-I-ELAB-USER-INFO`.

### VITA-W3007 · `W-ELAB-USER-WARNING` (Warning)
**An elaboration-time `$warning`.** A `$warning` at module level or inside a generate marking
a legal but suspicious design-time condition (IEEE 1800-2017 §20.11). Elaboration continues;
the exit code changes only under promotion.
```
if (LATENCY < 1) $warning("LATENCY=%0d is unusually small", LATENCY);
->  warning[VITA-W3007] W-ELAB-USER-WARNING: LATENCY=0 is unusually small
```
**Fix:** adjust the parameter, or accept it if intended. Suppress with `-Wno-`; promote with
`-Werror=W-ELAB-USER-WARNING` to fail CI on an RTL-authored warning without editing the RTL.

### VITA-W3008 · `W-ELAB-WIDTH-TRUNC` (Warning)
**A width mismatch was resolved by implicit truncation or extension.** Reserved for a
dedicated width-mismatch report on port connections, `assign` and parameter expressions,
naming both widths and the hierarchical path. An implicit size cast is legal under
IEEE 1800-2017 §11.6.1, so the run would continue; silent truncation is a classic defect
source, so it would be surfaced.

**Status at HEAD: no emitter.** The generic elaborate simplification channel reports
`W-ELAB-FEATURE-LIMIT` (`VITA-W3056`) instead, and that is also where the one measured
width-loss report lives — an implicit scalar net driven with more bits than it can hold.

**Fix:** make the cast explicit (`wide[3:0]`) or match the widths. Suppressible and
promotable if it fires.

### VITA-E3009 · `E-ELAB-UNSUPPORTED` (Error)
**The loud-reject surface for constructs elaborate cannot lower faithfully.** This is the
central code of the correct-or-loud contract: rather than produce a wrong value with no
error, elaboration stops. Most of the pipeline is implemented, so this code comes from the
remaining unsupported sub-forms. Everything listed below is measured against the binary at
HEAD; the full remaining list is [ROADMAP §3](../ROADMAP.md) and
[../manual/006_limitations.md](../manual/006_limitations.md).

This code also carries the elaborate error cap: at error 201 a single
`too many elaborate errors; further diagnostics suppressed (cap 200)` is emitted under this
code and every later elaborate error, and its notes, are dropped.

**1. A real value where an integral one is required.** IEEE 1800-2017 §11.5.1 requires select
indices and range bounds to be integral constants, and §11.4.12.2 requires the same of a
replication count. vita converts a real that is exactly integral at the context boundary and
rejects anything else, so it never rounds silently and never reads an f64 bit pattern as an
integer.
```systemverilog
module m;
  parameter real R = 1.5;          // not exactly integral
  logic [7:0] v;
  initial v[R] = 1'b1;
endmodule
->  m.sv:4:11: error[VITA-E3009] E-ELAB-UNSUPPORTED: a select index / bound / size must be
    integral, not real (IEEE §11.5.1) [in m]
```
The same code covers: select indices, part-select bounds, the offset and width of an indexed
part-select, array word indices, `new[N]` sizes, queue and associative-array indices and the
keys of `.exists()`/`.delete()`, string-method arguments, the address arguments of
`$readmem*`/`$writemem*`/`$fread`, and replication counts. A call to a `function real` in one
of those positions is included.
```systemverilog
module m;
  parameter real R = 8.5;
  logic [R-1:0] v;                 // the width does not fold to an integer
endmodule
->  m.sv:3:10: error[VITA-E3009] E-ELAB-UNSUPPORTED: a real parameter is not an integral
    constant and cannot be used in a width / range bound (assign it to an integer localparam
    first) [in m]
```
**Fix:** declare the value with an integer type (`parameter int`) if it is one, or convert
explicitly with `$rtoi()` or `int'()`. A real that folds to an exact integer, such as
`parameter real R = 4;`, is usable in an integral context as it stands.

**2. A parameter override that reads a real parameter.**
```systemverilog
module s #(parameter W = 8) (); endmodule
module m; parameter real R = 4.5; s #(.W(R)) u(); endmodule
->  m.sv:2:8: error[VITA-E3009] E-ELAB-UNSUPPORTED: a parameter override that reads a real
    parameter is unsupported (a real has no integral constant value) [in m]
```
An override that folds to an integer, such as `#(.R(3))` or `#(.R(i+2))`, applies normally.

**3. Other remaining sub-forms.** A part-select target of `force`/`release`; a runtime `==?`
pattern; a hierarchical reference to a real parameter; a `parameter real` binding in an
interface body or a generate scope; a whole-value assignment through a dynamic-storage handle.

**4. Per-entry equivalence for a block-local `automatic`.** v1 lowers a procedural block's
locals to a single flattened variable. An `automatic` local is accepted only when that
flattening is indistinguishable from real per-entry storage, and rejected under this code
otherwise. The rejections that remain are:

- A fixed-size array whose coverage cannot be proved: an element read that this entry did not
  write, or a computed index (`foreach (a[i]) a[i] = …;`) that fills it.
- A call whose callee body can reach the flattened name, whether by bare name or by a
  hierarchical self-path (`t.a`).
- Shadowing where one block *encloses* another and redeclares the same name. Reuse between
  sibling blocks is supported at any nesting depth.
- A statement that advances time inside a block that genuinely shares one flattened variable
  with another block of the same name — the scheduler can run the other block and write that
  single variable, so a later read sees a value its own storage could not have held.
- A hierarchical reference *to* an `automatic` block-local (`tb.a`, or `t.a` from a task in
  the same module). IEEE 1800-2017 §23.9 forbids it: automatic storage has no static address
  to name.

Accepted: a local that is never written anywhere in the block, of any type — the flattened
variable is initialised once to the type default and nobody changes it, so every entry sees
the default; a first write carrying timing (`#1 x = 7;`, `@(posedge clk) x = 7;`,
`x = #1 7;`, `wait (c) x = 7;`, all blocking); every statement form after a definite write;
and chained method calls (`s.substr(a,b).atoi()`).

A rejection produced because the analysis stopped carries a `note:` giving the
`file:line:col` of the statement that stopped it and one of six reasons: read here, partial
select write, an unmodelled statement form, an unproven call, an `input` actual read, or time
advancing in a shared variable. That position can be many statements after the declaration,
or in another file, which is why the note exists.

**5. The position of a call to a function with an output or inout formal.** Copy-out for such
a function (IEEE 1800-2017 §13.5.2) lowers to a *statement*, so the copy-out must be emitted
ahead of the expression that contains the call. Every position a statement evaluates exactly
once is supported: a direct RHS, a condition, a `case` scrutinee, a `repeat` count, a
concatenation element, an argument of another call, a select index, a cast operand, a
`$display` argument, a task argument, a nonblocking-assignment RHS, a `return` value, an
lvalue index, and the right operand of `&&`/`||` and the arms of `?:` at any depth
(a conditional position emits into a guard block, so a short circuit also skips the write,
and an x condition evaluates both arms).

The rejected positions are:

- **Continuously re-evaluated expressions** — `assign`, `force`, a `wait` condition. A
  copy-out cannot fire again on every change.
- **Intra-assignment delay** (`x = #1 f(...)`), `min:typ:max`, and constraint or `with`
  expressions.
- **Cases where evaluation order cannot be preserved.** Reading the output actual to the
  *left* of the call must see the pre-call value, so vita takes a copy first; three shapes
  make that copy useless and are rejected — the read is through a hierarchical path (`t.o`)
  or inside the called function's own body, so the substitution cannot reach it; the target
  is not a plain bit-vector net (an unpacked array or struct root cannot be copied to one
  net); or two calls in one expression write the same target, which needs two generations
  and has one copy.

The workaround is the same in every case: assign the call to a temporary first
(`t = f(...);`) and use `t`.

The diagnostic deliberately does not enumerate the *supported* positions, only the remaining
rejections. An enumeration of what works is the shape that goes stale and sends a reader
hunting for a workaround that is not needed.

**6. The snapshot slot for a dynamic-array formal.** `function f(input byte b []);` places a
marker immediately before the call expression that snapshots the caller's array into the
formal's slot, so the call may appear only where that marker can be placed. Supported,
measured across seventeen positions: a blocking or nonblocking assignment RHS, a `return`
value, a `?:` arm when the function has no side effects, and an unconditionally evaluated
operand of one of those (a concatenation, arithmetic, a comparison, a system-task argument).
Rejected: the right operand of `&&`/`||`, an argument of another call, a select or lvalue
index, a `case` scrutinee, a `repeat` count, and a cast or replication operand. There is one
slot per formal, so calling the same function twice in one expression, or calling it
recursively from its own body, is also rejected — both would read the last snapshot.

The workaround is to assign the call to a variable first (`x = f(arr);`). These rejections are
tracked in [ROADMAP §3](../ROADMAP.md); the general hoister already opens those positions for
output-formal calls.

**Fix:** the message names the specific sub-form; each subsection above states its workaround.
Not suppressible.

### VITA-E3010 · `E-ELAB-UNRESOLVED-NAME` (Error)
**A reference to an undeclared net or variable.** A name used in an `assign`, an expression or
an lvalue is not in the symbol table. This is the net-name counterpart to
`E-ELAB-UNRESOLVED-INSTANCE`, which is about module resolution.
```
module m; wire y; assign y = z; endmodule
->  m.sv:2:8: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `m.z` [in m]
```
**Fix:** add the missing declaration or correct the typo. In the two positions listed under
`W-PARSE-IMPLICIT-NET` an undeclared name becomes an implicit net instead — unless
`` `default_nettype none `` is in effect, which routes every position here. Not suppressible.

### VITA-W3011 · `W-ELAB-CASEZ-APPROX` (Warning)
**A `casez` treats an explicit `x` label bit as a don't-care.** Reserved for an approximate
`casez` lowering, where a label bit written as an explicit `x` would be matched like a `z`
wildcard.

**Status at HEAD: no emitter.** `casez` and `casex` lower to exact `CasezEq`/`CasexEq`
comparisons — `casez` treats only `z` and `?` on either side as wildcards and an explicit `x`
label bit matches only `x`, while `casex` treats both `x` and `z` as wildcards. Nothing
approximates, so nothing raises this code. The number is retained because numbers are never
reused.

**Fix:** nothing to do. Suppressible and promotable if it fires.

### VITA-E3018 · `E-ELAB-LVALUE-KIND` (Error)
**The assignment kind does not match the target's kind.** A user `assign` drives a variable
(`reg`, `integer`, `real`, `string`), or a procedural assignment in an `initial`/`always`
targets a `wire`. iverilog rejects both directions; Verilator reports `CONTASSREG` and
`PROCASSWIRE`. SystemVerilog `logic` passes either way, because IEEE 1800-2017 admits both a
single continuous driver and procedural writes. The implicit connections synthesised by port
binding are exempt — IEEE 1800-2017 §23.3.3 makes variable ports legal.
```
module m; reg r; assign r = 1'b1; endmodule
->  m.sv:1:8: error[VITA-E3018] E-ELAB-LVALUE-KIND: continuous assign drives variable `m.r`
    (declare it wire/logic) [in m]
```
The procedural direction reads
``procedural assignment to net `m.w` (declare it reg/logic)``.

**Fix:** change the declaration to `wire` or `reg` (or SystemVerilog `logic`) to match, or
change the form of the assignment. Not suppressible.

### VITA-W3056 · `W-ELAB-FEATURE-LIMIT` (Warning)
**A legal construct is accepted but simplified.** The general elaborate simplification
channel: constructs v1 deliberately approximates or drops rather than refuses — an
unconnected port, a unidirectional approximation of `inout`, a dropped intra-assignment
delay, a skipped system task, a shared scope for `fork`-local declarations. The IR survives
and the module keeps running; this is the opposite lever to `E-ELAB-UNSUPPORTED`, which
discards it.
```
module ch(input i); endmodule  module t; ch u1(); endmodule
->  t.sv:3:14: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: input port `i` left unconnected —
    it floats at `z`, and every value the instance derives from it is unknown (tie it off
    explicitly) [in t.u1]
```
It also carries the width report for an implicit net, which is scalar by IEEE 1364-2005 §3.5:
```
->  warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: `m.w` is an IMPLICIT net, so it is 1 bit wide
    (IEEE 1364-2005 §3.5) — this assignment drives it with 12 bits and the top 11 are
    discarded. Declare it with the width you meant.
```
**Fix:** usually nothing — read it as a confirmation of intent. Suppress with `-Wno-`, promote
with `-Werror=`.

### VITA-W3057 · `W-ELAB-AUTOTOP-AMBIGUOUS` (Warning)
**Auto-top chose among two or more uninstantiated roots because no top was pinned.** With no
`--top`, one-shot `vita <sources>` (or `-f`) elaborates every module that is instantiated
nowhere as an independent top, matching IEEE 1364 and iverilog. That is legal, but if one
particular top was intended, a `$finish` in another root can end the run and hide its output.
The warning names the root candidates.
```
module aaa; ... endmodule   module zzz; ... endmodule
$ vita tworoot.sv
->  warning[VITA-W3057] W-ELAB-AUTOTOP-AMBIGUOUS: auto-top selected 2 uninstantiated roots
    (aaa, zzz); all are elaborated as independent tops — pin one with `--top <module>` for a
    deterministic single top
```
**Fix:** pass `--top <module>` (supported by one-shot `vita` and by `-f`). If several
independent tops are intended, the warning is harmless. Suppress with `-Wno-`, promote with
`-Werror=`.

### VITA-W3058 · `W-ELAB-STR-TERNARY` (Warning)
**`$display`/`$write` was given one argument that is a ternary whose arms are both string
literals.** IEEE 1800-2017 §5.9 makes a string literal a packed integral value; the ternary
puts both arms in an integral context, and a single non-format argument prints as decimal.
```
$display(n == 1 ? "[PASS] all vectors" : "[FAIL] mismatch");
->  m.sv:2:30: warning[VITA-W3058] W-ELAB-STR-TERNARY: `$display` was given ONE argument that
    is a ternary of string literals — IEEE 1800 §5.9 makes those packed integers, so this
    prints a decimal number, not text (use `if`/`else`, or `$display("%s", cond ? "a" : "b")`)
     7954527441615041037357320920738594452107891
```
This is not a value defect. iverilog prints the identical number, so vita is not diverging
from the oracle and the value is deliberately unchanged. It is warned because the shape is
always a mistake — nobody wants the number — it is the ordinary way a PASS/FAIL line gets
written, and it silently makes a test log unreadable. The trigger is deliberately narrow: one
argument, a ternary, a string literal on *both* arms. A `%s` format, a ternary of string
variables, and a numeric ternary do not trip it.

**Fix:** use `if`/`else`, or `$display("%s", cond ? "a" : "b")`. Suppress with `-Wno-`, promote
with `-Werror=`.

### VITA-W3059 · `W-ELAB-STR-ESCAPE` (Warning)
**A string literal contains a backslash escape IEEE 1800-2017 Table 5-1 does not define.**
vita gives it a value; the warning is that other tools give it a different one.
```
if (c == "\r") ...
->  m.sv:2:30: warning[VITA-W3059] W-ELAB-STR-ESCAPE: `\r` is not a string escape in IEEE
    1800-2017 Table 5-1 — vita and Verilator read it as 0x0D, iverilog and Xcelium read it as
    the character `r`. Write the byte explicitly (`\015` octal or `\x0D` hex) to mean the same
    thing everywhere
```
This is not a value defect, and that is what makes it expensive: the design compiles on every
tool and evaluates differently on each. A vector-file parser that trims line ends with
`== "\r"` reads the letter `r` under Xcelium, cuts real `r` characters mid-record, and reports
hundreds of data mismatches — a symptom that looks like a DUT defect and is unreachable from
any single-simulator experiment.

Table 5-1 defines `\n \t \\ \" \v \f \a \ddd \xhh` and nothing else. Two shapes trip this
warning:

| Shape | vita and Verilator | iverilog and Xcelium |
|---|---|---|
| `\r` | 0x0D, the C meaning | the letter `r` |
| any other `\X` | both characters kept, so the string is one byte wider | just the character |

The oracles genuinely split on `\r`, so there is no majority to follow and the value is
deliberately unchanged. One line is emitted per distinct escape per literal, anchored at the
literal rather than at the statement, so two escapes on one line are two locations.

**Fix:** write the byte you mean — `"\015"` (octal) or `"\x0D"` (hex) are Table 5-1 escapes
and read the same everywhere. Suppress with `-Wno-`, promote with `-Werror=`.

---

## 4xxx · RUNTIME

### VITA-E4001 · `E-RUN-ASSERT-FAIL` (Error)
**An assertion failed and had no action block.** An immediate `assert(...)` or a concurrent
`assert property(...)` evaluated false with no `else` block has an implicit `$error` severity
under IEEE 1800-2017 §16.3. Reserved for reporting that failure under its own code, routed
through the same gate as an RTL `$error`.

**Status at HEAD: no emitter.** Assertions are implemented — immediate `assert` with `else`
severity, deferred `assert #0` and `assert final`, and concurrent `assert property` including
the sequence subset, multi-clock and named property and sequence. A failure is emitted by the
synthesised clocked checker or by the severity factory as a `$error`, which reports as
`VITA-E4003`.

**Fix:** fix the design or the bench, or give the assertion an explicit action block
(`assert(...) else $warning(...)`). Suppressible and promotable if it fires.

### VITA-E4002 · `E-RUN-RANGE` (Error)
**A runtime array index or bit/part select is out of range with a known index.** IEEE
1800-2017 §11.5.1 makes an out-of-range select read `x` and drops an out-of-range write rather
than trapping, so the value semantics are preserved and the access is reported instead of
being silently corrupting. `sim-ir` is span-free, so `file:line:col` is restored from the
location side-table and the simulation time is attached.
```
logic [7:0] mem [0:15];  int idx = 20;  $display("%0h", mem[idx]);
->  m.sv:3:19: error[VITA-E4002] E-RUN-RANGE: array word index of `m.mem` (out of range;
    read X / write ignored) [in m] [at time 1]
```
Reports are capped at eight per run, with a budget separate from the unknown-index cap so one
cannot starve the other; report eight becomes
`further out-of-range diagnostics suppressed` and later ones print nothing.

**Fix:** validate or clamp the index before the select, or size the array to match. The value
semantics are standard (read `x`, write dropped) and the run continues. Suppress with
`-Wno-E-RUN-RANGE`, promote with `-Werror=` to stop CI. An index that is *unknown* rather than
out of range is `VITA-W4029`.

### VITA-E4003 · `E-RUN-USER-ERROR` (Error)
**A simulation-time RTL `$error`.** Executed in a procedural or simulation context — an
`initial`, an `always`, a task, or an SVA `else $error`. IEEE 1800-2017 §20.11 prints the
message and continues; `$finish` is not called and the exit class does not change by itself.
The diagnostic interleaves with `$display` output in simulation-time order and carries
severity, `file:line:col`, the instance path and the simulation time.
```
if (dut_result !== expected) $error("MISMATCH got=%0h exp=%0h", dut_result, expected);
->  tb.sv:2:25: error[VITA-E4003] E-RUN-USER-ERROR: MISMATCH got=ab exp=cd [in m] [at time 0]
```
An `$error` with an empty message renders the code's title in its place. A failing assertion
with no action block also arrives here.

**Fix:** this is an intended bench output — fix the condition the bench flagged. An
accumulated Error latches the error exit class, so the run ends with exit 1 even though
simulation completed. Not suppressible (Error is spine).

### VITA-F4004 · `F-RUN-FATAL` (Fatal)
**A fatal condition ended the simulation stage.** After the diagnostic, the simulation stage
stops with exit class 1, deliberately distinct from the staleness class 2. Two distinct
families share this code, told apart by the message.

**(a) An RTL `$fatal`.** IEEE 1800-2017 §20.11: an implicit `$finish`, immediate termination.
The leading `n` (0, 1 or 2) is the IEEE finish_number, not a shell code; vita consumes and discards
it and prints no exit statistics at any level.
```
if (cfg_invalid) $fatal(1, "bad config word %0h", cfg);
->  dut.sv:2:115: fatal[VITA-F4004] F-RUN-FATAL: bad config deadbeef [in m] [at time 0]
```

**(b) An engine capability limit** — the runtime half of correct-or-loud. Rather than produce
a quietly wrong value the run stops. If the design contains no `$fatal` and this code appears,
it is this family: not a defect in the user's code but a shape the simulator does not support.

| Message head | Condition | Way around it |
|---|---|---|
| `internal: a frame-local dynamic array … was still held at frame entry…` | An invariant guard, not a capability limit. Concurrent activations are supported — each activation parks its own array off the heap while suspended. This message means that park/unpark invariant broke | Report the design; there is nothing to work around |
| ``writing an element of (or a whole store to) a dynamic-array `input` formal…`` | A write to a dynamic-array `input` formal, which is aliased read-only | Copy the value into a local dynamic array and modify that |
| ``an associative-array iteration (`first/next/last/prev`) whose key…`` | An associative iteration key outside the supported positions | Move the key into a frame-local variable |
| `` `X` does its work as a statement-level effect… `` | Statement-level effects — writing a destination or a `ref` argument, advancing a file descriptor, updating a seed — are performed only by the `&mut` process executor. On the synchronous `&self` frame executor the call falls back to a pure evaluation, which would return 0 and leave the destination unchanged. The family, spelled canonically by `sim_ir::sysfunc_frame_executor_cannot_perform`, is `$fgets`/`$fscanf`/`$sscanf`/`$fread`/`$feof`/`$fgetc`/`$ungetc`, `$fopen`, `$value$plusargs`, seeded `$random`/`$dist_*`, `$cast`, and queue pop. `$sformatf` is not in the family — the frame executor has a working intercept for it | Three positions remain: a class-method body, a continuously re-evaluated position (`assign`/`force`/a `wait` condition), and an intra-assignment delay (`x = #1 f(...)`). Module processes, and tasks or functions called from a statement, do work regardless of `automatic` or output formals — call there, assign to a variable, and use the variable |
| `frame-task recursion exceeded the depth limit (N)` / `frame-call …` | Recursion past `MAX_CALL_DEPTH` — an unbounded recursion is made loud instead of hanging | Check the termination condition |
| ``a subroutine running on the synchronous frame executor tried to write `NAME`…`` | The frame executor tried to write a net that is not frame-local — a module or instance net — and it has neither a flat store nor a dirty channel | Move the write into a `task` body (a task body's out-of-window write routes to the process executor automatically) or into the calling process |
| `fork exceeds the v1 tie-encoding limit …` | More than 65534 top-level processes, or more than 65536 arms, past the deterministic ordering encoding | Outside the range of real benches |

**Fix:** for (a), resolve the condition the RTL author declared unrecoverable. For (b), take
the way around from the table above or track the follow-on work in
[ROADMAP §3](../ROADMAP.md). Fatal is not suppressible — an abort cannot be un-aborted.
The elaboration-time `$fatal` is `F-ELAB-USER-FATAL`.

### VITA-I4005 · `I-RUN-USER-INFO` (Info)
**A simulation-time RTL `$info`.** Informational output from a procedural or simulation
context. IEEE 1800-2017 §20.11: purely informational, the run continues, the exit code is
unaffected. Severity, `file:line:col`, the instance path and the simulation time distinguish
it from a plain `$display`, which carries no severity and no code.
```
if (pass) $info("test PASSED (%0d vectors)", n);
->  tb.sv:2:98: info[VITA-I4005] I-RUN-USER-INFO: PASSED [in m] [at time 0]
```
**Fix:** nothing to do. Silence it with `-q` or `-Wno-I-RUN-USER-INFO`. Note that `-q`
suppresses only the stdout copy of progress and RTL text; diagnostics always reach stderr, and
`--log <file>` receives every line regardless of verbosity.

### VITA-W4006 · `W-RUN-NO-LOCATIONS` (Warning)
**The loaded `.velab` carries no location side-table, so runtime diagnostics cannot show
`file:line:col`.** Reserved for a snapshot shipped without locations: runtime diagnostics
would degrade to `(source location unavailable)` while the code, severity, simulation time and
instance path keep printing, and the warning would be issued once at load.

**Status at HEAD: no emitter.** `velab` always writes the elaborate-time-resolved `stmt_locs`
into the `.velab` trailer and `vrun` always threads it back, so a staged run prints the same
diagnostic line as one-shot; parity is pinned by a test that compares the whole line and pins
the location absolutely. There is no flag that strips the table. A `.vu` whose source-map tail
is absent or undecodable is refused loudly as `VITA-E9001` rather than degraded, because a
tolerant fallback would stamp every diagnostic at `:1:1`.

**Fix:** re-elaborate with the current tools. Suppressible and promotable if it fires.

### VITA-W4007 · `W-RUN-USER-WARNING` (Warning)
**A simulation-time RTL `$warning`.** IEEE 1800-2017 §20.11: print and continue, tool-
suppressible. It passes the same gate as a compile-time warning, so
`-Werror=W-RUN-USER-WARNING` turns an RTL `$warning` into a CI failure without editing the
RTL. The exit code changes only under promotion.
```
if (fifo_almost_full) $warning("fifo near full: depth=%0d", depth);
->  tb.sv:2:75: warning[VITA-W4007] W-RUN-USER-WARNING: near full [in m] [at time 0]
```
This code is the RTL's own `$warning` only. A `unique`/`priority` violation is the simulator's
own report and carries `VITA-W4031`, so suppressing one never suppresses the other.

**Fix:** handle the flagged condition, or accept it. Suppress with `-Wno-`, promote with
`-Werror=`.

### VITA-F4016 · `F-RUN-NO-CONVERGE` (Fatal)
**A time step did not reach a fixed point within the delta limit.** The delta-cycle count for
one time step exceeded `SimOpts::max_deltas` (default 1,000,000): a continuous-assign
oscillation, a procedural loop with no timing control, or zero-delay feedback. The simulation
stops immediately with exit class 1.
```
->  fatal[VITA-F4016] F-RUN-NO-CONVERGE: did not converge: delta limit (1000000) exceeded at
    time 0 (zero-delay loop / combinational oscillation)
```
One diagnostic per run: every overflow path — settle at time 0, the run loop, the interpreter's
in-body activation guard and the VM guard — funnels through one single-shot emitter.

**Fix:** break the feedback path (insert a register) or add timing control (`#` or `@`) to the
loop. If the chain is genuinely deep and intended, raise `SimOpts::max_deltas`. Not
suppressible. This is a different condition from `VITA-F4027`, which is one activation running
long, not the scheduler failing to settle.

### VITA-W4018 · `W-RUN-VCD-OPEN-FAIL` (Warning)
**The `$dumpfile` path could not be opened, and the simulation continues without a waveform.**
At `$dumpvars` time the dump file could not be created — a missing directory, permissions, a
read-only filesystem. Rather than let the primary artifact vanish silently, the failure is
reported with the path and the OS error and the run proceeds. The exit class is unchanged.
```
$dumpfile("/no/such/dir/wave.vcd"); $dumpvars;
->  warning[VITA-W4018] W-RUN-VCD-OPEN-FAIL: cannot open VCD dump file
    '/no/such/dir/wave.vcd': No such file or directory (os error 2)
```
**Fix:** create the directory or fix permissions, or redirect the output with `-o`. Promote
with `-Werror=` for CI.

### VITA-W4019 · `W-RUN-VCD-WRITE-FAIL` (Warning)
**A waveform write or flush failed, so the waveform may be truncated.** The final flush at end
of simulation failed (disk full, I/O error). Everything up to the last completed write remains
a valid, truncated VCD. The same code reports a failed FST transcode.
```
->  warning[VITA-W4019] W-RUN-VCD-WRITE-FAIL: VCD flush failed: <io error>
->  warning[VITA-W4019] W-RUN-VCD-WRITE-FAIL: FST transcode failed for 'wave.fst': <io error>
```
**Fix:** check free space and the mount state, then re-run. Promote with `-Werror=` for CI.

### VITA-W4020 · `W-RUN-DYN-DEGRADE` (Warning)
**A dynamic-storage operation took its specified degraded path.** The degradations, all of
which keep running with a defined value: `new[n]` with an x/z size gives an empty array; an
out-of-range index reads x and drops the write; a pop from an empty queue gives x; a pop
outside a direct-assignment position (a nonblocking RHS, a nested expression) gives x without
popping; a push, `new` or associative write past the element cap of `1<<24` is dropped or
clamped; an associative read with an x/z key gives x, and a write or `delete(k)` with one is
ignored while `exists` answers 0; an associative read of an absent key gives x; an associative
element chunk inside a concatenation lvalue is ignored. The same code carries the null
class-handle degrade — a read gives x, a write is a no-op.
```
->  warning[VITA-W4020] W-RUN-DYN-DEGRADE: new[] size is X/Z; array degraded to empty
```
Three related operations are legal and silent, not degradations: `q[size()] = v` on a queue is
equivalent to `push_back` (IEEE 1800-2017 §7.10.1); an associative write to an absent key
creates the element (§7.8); `delete(k)` on an absent key is a no-op (§7.9).

Emitted once per handle net, so a degraded operation inside a loop cannot flood the
diagnostic stream.

**Fix:** find and fix the source of the x in the size or index expression. Promote with
`-Werror=` for CI.

### VITA-W4021 · `W-RUN-DUMP-MULTI` (Warning)
**A second or later `$dumpvars` call was ignored.** The first call opens the waveform and
fixes the filter (depth, scope and net arguments). A VCD header can only be written once, so
the accumulating union of later calls is outside v1. Later calls are a no-op after one warning
per run.
```
->  warning[VITA-W4021] W-RUN-DUMP-MULTI: extra $dumpvars call ignored (v1: the first call wins)
```
**Fix:** collect every dump target into the first `$dumpvars` call. Suppress with `-Wno-`,
promote with `-Werror=`.

### VITA-W4022 · `W-RUN-BAD-FD` (Warning)
**A file operation received an invalid or already-closed descriptor, and was ignored.**
`$fdisplay`, `$fwrite`, `$fclose` and friends on a descriptor that was closed, that is x/z, or
that is the 0 returned by a failed `$fopen`. iverilog behaves the same way. Emitted once per
descriptor.
```
->  warning[VITA-W4022] W-RUN-BAD-FD: file operation on invalid/closed descriptor 0x80000003 ignored
```
**Fix:** check the descriptor's lifetime, and test that `$fopen` returned nonzero before
using its result. Suppress with `-Wno-`, promote with `-Werror=`.

### VITA-W4023 · `W-RUN-READMEM` (Warning)
**`$readmemb`/`$readmemh` could not open the file, or the token count does not match the
requested range.** The memory is partially loaded and execution continues. iverilog prints
its own "ERROR" text for a missing file but still exits 0; vita keeps that exit parity and
classifies the condition as a warning.
```
$readmemh("/no/such/file.hex", mem);
->  m.sv:2:39: warning[VITA-W4023] W-RUN-READMEM: $readmem: unable to open
    '/no/such/file.hex' for reading [in m] [at time 0]
```
**Fix:** correct the path, match the token count to the range, or pass explicit start and
finish arguments. An `@addr` directive outside the range has the same effect. Suppress with
`-Wno-`, promote with `-Werror=`.

### VITA-F4024 · `F-RUN-CLASS-LIMIT` (Fatal)
**The class-object budget was exceeded.** More than `SimOpts::max_class_objs` (default
1,000,000) class objects were allocated. The class heap is not garbage-collected, so an
unbounded `new()` in a loop grows without limit; on reaching the budget the run ends with a
graceful implicit `$finish` (exit class 1) rather than running out of memory. This is the same
resource-limit pattern as the delta limit.
```
->  fatal[VITA-F4024] F-RUN-CLASS-LIMIT: class object budget (1000000) exceeded — likely an
    unbounded `new()` (the class heap is not garbage-collected); raise
    SimOpts::max_class_objs if intended
```
**Fix:** reuse live objects instead of overwriting the handle (`h = new();` every cycle leaves
the previous object unreclaimable), or raise `SimOpts::max_class_objs` for an intentionally
large allocation. Not suppressible.

### VITA-W4025 · `W-RUN-WIDE-ARITH` (Warning)
**Multi-word arithmetic exceeds the width cap and the result is poisoned to X.** An operand of
`*`, `/`, `%` or `**` is wider than `WIDE_ARITH_CAP` (2^20, the same as `MAX_NET_WIDTH`). A
declared net cannot exceed 2^20 bits, but a replication concatenation (`{16{a}}`) can inflate
an *operand* to 16 M bits, where the super-linear kernels — `*` at O(n²), restoring `/` and `%`
at O(bits·n), `**` by square-and-multiply — stall for tens to hundreds of seconds. The result
is poisoned to X instead, following the divide-by-zero degrade precedent. One warning is
emitted at the start of `simulate` if such a node exists. `+` and `-` are O(n) and stay exact
at any width.
```
->  warning[VITA-W4025] W-RUN-WIDE-ARITH: multi-word arithmetic exceeds the 1048576-bit width
    cap; result poisoned to X (the kernel would otherwise stall — narrow the operands)
```
**Fix:** reduce the operand widths to `MAX_NET_WIDTH` or less. Arithmetic above 2^20 bits is
outside v1. Suppress with `-Wno-`, promote with `-Werror=`.

### VITA-W4026 · `W-RUN-VCD-PKGVAR-SKIP` (Warning)
**A package variable was named in `$dumpvars` and has no VCD surface, so it is excluded.**
Package-level variables live in the reserved `$pkg$<pkg>` scope and v1 gives them no waveform
surface. A bare `$dumpvars` never declares them — iverilog does not dump them either — so this
warning fires only when one is *explicitly* selected, as in `$dumpvars(…, pkg_var)`. The net
is excluded rather than silently ignored; iverilog aborts on an assertion at this point.
```
->  warning[VITA-W4026] W-RUN-VCD-PKGVAR-SKIP: a package variable has no VCD surface (v1):
    it is excluded from the dump
```
**Fix:** copy the value into a module variable and dump that, or observe it with `$display`
or the observability probe rail. Suppress with `-Wno-`, promote with `-Werror=`.

### VITA-F4027 · `F-RUN-BODY-STEP-LIMIT` (Fatal)
**One process activation ran past the body-step budget without suspending.** A single
activation executed `SimOpts::max_body_steps` block steps (default 100,000,000) without ever
reaching a `#delay`, an `@(…)` or a `wait`.
```
module m; integer x=0; initial while (1) x = x + 1; endmodule
->  fatal[VITA-F4027] F-RUN-BODY-STEP-LIMIT: one process executed 100000000 block steps at
    time 0 without reaching a `#delay`, `@(…)` or `wait` — either it is an unbounded loop, or
    it is a long computation that needs a larger budget (`SimOpts::max_body_steps`)
```
**Fix:** if it is an unbounded loop, add a suspension point. If it is a genuinely long
computation — parsing a vector file or preloading a memory at time 0 — raise the budget. Not
suppressible.

This is a different condition from `VITA-F4016`. F4016 means the *scheduler* did not reach a
fixed point, a zero-delay loop or a combinational oscillation; this one means *one activation*
ran long. A plain `for (i=0;i<500000;i++)` with no feedback and no oscillation belongs here,
and the diagnostic reports only what it observed.

### VITA-W4028 · `W-RUN-PLUSARGS-INVALID` (Warning)
**A matched plusarg's value could not be converted in the `$value$plusargs` format.** A
non-digit character in a `%d` value (`+N=5x9`), a leading underscore (`+N=_5`), a bare `+`
sign (`+N=+5`). The variable is written all-X and the status is 1, because a matching plusarg
*was* present — both are measured iverilog behaviour. Without this warning a misspelled
plusarg leaves nothing but X values and an exit code of 0.
```
$ vita design.sv +N=5x9
->  warning[VITA-W4028] W-RUN-PLUSARGS-INVALID: invalid decimal value "5x9" in a matched
    plusarg; variable written all-X
```
**Fix:** correct the value. x and z digits and underscore separators are valid
(`+A=1x2z`, `+F=1_2` parse by the literal convention with no warning), but an underscore
cannot lead. Suppress with `-Wno-`, promote with `-Werror=`.

### VITA-W4029 · `W-RUN-RANGE-UNKNOWN` (Warning)
**A runtime array word index or select offset evaluated to an unknown (x/z) value.** The read
answers all-X and the write is ignored, exactly as IEEE 1364-2005 §5.2.1 prescribes, so this
is legal behaviour rather than an error.
```
reg [7:0] mem [0:3];  reg [1:0] q;      // x until the first clock edge
assign o = mem[q];
->  warning[VITA-W4029] W-RUN-RANGE-UNKNOWN: array word index of `m.mem` is unknown (x/z);
    read X / write ignored [at time 0]
```
It is a separate code from `E-RUN-RANGE` because the two are different facts about a design:
an x index before reset is ordinary RTL, while a *known* index past the end of an array is
almost always a defect. Reporting both as errors filled the log during the reset window and set
exit 1 on a correct design. Capped per run independently of `E-RUN-RANGE`, so a flood of
unknown-index warnings cannot starve the out-of-range budget.

**Fix:** nothing, if the x window is expected. Suppress with `-Wno-W-RUN-RANGE-UNKNOWN`, or
restore error behaviour with `-Werror=W-RUN-RANGE-UNKNOWN`.

### VITA-W4030 · `W-RUN-BACKEND-FALLBACK` (Warning)
**The requested execution backend could not run this design, and a different one ran it.**
The three executors are gated on byte-identical output, so the answer is unaffected and only
the speed changes. That is why it is a warning, not an error.
```
$ vita --backend native design.sv
->  warning[VITA-W4030] W-RUN-BACKEND-FALLBACK: requested backend `native` cannot run this
    design (a task frame that FORKS (a `fork` inside the body): S3b); ran on `vm` instead —
    the result is unaffected, the speed is
```
A silent fall-back is the thing this code exists to prevent: a design run with
`--backend native` that actually executed on the VM produces identical output, so the run
reads as agreement between the native backend and the oracle. The facts were always in
`run.json` under `backend_requested`, `backend` and `native.refused`, but only for a reader who
went looking.

The severity follows the accuracy ladder. A fall-back is a slower answer, not a wrong one, so
making it `exit != 0` in the default build would demote correct-support to loud. In a build
where the fall-back target is not compiled (`--no-default-features`), the only choices are loud
or wrong, and there a refusal is a graceful fatal instead.

**Status at HEAD: the emitter exists and nothing reaches it.** Every reachable row of the three
eligibility layers is closed — the workload corpus records 6,470 of 6,470 designs eligible with
zero refusals — so no source design produces this warning. It is built fail-closed and its
teeth are exercised by a corrupted sidecar forcing a storage-layer refusal; a new gate row will
be reported by it automatically.

**Fix:** none needed — the result is correct. Suppress with `-Wno-W-RUN-BACKEND-FALLBACK`,
promote with `-Werror=W-RUN-BACKEND-FALLBACK`.

### VITA-W4031 · `W-RUN-UNIQUE-VIOLATION` (Warning)
**A `unique` or `priority` `case`/`if` matched no branch.** The IEEE 1800-2017 §12.4.2 /
§12.5.3 violation report. `unique0` and `priority0` deliberately suppress this check and do not
fire. A statement with an `else` or a `default` cannot miss, and does not fire either.
```
logic [1:0] s = 2'b11;
unique case (s) 2'b00: ; 2'b01: ; endcase
->  m.sv:3:26: warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION: value is unhandled for priority or
    unique case statement [in m] [at time 1]
```
This is the simulator's own report, not the RTL's `$warning`, and IEEE places the two in
different clauses — §12.5.3 is a violation report, §20.10 is a severity task; one is a task the
design called and the other is a fact the tool produced. vita desugars the violation arm into a
`$warning` statement in the parser, which is why the two need distinct codes at the reporting
end: sharing one made `-Wno-W-RUN-USER-WARNING` delete every RTL `$warning` in order to silence
one benign violation, and made `-Werror=W-RUN-USER-WARNING` break CI on designs containing no
`$warning` at all.

The message text (`value is unhandled for priority or unique case statement`) is pinned to
iverilog, because the differential oracle answers with that exact string. Only the code
differs.

**Fix:** cover the missing value, add a `default`, or use `unique0`/`priority0` if the gap is
intended. Suppress with `-Wno-W-RUN-UNIQUE-VIOLATION`, promote with
`-Werror=W-RUN-UNIQUE-VIOLATION` — both independent of `$warning`.

---

## 8xxx · FILELIST

Filelist diagnostics are emitted during argv expansion, before the gate policy exists, so they
are not affected by `-Wno-`/`-Werror=` and a run that dies during expansion prints no counts
epilogue. `W-FLIST-OVERRIDE` is the exception: it is recorded during parsing and replayed
through the gated sink once the pipeline starts.

### VITA-E8001 · `E-FLIST-CYCLE` (Error)
**A filelist cycle: a `.f` already on the active stack was re-included.** A nested `-f`/`-F`
resolved, after base resolution and lexical canonicalisation, to a path already open on the
active stack. Flattening must be a tree, so a back edge is reported with the whole chain rather
than skipped silently. A diamond — the same file reached by a second branch after the first has
popped — is not a cycle.
```
# g2.f contains: -f g2.f
error[VITA-E8001] E-FLIST-CYCLE: filelist cycle: /tmp/g2.f -> /tmp/g2.f
```
**Fix:** break the cycle by removing the self or ancestor reference; put shared content in a
leaf `.f`, which makes it a legal diamond. Not suppressible; exit class 3.

### VITA-E8002 · `E-FLIST-DEPTH` (Error)
**Filelist nesting exceeded the backstop depth cap of 256.** Nesting is effectively unlimited —
the cycle guard is the real protection — but a non-cyclic runaway chain would exhaust the OS
stack, so expansion stops at 256 frames.
```
# a generated chain f0.f -> f1.f -> …  (not a cycle, just deep)
error[VITA-E8002] E-FLIST-DEPTH: filelist nesting exceeded 256 levels at 'f256.f'
```
**Fix:** flatten the generation (a generator can usually emit one or two levels), or split into
separate top-level invocations. Not suppressible; exit class 3.

### VITA-E8003 · `E-FLIST-DUP-CTX-CONFLICT` (Error)
**The same canonical source appears twice under differing inherited sticky context.** Sources
are not deduplicated in general — a duplicated module is `E-DUP-UNIT`. The same canonical path
appearing twice *is* deduplicated, but only when both occurrences carry the same inherited
sticky directive context. The tracked context is the inherited `` `timescale ``, detected by a
comment- and string-aware light scan and walked only when a duplicate actually exists. When the
two contexts differ they are not the same input, so silent dedup would drop one, and both
contexts are presented instead.
```
# a.f:  `timescale 1ns/1ps  then shared.sv
# b.f:  `timescale 1ps/1ps  then shared.sv     (same path, different inherited timescale)
error[VITA-E8003] E-FLIST-DUP-CTX-CONFLICT: rtl/shared.sv included twice under differing
  sticky context
```
**Fix:** make the two occurrences agree (the same sticky directive ahead of each, or include
the file once), or make the file self-contained by declaring its own `` `timescale ``. Not
suppressible; exit class 3.

### VITA-E8004 · `E-FLIST-GLOB` (Error)
**A glob or wildcard in a filelist is refused.** `*`, `?` and `[...]` in a source or directory
token are rejected: `readdir` order is not stable across platforms, which would make the
sticky-inheritance ordering non-deterministic and break byte-identical output across operating
systems. Expansion is never performed silently.
```
rtl/*.sv
error[VITA-E8004] E-FLIST-GLOB: wildcard '*.sv' not allowed in a filelist
```
**Fix:** list explicitly sorted paths. If the list is generated, sort it in the generator and
emit an explicit `.f`. Not suppressible; exit class 3.

### VITA-E8005 · `E-FLIST-NOT-FOUND` (Error)
**A filelist or a path it references does not exist after frame-base resolution.** The `-f`/`-F`
target, a source, or a search directory does not exist relative to its frame base — the
invocation CWD for `-f`, the `.f` file's own directory for `-F`. Canonicalisation does not
case-fold, so a path differing only in case surfaces here rather than silently aliasing on a
case-insensitive filesystem.
```
-F ./ip/Core.f                    # the real file is ip/core.f
error[VITA-E8005]: cannot read './ip/Core.f': No such file or directory (os error 2)
```
**Fix:** correct the path, including its case, or the base you expected. `-f` is CWD-relative
and `-F` is file-directory-relative, which is easy to confuse; `--dump-filelist` shows origin,
base and canonical path for each entry. Not suppressible; exit class 3.

### VITA-E8006 · `E-FLIST-UNDEF-ENV` (Error)
**A filelist references an undefined environment variable.** `$VAR`, `${VAR}` or `$(VAR)` with
no value in the environment. Substituting an empty string would produce a wrong path — often
collapsing to the filesystem root — and a different hash per environment, so it is a hard
error.
```
$RTL_ROOT/cpu/alu.sv              # RTL_ROOT not exported
error[VITA-E8006] E-FLIST-UNDEF-ENV: undefined environment variable '$RTL_ROOT'
```
**Fix:** export the variable, or use a concrete or relative path. CI should set the variables
it needs explicitly. Not suppressible; exit class 3.

### VITA-E8007 · `E-FLIST-WRONG-STAGE` (Error)
**A filelist directive belongs to a bucket the invoking stage does not own.** Every stage's
expander parses the full `.f` grammar, but a directive owned by another stage is a hard error
rather than a silent no-op. `+define+` reaching `velab` is the canonical case: `velab` has no
preprocess pass, so ignoring it would violate the user's intent.
```
$ vita velab top.vu -f elab.f       # elab.f contains +define+WIDTH=8
error[VITA-E8007]: +define+/+incdir+/-D/-I are compile-stage (vcmp/vita) inputs — 'velab' has
no preprocess pass, so they would be silently meaningless
```
The reverse is refused symmetrically:
`runtime plusargs (+FOO) are vita/vrun arguments — 'vcmp' compiles, it does not simulate`.
The same family covers `--backend` on a compile stage, `-G` on `vcmp`/`vrun`, `--obs-dir`,
`--probe` and `--obs-procs` on a staged applet, and the work-library flags on the wrong stage.

**Fix:** move the directive to the stage that owns it — `+define+`/`+incdir+`/`-D`/`-I` to
`vcmp`, `--top`/`-L` to `velab`, plusargs to `vrun` — or use one-shot `vita`, which accepts the
union. Pass `-v` to print the resolved invocation block (`invocation`, `cwd`, `filelists`,
`sources`, `defines`, `plusargs`) at the head of the transcript, which `-l`/`--log` captures to
the same file. Not suppressible; exit class 3.

### VITA-W8008 · `W-FLIST-MIXED-BASE` (Warning)
**A `-f` line inside a `-F` frame re-anchors a relocatable subtree to the CWD.** `-F` resolves
against its own directory, which is what makes a vendor IP subtree relocatable; a `-f` line
inside it re-anchors that subtree to the invocation CWD and destroys the relocatability. It is
almost always a packaging defect, but the meaning is well defined, so it is a warning.
```
# vendor.F:  -F ./rtl/core.F  /  -f ./sub/inner.f
warning[VITA-W8008] W-FLIST-MIXED-BASE: '-f ./sub/inner.f' inside a -F frame resolves against
  the invocation CWD, not the filelist directory
```
**Fix:** use `-F` for nested includes inside a `-F` tree. If CWD anchoring is intended, suppress
with `-Wno-W-FLIST-MIXED-BASE`; promote with `-Werror=`.

### VITA-W8009 · `W-FLIST-OVERRIDE` (Warning)
**A single-value knob was set twice; the last one wins.** A knob that holds one value appeared
more than once across the flattened `-f`/`-F` stream and the command line. Command-line tokens
are appended after expansion, so the command line overrides the filelist. Rather than override
silently, both values are shown. The knobs that record this are `-o`/`--out`, `--threads`/`-j`,
`--backend`, `--timeout`, `--upstream`, `--work`, `--workdir`, `-l`/`--log` and `--obs-dir`.
Accumulating flags do not: `-L`, `--top`, `-G`, `-D`, `-I`, `--hier-tree`, `--inst-paths`,
`--probe`, `--probe-file` and the verbosity flags.
```
$ vita design.sv -o o1.vcd -o o2.vcd
warning[VITA-W8009] W-FLIST-OVERRIDE: -o 'o1.vcd' overridden by 'o2.vcd' (last wins)
```
The events are collected during parsing and replayed through the gated sink at pipeline start,
so `-Wno-`/`-Werror=` apply to them and the counts epilogue includes them — unlike every other
filelist diagnostic.

**Fix:** an intended override is a supported workflow and the warning is informational. To
remove the noise, set the knob in one place; the command line is the usual home for
build intent. Promote with `-Werror=W-FLIST-OVERRIDE` for a strict CI.

---

## 9xxx · ARTIFACT / STALENESS

### VITA-E9001 · `E-ART-FORMAT-MISMATCH` (Error)
**The artifact's magic or `format_version` does not match this build.** A header-only decode,
before any body deserialisation, found a magic (`VITWORKU` or `VELAB\0`) or a `format_version`
that this tool does not expect — a foreign or damaged file, or an incompatible container
layout. The body is never read, so a misparse is impossible; the file is refused with a rebuild
hint. This is the lowest gate, below the type-shape gate `E-ART-SCHEMA-MISMATCH`.
```
$ vita vrun bad.velab
error[VITA-E9001] E-ART-FORMAT-MISMATCH: bad or missing velab magic
errors=1 warnings=0 notes=0
EXIT=2
```
Other messages under this code: `undecodable {velab|vu} header: {e}`,
`` format_version={h} but this tool expects {t}; regenerate with `velab` ``, and
`undecodable .vu source-map trailer: {e}` — the last because a tolerant fallback would stamp
every staged diagnostic at `:1:1`.

**Status at HEAD: also emitted at Fatal severity.** Two runtime sidecar guards reuse this code
as a can't-happen check: a `.velab` missing its `fork` join-mode trailer entries — a truncated
or stale artifact — ends the run immediately at Fatal rather than inventing a join mode, with
exit class 1. The primary use described above, the header-only load gate, stays Error with exit
class 2.

**Fix:** regenerate with the current tools (`vcmp`, then `velab`). Artifacts are always
regenerable, so the policy is refuse-and-rebuild with no silent migration. Not suppressible.

### VITA-E9002 · `E-ART-SCHEMA-MISMATCH` (Error)
**The artifact's `schema_hash` differs from this tool's structural type-shape hash.** The
header's `schema_hash` — the `#[derive(SchemaHash)]` structural digest specified in
[16-schema-hash-spec.md](16-schema-hash-spec.md) — differs from the value compiled into the
running tool. Adding, removing, reordering or retyping a field or variant, or changing a
wire-affecting serde attribute, flips that hash, and the mismatch is refused at the header
stage so an artifact from an incompatible shape can never be misparsed.
```
error[VITA-E9002] E-ART-SCHEMA-MISMATCH: sim-ir type shape changed between builds; rerun `velab`
```
**Fix:** rebuild with the current tools (`velab`, or `vcmp` then `velab`). Version-gate policy,
no migration machinery. Exit class 2. Not suppressible.

### VITA-E9003 · `E-ART-STALE-UPSTREAM` (Error)
**`vrun` re-hashed the live sources and the snapshot is stale (RULE V).** Two paths reach this
code. `vrun --upstream <file.vu>` re-hashes the named live artifact and compares it with the
`composite_input_hash` recorded in the `.velab`. In library mode, a `.velab` records the
digests of every manifest, compiled-unit blob, source and include it consumed, and a bare
`vrun` re-hashes all of them; any difference is stale. Modification times are never used —
content hashes only.
```
$ velab -o top.velab top.vu && vita vrun top.velab      # ok
$ echo '// edit' >> rtl/alu.sv ; vita vrun top.velab
error[VITA-E9003] E-ART-STALE-UPSTREAM: work library `w`: rtl/alu.sv changed since the .velab
  snapshot (re-run velab)
```
**Fix:** re-run the stale stage (`vcmp`, `velab`) and then `vrun`. Exit class 2 tells CI to
rebuild rather than to debug RTL. There is no silent reuse. Not suppressible.

### VITA-E9004 · `E-ART-VERSION-GATE` (Error)
**The producing tool's semver major is incompatible with the consuming tool.** The producer's
semver major, recorded in the artifact's provenance, is incompatible — independently of whether
the container format and schema hash happen to agree. Build fingerprints (git SHA, dirty flag,
profile) are provenance only and are *not* staleness keys, so a dirty tree alone does not trip
this gate.
```
error[VITA-E9004] E-ART-VERSION-GATE: produced by vitamin 2.x, this tool is 1.x; regenerate or
  install a matching vitamin
```
**Fix:** regenerate with a tool whose major matches, or install the matching vitamin. Refuse-
and-rebuild; migration is deferred until artifacts become a distribution format. Exit class 2.
Not suppressible.

### VITA-E9005 · `E-WORK-MANIFEST` (Error)
**A work-library manifest (`lib.toml`) is missing or not in canonical form.** The directory a
`-L` names has no `lib.toml`, or the file departs from the machine-written canonical form and
the strict parser refused it, or the logical name it declares differs from the one requested,
or a referenced library blob is missing. The manifest is written by `vcmp --work`; hand-editing
it changes its content hash and makes downstream snapshots stale, which is `E-ART-STALE-UPSTREAM`.
This code is the earlier failure: reading or parsing it at all.
```
$ velab -L work=./w --top cpu
error[VITA-E9005] E-WORK-MANIFEST: ./w/lib.toml: not a canonical work manifest (line 1)
```
Related messages under this code: ``./w/lib.toml: directory holds library `x` (requested `work`)``
and ``./w/…: <io error> (library blob missing — re-run `vcmp --work`)``.

**Fix:** regenerate the library with `vcmp --work`, or correct the `-L` path. Exit class 2 —
an artifact-class failure, not an RTL defect. Not suppressible.

---

## Appendix A · Reserved codes (survey inventory)

The body sections above define the 68 codes registered in the `MsgCode` enum. This appendix is
a separate inventory: 96 additional error and warning conditions defined by IEEE 1800-2017 and
IEEE 1364-2005, and by the published documentation of Verilator, Icarus iverilog, VCS, Xcelium
and GHDL. They are collected in advance so that implementing one of those conditions starts
from a named code and a cited source rather than from a blank page. **None of these codes is
registered in the enum and none can be emitted.** A code moves out of this appendix by gaining
an enum variant and a full body entry — cause, example, fix — in the same change, which is also
what puts it under the bijection gate.

What "reserved" means here differs by scope tag. A `MVP-SIM` code names a behaviour the design
requires: reserved says the prose entry and the enum row do not exist yet, not that the
behaviour is optional. That is the difference from the `LINT` tag and from the future `SVA`,
`SV-TYPE` and `VHDL` bands, which are genuinely optional or later. So when another document
refers to a rule such as partial-`` `timescale `` handling as fixed, that means the *behaviour
rule* is decided; the code itself is promoted when implemented.

Scope tags: `MVP-SIM` = a Verilog-2005 / SystemVerilog-subset simulator must handle it ·
`LINT` = style or lint, optional, many off by default in Verilator · `SVA` / `SV-TYPE` /
`VHDL` = reserved bands for later features. The `sev` column abbreviates Error as `Erro` and
Fatal as `Fata`.

Numbering follows the same governance as the body: the initial seed allotment was assigned
alphabetically within each category, and every code added since takes the next free number in
its band permanently, with no re-sorting. That is why this appendix is not in alphabetical
order. The mnemonic remains the primary key. Because these are unimplemented, some numbers may
still be merged or relocated — that latitude applies to this inventory only.

Some surveyed conditions are sub-cases of a code that already exists and were not given their
own: Verilator's `WIDTHCONNECT` (port width mismatch) belongs to `W-ELAB-WIDTH-TRUNC` (W3008),
and Verilator's `MULTIDRIVEN` for multiple-clock driving belongs to `E-ELAB-MULTIDRIVER`
(E3001). Partial `` `timescale `` specification is offered as two policy codes for one
condition — strict `E-PP-TIMESCALE-PARTIAL` (E1011) and lenient `W-PARSE-TIMESCALE-PARTIAL`
(W2016) — with lenient as the intended out-of-box default, matching iverilog's `-Wtimescale`
convention; the policy selector for that pair does not exist at HEAD, and the mixed-timescale
condition is reported today by the live `W-PP-TIMESCALE-MIXED` (W1018) at Warning. The case
where *no* module specifies one is a separate live code, `W-PP-TIMESCALE-DEFAULT` (W1017).

### 1xxx · PREPROCESS  (8)

E1004, E1005, E1013, W1007, W1008 and W1017 were promoted out of this inventory into the body.

| Number | Mnemonic | sev | scope | Condition | Source / mapping |
|---|---|---|---|---|---|
| E1006 | `E-PP-REDEFINE-DIRECTIVE` | Erro | MVP-SIM | Redefining a reserved compiler directive as a macro | IEEE 1364-2005 §19.3.1 |
| E1009 | `E-PP-UNDEF-MACRO-USE` | Erro | MVP-SIM | Use of an undefined text macro | IEEE 1364-2005 §19.3.1 |
| E1010 | `E-PP-UNBALANCED-CONDITIONAL` | Erro | MVP-SIM | Unbalanced `` `ifdef/`else/`endif `` | IEEE 1364-2005 §19.4 |
| E1011 | `E-PP-TIMESCALE-PARTIAL` | Erro | MVP-SIM | Some modules have `` `timescale `` and others do not (strict policy) | IEEE 1364-2005 §19.8 |
| E1012 | `E-PP-RESETALL-IN-MODULE` | Erro | MVP-SIM | `` `resetall `` inside a module or UDP declaration | IEEE 1364-2005 §19.6 |
| W1014 | `W-PP-IFDEF-VALUE-ZERO` | Warn | MVP-SIM | `` `ifdef `` tests a macro defined as 0 (definedness is not value) | Verilator PREPROCZERO |
| W1015 | `W-PP-BACKSLASH-SPACE` | Warn | MVP-SIM | Backslash followed by whitespace before a newline | Verilator BSSPACE |
| W1016 | `W-PP-DEF-OVERRIDE` | Warn | MVP-SIM | A command-line `+define` overrides an in-source `` `define `` | Verilator DEFOVERRIDE |

### 2xxx · PARSE  (17)

| Number | Mnemonic | sev | scope | Condition | Source / mapping |
|---|---|---|---|---|---|
| E2004 | `E-PARSE-UNSIZED-CONCAT` | Erro | MVP-SIM | Unsized operand inside a concatenation or replication | Verilator WIDTHCONCAT; IEEE 1364-2005 §5.1.14 |
| E2005 | `E-PARSE-ZERO-REPL` | Erro | MVP-SIM | Zero replication count outside an enclosing concatenation | Verilator ZEROREPL; IEEE 1800 §11.4.12.1 |
| E2006 | `E-PARSE-RESERVED-KEYWORD` | Erro | MVP-SIM | Reserved keyword used as an identifier | IEEE 1364-2005 §3.7.2 / Annex B |
| E2007 | `E-PARSE-ILLEGAL-NUMBER` | Erro | MVP-SIM | Malformed number literal | IEEE 1364-2005 §3.5.1 |
| E2008 | `E-PARSE-UNTERMINATED-TOKEN` | Erro | MVP-SIM | EOF inside a block comment or a string literal | IEEE 1364-2005 §3.3 / §3.6 |
| E2009 | `E-PARSE-END-LABEL` | Erro | MVP-SIM | Mismatched `end`/`endmodule` block label | Verilator ENDLABEL; IEEE 1800 §9.3.4 |
| E2010 | `E-PARSE-NOT-REDOP` | Erro | MVP-SIM | Logical NOT before an unparenthesised reduction operator | Verilator NOTREDOP |
| E2011 | `E-PARSE-NULL-PORTLIST` | Erro | MVP-SIM | Empty or null element in a module port list | Xcelium `*E,NULLLP`; IEEE 1364-2005 §12.3 |
| E2012 | `E-PARSE-DECL-AFTER-STMT` | Erro | MVP-SIM | Declaration after a statement (Verilog-2005) | Xcelium `*E,BADDCL`; iverilog |
| W2013 | `W-PARSE-IMPLICIT-DIMENSIONS` | Warn | MVP-SIM | Port or net redeclaration missing dimensions | iverilog `-Wimplicit-dimensions` |
| W2014 | `W-PARSE-ANACHRONISM` | Warn | MVP-SIM | Deprecated or removed feature for the selected standard | iverilog `-Wanachronisms` |
| W2015 | `W-PARSE-NEWER-STD` | Warn | MVP-SIM | Construct requires a newer language standard | Verilator NEWERSTD; iverilog `-g<year>` |
| W2016 | `W-PARSE-TIMESCALE-PARTIAL` | Warn | MVP-SIM | Some modules set `` `timescale ``, others inherit (lenient policy) | Verilator TIMESCALEMOD; iverilog `-Wtimescale` |
| W2017 | `W-PARSE-DECL-AFTER-USE` | Warn | MVP-SIM | Identifier declared after first use (tolerated) | iverilog `-Wdeclaration-after-use` |
| W2018 | `W-LINT-ASCENDING-RANGE` | Warn | LINT | Ascending `[0:N]` packed range instead of `[N:0]` | Verilator ASCRANGE / LITENDIAN |
| W2019 | `W-LINT-DECL-FILENAME` | Warn | LINT | Module name differs from the file basename | Verilator DECLFILENAME |
| W2020 | `W-LINT-MISINDENT` | Warn | LINT | Misleading indentation suggests the wrong grouping | Verilator MISINDENT |

### 3xxx · ELABORATE  (44)

E3009 and E3010 were promoted into the body as `E-ELAB-UNSUPPORTED` and
`E-ELAB-UNRESOLVED-NAME`. The reserved `E-ELAB-DUP-DECL` and `E-ELAB-IMPLICIT-NET-NONE` were
reassigned to E3005 and E3006 to avoid the collision, and the reserved E3023 was dropped
because the body's E3009 covers it.

| Number | Mnemonic | sev | scope | Condition | Source / mapping |
|---|---|---|---|---|---|
| E3005 | `E-ELAB-DUP-DECL` | Erro | MVP-SIM | Name declared twice in the same scope | IEEE 1364-2005 §4.11 / §12.3.3 |
| E3006 | `E-ELAB-IMPLICIT-NET-NONE` | Erro | MVP-SIM | Undeclared net under `` `default_nettype none `` | IEEE 1364-2005 §19.2 |
| E3011 | `E-ELAB-MIXED-PARAM-OVERRIDE` | Erro | MVP-SIM | Mixed ordered and named parameter overrides | IEEE 1364-2005 §12.2.1 |
| E3012 | `E-ELAB-OVERRIDE-LOCALPARAM` | Erro | MVP-SIM | Override targets a localparam | IEEE 1364-2005 §4.10.2 |
| E3013 | `E-ELAB-GENLOOP-NONTERMINATING` | Erro | MVP-SIM | Non-terminating generate-for loop, or genvar reuse | IEEE 1364-2005 §12.4 |
| E3014 | `E-ELAB-GENBLOCK-NAME-CONFLICT` | Erro | MVP-SIM | Generate-block name conflicts with another declaration | IEEE 1364-2005 §12.4 |
| E3015 | `E-ELAB-UWIRE-MULTIDRIVER` | Erro | MVP-SIM | `uwire` net driven by more than one source | IEEE 1364-2005 §4.6.5; IEEE 1800 §6.6 |
| E3016 | `E-ELAB-HIER-NAME-UNRESOLVED` | Erro | MVP-SIM | Hierarchical name resolves to no object | IEEE 1364-2005 §12.4 / §3.13 |
| E3017 | `E-ELAB-ASSIGN-INPUT` | Erro | MVP-SIM | Assignment to a module input port | Verilator ASSIGNIN; IEEE 1800 §23.3.3 |
| E3019 | `E-ELAB-CONTASS-INIT` | Erro | MVP-SIM | Variable both initialised and continuously assigned | Verilator CONTASSINIT |
| E3020 | `E-ELAB-PARAM-NO-DEFAULT` | Erro | MVP-SIM | Parameter without a required default | Verilator PARAMNODEFAULT |
| E3021 | `E-ELAB-FUNC-TIMING` | Erro | MVP-SIM | Time control or task call inside a function | Verilator FUNCTIMECTL; IEEE 1800 §13.4 |
| E3022 | `E-ELAB-PROTOTYPE-MISMATCH` | Erro | MVP-SIM | Out-of-block method definition disagrees with the prototype | Verilator PROTOTYPEMIS |
| E3057 | `E-ELAB-UNDEF-SYSTASK` | Erro | MVP-SIM | Call to an unrecognised system task or function | Xcelium `*E,MSSYSTF`; IEEE 1800 §20 |
| W3024 | `W-ELAB-WIDTH-EXPAND` | Warn | MVP-SIM | Rvalue narrower than the lvalue, silently zero-extended | Verilator WIDTHEXPAND |
| W3025 | `W-ELAB-WIDTH-XZEXPAND` | Warn | MVP-SIM | X/Z value expanded to a wider target | Verilator WIDTHXZEXPAND |
| W3026 | `W-ELAB-BLOCKING-MIX` | Warn | MVP-SIM | Same variable driven by both blocking and nonblocking assignments | Verilator BLKANDNBLK (Error); IEEE 1800 §4 |
| W3027 | `W-ELAB-NBA-IN-COMB` | Warn | MVP-SIM | Nonblocking assignment in a combinational block | Verilator COMBDLY; IEEE 1800 §10.4.2 |
| W3028 | `W-ELAB-NBA-IN-INITIAL` | Warn | MVP-SIM | Nonblocking assignment in an `initial` or `final` block | Verilator INITIALDLY |
| W3029 | `W-ELAB-CASE-INCOMPLETE` | Warn | MVP-SIM | `case` with no `default` that does not cover all selector values | Verilator CASEINCOMPLETE |
| W3030 | `W-ELAB-CASE-OVERLAP` | Warn | MVP-SIM | Overlapping case items (a later one unreachable) | Verilator CASEOVERLAP |
| W3031 | `W-ELAB-CASE-WITH-X` | Warn | MVP-SIM | Plain `case` item contains a literal x or z bit | Verilator CASEWITHX |
| W3032 | `W-ELAB-LATCH` | Warn | MVP-SIM | Latch inferred in a combinational block | Verilator LATCH / NOLATCH |
| W3033 | `W-ELAB-IMPLICIT-STATIC` | Warn | MVP-SIM | Implicit static lifetime on a task or function variable | Verilator IMPLICITSTATIC |
| W3034 | `W-ELAB-SELRANGE` | Warn | MVP-SIM | Constant bit or part select provably out of range | Verilator SELRANGE; iverilog `-Wselect-range` |
| W3035 | `W-ELAB-CMP-CONST` | Warn | MVP-SIM | Comparison provably always true or always false | Verilator CMPCONST |
| W3036 | `W-ELAB-UNSIGNED-CMP` | Warn | MVP-SIM | Unsigned comparison with a constant result | Verilator UNSIGNED |
| W3037 | `W-ELAB-REAL-CONVERT` | Warn | MVP-SIM | Implicit real-to-integer conversion (precision loss) | Verilator REALCVT; IEEE 1800 §6.12.2 |
| W3038 | `W-ELAB-INFINITE-LOOP` | Warn | MVP-SIM | Statically always-true loop with no exit | Verilator INFINITELOOP; iverilog `-Winfloop` (opt-in) |
| W3039 | `W-ELAB-PIN-MISSING` | Warn | MVP-SIM | Instance leaves a declared port unconnected | Verilator PINMISSING; iverilog `-Wportbind`; VCS TFIPC-L |
| W3041 | `W-ELAB-PORT-SHORT` | Warn | MVP-SIM | Module output port tied to a constant | Verilator PORTSHORT |
| W3042 | `W-ELAB-MULTITOP` | Warn | MVP-SIM | Multiple uninstantiated top modules | Verilator MULTITOP; IEEE 1800 §3.12 |
| W3043 | `W-ELAB-IGNORED-RETURN` | Warn | MVP-SIM | Non-void function called as a statement | Verilator IGNOREDRETURN |
| W3044 | `W-ELAB-NO-RETURN` | Warn | MVP-SIM | Non-void function never sets its return value | Verilator NORETURN |
| W3045 | `W-ELAB-NO-EFFECT` | Warn | MVP-SIM | Statement or expression has no observable effect | Verilator NOEFFECT |
| W3046 | `W-ELAB-ALWCOMBORDER` | Warn | MVP-SIM | `always_comb` reads a variable before assigning it | Verilator ALWCOMBORDER |
| W3047 | `W-ELAB-ALWAYS-NEVER` | Warn | MVP-SIM | `always @*` with an empty sensitivity list never triggers | Verilator ALWNEVER |
| W3048 | `W-ELAB-SENS-ENTIRE-ARRAY` | Warn | MVP-SIM | `always @*` word select pulls a whole array into the sensitivity list | iverilog `-Wsensitivity-entire-array` |
| W3049 | `W-ELAB-SENS-ENTIRE-VECTOR` | Warn | LINT | `always @*` part select pulls a whole vector into the sensitivity list | iverilog `-Wsensitivity-entire-vector` (opt-in) |
| W3050 | `W-ELAB-FLOATING-NET` | Warn | LINT | Net present in the design but with no drivers | iverilog `-Wfloating-nets` (opt-in) |
| W3052 | `W-LINT-DEFPARAM` | Warn | MVP-SIM | Deprecated `defparam` parameter override | Verilator DEFPARAM; IEEE 1364-2005 §12.2.1 |
| W3053 | `W-LINT-VAR-HIDDEN` | Warn | LINT | Variable shadows one in an enclosing scope | Verilator VARHIDDEN |
| W3054 | `W-LINT-UNUSED` | Warn | LINT | Signal, parameter or genvar unused or undriven | Verilator UNUSEDSIGNAL / UNDRIVEN / UNUSEDPARAM |
| W3055 | `W-LINT-STYLE-MISC` | Warn | LINT | Assorted off-by-default style issues (catch-all) | Verilator BLKSEQ / EOFNEWLINE / IMPORTSTAR and others |

### 4xxx · RUNTIME  (8)

The reserved `W-RUN-UNIQUE-VIOLATION` was promoted into the body and holds the number
`VITA-W4031`.

| Number | Mnemonic | sev | scope | Condition | Source / mapping |
|---|---|---|---|---|---|
| E4008 | `E-RUN-DIV-ZERO` | Erro | MVP-SIM | Integer division or modulo by zero (result x) | IEEE 1364-2005 §5.1.5 |
| E4009 | `E-RUN-ILLEGAL-SCALAR-SELECT` | Erro | MVP-SIM | Bit or part select of a scalar or a real value | IEEE 1364-2005 §4.2.1 |
| I4015 | `I-RUN-STOP` | Info | MVP-SIM | `$stop` executed (simulation suspended) | IEEE 1800 §20.2; iverilog / vvp `-n`/`-N` |
| W4010 | `W-RUN-FORMAT-MISMATCH` | Warn | MVP-SIM | Format specifier versus argument count or type mismatch | IEEE 1364-2005 §17.1.1.2 |
| W4011 | `W-RUN-WAIT-CONST` | Warn | MVP-SIM | `wait` on a compile-time constant condition | Verilator WAITCONST |
| W4012 | `W-RUN-STMT-DELAY` | Warn | MVP-SIM | Procedural statement delay under a limited delay model | Verilator STMTDLY |
| W4013 | `W-RUN-ZERO-DELAY` | Warn | MVP-SIM | `#0` zero delay (inactive-region scheduling) | Verilator ZERODLY; IEEE 1800 §15.4 |
| W4014 | `W-LINT-ASSIGN-DELAY` | Warn | LINT | Intra-assignment delay on a nonblocking assignment | Verilator ASSIGNDLY (off by default) |

### 5xxx · ASSERTION / SVA  (3)

| Number | Mnemonic | sev | scope | Condition | Source / mapping |
|---|---|---|---|---|---|
| E5001 | `E-SVA-CONCURRENT-ASSERT-FAIL` | Erro | SVA | Concurrent assertion property fails (default `$error`) | IEEE 1800-2017 §16.5 / §16.3 |
| W5002 | `W-SVA-ASSUME-COVER` | Warn | SVA | `assume` fails, or a `cover` property is never hit | IEEE 1800-2017 §16.12 / §16.13 |
| W5004 | `W-SVA-PAST-DEPTH` | Warn | SVA | `$past` delay exceeds a practical depth | Verilator TICKCOUNT; IEEE 1800 §16.9 |

### 6xxx · SV-TYPE  (7)

| Number | Mnemonic | sev | scope | Condition | Source / mapping |
|---|---|---|---|---|---|
| E6001 | `E-TYPE-ENUM-VALUE` | Erro | SV-TYPE | Enum assigned a non-member value without a cast | Verilator ENUMVALUE; IEEE 1800 §6.19 |
| E6002 | `E-TYPE-ENUM-ITEM-WIDTH` | Erro | SV-TYPE | Enum item value does not fit the enum base width | Verilator ENUMITEMWIDTH |
| E6003 | `E-TYPE-CONST-WRITTEN` | Erro | SV-TYPE | Assignment to a `const` after initialisation | Verilator CONSTWRITTEN |
| E6004 | `E-TYPE-CAST-FAILURE` | Erro | SV-TYPE | Dynamic `$cast` failure | IEEE 1800-2017 §6.24.2; Verilator CASTCONST |
| E6006 | `E-TYPE-CLASS-RULE` | Erro | SV-TYPE | SystemVerilog class or OOP rule violation | Verilator ENCAPSULATED / LIFETIME and others |
| W6005 | `W-TYPE-RANDOM-LIMIT` | Warn | SV-TYPE | Constrained-random or coverage construct unsupported or unsatisfiable | Verilator CONSTRAINTIGN / COVERIGN / RANDC |
| W6007 | `W-TYPE-REAL-CONVERT` | Warn | SV-TYPE | Real-to-integer conversion in a typed context (duplicate of W3037) | Verilator REALCVT; IEEE 1800 §6.12.2 |

### 7xxx · VHDL  (9)

A design note for this band: VHDL bound-check and overflow failures abort, whereas a Verilog
out-of-range select reads x and continues. The `E-RUN-RANGE` semantics must not be reused for
VHDL.

| Number | Mnemonic | sev | scope | Condition | Source / mapping |
|---|---|---|---|---|---|
| E7001 | `E-VHDL-NOT-DECLARED` | Erro | VHDL | VHDL name has no visible declaration | GHDL "no declaration for"; IEEE 1076 |
| E7002 | `E-VHDL-UNIT-NOT-FOUND` | Erro | VHDL | VHDL design unit not found in the library | GHDL "unit not found in library" |
| E7003 | `E-VHDL-DUP-DECLARATION` | Erro | VHDL | Identifier already used in the declarative region | GHDL "identifier already used" |
| E7004 | `E-VHDL-TYPE-MISMATCH` | Erro | VHDL | Type incompatibility or association failure | GHDL type and association errors |
| E7009 | `E-VHDL-UNRESOLVED-MULTIDRIVER` | Erro | VHDL | Multiple drivers on an unresolved-type signal | GHDL resolution-function enforcement |
| F7005 | `F-VHDL-ASSERTION-FAILURE` | Fata | VHDL | `assert`/`report` at or above the stopping severity | GHDL `--assert-level`; IEEE 1076 §8.2 |
| F7006 | `F-VHDL-BOUND-CHECK` | Fata | VHDL | Runtime constraint (bound-check) failure | GHDL "bound check failure" |
| F7007 | `F-VHDL-OVERFLOW` | Fata | VHDL | Arithmetic overflow (CONSTRAINT_ERROR) | GHDL "overflow" |
| W7008 | `W-VHDL-METAVALUE` | Warn | VHDL | NUMERIC_STD metavalue detected in a conversion | GHDL `--ieee-asserts` |

### Excluded from the inventory

Purely synthesis-only and compiled-model artefacts carry no code: Verilator GENCLK (does not
occur past 5.000), SYMRSVDWORD (a C++ keyword clash, and vitamin does no C++ code generation),
NEEDTIMINGOPT and NOTIMING (`--timing`, opt-in), UNOPTFLAT (a compiled static-schedule
performance note; a real combinational loop is covered by `F-RUN-NO-CONVERGE`), and the
multithreaded-build diagnostics BLKLOOPINIT, UNOPTTHREADS and HIERBLOCK, which have no
counterpart in an interpreter.

---

## Sources

- [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md) — severity lattice, the
  `MsgCode` scheme, gating, exit codes and RTL severity integration; the design above this
  catalogue.
- [14-staged-artifacts.md](14-staged-artifacts.md) — hashing, staleness and filelist semantics
  for the FLIST and ARTIFACT codes.
- [09-testing-and-verification.md](09-testing-and-verification.md) — how the corpus asserts on
  codes and classifies exits.
- [16-schema-hash-spec.md](16-schema-hash-spec.md) — the structural digest behind
  `E-ART-SCHEMA-MISMATCH`.
- [08-timescale-and-timing.md](08-timescale-and-timing.md) — the `1ns/1ns` base and sticky
  timescale inheritance.
- [../manual/007_error-codes.md](../manual/007_error-codes.md) — the user-facing summary;
  [../manual/004_cli-reference.md](../manual/004_cli-reference.md) — the flag surface;
  [../manual/006_limitations.md](../manual/006_limitations.md) — the supported-subset boundary.
- [hdl-reference/system-tasks/04-simulation-control.md](hdl-reference/system-tasks/04-simulation-control.md),
  [hdl-reference/system-tasks/01-display-io.md](hdl-reference/system-tasks/01-display-io.md),
  [hdl-reference/system-tasks/13-misc.md](hdl-reference/system-tasks/13-misc.md) and
  [hdl-reference/systemverilog/07-assertions-sva.md](hdl-reference/systemverilog/07-assertions-sva.md)
  — `$info` / `$warning` / `$error` / `$fatal` and assertion severity.
- IEEE 1800-2017 §16 (assertions), §20.10–20.12 (severity and elaboration tasks), §22
  (preprocessing); IEEE 1364-2005 §19.
- Appendix A sources: the Verilator warning list (https://verilator.org/guide/latest/warnings.html);
  Icarus Verilog `-W` flags
  (https://steveicarus.github.io/iverilog/usage/command_line_flags.html); the Synopsys VCS and
  Cadence Xcelium message classes; GHDL diagnostics (https://ghdl.github.io/ghdl/);
  IEEE 1800-2017 §11 (operators and widths), §12 (case and generate), §13 (tasks and functions),
  §6 (types); IEEE 1364-2005 §5 and §12; IEEE 1076-2008.
