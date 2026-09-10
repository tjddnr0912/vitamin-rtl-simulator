# 05 · Time Functions

## Overview

The category of system functions that report the current simulation time.
`$time` and `$realtime`, together with the `%t` format specifier of `$display`,
are the most frequently used of all. For the background on how simulation time
is represented internally, see
[08-timescale-and-timing.md](../../08-timescale-and-timing.md).

## Scope of this page

- `$time`, `$stime` and `$realtime`.
- How each of them interacts with the enclosing module's `timescale`, and with
  the `%t` specifier and `$timeformat`.

These notes describe the standard. For vita's own rounding, its `%t` rendering
and its `$timeformat` handling, see
[manual/005_system-tasks.md](../../../manual/005_system-tasks.md) and
[manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Item detail

### `$time`

- **Signature**: `$time` (no arguments)
- **Standard**: IEEE 1800-2017 §20.3 / IEEE 1364-2005 §17.7.1
- **Return type**: 64-bit unsigned integer
- **Meaning**: returns the current simulation time as an integer, in the time
  unit of the calling module's `timescale`. The value is rounded per the
  precision, then converted into units.

```sv
`timescale 1ns / 100ps
initial begin
  #5;                                  // 5 ns of delay
  $display("time = %0d ns", $time);   // time = 5 ns
  #0.3;                                // 300 ps of delay → rounded to the 100 ps precision
  $display("time = %0d ns", $time);   // time = 5 ns  (5.3 ns is 5 as an integer count of ns)
  // note: $time counts whole ns here, so 5.3 ns reads as 5
end
```

**Integer conversion rule**: the time, rounded at the precision boundary, is
divided by the unit and converted to an integer — rounded, not truncated.
For example, under a `1ns/100ps` timescale 5.25 ns rounds at the precision to
5.3 ns, and `$time` is 5 (in units of ns, with the fraction gone).
Unlike `$realtime`, it cannot show the fractional part.

---

### `$stime`

- **Signature**: `$stime` (no arguments)
- **Standard**: IEEE 1800-2017 §20.3 / IEEE 1364-2005 §17.7.2
- **Return type**: 32-bit unsigned integer
- **Meaning**: returns the **low 32 bits** of `$time`.
  Once the value passes 2³² − 1 (~4.3 billion) it wraps around.
  A legacy function, rarely used in modern code.

```sv
// where the wrap-around bites (1 ns units, a run longer than 4.3 seconds)
$display("stime=%0d", $stime);   // only the low 32 bits
$display("time=%0d", $time);     // the full 64 bits (preferred)
```

**Recommendation**: use `$time` rather than `$stime`. `$stime` survives only for
compatibility with legacy code that expects a 32-bit integer, the way a hardware
register would.

---

### `$realtime`

- **Signature**: `$realtime` (no arguments)
- **Standard**: IEEE 1800-2017 §20.3 / IEEE 1364-2005 §17.7.3
- **Return type**: `real` (IEEE 754 double-precision floating-point)
- **Meaning**: returns the current simulation time as a **fractional** value.
  It carries the timescale precision, so a fractional time is represented
  exactly.

```sv
`timescale 1ns / 100ps
initial begin
  #2.5;                                       // 2.5 ns
  $display("time=%0d realtime=%g", $time, $realtime);
  // time=2  realtime=2.5
  // ($time is an integer count of ns → 2; $realtime keeps the fraction → 2.5)
end
```

**Precision limit**: `real` is an IEEE 754 double (roughly 15 to 17 significant
digits), so an extremely long run can lose precision. Within the range of an
ordinary RTL simulation this is not a concern.

---

## $time vs $realtime

| Item | `$time` | `$realtime` |
|------|---------|-------------|
| Return type | 64-bit unsigned int | real (double) |
| Fractional part | ❌ (integer only) | ✅ (carries the precision) |
| With the `%t` specifier | the recommended pairing | usable |
| Overflow | the 64-bit limit (~1.8×10¹⁹) | the precision limit of a double |
| How often it is used | high (the default choice) | when the fraction matters |

---

## Conversion examples (how the timescale interacts)

```sv
`timescale 10ns / 1ns   // unit=10ns, precision=1ns

initial begin
  #1.5;     // 1.5 × 10ns = 15ns → rounded at the precision → 15ns → $time = 1 (in units of 10ns)
  $display("time=%0d (×10ns)  realtime=%g (×10ns)", $time, $realtime);
  // time=1  realtime=1.5    (the fractional value, still in units of 10ns)

  #3;       // 3 × 10ns = 30ns more
  $display("time=%0d  realtime=%g", $time, $realtime);
  // time=4  realtime=4.5
end
```

The full definition of the timescale rounding and conversion rules is in
[08-timescale-and-timing.md](../../08-timescale-and-timing.md).

---

## Icarus / Verilator support

| Function | Icarus | Verilator |
|----------|--------|-----------|
| `$time` | fully supported | generally supported |
| `$stime` | fully supported | generally supported |
| `$realtime` | fully supported | generally supported |

Verilator's own documentation classifies all three as "generally supported".
Where they differ is in the precision of the timescale handling, so when
comparing simulators it is advisable to compare `$realtime` values with an
epsilon tolerance rather than exactly.

---

## Synthesizability

❌ Not synthesizable — simulation-only functions.
Synthesis tools either ignore a `$time`/`$realtime` call or reject it.

---

## Sources

- IEEE 1800-2017 §20.3
- IEEE 1364-2005 §17.7
- [08-timescale-and-timing.md](../../08-timescale-and-timing.md) (internal, the timescale conversion rules)
- research-log: [system-tasks-display-time-2026-05-28.md](../../../history/research-log/system-tasks-display-time-2026-05-28.md)
- [circuitcove.com Time Functions](https://circuitcove.com/system-tasks-time/)
- [chipverify.com Verilog Timescale](https://www.chipverify.com/verilog/verilog-timescale)
- [verilator.org Input Languages](https://verilator.org/guide/latest/languages.html)
