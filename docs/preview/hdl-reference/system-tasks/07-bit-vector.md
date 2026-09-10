# 07 · Bit-Vector Query Functions

## Overview

This category covers the system functions that query a bit vector: its size, its
bit counts, one-hot patterns, and whether it holds unknown bits.
`$bits` and `$clog2` evaluate to an elaboration-time constant, so they may appear in
parameter declarations.
`$countbits`, `$countones`, `$onehot`, `$onehot0` and `$isunknown` are run-time queries over
4-state values.

## vita support

This note describes the language, not the simulator. What vita accepts today is recorded in
[docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Entries

### `$bits`

- **Signature**: `$bits(type_or_expression)` → `integer`
- **Standard**: IEEE 1800-2017 §20.6
- **Return type**: `integer` (an elaboration-time constant)
- **Meaning**: returns the **bit width** of an expression or a data type.
  Unlike C's `sizeof()`, which counts bytes, this is a count of bits.

Its distinguishing feature is that it accepts a **type name directly** as its argument.
It also accepts an expression (a signal or a variable). Both forms evaluate to an
elaboration-time constant.

```sv
// a type name used directly as the argument
$bits(logic)              // 1
$bits(logic [7:0])        // 8
$bits(integer)            // 32

typedef struct packed {
    logic [7:0] addr;
    logic [15:0] data;
    logic        valid;
} bus_t;
$bits(bus_t)              // 25 (8+16+1)

// used with an expression
logic [3:0] nibble;
$bits(nibble)             // 4

// used in a parameter declaration
parameter int DATA_W = 32;
logic [$bits(logic [DATA_W-1:0])-1:0] buffer;  // a buffer DATA_W bits wide
```

**Arrays**: a packed array answers with its total bit count. The result for an unpacked array
is simulator defined.

---

### `$clog2`

- **Signature**: `$clog2(unsigned_val)` → `integer`
- **Standard**: IEEE 1800-2017 §20.8.1 (it sits in the integer math functions section)
- **Return type**: `integer` (signed — stated explicitly in IEEE Verilog 2005 §17.11.1)
- **Meaning**: `ceiling(log₂(N))`, that is the **minimum number of bits** needed to hold a binary
  index over N items. The argument is treated as an unsigned value.

| Input N | `$clog2(N)` | Note |
|--------|-------------|------|
| 0 | **0** | IEEE 1800-2017 §20.8.1 states: "argument value of 0 shall produce a result of 0" |
| 1 | **0** | log₂(1) = 0, ceiling(0) = 0 |
| 2 | 1 | |
| 3 | 2 | log₂(3) ≈ 1.585 → ceiling = 2 |
| 4 | 2 | log₂(4) = 2 (an exact power of two) |
| 5–8 | 3 | |
| 100 | 7 | 2⁷ = 128 > 100 |
| 1024 | 10 | an exact power of two → log = ceiling |

```sv
// computing a memory address bus width
parameter int MEM_DEPTH = 256;
logic [$clog2(MEM_DEPTH)-1:0] addr;   // [7:0] — 8 bits

// FIFO depth → pointer width
parameter int FIFO_DEPTH = 100;
logic [$clog2(FIFO_DEPTH):0] wr_ptr;  // +1 to distinguish full from empty

// the $clog2(1) = 0 edge case: a 1-entry memory needs a 0-bit address
parameter int SINGLE = 1;
// $clog2(SINGLE) = 0 → [0-1:0] = [-1:0] — some simulators make this a 0-bit wire, others an error
// defensive pattern: $clog2(DEPTH > 1 ? DEPTH : 2)
```

**The return-type dispute**: the IEEE standard states `integer` (signed), but some tools —
Yosys among them — return an unsigned value.
A subtraction such as `$clog2(N) - 1` written without a signed context can therefore underflow
when N = 1.
Safe patterns: `int'($clog2(N)) - 1`, or `$clog2(N) > 0 ? $clog2(N) - 1 : 0`.

---

### `$countones`

- **Signature**: `$countones(expression)` → `int`
- **Standard**: IEEE 1800-2017 §20.9
- **Return type**: `int` (32-bit signed integer)
- **Meaning**: returns the **number of 1-valued bits** in the expression.
  Shorthand for `$countbits(expression, 1'b1)`.
  x and z bits are **not** counted (they are not 1).

```sv
logic [7:0] v = 8'b1010_1100;
$countones(v)            // 4 (bits 7, 5, 3 and 2 are 1)

logic [3:0] x_val = 4'b1x01;
$countones(x_val)        // 2 (bits 3 and 0 are 1; bit 2 is x, so it is not counted)

// Hamming weight
logic [15:0] data;
int weight;
always_comb weight = $countones(data);
```

---

### `$countbits`

- **Signature**: `$countbits(expression, control_bit [, control_bit ...])` → `int`
- **Standard**: IEEE 1800-2017 §20.9
- **Return type**: `int`
- **Meaning**: returns the **number of bits in the expression that match** one of the given bit
  values (0, 1, x, z).
  Each control_bit is a 1-bit logic value; listing several sums their counts.
  This is the function that covers the x and z states of a 4-state vector in full.

```sv
logic [7:0] v = 8'b1010_xxzz;

$countbits(v, 1'b1)      // 2 (bits 7 and 5)
$countbits(v, 1'b0)      // 2 (bits 6 and 4 — x and z do not match)
$countbits(v, 1'bx)      // 2 (bits 3 and 2)
$countbits(v, 1'bz)      // 2 (bits 1 and 0)

// summing several control bits
$countbits(v, 1'bx, 1'bz)  // 4 (the x count plus the z count)

// equivalent to $isunknown
$countbits(v, 1'bx, 1'bz) != 0   // same result as $isunknown(v)

// 4-state diagnosis: the four counts must add up to the width
logic [3:0] w = 4'b10xz;
// $countbits(w, 0) + $countbits(w, 1) + $countbits(w, 'x) + $countbits(w, 'z) == 4
```

---

### `$onehot`

- **Signature**: `$onehot(expression)` → `bit`
- **Standard**: IEEE 1800-2017 §20.9
- **Return type**: `bit` (0 or 1)
- **Meaning**: returns 1 when **exactly one** bit of the vector is 1.
  Mostly used to check FSM one-hot encodings and in assertions.

Handling of x and z bits: when the vector contains x or z there is no unambiguous one-hot answer,
and most simulators return 0.
IEEE 1800-2017 does not state this case, so it is implementation defined.

```sv
logic [3:0] fsm_state;

// the FSM one-hot assertion pattern
always_ff @(posedge clk) begin
    assert ($onehot(fsm_state)) else
        $error("FSM not one-hot: %b", fsm_state);
end

$onehot(4'b0001)    // 1 (only bit 0 is set)
$onehot(4'b0100)    // 1 (only bit 2 is set)
$onehot(4'b0000)    // 0 (no bit is set)
$onehot(4'b0011)    // 0 (two bits are set)
$onehot(4'b01x1)    // 0 or x (implementation defined)
```

---

### `$onehot0`

- **Signature**: `$onehot0(expression)` → `bit`
- **Standard**: IEEE 1800-2017 §20.9
- **Return type**: `bit`
- **Meaning**: returns 1 when **at most one** bit is 1.
  It differs from `$onehot` in that an all-zero vector also answers 1.
  This is the "one-hot or all-zero" check.

```sv
$onehot0(4'b0001)   // 1 (one bit set)
$onehot0(4'b0000)   // 1 (no bit set — allowed)
$onehot0(4'b0011)   // 0 (two bits set — a violation)
```

| Pattern | `$onehot` | `$onehot0` |
|------|-----------|------------|
| all-zero (0000) | 0 | **1** |
| one-hot (0010) | 1 | 1 |
| two-hot (0110) | 0 | 0 |

---

### `$isunknown`

- **Signature**: `$isunknown(expression)` → `bit`
- **Standard**: IEEE 1800-2017 §20.9
- **Return type**: `bit`
- **Meaning**: returns 1 when **any bit** of the expression is x or z.
  Equivalent to `$countbits(e, 1'bx, 1'bz) != 0`.
  Commonly used to check that a 4-state signal is valid, and as a guard ahead of an assertion.

```sv
logic [3:0] sig;

// check validity before asserting
always @(posedge clk) begin
    if (!$isunknown(sig))
        assert (sig != 4'd0) else $error("zero value");
end

$isunknown(4'b0000)    // 0 (all bits known)
$isunknown(4'b1010)    // 0
$isunknown(4'b10x0)    // 1 (contains x)
$isunknown(4'b10z0)    // 1 (contains z)
$isunknown(4'bxxxx)    // 1
```

---

## How $countbits, $countones and $isunknown relate

```
$countones(v)            ≡ $countbits(v, 1'b1)
$isunknown(v)            ≡ $countbits(v, 1'bx, 1'bz) != 0
$onehot(v)               ≡ $countbits(v, 1'b1) == 1
$onehot0(v)              ≡ $countbits(v, 1'b1) <= 1
```

---

## Icarus / Verilator support

| Function | Icarus | Verilator |
|------|--------|-----------|
| `$bits` | full | Generally supported |
| `$clog2` | full | Generally supported |
| `$countones` | full | Generally supported |
| `$countbits` | full | Generally supported |
| `$onehot` | full | Generally supported |
| `$onehot0` | full | Generally supported |
| `$isunknown` | full | Generally supported |

---

## Synthesizability

| Function | Synthesis |
|------|------|
| `$bits`, `$clog2` | ✅ — elaboration-time constants, synthesizable |
| `$countones` | ✅ — supported by most synthesis tools |
| `$countbits`, `$onehot`, `$onehot0` | ⚠️ — tool- and version-dependent (FPGA tools usually support them) |
| `$isunknown` | ❌ — x and z are simulation-only states (removed during synthesis) |

---

## Sources

- IEEE 1800-2017 §20.6 ($bits), §20.8.1 ($clog2), §20.9 (count/onehot/isunknown)
- research-log: [system-tasks-conversion-math-2026-05-28.md](../../../history/research-log/system-tasks-conversion-math-2026-05-28.md)
- [circuitcove.com — Bit Vector Functions](https://circuitcove.com/system-tasks-vector/) (WebFetch ✓)
- [circuitcove.com — $clog2](https://circuitcove.com/system-tasks-clog2/) (WebFetch ✓)
- [systemverilog.io — Ten Utilities](https://www.systemverilog.io/verification/ten-utilities/) (WebFetch ✓)
- [YosysHQ/yosys issue #708 — $clog2 return type](https://github.com/YosysHQ/yosys/issues/708) (WebFetch ✓)
- [chipverify.com — Verilog Math Functions](https://chipverify.com/verilog/verilog-math-functions)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
