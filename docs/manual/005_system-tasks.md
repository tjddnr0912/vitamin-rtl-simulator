# 005 · System Tasks & Functions

vitamin supports the subset of Verilog/SystemVerilog system tasks (`$display`,
`$finish`, …) and system functions (`$time`, `$signed`, …) needed to drive a
synthesizable RTL testbench: print results, end the run, dump waveforms, read
the clock, and do basic numeric conversions.

This chapter is a **support matrix** — it lists what is actually implemented in
the simulator today. The broader catalog under `docs/preview/hdl-reference/` is a
*specification* of the eventual surface, not a statement of current support;
where the two disagree, this chapter is authoritative.

Anything not listed here as supported is treated as **deferred**: tasks
warn-and-skip, functions are a hard elaboration error. See
[Deferred / not yet supported](#deferred--not-yet-supported) at the end. vitamin
never silently produces wrong output for an unimplemented builtin.

Platform note: vitamin builds and runs on **Linux and macOS only**. Windows is
out of scope; nothing in this chapter assumes it.

Related: [Installation](001_installation.md) · [Quickstart](002_quickstart.md) ·
[Language Reference](003_language-reference.md) · [CLI Reference](004_cli-reference.md) ·
[Limitations](006_limitations.md)

---

## Display & output

`$display`, `$write`, `$monitor`, and `$strobe` are all supported, including the
radix-suffixed forms `$displayb` / `$displayo` / `$displayh` and `$writeb` /
`$writeo` / `$writeh` (the suffix is accepted but does not change the default
radix of bare arguments).

| Task | Behavior |
|---|---|
| `$display` | Format the arguments and print **with** a trailing newline. |
| `$write` | Same as `$display` but **no** trailing newline. |
| `$strobe` | Print **once, at the end of the current time step**, after all values have settled. Multiple `$strobe` calls in one step print in call order. |
| `$monitor` | Register a monitor that re-prints whenever any argument changes. There is **one** active monitor for the whole simulation — a later `$monitor` replaces the earlier one (per IEEE). |

```systemverilog
initial begin
  $display("hello, t=%0t", $time);          // newline
  $write("a=%h ", a); $write("b=%h\n", b);   // no auto-newline
  $strobe("settled: q=%b", q);               // end-of-step value
  $monitor("clk=%b data=%0d", clk, data);    // re-prints on change
end
```

If a `$display`/`$write`/`$monitor`/`$strobe` call has **no** format string (the
first argument is not a string literal), the arguments are printed space-joined
in decimal.

### Format specifiers

The format engine is 4-state aware: if any bit of an argument is `x`/`z`, the
field renders as `x`/`z` rather than a fabricated number.

| Spec | Meaning |
|---|---|
| `%d` | Decimal. Signed operands print signed. Any `x`/`z` → `x`. |
| `%h` / `%x` | Hexadecimal. Per-nibble `x`/`z` shows as `x`/`z`. |
| `%o` | Octal (per-digit `x`/`z`). |
| `%b` | Binary (per-bit `x`/`z`). |
| `%c` | Low 8 bits of the argument as one ASCII character. |
| `%s` | String argument (a string literal or a vector holding packed bytes). |
| `%t` | Time value (see the note below). |
| `%f` | Real, fixed-point. Default 6 fractional digits. |
| `%e` | Real, scientific. Default 6 mantissa digits; exponent is signed and at least 2 digits (`1.500000e+03`). |
| `%g` | Real, shortest of `%f`/`%e` with trailing zeros stripped (default 6 significant digits). |
| `%m` | Current scope name. |
| `%%` | A literal `%`. |

`%f`, `%e`, and `%g` are for **real** values. Pairing a radix specifier
(`%b`/`%h`/`%o`/`%x`) with a real-typed argument is rejected at elaboration
(use `$realtobits` first).

### Width and zero-pad forms

Two width forms are recognized on the integer/real specifiers:

- **`%0d` / `%0h` / `%0b` / `%0o`** — minimum width. Leading zeros are stripped
  (at least one digit is kept). This is the common idiom for compact output.
- **`%Nd`** (e.g. `%5d`, `%8h`) — a fixed field of `N` columns, right-justified
  and space-padded.
- **bare `%d`** — the operand's *default* decimal width: the digit count of an
  `n`-bit value's maximum, right-justified and space-padded (e.g. an 8-bit value
  occupies 3 columns, a 32-bit value 10). Bare `%h`/`%o`/`%b` keep the full
  vector width (leading zeros retained).

`%f`/`%e`/`%g` also accept a `%W.Pf` field-width / precision form (e.g. `%8.2f`).

```systemverilog
$display("%0d", count);    // "42"      (compact)
$display("%5d", count);    // "   42"   (5-col field)
$display("%d",  byte_val); // " 42"     (3-col default for 8-bit)
$display("%08h", word);    // hex, full width
$display("%8.2f", volts);  // "    3.14"
```

