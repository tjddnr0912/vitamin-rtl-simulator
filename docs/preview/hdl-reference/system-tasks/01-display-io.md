# 01 · Display · I/O System Tasks

## Overview

The category of tasks that print messages to stdout during simulation. Each one
takes a printf-style format string plus optional arguments and writes text to the
console. All of them are simulation-only and cannot be synthesized.

## Scope of this page

- The four print tasks `$display`, `$write`, `$monitor` and `$strobe`, plus their
  b/o/h radix variants — 16 spellings in all.
- Monitor control: `$monitoron` / `$monitoroff`.

These notes describe the standard. For what vita accepts and how it renders each
conversion, see
[manual/005_system-tasks.md](../../../manual/005_system-tasks.md) and
[manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Item detail

### `$display(format_string, arg1, arg2, ...)`

- **Signature**: `$display([mcd,] "format_string" [, arg1, arg2, ...])`
- **Standard**: IEEE 1800-2017 §20.10 / IEEE 1364-2005 §17.1
- **Meaning**: prints to stdout from the Active/Inactive event region, at the
  moment of the call. A **newline** (`\n`) is appended automatically.
  The values printed are the values as of the call, so the effect of a
  nonblocking assignment (`<=`) is not yet visible.
- **Returns**: void
- **Example**:

```sv
// plain decimal output
$display("a=%d b=%h time=%t", a, b, $time);

// the automatic newline
$display("first");
$display("second");
// output: first\nsecond\n
```

---

### `$displayb`, `$displayo`, `$displayh`

Change the **default radix** applied to arguments that no explicit format
specifier consumes. `b` = binary, `o` = octal, `h` = hexadecimal.

```sv
logic [7:0] val = 8'hFF;

$display("val=", val);   // val=255   (default decimal)
$displayh("val=", val);  // val=ff    (default hex)
$displayb("val=", val);  // val=11111111
$displayo("val=", val);  // val=377

// an explicit specifier beats the default radix
$displayh("val=%d", val); // val=255  (%d wins)
```

---

### `$write(format_string, arg1, arg2, ...)`

- **Signature**: `$write([mcd,] "format_string" [, arg1, arg2, ...])`
- **Standard**: IEEE 1800-2017 §20.10 / IEEE 1364-2005 §17.1
- **Meaning**: identical to `$display` except that **no** newline is appended.
  Used to assemble one line of output out of several calls.
- **Returns**: void
- **Example**:

```sv
$write("a=%d ", a);
$write("b=%d", b);
$write("\n");          // the newline has to be explicit
// output: a=3 b=7\n
```

---

### `$writeb`, `$writeo`, `$writeh`

The radix variants of `$write`. They set the default radix exactly as
`$displayb/o/h` do.

---

### `$monitor(format_string, arg1, arg2, ...)`

- **Signature**: `$monitor([mcd,] "format_string" [, arg1, arg2, ...])`
- **Standard**: IEEE 1800-2017 §20.12 / IEEE 1364-2005 §17.3
- **Meaning**: prints automatically whenever any signal in the argument list
  changes value. The print happens in the **Postponed region** — after every
  event at the current simulation time, nonblocking updates included, has been
  processed. The values shown are therefore the settled, final ones for that
  time step. A newline is appended.
- **Restrictions**:
  - Only **one `$monitor` is active** at a time.
  - A new `$monitor` call deactivates the previous one.
- **Returns**: void
- **Example**:

```sv
initial $monitor("time=%0t a=%b b=%b", $time, a, b);
// prints by itself every time a or b changes
```

#### `$monitoron` / `$monitoroff`

```sv
$monitoroff;   // suspend $monitor output
// ... a noisy stretch of the run ...
$monitoron;    // resume
```

Monitoring is on by default at the start of a run. Value changes that occur
while it is off are simply not printed — nothing is buffered and replayed.

---

### `$monitorb`, `$monitoro`, `$monitorh`

The radix variants of `$monitor`: the default radix applies to arguments with no
explicit specifier.

---

### `$strobe(format_string, arg1, arg2, ...)`

- **Signature**: `$strobe([mcd,] "format_string" [, arg1, arg2, ...])`
- **Standard**: IEEE 1800-2017 §20.11 / IEEE 1364-2005 §17.2
- **Meaning**: prints in the **Postponed region of the current time step**, which
  is what makes it able to capture the final value of a variable updated by a
  nonblocking assignment at that same time. A newline is appended. Unlike
  `$display`, the print is deferred, so nonblocking results are included.
- **Returns**: void
- **Example**:

```sv
always @(posedge clk) begin
  q <= d;                             // NBA: q is updated
  $display("q=%b (display)", q);      // prints q from before the NBA
  $strobe("q=%b (strobe)", q);        // prints q after the NBA (Postponed)
end
// with d=1 at the rising edge of clk:
// display: q=0 (the NBA has not landed yet)
// strobe:  q=1 (after the NBA)
```

---

### `$strobeb`, `$strobeo`, `$strobeh`

The radix variants of `$strobe`.

---

## Format specifiers in detail

Per IEEE 1800-2017 §20.10 / IEEE 1364-2005 §17.1.

| Specifier | Meaning | Argument type | Notes |
|-----------|---------|---------------|-------|
| `%d` / `%D` | decimal | integer / bit vector | the default form |
| `%b` / `%B` | binary | bit vector | |
| `%h` / `%H` | hexadecimal | bit vector | `%x`/`%X` are synonyms |
| `%x` / `%X` | hexadecimal | bit vector | same as `%h` |
| `%o` / `%O` | octal | bit vector | |
| `%c` / `%C` | ASCII character | 8-bit | the low 8 bits |
| `%s` / `%S` | string | string / byte array | |
| `%t` / `%T` | time | time | scaled, given its precision and suffix, by `$timeformat`; default minimum field width 20 |
| `%v` / `%V` | net strength | net (4-state) | strength + value |
| `%e` / `%E` | real, exponent form | real | e.g. `1.23e+02` |
| `%f` / `%F` | real, fixed-point form | real | e.g. `123.000000` |
| `%g` / `%G` | real, shorter of the two | real | picks `e` or `f` |
| `%m` / `%M` | hierarchical module name | (consumes no argument) | for debugging |
| `%p` / `%P` | assignment pattern | struct/enum/dynamic | SV §20.10.2 |
| `%u` | unformatted 2-value data | bit vector | binary dump |
| `%z` | unformatted 4-value data | bit vector | 4-state dump |
| `%l` / `%L` | library binding name | (consumes no argument) | |

### Width modifiers

| Example | Meaning |
|---------|---------|
| `%6d` | field width 6, right-justified (leading spaces) |
| `%06d` | field width 6, zero-padded |
| `%0d` / `%0h` | minimum width (no leading space, no leading zero) |
| `%6.2f` | real, total width 6, 2 digits after the point |

```sv
// width modifiers
$display("%6d", 42);    // "    42"  (4 spaces + 42)
$display("%06d", 42);   // "000042"
$display("%0d", 42);    // "42"      (compact)
$display("%0h", 8'hA);  // "a"       (compact hex)
```

---

## Icarus / Verilator differences

| Item | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$display` in the Active region | conforms | conforms |
| `$display` in a combinational block | runs once | may run several times (event reordering) |
| `$strobe` | fully supported | supported |
| `$monitor` | fully supported | supported |
| `$monitoron`/`$monitoroff` | supported | supported |
| 4-state `%v` specifier | supported | treats Z as 0 (2-state limit) |

**Verilator advice**: avoid `$display` inside `always_comb` / `always @(*)`.
When a combinational block is re-evaluated several times at the same simulation
time, the `$display` runs each time. Print from a sequential block
(`always_ff`, `initial`) or use `$strobe` instead.

---

## Synthesizability

❌ Not synthesizable — every task here is simulation-only.
Synthesis tools ignore calls to the `$display` family.

---

## Sources

- IEEE 1800-2017 §20.10 (display/write), §20.11 (strobe), §20.12 (monitor)
- IEEE 1364-2005 §17.1, §17.2, §17.3
- research-log: [system-tasks-display-time-2026-05-28.md](../../../history/research-log/system-tasks-display-time-2026-05-28.md)
- [hdlworks.com System Display Tasks](https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemDisplayTasks.htm)
- [chipverify.com Verilog Display Tasks](https://chipverify.com/verilog/verilog-display-tasks)
- [peterfab.com Verilog Display Tasks](https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00013.htm)
- [circuitcove.com Format Specifiers](https://circuitcove.com/system-tasks-format-spec/)
