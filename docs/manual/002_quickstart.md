# Quickstart

A five-minute tour: write a small design, simulate it with one command, read the
output, and open the waveform. This assumes you already have `vita` built and on
your `PATH` — see [Installation](001_installation.md) if not. vitamin runs on
**Linux and macOS**; Windows is not supported.

If you know Verilog or SystemVerilog, nothing here will surprise you. The point
is to see the full loop — source in, transcript and VCD out — end to end.

## 1. The first design: a 4-bit counter

Create `examples/000_counter.sv`. It has two modules: a 4-bit synchronous
counter (increments on every rising clock edge, clears to `0` while `rst` is
high) and a testbench that drives the clock, pulses reset, prints the count with
`$display`, dumps a VCD, and ends with `$finish`.

```systemverilog
`timescale 1ns/1ps

// A 4-bit synchronous counter: increments on each rising clock edge,
// clears to 0 when `rst` is high.
module counter (
    input  wire       clk,
    input  wire       rst,
    output reg  [3:0] count
);
    always @(posedge clk)
        if (rst) count <= 4'd0;
        else     count <= count + 4'd1;
endmodule

// Testbench: drive the clock, pulse reset, then count for a few cycles.
module tb;
    reg        clk;
    reg        rst;
    wire [3:0] count;
    integer    i;

    counter dut (.clk(clk), .rst(rst), .count(count));

    initial begin
        $dumpfile("counter.vcd");   // where the waveform goes
        $dumpvars(0, tb);           // dump the whole tb hierarchy

        clk = 0;
        rst = 1;
        #5 clk = 1;            // first edge while in reset -> count stays 0
        #5 clk = 0; rst = 0;   // release reset

        for (i = 0; i < 6; i = i + 1) begin
            #5 clk = 1;
            $display("t=%0t  count=%0d", $time, count);
            #5 clk = 0;
        end

        $finish;
    end
endmodule
```

A few notes:

- The `` `timescale 1ns/1ps `` directive sets the time unit (`1ns`) and
  precision (`1ps`). Without it vitamin assumes a `1ns/1ns` base and prints one
  warning.
- `$dumpfile`/`$dumpvars` are what make a VCD appear. vitamin **never** dumps a
  waveform unless the design asks for one — no dump tasks, no `.vcd` file.
- `$finish` ends the run cleanly (exit code `0`).

## 2. Run it

One command compiles, elaborates, and simulates:

```bash
vita examples/000_counter.sv
```

`vita` is the one-shot driver: it runs the whole pipeline
(preprocess → lex → parse → elaborate → simulate → VCD) in a single invocation.
If you prefer to stage compile / elaborate / simulate separately (the
`vcmp` → `velab` → `vrun` flow), see the [CLI Reference](004_cli-reference.md).

## 3. What you see

The `$display` lines go to **stdout**, followed by a one-line run summary:

```text
t=15  count=0
t=25  count=1
t=35  count=2
t=45  count=3
t=55  count=4
t=65  count=5
simulation ended (Finish) at time 70000
```

The first edge fires while `rst` is still high, so `count` is `0` at `t=15`; it
then climbs `1, 2, 3, …` on each subsequent rising edge. `%0t` prints in time
units (ns here), so `t=15` is 15 ns. The summary line reports the final time in
*precision* units (`1ps`), so `70000` is 70 ns.

vita's exit code tells you how the run went: `0` clean, `1` for a
design/runtime error (lex/parse/elaborate failure, `$fatal`), `3` for a usage
error (missing file, bad flag). Diagnostics go to **stderr**, so they stay out
of the transcript above.

The waveform is written next to where you ran the command, at the path passed to
`$dumpfile` — here, `counter.vcd`. It is a standard VCD with hierarchical scopes
(`tb`, and `dut` nested inside it):

```text
$timescale 1ps $end
$scope module tb $end
$var reg  1 ! clk $end
$var reg  1 " rst $end
$var wire 4 # count $end
...
$scope module dut $end
...
$upscope $end
$upscope $end
$enddefinitions $end
```

> To send the VCD somewhere else without editing the source, pass `-o`:
> `vita examples/000_counter.sv -o build/counter.vcd`.

## 4. Open the waveform

`counter.vcd` opens in any standard VCD waveform viewer. Two common ones:

- **[GTKWave](https://gtkwave.sourceforge.net/)** — the long-standing viewer;
  available via most package managers (`apt install gtkwave`,
  `brew install gtkwave`).
- **[Surfer](https://surfer-project.org/)** — a modern Rust waveform viewer.

```bash
gtkwave counter.vcd
# or
surfer counter.vcd
```

Add `clk`, `rst`, and `count` to the view and you will see `count` step up one
per rising `clk` edge, holding at `0` through the reset pulse.

## Next steps

- [Installation](001_installation.md) — building `vita` from source on Linux/macOS.
- [Language Reference](003_language-reference.md) — the supported Verilog /
  SystemVerilog RTL subset.
- [CLI Reference](004_cli-reference.md) — every command and flag, including the
  staged `vcmp` / `velab` / `vrun` flow.
