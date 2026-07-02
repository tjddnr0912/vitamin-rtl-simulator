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
| `interface` / `modport`, `package`, `program`, `class` | Yes | Interfaces bind as signal aliases (modport direction enforcement pending); packages with `import`; `program` blocks; classes with inheritance + virtual dispatch, parameterized classes, and constrained-random (`rand`/`constraint`/`randomize()`). |

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

Implicit port connections (`.name` and `.*`) are **supported**: `.clk` expands
to `.clk(clk)`, and `.*` auto-connects every unlisted port to the same-named
net or variable in the instantiating scope (a same-named *constant* or missing
name is a loud error, never a silent float).

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
| `string` | Yes | Dynamic `string` variables with `len`/`getc`/`putc`/`substr`/`toupper`/`tolower`/`compare`, `atoi`-family and `itoa`-family conversions, element indexing `s[i]`, comparisons, and `{a, b}` concatenation on assignment. String *queues* (`string sq[$]`) are not yet supported. |

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
| `struct` (unpacked) | Rejected loud (packed structs only) |
| `union packed` | Yes (overlay semantics; member reads/writes share storage) |
| Dynamic arrays `int d[]` | Yes (`new[n]`, `new[n](src)`, `.size()`, `.delete()`, element r/w, whole-copy `b = a`) |
| Associative arrays `int a[integer]` | Yes (signed-64 key domain; `.num()`/`.exists()`/`.delete()`/`.first()`/`.next()`, whole-copy) — key-type spellings other than `[integer]`/`[time]` are not parsed yet |
| Queues `int q[$]`, bounded `[$:N]` | Yes (push/pop both ends, `.insert()`/`.delete()`, `q[$]`, bounded truncation, whole-copy `r = q`; the slice read `q[a:b]` is a loud reject) |

---

## 3. Procedural blocks

| Block | Status | Notes |
|---|---|---|
| `initial` | Yes | Runs once at time 0. |
| `always` | Yes | General process. |
| `always_ff` | Yes | SV sequential. |
| `always_comb` | Yes | SV combinational. |
| `always_latch` | Yes | SV latch. |
| `final` | Yes | Runs once after the main loop ends, whatever the finish reason. |

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
| `casez` | Yes | Precise IEEE wildcard split — see note below. |
| `casex` | Yes | |
| `for` | Yes | |
| `while` | Yes | |
| `repeat` | Yes | |
| `forever` | Yes | |
| `begin` / `end`, named `begin : name` | Yes | Block-local declarations supported. |
| `fork` / `join` | Yes | |
| `fork` / `join_any` | Yes | |
| `fork` / `join_none` | Yes | |
| `disable name;` | Yes | Aborts the named enclosing block (loop `break`/`continue` desugar onto this machinery). |
| `disable fork;` | Yes | Cancels the calling process's forked children. |
| `#delay` (statement) | Yes | Scaled by timescale, see §8. |
| `@(event)` (statement) | Yes | |
| `wait (expr)` | Yes | Level-sensitive wait (testbench). |
| `foreach` | Yes | Fixed-size unpacked, multi-dimensional (`foreach (m[i,j])`), and dyn/queue/assoc iteration; packed-vector foreach is not supported. |
| `unique` / `priority` case·if | Yes | Runtime no-match / multi-match checks emit `VITA-W4007`; `unique0`/`priority0` are not parsed yet. |
| `do` - `while` | Yes | |

**`casez`/`casex` wildcard precision:** the IEEE split is implemented — a
`casez` bit is don't-care iff either side is `z`/`?` (an `x` never matches),
and a `casex` bit is don't-care iff either side is `x` or `z`. The remaining
positions compare 4-state exact.

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
| `automatic` qualifier | Yes | Per-call frame storage; recursive functions/tasks work (recursion depth is capped loudly). |

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

Most of the original Phase-1 deferral list has since been implemented —
intra-assignment timing (`a = #5 b;`, `q <= #1 d;` with true capture-now /
write-later semantics), `disable`-based control flow, `defparam`
(direct-child form), recursive/`automatic` subroutines, implicit ports
(`.name`/`.*`), dynamic/associative/queue storage, the `string` type,
`interface`/`package`/`program`/`class`, `final` blocks,
`foreach`/`unique`/`priority`/`do`-`while`, and instance arrays
(`dff u[3:0](...)`) all work today (each adversarially differential-tested
against Icarus Verilog or pinned to the LRM where Icarus has no support).

Still deferred or intentionally loud:

| Construct | Behavior today |
|---|---|
| Unpacked `struct` | Loud reject (packed structs and packed unions work). |
| `unique0` / `priority0` | Not parsed. |
| String queues (`string sq[$]`) | Loud reject. |
| Queue slice read (`q[a:b]`) | Loud reject (Icarus itself mis-executes this form; a hand-LRM implementation is tracked). |
| Assoc keys other than `[integer]`/`[time]` | Declaration spelling not parsed (`[int]`/`[longint]`/`[string]`/`[*]`). |
| Array `parameter`s (`parameter int P[0:3]`) | Loud reject (single-value parameter model). |
| Hierarchical function calls (`u1.f(x)`) | Loud reject. |
| `force`/`release` on a bit/part-select | Loud reject (whole nets/variables only). |
| Modport direction enforcement | Interface signals bind, but modport read/write direction is not checked. |

---

## 9. Where this matrix comes from

The Phase-1 freeze is defined by the IN-MVP / deferred table in
`docs/preview/01-goals-and-scope.md`, and the v1 simplifications by the "알려진
v1 단순화" table in the same document. This chapter additionally reflects
constructs that the parser and elaborator accept today beyond the original
freeze — notably `enum`/`typedef`/packed `struct`, `real`/`time`, the full
fork-join family, and a real control-flow `disable` — verified against
`crates/hdl-parser/src/lib.rs`, `crates/hdl-lexer/src/lib.rs`, and
`crates/elaborate/src/lib.rs`. When tools disagree, the IEEE LRM is the final
authority; when this document disagrees with the code, the code is the ground
truth and this matrix is the bug.
