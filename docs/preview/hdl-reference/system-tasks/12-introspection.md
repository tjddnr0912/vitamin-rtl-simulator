# 12 · Introspection Functions

## Overview

SystemVerilog's introspection functions are the means by which running code queries type, array and
parameter information from inside the simulation.
`$typename` returns a type's name as a string, `$cast` performs a run-time dynamic cast, and
`$isunbounded` reports whether a parameter is unbounded.
`$size`, `$left`, `$right`, `$low`, `$high`, `$increment`, `$dimensions` and
`$unpacked_dimensions` query an array's dimension information at run time.

These functions are most useful when writing type-independent, general-purpose code: parameterised
modules, generic testbenches and dynamic OOP code.

## vita support

This note describes the language, not the simulator. What vita accepts today is recorded in
[docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Type queries

### `$typename(expr_or_type)` — the type name as a string

- **Standard**: IEEE 1800-2017 §20.6
- **Returns**: string — the argument's "resolved type name"
- It accepts either an expression or a data type as its argument.

A typedef, an enum or a parameterised type all come back under their original name:

```sv
typedef logic [7:0] byte_t;
typedef enum logic [1:0] { IDLE, RUN, DONE } state_e;

byte_t      b;
state_e     s;
logic [3:0] nibble;
int         n;

$display("%s", $typename(b));       // "byte_t"
$display("%s", $typename(s));       // "state_e"
$display("%s", $typename(nibble));  // "logic [3:0]"
$display("%s", $typename(n));       // "int"
$display("%s", $typename(byte_t));  // "byte_t"  (a type may be passed directly)
```

It is useful for logging which type got bound inside a parameterised module:

```sv
module checker #(type T = logic [7:0]) (input T data);
  initial $display("data type: %s", $typename(data));
endmodule
```

**Careful**: the exact format of the returned string is implementation defined.
Parsing that string to drive branch logic destroys portability.
Use it for debugging and logging only.

**Icarus**: the built-in types work; typedef-name preservation is partial.
**Verilator**: supported (`--sv` mode).

---

### `$cast(dest, src)` — a run-time dynamic cast

- **Standard**: IEEE 1800-2017 §20.5
- Checks at run time whether the source value (src) fits the destination type (dest), then assigns.
- A static cast (`type'(expr)`) is a compile-time check, whereas `$cast` **examines the value
  itself at run time** — it looks at the actual value, not at the variable's declared type.

It has two call forms:

#### The task form

On a failed cast it raises a run-time error and leaves dest unchanged.

```sv
$cast(dest_handle, src_handle);
```

#### The function form

Returns 1 on success and 0 on failure. A failure raises no run-time error and leaves dest
unchanged. Use it when you want to write your own error message.

```sv
if (!$cast(dest_handle, src_handle))
  $error("cast failed: src type mismatch at %0t", $time);
```

**Example 1 — checking an enum range**:

```sv
typedef enum logic [1:0] { S0, S1, S2 } fsm_t;
logic [1:0] raw_val;
fsm_t       state;

// raw_val = 2'b11 fails the cast (it is not one of the enumerated values)
if (!$cast(state, raw_val))
  $error("invalid state encoding: %0b", raw_val);
else
  $display("state = %s", state.name());
```

**Example 2 — a downcast within a class hierarchy**:

```sv
class Packet;
  int id;
endclass

class EthPacket extends Packet;
  int vlan;
endclass

Packet    base_pkt;
EthPacket eth_pkt;

base_pkt = new EthPacket();   // a child object referenced through a parent handle (an upcast)

// the downcast — it succeeds only when the parent handle really points at an EthPacket
if ($cast(eth_pkt, base_pkt))
  $display("vlan=%0d", eth_pkt.vlan);
else
  $error("not an EthPacket");
```

**The essential point**: `$cast` looks at the **value, not the type**, so the same code can produce
different outcomes depending on the run-time value. The recommended pattern is always to call the
function form and check its return value.

**Icarus**: class OOP support is limited — casting across a class hierarchy is restricted; enum
casts are partially supported.
**Verilator**: fully supported, class OOP included.

---

### `$isunbounded(expr)` — is the parameter unbounded

- **Standard**: IEEE 1800-2017 §20.6
- Returns 1'b1 (true) when the argument is the unbounded value (`$`), and 1'b0 (false) otherwise.

In SystemVerilog a parameter may take `$` (unbounded) as its value.
`$isunbounded` detects that value so the code can branch on it.

```sv
module memctrl #(
  parameter int DEPTH = $,    // default: unbounded
  parameter int WIDTH = 8
) (...);

  // parameter validity check
  initial begin
    if (!$isunbounded(DEPTH) && DEPTH < 4)
      $fatal(1, "DEPTH must be >= 4 or $ (unbounded), got %0d", DEPTH);
    if (!$isunbounded(DEPTH))
      $display("finite DEPTH=%0d", DEPTH);
    else
      $display("unbounded DEPTH — dynamic sizing");
  end
endmodule
```

It is also used in combination with SVA's unbounded repetition (`##[n:$]`).

**Icarus / Verilator**: supported.

---

## Array dimension queries

### The dimension numbering (the dim argument)

Every array query function takes an optional `dim` argument. The numbering rule:

- `dim = 1`: the leftmost **unpacked** dimension
- the unpacked dimension numbers increase rightwards from there
- once the unpacked dimensions are exhausted, the **packed** dimension numbers continue, again
  left to right
- **dim omitted**: defaults to 1 (the first unpacked dimension)

The declaration below is the reference for the examples:

```sv
logic [7:0][15:0] arr [3:0][0:7];
//  unpacked: dim=1 → [3:0],  dim=2 → [0:7]
//  packed:   dim=3 → [7:0],  dim=4 → [15:0]
```

---

### `$size(arr [, dim])` — the element count

- **Standard**: IEEE 1800-2017 §20.7
- Returns the number of elements in the named dimension. Equivalent to
  `$high(arr,dim) - $low(arr,dim) + 1`.
- With dim omitted, dim = 1.

```sv
logic [7:0] byte_arr [0:3];

$size(byte_arr)      // = 4  (dim=1, unpacked)
$size(byte_arr, 1)   // = 4
$size(byte_arr, 2)   // = 8  (dim=2, the packed [7:0])
```

A dynamic array answers with its currently allocated size and a queue with its current element
count.
`$size` cannot be used on an associative array — use the `.num()` method instead.

---

### `$left(arr [, dim])` / `$right(arr [, dim])` — the declared bounds

- **Standard**: IEEE 1800-2017 §20.7
- `$left`: returns the bound written on the **left** in the declaration.
- `$right`: returns the bound written on the **right**.
- Both reflect the declared direction (up-counting or down-counting) as written.

```sv
logic [7:0] a_down [3:0];   // left=3, right=0 (down-counting)
logic [7:0] a_up   [0:3];   // left=0, right=3 (up-counting)

$left(a_down)   // = 3
$right(a_down)  // = 0
$left(a_up)     // = 0
$right(a_up)    // = 3
```

---

### `$low(arr [, dim])` / `$high(arr [, dim])` — the absolute minimum and maximum

- **Standard**: IEEE 1800-2017 §20.7
- Regardless of the declared direction, `$low` is always the **smaller** value and `$high` the
  **larger**.

```sv
logic [7:0] a_down [3:0];
logic [7:0] a_up   [0:3];

$low(a_down)    // = 0   (the smaller of 3 and 0)
$high(a_down)   // = 3
$low(a_up)      // = 0
$high(a_up)     // = 3
```

Use `$low` and `$high` to write array traversal that does not depend on the declared direction.
Use `$left` and `$right` when the declared direction itself is what you need to know.

---

### `$increment(arr [, dim])` — the index direction

- **Standard**: IEEE 1800-2017 §20.7
- Returns **1** when `$left >= $right` and **-1** when `$left < $right`.
- Used to detect the index traversal direction at run time.

```sv
logic [7:0] a_down [7:0];   // $left=7 >= $right=0 → +1
logic [7:0] a_up   [0:7];   // $left=0 < $right=7  → -1

// direction-independent traversal
for (int i = $low(arr); i <= $high(arr); i++)
  process(arr[i]);

// choosing forward or reverse using increment
initial begin
  automatic int step = $increment(arr);
  automatic int idx  = $left(arr);
  repeat ($size(arr)) begin
    process(arr[idx]);
    idx -= step;  // down-counting: -(+1) = -1 → decrement; up-counting: -(-1) = +1 → increment
  end
end
```

---

### `$dimensions(arr)` / `$unpacked_dimensions(arr)` — the dimension count

- **Standard**: IEEE 1800-2017 §20.7
- `$dimensions`: the total dimension count, packed plus unpacked.
  A 1-D scalar bit vector or a string answers 1. A non-array type answers 0.
- `$unpacked_dimensions`: the unpacked dimension count only. A packed-only array answers 0.

```sv
logic [7:0][15:0] arr [3:0][0:7];

$dimensions(arr)             // = 4 (2 unpacked + 2 packed)
$unpacked_dimensions(arr)    // = 2

logic [7:0] packed_only;
$dimensions(packed_only)     // = 1 (one packed dimension)
$unpacked_dimensions(packed_only) // = 0
```

**A generic-testbench pattern** — branch on the dimension count checked at run time:

```sv
module auto_checker #(type T = logic [7:0]) (input T dut_out, T ref_out);
  initial begin
    if ($dimensions(dut_out) > 1)
      $display("multi-dim array: %0d dims", $dimensions(dut_out));
    // per-dimension loops are written with a generate block or a recursive task
  end
endmodule
```

---

## The functions side by side

| Function | Returns | Main use |
|------|------|---------|
| `$typename(e)` | string | printing a type name while debugging |
| `$cast(dst, src)` | 1/0 (function form) | a run-time dynamic type cast |
| `$isunbounded(e)` | bit | checking whether a parameter is unbounded |
| `$size(arr [,dim])` | int | the element count of a dimension |
| `$left(arr [,dim])` | int | the left declared bound |
| `$right(arr [,dim])` | int | the right declared bound |
| `$low(arr [,dim])` | int | the absolute minimum bound |
| `$high(arr [,dim])` | int | the absolute maximum bound |
| `$increment(arr [,dim])` | 1 or -1 | the index direction |
| `$dimensions(arr)` | int | the total dimension count |
| `$unpacked_dimensions(arr)` | int | the unpacked dimension count |

---

## Icarus / Verilator support

| Function | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$typename` | built-in types supported, typedefs partial | supported (`--sv`) |
| `$cast` (enum) | partial | supported |
| `$cast` (class) | limited (OOP incomplete) | supported |
| `$isunbounded` | supported | supported |
| `$size` | supported (simple 1-dim) | supported |
| `$left/$right/$low/$high` | partial | supported |
| `$increment` | limited | supported |
| `$dimensions/$unpacked_dimensions` | limited | supported |

Icarus implements only a subset of the SV array queries. A query on a multi-dimensional array that
names a dim argument may not work, or may return the wrong value.
Verilator supports these functions reliably across its SystemVerilog subset.

---

## Synthesizability

❌ None of these functions is synthesizable — they are for simulation and verification only.
`$typename`, `$dimensions`, `$size` and the rest can be computed as elaboration-time constants, but
most synthesis tools do not recognise them, so keep them out of RTL.

---

## Sources

- IEEE 1800-2017 §20.5 ($cast), §20.6 ($typename, $isunbounded), §20.7 (array dimension query)
- research-log: [system-tasks-introspection-misc-2026-05-28.md](../../../history/research-log/system-tasks-introspection-misc-2026-05-28.md)
- [circuitcove.com — Data and Array Query Functions](https://circuitcove.com/system-tasks-query/) (WebFetch ✓)
- [vlsiverify.com — SystemVerilog Casting](https://vlsiverify.com/system-verilog/systemverilog-casting/) (WebFetch ✓)
- [siemens verificationhorizons — $cast() runtime checks](https://blogs.sw.siemens.com/verificationhorizons/2021/06/28/runtime-checks-with-the-cast-method/) (WebFetch ✓)
- [vlsi.pro — Array Querying System Functions](https://vlsi.pro/system-verilog/array-querying-system-functions/) (WebFetch ✓)
