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

Platform note: vitamin builds and tests on **Linux, macOS, and Windows**
(3-OS CI, byte-identical outputs).

Related: [Installation](001_installation.md) · [Quickstart](002_quickstart.md) ·
[Language Reference](003_language-reference.md) · [CLI Reference](004_cli-reference.md) ·
[Limitations](006_limitations.md)

---

## Display & output

`$display`, `$write`, `$monitor`, and `$strobe` are all supported, including the
radix-suffixed forms `$displayb` / `$displayo` / `$displayh` and `$writeb` /
`$writeo` / `$writeh` — the suffix sets the default radix of bare (unformatted)
arguments, matching Icarus.

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
| `%s` | String argument (a string literal, a `string` variable, or a vector holding packed bytes). |
| `%t` | Time value, full IEEE §21.3.2 semantics: rescaled to the `$timeformat` units (default = the global precision) and right-justified in the min field width (default 20). `%0t`/`%Nt`/`%0Nt` override the width. |
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
- **`%0Nd`** (e.g. `%06d`, `%08h`) — a fixed field of `N` columns,
  right-justified and **zero**-padded (sign-aware for `%d`: `-00042`).
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

> **`%t` and `$timeformat`.** `%t` implements the full IEEE §21.3.2 model:
> the value (in the displaying module's time unit) is rescaled to the
> `$timeformat` units — default = the global precision, so a `1ns/1ps` design
> prints `$time`=5 as `5000` — and right-justified in the min field width
> (default 20). `$timeformat(units, precision, suffix, min_width)` is a
> runtime statement (last call wins; zero args reset the defaults; 1–3
> arguments are a compile error). Integer time values truncate at the
> precision digit; real values (`$realtime`) round like `%f`.

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

`$stime` (the low 32 bits of `$time`, unsigned) is also implemented.

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

## More implemented families

The original Phase-1 deferral list has since been implemented. Highlights
(each differential-tested against Icarus or LRM-pinned where Icarus has no
support):

- **Plusargs** — `$test$plusargs("NAME")` and `$value$plusargs("N=%d", var)`
  read the CLI's `+NAME[=VAL]` arguments (`vita tb.sv +VERBOSE +N=42`); the
  value form is also usable directly as an `if` condition.
- **Memory load/store** — `$readmemh`/`$readmemb`/`$writememh`/`$writememb`
  with optional start/finish addresses.
- **File I/O** — `$fopen` (fd and MCD modes), `$fclose`, `$fwrite`/`$fdisplay`
  (+ b/o/h variants), `$fgetc`/`$ungetc`/`$feof`/`$fgets`/`$fread`/`$fscanf`/
  `$sscanf`, `$sformat`/`$sformatf`, and the pre-opened STDOUT/STDERR
  descriptors (`32'h8000_0001/2`). Reads are supported as the direct rhs of a
  blocking assignment (`n = $fscanf(fd, …)`). STDIN reads are deferred
  (a stdin-driven simulation breaks byte-determinism).
- **Time formatting** — `$timeformat` + full `%t` (see above).
- **Randomization** — `$random`/`$urandom`/`$urandom_range` (seeded, Icarus
  bit-stream compatible) and the non-uniform `$dist_uniform`/`$dist_normal`/
  `$dist_exponential`/`$dist_poisson`/`$dist_chi_square`/`$dist_t`/
  `$dist_erlang` family.
- **Real math** — the full IEEE §20.8.2 set: `$sqrt`, `$pow`, `$ln`, `$log10`,
  `$exp`, `$floor`, `$ceil`, trig/hyperbolic (`$sin` … `$atanh`), `$hypot`,
  `$atan2` (vendored pure-Rust libm, bit-identical across the 3 OSes).
- **Introspection / bit-vector** — `$bits`, `$size`, `$dimensions`,
  `$unpacked_dimensions`, `$left`/`$right`/`$high`/`$low`/`$increment`,
  `$countones`, `$countbits`, `$onehot`/`$onehot0`, `$isunknown`, `$typename`,
  `$clog2`, `$stime`, `$sampled`/`$changed` (SVA), `$cast`, `$exit`.
- **Assertion control** — `$assertoff`/`$asserton`/`$assertkill` (global form).

## Deferred / not yet supported

Anything genuinely unimplemented stays predictable — unknown *tasks* warn and
skip (`VITA-W3056`), unknown *functions* are a hard elaboration error
(`E-ELAB-UNSUPPORTED`); vitamin never silently fabricates a result.

| Category | Examples | Status |
|---|---|---|
| File monitor variants | `$fmonitor`, `$fstrobe` | task → warn-skip |
| Flush | `$fflush` | task → warn-skip (vitamin's file writes are unbuffered, so output bytes are unaffected) |
| Timescale print | `$printtimescale` | task → warn-skip |
| STDIN reads | `$fgetc(32'h8000_0000)`, … | −1 + warn (determinism-preserving deferral) |
| Random | `$random`, `$urandom`, `$urandom_range`, `$dist_*` | function → error |
| Legacy time | `$stime` | function → error |

These map to Phase 2+ of the roadmap. If you hit a deferred task in a working
testbench, the warning (for tasks) or `E-ELAB-UNSUPPORTED` diagnostic (for
functions) will name it explicitly.