> **`%t` quirk.** vitamin currently renders `%t` as a plain decimal of the time
> value with **no field width** — unlike some simulators that pad `%t` to a fixed
> column. A width modifier on `%t` is parsed but ignored, and there is no
> `$timeformat` default to fall back on (`$timeformat` is not implemented — see
> [Deferred](#deferred--not-yet-supported)). If you need aligned time columns,
> read the time with `$time` and format it explicitly with `%Nd`.

---

## Simulation control

| Task | Behavior |
|---|---|
| `$finish` | End the simulation and exit. The normal way to terminate a testbench. |
| `$stop` | Also ends the batch run, but is reported as a **distinct exit class** from `$finish`. vitamin is a batch simulator: `$stop` does **not** open an interactive breakpoint/console. |

An optional severity argument (`$finish(0|1|2)`) is accepted and parsed but does
not change the outcome in v1.

```systemverilog
initial begin
  #100 $display("done");
  $finish;
end
```

Without a `$finish` (or `$stop`) a self-driving testbench may never terminate —
include one.

---

## VCD waveform dump

VCD output is **RTL-driven**: a waveform file is produced only when the design
calls the dump tasks. There is no always-on dump.

| Task | Behavior |
|---|---|
| `$dumpfile("name.vcd")` | Set the output path. If omitted, `$dumpvars` defaults to `dump.vcd`. |
| `$dumpvars(...)` | Open the file, declare every net in the design (hierarchically scoped), write the header, and emit the initial values. From here on, value changes are recorded. |
| `$dumpoff` | Stop recording changes (checkpoint at the current time). |
| `$dumpon` | Resume recording (re-emits a full snapshot). |
| `$dumpall` | Emit a full snapshot of all current values at the current time. |

```systemverilog
initial begin
  $dumpfile("wave.vcd");
  $dumpvars;          // or $dumpvars(0, top);
end
```

> **`$dumpvars` scope/depth arguments are ignored — the whole design is dumped.**
> The classic `$dumpvars(0, top)` form is accepted, but the level and scope
> arguments are **dropped**: vitamin always dumps *all* nets (a valid superset of
> any requested depth/scope). A scope/module name argument is silently dropped so
> the common idiom does not spew warnings.
>
> **Arrays dump element 0 only.** For an array net, only word 0 is written to the
> VCD in v1. Per-element dumping is a future refinement.

---

## Time functions

| Function | Returns |
|---|---|
| `$time` | Current simulation time as a 64-bit integer in the **calling module's** time unit, truncated. |
| `$realtime` | Same, but as a `real` keeping the sub-unit fraction. |

Both are unit-correct across modules with different `` `timescale `` directives:
the global tick count is scaled by the caller's unit multiplier.

```systemverilog
$display("t=%0t now=%0d real=%g", $time, $time, $realtime);
```

> `$stime` is **not implemented** — despite appearing in the Phase-1 reference
> catalog. Using it is an elaboration error. Use `$time` with a `%0d`/`%Nd`
> field instead.

---

## Conversion functions

These are evaluated by the engine, not warn-skipped:

| Function | Behavior |
|---|---|
| `$signed(x)` | Re-interpret `x` as signed (re-stamps the sign flag; extension fill follows the surrounding signedness rules). |
| `$unsigned(x)` | Re-interpret `x` as unsigned. |
| `$rtoi(r)` | Real → integer, **truncated toward zero**, as a 32-bit signed value. |
| `$itor(i)` | Integer → real (exact). |
| `$realtobits(r)` | Real → its raw 64-bit IEEE-754 bit pattern (a plain 64-bit vector). |
| `$bitstoreal(v)` | 64-bit vector → real with the same bits. If `v` contains `x`/`z` the result is NaN (X/Z cannot become a real). |

```systemverilog
real    r;
integer i;
logic [63:0] bits;

i    = $rtoi(3.9);          // 3
r    = $itor(i);            // 3.0
bits = $realtobits(r);      // IEEE-754 pattern
r    = $bitstoreal(bits);   // round-trips
```

### Also implemented

`$clog2(n)` (ceiling log2, 32-bit result; `$clog2(0)` and `$clog2(1)` → 0;
`x`/`z` operand → `x`) is supported even though it is not in the Phase-1 "core
set" table. It is commonly used to size address widths in parameterized RTL.

---

## Deferred / not yet supported

These are recognized as syntax but **not** implemented. vitamin handles them
predictably rather than producing wrong output:

- **System tasks** (e.g. `$readmemh`, `$readmemb`, `$writememh`, `$writememb`,
  `$timeformat`, `$monitoron`/`$monitoroff`, file I/O `$fopen`/`$fwrite`/`$fdisplay`/…)
  → **warn and skip**. No statement is emitted; the testbench keeps running.
  Memory-initialization tasks (`$readmemh`/`$readmemb`) and `$timeformat`
  in particular fall here — provide initial memory contents in RTL, and align
  time output manually with `%0t`/`%Nt`.

- **System functions** used in an expression that are not in the supported list
  above (e.g. `$bits`, `$random`, `$urandom`, `$urandom_range`, the math family
  `$sqrt`/`$pow`/`$ln`/…, `$stime`, `$countones`, `$onehot`, …) → **hard
  elaboration error** (`E-ELAB-UNSUPPORTED`). A function returns a value, so it
  cannot be silently skipped; the run stops with a diagnostic instead of
  fabricating a result.

| Category | Examples | Status |
|---|---|---|
| Memory load | `$readmemh`, `$readmemb`, `$writememh`, `$writememb` | task → warn-skip |
| Time format | `$timeformat` | task → warn-skip |
| File I/O | `$fopen`, `$fclose`, `$fwrite`, `$fdisplay`, `$fscanf`, `$sformatf` | task → warn-skip |
| Bit-vector | `$bits`, `$countones`, `$onehot`, `$isunknown` | function → error |
| Math | `$sqrt`, `$pow`, `$ln`, `$log10`, `$exp`, `$sin`, … | function → error |
| Random | `$random`, `$urandom`, `$urandom_range`, `$dist_*` | function → error |
| Legacy time | `$stime` | function → error |

These map to Phase 2+ of the roadmap. If you hit a deferred task in a working
testbench, the warning (for tasks) or `E-ELAB-UNSUPPORTED` diagnostic (for
functions) will name it explicitly.
