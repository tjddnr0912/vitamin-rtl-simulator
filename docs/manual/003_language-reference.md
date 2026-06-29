# Language Reference

This chapter is the authoritative support matrix for **vitamin** Phase-1: exactly
which Verilog (IEEE 1364) and SystemVerilog (IEEE 1800) constructs the simulator
accepts and elaborates today. Phase-1 targets the **synthesizable
SystemVerilog RTL subset** and includes all of Verilog-2005 RTL, plus the
simulation-only constructs (`initial`, `#delay`, `$display`, `$finish`, …) that
a simulator cannot do without.

If a construct is not listed here as supported, treat it as **not yet
implemented**. When a synthesizable construct is missing that you need, it is a
gap, not a design choice — file it.

> **Platforms:** vitamin builds and runs on **Linux and macOS** only. Windows is
> out of scope. See [Installation](001_installation.md).

Related chapters: [Installation](001_installation.md) ·
[Quick Start](002_quickstart.md) ·
[CLI Reference](004_cli-reference.md) ·
[System Tasks](005_system-tasks.md).

Legend: **Yes** = supported · **Partial** = supported with a documented
simplification · **Deferred** = parsed-and-ignored or rejected, planned for
Phase-2.

---

## 1. Design units

| Construct | Status | Notes |
|---|---|---|
| `module` / `endmodule` | Yes | The top-level design unit. |
| ANSI port headers | Yes | `module m (input logic a, output reg [7:0] q);` |
| Non-ANSI port headers | Yes | `module m (a, q); input a; output [7:0] q;` |
| `parameter` | Yes | Overridable at instantiation. |
| `localparam` | Yes | Not overridable. |
| Parameter override — positional `#(8)` | Yes | |
| Parameter override — named `#(.W(8))` | Yes | |
| `generate` / `endgenerate`, `genvar` | Yes | `for`/`if`/`case` generate constructs. |
| `function` / `endfunction` | Yes | See §6. |
| `task` / `endtask` | Yes | See §6. |
| Module instantiation & hierarchy | Yes | Named (`.p(x)`) and positional port maps; arbitrary nesting. |
| `interface` / `modport`, `package`, `program`, `class` | Deferred | Phase-2. |

### Ports

Both port styles are accepted:

```systemverilog
// ANSI
module adder #(parameter W = 8)
              (input  logic [W-1:0] a, b,
               output logic [W-1:0] sum);
  assign sum = a + b;
endmodule

// Non-ANSI
module adder (a, b, sum);
  parameter W = 8;
  input  [W-1:0] a, b;
  output [W-1:0] sum;
  assign sum = a + b;
endmodule
```

Implicit port connections (`.*` and `.name`) are **deferred** — `.*` is parsed
but ignored with a diagnostic; write the ports explicitly.

---

## 2. Data types

### Scalar / net types

All of the following declaration keywords are accepted:

`wire`, `tri`, `wand`, `triand`, `wor`, `trior`, `tri0`, `tri1`, `supply0`,
`supply1`, `trireg`, `uwire`, `reg`, `logic`, `integer`, `time`, `real`,
`realtime`.

| Type | Status | Notes |
|---|---|---|
| `wire` / `tri` and net families | Yes | 4-state. |
| `reg` | Yes | 4-state procedural variable. |
| `logic` | Yes | 4-state (SystemVerilog); behaves as `reg`/`wire` per context. |
| `integer` | Yes | 32-bit signed, 4-state. |
| `time` | Yes | 64-bit, 4-state. |
| `real` / `realtime` | Yes | IEEE-754 f64, 2-state. `realtime` is a synonym. |
| `signed` / `unsigned` qualifier | Yes | e.g. `reg signed [7:0] x;`, `'sd5`. |
| `string` | Deferred | String literals as `$display` arguments work; the `string` type does not. |

`real`/`realtime` support includes the conversion system functions `$rtoi`,
`$itor`, `$realtobits`, `$bitstoreal`.

### Vectors and arrays

| Construct | Status | Notes |
|---|---|---|
| Packed vector `[7:0]` | Yes | Ascending or descending ranges. |
| Multi-dimensional **packed** array `[3:0][7:0]` | Yes | |
| Multi-dimensional **unpacked** array `mem [0:255]`, `m [4][8]` | Yes | See simplifications below. |
| 4-state X/Z values | Yes | Word-parallel (u64) bitwise/reduction acceleration. |

**Documented simplifications** (intentional v1 behavior, deterministic and
documented):

- **Out-of-range index:** read yields all-X; write is ignored (not clamped). A
  rate-limited `E-RUN-RANGE` (`VITA-E4002`) runtime diagnostic is emitted.
