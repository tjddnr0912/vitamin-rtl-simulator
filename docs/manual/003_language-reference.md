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
| Parameter typed by a `typedef` | Yes | `localparam my_struct_t C = '{a: 1'b1, b: 5'd3};` — vector, signed, atom (`int`/`byte`/…), packed struct (members read as constants, positional or named `'{…}`), enum (labels and `.name()`/`.next()`), scoped `pkg::t`; bindings follow `import pkg::*`. Loud in v1: an unpacked struct, a `'{…}` given as an instance override. |
| Parameter override — positional `#(8)` | Yes | |
| Parameter override — named `#(.W(8))` | Yes | |
| `generate` / `endgenerate`, `genvar` | Yes | `for`/`if`/`case` generate constructs. The `generate`/`endgenerate` keywords are **optional** (IEEE 1800-2017 §27.3), and the loop variable may be declared in the header — `for (genvar i = 0; i < 4; i++)` (§27.4). |
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
| `string` | Yes | Dynamic `string` variables with `len`/`getc`/`putc`/`substr`/`toupper`/`tolower`/`compare`, `atoi`-family and `itoa`-family conversions (the `ato*` scan takes only leading digits and underscores per IEEE 1800 §6.16.9 — no whitespace skipping, no sign, and `_` is skipped rather than terminating: `" 3".atoi()` and `"-7".atoi()` are both `0`, `"1_0".atoi()` is `10`), element indexing `s[i]` (a byte write, and since the 2026-07-31 release it also lands on a `string` declared **inside** an `automatic` task or function — those live in the call frame, and the element write used to be silently dropped there), comparisons, and `{a, b}` concatenation on assignment. String *queues* (`string sq[$]`) are not yet supported. |

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
  rate-limited `E-RUN-RANGE` (`VITA-E4002`) runtime diagnostic is emitted — or
  `W-RUN-RANGE-UNKNOWN` (`VITA-W4029`, a warning) when the index is UNKNOWN
  (x/z) rather than a known value past the end, which is ordinary during reset.
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
| Multi-dimensional packed parameter `parameter logic [N-1:0][M-1:0] P = {…}` / `localparam perm_t P = …` | Yes — body, ANSI header (default + instance override, dims may name the instance's own parameters), package (wildcard / explicit / `p::P[i]`); reads `P[i]`, `P[i][j]`, `P[i][a:b]`, `P[i][o+:w]`, `P[a:b]`, runtime or constant index, `$bits(P)` / `$bits(P[i])`. Loud: `$size`/`$left`/`$dimensions` on it, a `'{…}` value, an array parameter of such a type, a select chain deeper than the dims or with a non-final range |
| Array parameter of a struct/enum typedef `localparam st_t P[N] = '{ '{…}, … }` | Yes — 1-D, body `localparam` / package `parameter` or `localparam` / generate; element patterns positional, named or `default:`; read whole, by member, `$size`/`$bits`, `foreach`, `case`. Loud: module-body `parameter` array, multi-dimensional, a member of an element inside a constant expression |
| Array parameter in the ANSI header `module m #(parameter T A[N] = pkg::Rst, …)` | Yes — a whole-array default (`= pkg::Arr`, an imported bare name, a sibling header array; also a body `localparam L[N] = pkg::Arr`); instance override with a `'{…}` of constants, `pkg::Arr`, the parent's own array parameter, a pattern of the parent's elements, named or positional, instance arrays; elements = vectors, `int`/signed atoms, struct/enum typedefs. Loud: a nested (2-D) override pattern, an override / whole-array default of elements wider than 64 bits, `defparam` onto it, `-G` onto it, an interface-header array parameter, a multi-dimensional packed element type, `$size(A)` in a constant, an element select `A[i][a-:w]` as a child override |
| Dynamic arrays `int d[]` | Yes (`new[n]`, `new[n](src)`, `.size()`, `.delete()`, element r/w, whole-copy `b = a`) |
| Associative arrays `int a[integer]` | Yes (signed-64 key domain; `.num()`/`.exists()`/`.delete()`/`.first()`/`.next()`, whole-copy) — key-type spellings other than `[integer]`/`[time]` are not parsed yet |
| Queues `int q[$]`, bounded `[$:N]` | Yes (push/pop both ends, `.insert()`/`.delete()`, `q[$]`, bounded truncation, whole-copy `r = q`; the slice read `q[a:b]` is a loud reject) |
| Array manipulation methods (IEEE 1800 §7.12) | Yes on dynamic arrays, queues, associative arrays **and — since the 2026-08-26 release — 1-D fixed-size unpacked arrays** (`int a[4]`, `int a[3:0]`, `int a[-1:1]`): the reductions `.sum()`/`.product()`/`.and()`/`.or()`/`.xor()` with or without a `with (expr)` clause, and the ordering methods `.sort()`/`.rsort()`/`.reverse()`. `item.index` inside a `with` clause is the **declared** index, so `int a[-1:1]` yields -1, 0, 1. Still loud on a fixed array: multi-dimensional receivers (no simulator oracle agrees on whether the fold is over rows or leaves), `real`/`string`/class-handle elements, packed vectors (`logic [3:0] v; v.sum()` is not an array method), a subroutine-local array, and `.sort()` on a `wire` array (that is a procedural net write, `E3018`). |

