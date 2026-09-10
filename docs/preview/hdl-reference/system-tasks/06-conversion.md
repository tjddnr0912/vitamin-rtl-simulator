# 06 · Conversion Functions

## Overview

The category of system functions that convert between integer and real, and
between signed and unsigned. `$signed`/`$unsigned` control how HDL arithmetic
interprets a sign, `$rtoi`/`$itor` convert values between integer and real, and
`$realtobits`/`$bitstoreal` are what carry an IEEE 754 bit pattern through a
port.

## Scope of this page

- `$signed`, `$unsigned` — reinterpret the signedness of an expression.
- `$rtoi`, `$itor` — convert between `integer` and `real`.
- `$realtobits`, `$bitstoreal` — move a `real` through a 64-bit vector, and the
  `shortreal` pair `$shortrealtobits`/`$bitstoshortreal`.

These notes describe the standard. For what vita accepts and how it rounds each
conversion, see
[manual/005_system-tasks.md](../../../manual/005_system-tasks.md) and
[manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Item detail

### `$signed`

- **Signature**: `$signed(expression)`
- **Standard**: IEEE 1800-2017 §20.9 / IEEE 1364-2005 §17.10 (present from Verilog 2005 on)
- **Return type**: a signed value of **the same bit width** as the input — the
  bit pattern does not change
- **Meaning**: leaves the width of the expression alone and switches only its
  interpretation to signed (two's complement). `$signed` itself neither adds nor
  removes bits.

Sign extension does not come from the function; it comes from meeting **a wider
assignment target**. When `$signed(a)` is connected to a wider wire, the
SystemVerilog assignment rules replicate the MSB and sign-extend.

If the MSB is x or z, assigning to a wider target sign-extends with x or z.

```sv
logic [7:0] u = 8'hFF;         // unsigned → 255
logic signed [7:0] s;
logic signed [15:0] wide;

s    = $signed(u);              // -1 (read as 8-bit two's complement)
wide = $signed(u);              // -1 (sign extension: 16'hFFFF)
// the top 8 bits of wide are filled with u's MSB (1)

// x propagation
logic [3:0] x_val = 4'b1xxx;
logic signed [7:0] ext;
ext = $signed(x_val);           // ext = 8'b1xxx_xxxx (sign-extended with x)
```

**A common mistake**: in an assignment of the same width, `$signed` has no
effect at all — the bit pattern is identical. Its purpose is to cast the operand
in an arithmetic context (`+`, `-`, `*`, `>>`) or a comparison where the other
expression is unsigned.

---

### `$unsigned`

- **Signature**: `$unsigned(expression)`
- **Standard**: IEEE 1800-2017 §20.9
- **Return type**: an unsigned value of **the same bit width** as the input
- **Meaning**: reinterprets the value as unsigned while keeping the bit pattern.
  Connected to a wider assignment target, **zero extension** applies.

```sv
logic signed [7:0] s = -1;      // 8'hFF, MSB=1
logic [15:0] u;

u = $unsigned(s);               // 16'h00FF (zero-extended; the MSB is no longer a sign)
// compare: assigning $signed(s) would give u = 16'hFFFF

// in a comparison
if ($unsigned(s) > 8'd200)  // s reads as 255, which is above 200 → true
    $display("unsigned comparison");
```

---

### `$rtoi`

- **Signature**: `$rtoi(real_val)` → `integer`
- **Standard**: IEEE 1800-2017 §20.9.1 / IEEE 1364-2005 §17.10
- **Return type**: `integer` (32-bit signed)
- **Meaning**: converts a real to an integer by **truncating toward zero**. It
  does not round.

| Input `real_val` | `$rtoi` result | Note |
|------------------|----------------|------|
| 192.15 | 192 | the fraction is dropped |
| 7.9 | 7 | not rounded |
| -3.9 | -3 | toward zero: -3, not -4 |
| -0.1 | 0 | |

```sv
real r = 7.8;
integer i;
i = $rtoi(r);   // i = 7 (the fractional part of 7.8 is dropped)

r = -3.9;
i = $rtoi(r);   // i = -3 (truncation is toward zero, so not -4)
```

**Note**: this is the same truncation direction as a C `(int)` cast. When
rounding is what is wanted, it has to be written out — `$rtoi(r + 0.5)`, or
`$rtoi($round(r))` (though `$round` is not a standard SV function).

---

### `$itor`

- **Signature**: `$itor(integer_val)` → `real`
- **Standard**: IEEE 1800-2017 §20.9.1 / IEEE 1364-2005 §17.10
- **Return type**: `real` (IEEE 754 double precision)
- **Meaning**: converts an integer to a real. An IEEE 754 double carries about 15
  to 16 significant digits, so a 32-bit integer is representable exactly — no
  precision is lost.

```sv
integer i = 14;
real r;
r = $itor(i);   // r = 14.0

// getting a fractional quotient
real ratio;
ratio = $itor(7) / $itor(3);   // 2.333...  (a real result, not the integer division's 2)
```

---

### `$realtobits`

- **Signature**: `$realtobits(real_val)` → `[63:0]` (a 64-bit logic vector)
- **Standard**: IEEE 1800-2017 §20.9.2 / IEEE 1364-2005 §17.10
- **Return type**: 64-bit logic vector `[63:0]`
- **Meaning**: extracts the IEEE 754 double-precision encoding **as it is** into
  a 64-bit vector. It is not a numeric conversion but a bit-level cast.

Its main use: a Verilog/SV module port cannot carry a real directly. Convert to
a 64-bit vector with `$realtobits`, pass that through the port, and restore it on
the other side with `$bitstoreal`.

```sv
// passing a real value through module ports
module sender(output logic [63:0] data_bits);
    real data = 3.14159;
    assign data_bits = $realtobits(data);
endmodule

module receiver(input logic [63:0] data_bits);
    real data;
    always_comb data = $bitstoreal(data_bits);
endmodule

// inspecting the 64-bit IEEE 754 layout (1 sign + 11 exponent + 52 mantissa)
real pi = 3.14159265358979;
logic [63:0] bits = $realtobits(pi);
// bits[63]    = 0 (positive)
// bits[62:52] = the 11-bit exponent (with the 1023 bias)
// bits[51:0]  = the 52-bit mantissa
```

---

### `$bitstoreal`

- **Signature**: `$bitstoreal(64bit_vec)` → `real`
- **Standard**: IEEE 1800-2017 §20.9.2 / IEEE 1364-2005 §17.10
- **Return type**: `real` (IEEE 754 double precision)
- **Meaning**: converts a 64-bit vector back into an IEEE 754 double — the
  inverse of `$realtobits`. If the input is not exactly 64 bits, the behaviour is
  undefined.

```sv
logic [63:0] encoded;
real decoded;

// the round trip
encoded = $realtobits(2.718281828);
decoded = $bitstoreal(encoded);
// decoded == 2.718281828  (bit-exact round trip)

// NaN and Inf patterns come back unchanged too
encoded = 64'h7FF8000000000000;  // an IEEE 754 qNaN
decoded = $bitstoreal(encoded);   // real NaN
```

**$shortrealtobits / $bitstoshortreal**: the pair that converts a `shortreal`
(IEEE 754 single precision) to and from a 32-bit vector. The same pattern, at
half the width.

---

## $signed/$unsigned vs implicit sign conversion

| Approach | Width change | Extension | Use |
|----------|--------------|-----------|-----|
| `$signed(a)` assigned to a wider target | none (the function); the assignment extends | sign extension | to use an unsigned value in signed arithmetic |
| `$unsigned(a)` assigned to a wider target | none (the function); the assignment extends | zero extension | to use a signed value in unsigned arithmetic |
| implicit conversion | context-dependent | depends on the type rules | intent is unclear; avoid |

---

## Icarus / Verilator support

| Function | Icarus | Verilator |
|----------|--------|-----------|
| `$signed` | fully supported | generally supported |
| `$unsigned` | fully supported | generally supported |
| `$rtoi` | fully supported | generally supported |
| `$itor` | fully supported | generally supported |
| `$realtobits` | fully supported | generally supported |
| `$bitstoreal` | fully supported | generally supported |

---

## Synthesizability

| Function | Synthesizable |
|----------|---------------|
| `$signed`, `$unsigned` | ✅ — synthesizable (interpretation only; no logic is added) |
| `$rtoi`, `$itor` | ❌ — not synthesizable (real arithmetic) |
| `$realtobits`, `$bitstoreal` | ❌ — not synthesizable (real-typed operations) |

---

## Sources

- IEEE 1800-2017 §20.9 (conversion system functions)
- IEEE 1364-2005 §17.10 (Verilog 2005; absorbed into 1800)
- research-log: [system-tasks-conversion-math-2026-05-28.md](../../../history/research-log/system-tasks-conversion-math-2026-05-28.md)
- [circuitcove.com — Conversion Functions](https://circuitcove.com/system-tasks-conversion/)
- [chipverify.com — Verilog Conversion Functions](https://chipverify.com/verilog/verilog-conversion-functions)
- [hdlworks.com — System Real Conversion Functions](https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemRealConversionFuncs.htm)
- [01signal.com — Signed Arithmetic](https://www.01signal.com/verilog-design/arithmetic/signed-wire-reg/)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