- **X/Z index:** read yields all-X; write is a no-op.
- **Unpacked sub-dimension index** is aliased into the flat element space
  (per-dimension bounds are not separately checked; the low end is normalized).
- **Partial unpacked slices** — indexing an unpacked array with *fewer* indices
  than it has dimensions (`g[i]` on a 2-D array) is a **loud reject**
  (`VITA-E3009`): supply every dimension.
- **`>128`-bit unsigned / `>64`-bit signed arithmetic** is poisoned to X
  (fail-safe) rather than computed with full multi-word precision.

### SystemVerilog user-defined types

vitamin implements three SV type constructs in Phase-1: **`enum`**, **`typedef`**,
and **packed `struct`**.

**`typedef enum`** — labels lower to integer constants (first = 0, then
incrementing, with explicit `= expr` overrides); the underlying storage is `int`
(32-bit signed):

```systemverilog
typedef enum { RED, GREEN, BLUE } color_t;   // RED=0, GREEN=1, BLUE=2
color_t c;
initial c = GREEN;
```

**`typedef`** — names an underlying type so `T x;` declares a variable of that
type:

```systemverilog
typedef logic [7:0] byte_t;
byte_t data;
```

**Packed `struct`** — members are packed MSB-first into one flat vector; field
access (`s.field`) lowers to a constant part-select:

```systemverilog
typedef struct packed {
  logic [3:0] hi;
  logic [3:0] lo;
} nibble_pair_t;

nibble_pair_t p;
initial begin
  p.hi = 4'hA;
  p.lo = 4'h5;   // p is now 8'hA5
end
```

| SV type | Status |
|---|---|
| `enum` (via `typedef enum`) | Yes |
| `typedef` | Yes |
| `struct packed` | Yes |
| `struct` (unpacked) / `union` | Deferred (Phase-2) |
| Dynamic arrays, associative arrays, queues | Deferred (Phase-2) |

---

## 3. Procedural blocks

| Block | Status | Notes |
|---|---|---|
| `initial` | Yes | Runs once at time 0. |
| `always` | Yes | General process. |
| `always_ff` | Yes | SV sequential. |
| `always_comb` | Yes | SV combinational. |
| `always_latch` | Yes | SV latch. |
| `final` | Deferred | Phase-2. |

### Sensitivity / event control

| Form | Status | Example |
|---|---|---|
| Implicit `@*` / `@(*)` | Yes | `always @* y = a & b;` |
| Edge list | Yes | `always @(posedge clk or negedge rst_n)` |
| Level list | Yes | `always @(a or b or sel)` |

---

## 4. Statements

| Statement | Status | Notes |
|---|---|---|
| Blocking assign `=` | Yes | |
| Non-blocking assign `<=` | Yes | |
| `if` / `else` | Yes | |
| `case` | Yes | |
| `casez` | Yes (Partial) | See wildcard note. |
| `casex` | Yes (Partial) | See wildcard note. |
| `for` | Yes | |
| `while` | Yes | |
| `repeat` | Yes | |
| `forever` | Yes | |
| `begin` / `end`, named `begin : name` | Yes | Block-local declarations supported. |
| `fork` / `join` | Yes | |
| `fork` / `join_any` | Yes | |
| `fork` / `join_none` | Yes | |
| `disable name;` | Parsed (no-op) | Parses and elaborates but does **not** abort control flow; emits a warning. See [Limitations](006_limitations.md). |
| `disable fork;` | Parsed (no-op) | Same — parsed but does not cancel forked processes in v1. |
| `#delay` (statement) | Yes | Scaled by timescale, see §8. |
| `@(event)` (statement) | Yes | |
| `wait (expr)` | Yes | Level-sensitive wait (testbench). |
| `foreach`, `unique`/`priority`, `do`-`while` | Deferred | Phase-2. |

**`casez`/`casex` wildcard simplification (v1):** both currently mask *every*
x/z in the scrutinee and the label as don't-care
(`reduction_or(scrut ^ label) !== 1`). The precise IEEE split — `casez` matching
only `z`/`?`, `casex` matching `x`/`z` — is a Phase-1.x refinement.

### Continuous assignment

| Construct | Status | Notes |
|---|---|---|
| `assign lhs = rhs;` | Yes | |
| `assign #d lhs = rhs;` | Yes (Partial) | Transport delay only (no inertial pulse rejection). |

---

## 5. Operators

The full Verilog precedence table is implemented (Pratt expression parser,
verified against the HDL reference 14-level table; level 1 binds tightest).

