# 08 · Math Functions

## Overview

This category covers the math functions that operate on IEEE 754 double-precision reals.
Every function in it takes and returns `real`, and each corresponds one-to-one with a routine
in the C standard math library (`<math.h>`). They are simulation-only and not synthesizable.

## vita support

This note describes the language, not the simulator. What vita accepts today is recorded in
[docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Entries

### The complete function list

Every function here returns `real` (IEEE 754 double precision).
The exception in this clause is `$clog2`: it belongs to §20.8.1 but returns an integer, so it is
covered in [07-bit-vector.md](07-bit-vector.md).

| Function | Signature | C equivalent | Group |
|------|---------|--------|------|
| `$ln` | `$ln(x: real)` → real | `log(x)` | logarithm |
| `$log10` | `$log10(x: real)` → real | `log10(x)` | logarithm |
| `$exp` | `$exp(x: real)` → real | `exp(x)` | exponential |
| `$sqrt` | `$sqrt(x: real)` → real | `sqrt(x)` | square root |
| `$pow` | `$pow(x: real, y: real)` → real | `pow(x, y)` | power |
| `$floor` | `$floor(x: real)` → real | `floor(x)` | round down |
| `$ceil` | `$ceil(x: real)` → real | `ceil(x)` | round up |
| `$sin` | `$sin(x: real)` → real | `sin(x)` | trigonometric |
| `$cos` | `$cos(x: real)` → real | `cos(x)` | trigonometric |
| `$tan` | `$tan(x: real)` → real | `tan(x)` | trigonometric |
| `$asin` | `$asin(x: real)` → real | `asin(x)` | inverse trigonometric |
| `$acos` | `$acos(x: real)` → real | `acos(x)` | inverse trigonometric |
| `$atan` | `$atan(x: real)` → real | `atan(x)` | inverse trigonometric |
| `$atan2` | `$atan2(y: real, x: real)` → real | `atan2(y, x)` | inverse trigonometric (quadrant aware) |
| `$sinh` | `$sinh(x: real)` → real | `sinh(x)` | hyperbolic |
| `$cosh` | `$cosh(x: real)` → real | `cosh(x)` | hyperbolic |
| `$tanh` | `$tanh(x: real)` → real | `tanh(x)` | hyperbolic |
| `$hypot` | `$hypot(x: real, y: real)` → real | `hypot(x, y)` | √(x²+y²) |
| `$asinh` | `$asinh(x: real)` → real | `asinh(x)` | inverse hyperbolic |
| `$acosh` | `$acosh(x: real)` → real | `acosh(x)` | inverse hyperbolic |
| `$atanh` | `$atanh(x: real)` → real | `atanh(x)` | inverse hyperbolic |

---

### Logarithm and exponential (`$ln`, `$log10`, `$exp`)

```sv
real x = 2.71828182845904523536;  // e

$ln(x)         // ≈ 1.0
$ln(1.0)       // 0.0
$log10(100.0)  // 2.0
$log10(1.0)    // 0.0
$exp(1.0)      // ≈ 2.71828...
$exp(0.0)      // 1.0
```

**Domain errors**: IEEE 1800-2017 leaves the behaviour on a domain error **implementation
defined**. In a simulator that delegates to the C runtime's IEEE 754 behaviour the results are:

| Call | Expected result | Reason |
|------|----------|------|
| `$ln(0.0)` | −∞ | C `log(0)` = -INFINITY |
| `$ln(-1.0)` | NaN | the logarithm of a negative number is outside the reals |
| `$log10(0.0)` | −∞ | same |
| `$exp(1000.0)` | +∞ | double overflow |

These are IEEE 754 special values (NaN, ±Inf), so they propagate through every later operation.
When a production testbench suspects a domain error, check the result with `$isnan()` — but note
that `$isnan` is not a standard SV function, so use the Verilog-A extension or a direct comparison
(`r != r` is true only for NaN).

---

### `$sqrt`

- **Signature**: `$sqrt(x: real)` → `real`
- **Standard**: IEEE 1800-2017 §20.8.2
- **Meaning**: the square root of a non-negative `x`. Equivalent to C `sqrt()`.

```sv
$sqrt(4.0)     // 2.0
$sqrt(2.0)     // ≈ 1.41421356...
$sqrt(0.0)     // 0.0

// domain error
$sqrt(-1.0)    // NaN (the C sqrt rule — implementation defined)
```

**Precision**: IEEE 754 double precision — roughly 15 to 16 significant digits, which is ample for
computing digital-design parameters.

---

### `$pow`

- **Signature**: `$pow(x: real, y: real)` → `real`
- **Standard**: IEEE 1800-2017 §20.8.2
- **Meaning**: `x` raised to the power `y`. Equivalent to C `pow(x, y)`.

```sv
$pow(2.0, 10.0)    // 1024.0
$pow(2.0, -1.0)    // 0.5
$pow(4.0, 0.5)     // 2.0 (the same as $sqrt(4.0))
$pow(0.0, 0.0)     // 1.0 (the mathematical convention: 0^0 = 1)

// domain errors
$pow(-2.0, 0.5)    // NaN (a negative base to a non-integer power — the C pow rule)
$pow(0.0, -1.0)    // +∞ (the C pow rule)
```

**The SV `**` operator**: SystemVerilog also provides `x ** y`. On integer types `a ** b` can
behave differently from `$pow` (it is integer arithmetic), so in a real context prefer `$pow`, or
write the cast explicitly as `real'(a) ** real'(b)`.

---

### `$floor` / `$ceil`

- **Signature**: `$floor(x: real)` → `real`, `$ceil(x: real)` → `real`
- **Standard**: IEEE 1800-2017 §20.8.2

```sv
$floor(3.7)    // 3.0  (round down — the largest integer ≤ x)
$floor(-3.7)   // -4.0 (rounds towards negative infinity)
$ceil(3.2)     // 4.0  (round up — the smallest integer ≥ x)
$ceil(-3.2)    // -3.0
```

**Note on the return type**: C's `floor()` and `ceil()` return a `double`, and SystemVerilog's
`$floor` and `$ceil` likewise **return a `real`**.
An integer result takes an extra conversion:

```sv
// integer conversion patterns
integer i = $rtoi($floor(3.7));   // i = 3
integer j = $rtoi($ceil(3.2));    // j = 4
// or the SV cast
int k = int'($floor(3.7));        // k = 3
```

---

### Trigonometric functions (`$sin`, `$cos`, `$tan`)

- **Standard**: IEEE 1800-2017 §20.8.2
- **Units**: **radians**, not degrees

```sv
real pi = 3.14159265358979323846;

$sin(0.0)       // 0.0
$sin(pi/2.0)    // 1.0
$cos(0.0)       // 1.0
$cos(pi)        // -1.0 (≈ −1.0 + epsilon, an IEEE 754 approximation)
$tan(pi/4.0)    // ≈ 1.0

// converting degrees to radians
real deg = 45.0;
real rad = deg * pi / 180.0;
$sin(rad)       // ≈ 0.7071 (sin(45°))
```

**The `$tan` singularity**: `$tan` is theoretically ±∞ at `pi/2`, but `pi/2` is not exactly
representable in an IEEE 754 double, so the result is a large finite value. This falls under
implementation defined behaviour.

---

### Inverse trigonometric functions (`$asin`, `$acos`, `$atan`, `$atan2`)

```sv
$asin(1.0)      // pi/2 ≈ 1.5707963...
$acos(1.0)      // 0.0
$atan(1.0)      // pi/4 ≈ 0.7853981...
$atan(-1.0)     // -pi/4

// $atan2 — the two-argument form, quadrant aware
$atan2(1.0, 1.0)   // pi/4  (first quadrant, x=1 y=1)
$atan2(1.0, -1.0)  // 3*pi/4 (second quadrant)
$atan2(-1.0, 0.0)  // -pi/2 (the negative y axis)
```

`$atan2(y, x)` returns the angle of the vector `(x, y)` in the range −π to +π.
Mind the argument order: **y first, x second** — the same as C's `atan2(y, x)`.

Domain errors: `$asin(2.0)` and `$acos(-2.0)` (|x| > 1) → NaN.

---

### Hyperbolic functions (`$sinh`, `$cosh`, `$tanh`)

```sv
$sinh(0.0)    // 0.0
$cosh(0.0)    // 1.0
$tanh(0.0)    // 0.0
$tanh(100.0)  // ≈ 1.0 (saturates at 1)
```

---

## Domain errors at a glance

IEEE 1800-2017 does not state what happens on a domain error (it is implementation defined).
What simulators that delegate to the C runtime — Icarus among them — actually do:

| Call | Actual result | IEEE 754 special value |
|----------|----------|----------------|
| `$sqrt(-1.0)` | NaN | quiet NaN |
| `$ln(0.0)` | −∞ | -INFINITY |
| `$ln(-1.0)` | NaN | quiet NaN |
| `$pow(-2.0, 0.5)` | NaN | quiet NaN |
| `$pow(0.0, -1.0)` | +∞ | +INFINITY |
| `$asin(2.0)` | NaN | quiet NaN |
| `$acosh(0.5)` | NaN | x < 1 is outside the domain |

Because a NaN propagates through arithmetic, code that can hit a domain error should range-check
its arguments first.

---

## Icarus / Verilator support

| Group | Icarus | Verilator |
|------|--------|-----------|
| $ln, $log10, $exp, $sqrt, $pow | full | **AMS mode only** (`--language VAMS`) |
| $floor, $ceil | full | AMS mode only |
| $sin, $cos, $tan | full | AMS mode only |
| $asin, $acos, $atan, $atan2 | full | AMS mode only |
| $sinh, $cosh, $tanh | full | AMS mode only |

**The Verilator restriction**: standard SV mode (`--language 1800-2017`) does not support the math
functions. A design that uses them only to compute parameters may be fine, since those calls fold
to elaboration-time constants, but a testbench that calls a math function at simulation run time
has to run under Icarus.

---

## Synthesizability

❌ None of these functions is synthesizable — synthesis tools do not support real arithmetic.
Some tools accept math functions other than `$clog2` in the elaboration-time context of a parameter
declaration, but the standard guarantees nothing there.

---

## Sources

- IEEE 1800-2017 §20.8.2 (mathematical functions)
- research-log: [system-tasks-conversion-math-2026-05-28.md](../../../history/research-log/system-tasks-conversion-math-2026-05-28.md)
- [circuitcove.com — Math Functions](https://circuitcove.com/system-tasks-math/) (WebFetch ✓)
- [chipverify.com — Verilog Math Functions](https://chipverify.com/verilog/verilog-math-functions) (WebFetch ✓)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html) (WebFetch ✓)