**Assignment patterns `'{…}`** (IEEE 1800 §10.9) come in three spellings. Prefer
the NAMED one for a struct: a positional pattern is coupled to the declaration
order, so inserting a member silently shifts every later value — a wrong result,
not an error.

```systemverilog
typedef struct packed { logic [3:0] mode; logic en; logic [7:0] len; } cfg_t;

cfg_t c;
int   a [0:3];
initial begin
  c = '{4'h3, 1'b1, 8'd7};                  // positional  (§10.9)
  c = '{len: 8'd7, mode: 4'h3, en: 1'b1};   // named       (§10.9.2) — order-free
  c = '{mode: 4'h5, default: 1'b0};         // named + default (§10.9.1)
  a = '{default: 5};                        // default on a whole array
end
```

| Spelling | Status |
|---|---|
| Positional `'{e0, …}` | Yes — packed struct/union, fixed-size unpacked array (1-D and multi-dim, nested), dynamic array, queue |
| Named `'{name: v, …}` | Yes for a packed struct (and a packable unpacked record), in a procedural assignment, a declaration initializer, and a `push_back`/`push_front`/`insert` actual. Field order comes from the DECLARATION. Every member must be covered exactly once, or it is a loud reject |
| `'{default: v}` | Yes for a packed struct and for a fixed-size unpacked array (any bounds, 1-D or multi-dim). `v` is applied to each member/element separately, so it takes each one's own width — `'{default: 1'b1}` on the `cfg_t` above is `13'h0301`, not all-ones |
| Mixed `'{name: v, default: v}` | Yes (`default` covers whatever no name gave) |
| Integer key `'{0: a, 1: b}` | Loud reject |
| Type key `'{int: 0}` | Loud reject |
| Replication `'{N{e}}` | Loud reject |

A keyed pattern is also loud on a target vita cannot resolve the keys against —
a dynamic array, a queue, a packed array, a subroutine argument, a continuous
assign, or a keyed pattern nested inside a positional multi-dimensional one. It
is refused, never guessed. A call in the `default:` value is refused too, because
that value is applied once per member and a call would run once per member.

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
| `begin` / `end`, named `begin : name` | Yes | Block-local declarations supported, including `automatic` ones and sibling blocks reusing a name at any nesting depth. A block that ENCLOSES another and redeclares the same name is shadowing, which is rejected loudly. See [Limitations](006_limitations.md). |
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
| `automatic` qualifier | Yes | Per-call frame storage; recursive functions/tasks work (recursion depth is capped loudly). On a procedural block-local it is accepted where the flattening is indistinguishable from per-entry storage — a declaration initializer re-runs on each block entry, and a read that may precede the first write is rejected loudly. See [Limitations](006_limitations.md). |

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
| §7.12 array methods on a 2-D fixed array, or on a subroutine-local one | Loud reject. The 2-D form has no oracle at all (Icarus has no fixed-array method; Verilator 5.050 fails to compile it), and a subroutine-local array lives in the call frame, which the method receiver does not resolve through yet. |
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