| Class | Operators | Status |
|---|---|---|
| Arithmetic | `+` `-` `*` `/` `%` `**` | Yes |
| Bitwise | `&` `|` `^` `~` `~^` / `^~` | Yes |
| Reduction (unary) | `&` `~&` `|` `~|` `^` `~^` | Yes |
| Logical | `&&` `||` `!` | Yes |
| Relational | `<` `<=` `>` `>=` | Yes |
| Equality | `==` `!=` `===` `!==` | Yes |
| Shift — logical | `<<` `>>` | Yes |
| Shift — arithmetic | `<<<` `>>>` | Yes |
| Ternary | `?:` | Yes |
| Concatenation | `{a, b, c}` | Yes |
| Replication | `{N{x}}` | Yes |
| Bit select | `x[i]` | Yes |
| Part select | `x[m:l]` | Yes |
| Indexed part select | `x[base+:w]` / `x[base-:w]` | Yes |

The `**` power operator (including `2**N` width computations) is supported.

---

## 6. Functions and tasks

| Feature | Status | Notes |
|---|---|---|
| `function` with return value | Yes | ANSI or non-ANSI ports; range and `signed` qualifiers. |
| `task` (may consume time) | Yes | |
| Local declarations in func/task bodies | Yes | |
| `automatic` qualifier | Deferred | The keyword is parsed but vitamin does not provide per-call automatic storage; recursive func/task relying on it is Phase-2. |

---

## 7. Timescale and timing

`` `timescale `` is a preprocessor directive at file top level, outside any
module:

```systemverilog
`timescale 1ns / 1ps
```

| Feature | Status | Notes |
|---|---|---|
| `` `timescale unit/precision `` | Yes | Full per-module model (doc-08). |
| `#delay` scaling by unit | Yes | Delays scale to the global precision-based 64-bit time axis. |
| `$time` | Yes | Scaled to the calling module's unit (per-process, accurate across mixed timescales). |
| `$realtime` | Yes | Real-valued time in the calling module's unit. |
| Mixed timescales across modules | Yes | A single global integer time axis is maintained; modules with different timescales stay consistent. |

VCD waveform output is produced **only** when the RTL calls a dump system task
(`$dumpfile`, `$dumpvars`, …); there is no always-on dumping. See
[System Tasks](005_system-tasks.md) and [CLI Reference](004_cli-reference.md).

---

## 8. Deferred to Phase-2

The following are explicitly **not** supported in Phase-1. Where noted as
*parsed-and-ignored*, vitamin emits an advisory diagnostic and continues; the
construct has no effect.

| Construct | Behavior today |
|---|---|
| Unpacked `struct` / `union` | Rejected / unsupported. |
| Intra-assignment timing (`a = #5 b;`, `q <= #1 d;`) | Parsed-and-ignored (the `#delay`/`@event` after `=`/`<=` is discarded with a warning). |
| `disable`-based control flow | `disable name;` / `disable fork;` **parse but are a no-op** (control flow is not aborted; a warning is emitted). See [Limitations](006_limitations.md). |
| `defparam` | Deferred; use parameter overrides at instantiation. |
| Recursive / `automatic` functions and tasks | `automatic` keyword tolerated, semantics deferred. |
| SV implicit ports `.*` / `.name` | `.*` parsed-and-ignored with a diagnostic; connect ports explicitly. |
| Dynamic arrays, associative arrays, queues | Deferred. |
| `string` type | Deferred (string *literals* as task arguments still work). |
| `interface` / `modport`, `package`, `program`, `class` | Deferred. |
| `final` blocks | Deferred. |
| `foreach`, `unique`/`priority`, `do`-`while` | Deferred. |
| Instance arrays (`dff u[3:0](...)`) | Loud reject (`VITA-E3009`), Phase-1.x. |

---

## 9. Where this matrix comes from

The Phase-1 freeze is defined by the IN-MVP / deferred table in
`docs/preview/01-goals-and-scope.md`, and the v1 simplifications by the "알려진
v1 단순화" table in the same document. This chapter additionally reflects
constructs that the parser and elaborator accept today beyond the original
freeze — notably `enum`/`typedef`/packed `struct`, `real`/`time`, and the full
fork-join family (`disable` parses but is a no-op) — verified against
`crates/hdl-parser/src/lib.rs`, `crates/hdl-lexer/src/lib.rs`, and
`crates/elaborate/src/lib.rs`. When tools disagree, the IEEE LRM is the final
authority; when this document disagrees with the code, the code is the ground
truth and this matrix is the bug.
