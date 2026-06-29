# vitamin examples

Four small, self-contained SystemVerilog designs that simulate end to end with
**vitamin** (`vita`). Each file is a single `.sv` containing the DUT plus a
`module tb` testbench that drives it, prints results with `$display`, dumps a VCD
waveform, and ends with `$finish`. They use only the Phase-1 synthesizable subset
the simulator supports — see [goals and scope](../docs/preview/01-goals-and-scope.md).

vitamin runs on **Linux and macOS**. Windows is not supported.

## Build the simulator

There is no `build.rs` and no prebuilt binary — build from source with cargo:

```bash
cargo build -p cli            # produces target/debug/vita
```

The `vita` one-shot driver compiles, elaborates, and simulates in a single
command. See the [installation](../docs/manual/001_installation.md) and
[quickstart](../docs/manual/002_quickstart.md) chapters for the full tour.

## Run an example

From the repository root, point `vita` at a file:

```bash
target/debug/vita examples/000_counter.sv
```

The `$display` transcript goes to **stdout**; a clean run ends with a
`simulation ended (Finish) …` line and exit code `0`. The VCD named in
`$dumpfile` is written to the **current working directory** — run from inside
`examples/` if you want the `.vcd` to land next to the source:

```bash
cd examples && ../target/debug/vita 000_counter.sv
```

Open the resulting `.vcd` in any standard waveform viewer (GTKWave, Surfer);
vitamin does not ship a GUI.

## The examples

| File | What it shows | Run |
|------|---------------|-----|
| [`000_counter.sv`](000_counter.sv) | 4-bit synchronous up-counter, active-high reset — the canonical quickstart | `vita examples/000_counter.sv` |
| [`001_alu.sv`](001_alu.sv) | 8-bit combinational ALU (add / sub / and / or via a 2-bit op) | `vita examples/001_alu.sv` |
| [`002_traffic_fsm.sv`](002_traffic_fsm.sv) | Clocked traffic-light FSM using a `typedef enum` state | `vita examples/002_traffic_fsm.sv` |
| [`003_shift_register.sv`](003_shift_register.sv) | 8-bit serial-in / serial-out shift register | `vita examples/003_shift_register.sv` |

### 000_counter.sv — synchronous counter

A `WIDTH`-parameterized counter that increments on every `posedge clk` and clears
to `0` while `rst` is high. The testbench runs a free-running clock, pulses reset,
then prints the count for 12 cycles and dumps `counter.vcd`. Demonstrates
`always @(posedge clk)`, nonblocking assignment, `parameter`, `@(posedge clk)`
event control, and hierarchical `$dumpvars`.

```
t=16  cnt=1 (0x1)
t=26  cnt=2 (0x2)
...
done: final cnt=12
```

### 001_alu.sv — combinational ALU

A purely combinational `always @*` block selecting add / subtract / and / or with
a 2-bit `op`. The testbench drives a pair of operands through all four operations
(plus a wrapping subtract) and prints each result, dumping `alu.vcd`. Demonstrates
combinational sensitivity (`@*`), `case`, and `%h` / `%0d` format specifiers.

```
ADD: f0 + 0f = ff (255)
SUB: f0 - 0f = e1 (225)
AND: f0 & 0f = 00
OR : f0 | 0f = ff
SUB: 10 - 25 = 241 (wraps mod 256)
```

### 002_traffic_fsm.sv — enum-state FSM

A clocked traffic-light FSM whose state register is a SystemVerilog `typedef enum`
(`GREEN`, `YELLOW`, `RED`). Combinational next-state logic switches on the enum
labels; a sequential `always @(posedge clk)` advances the state under synchronous
reset. The testbench prints each state by name and dumps `traffic_fsm.vcd`.
Demonstrates the Phase-2 datatypes (`typedef enum`), `case` over enum labels, and
a `task`.

```
t=11  state=GREEN  (0)
t=16  state=YELLOW (1)
t=26  state=RED    (2)
...
```

### 003_shift_register.sv — serial shift register

An 8-bit shift register that shifts left on each clock edge, taking `sin` into the
LSB while the MSB falls out on `sout`. The testbench streams a byte in MSB-first,
reads it back, and self-checks the captured value with `===`, dumping
`shift_register.vcd`. Demonstrates concatenation (`{q[WIDTH-2:0], sin}`),
`assign`, bit-select indexing, and `%b` formatting.

```
in=1 -> q=00000001 sout=0
...
in=0 -> q=10110010 sout=1
captured byte = 10110010 (0xb2)
PASS: register holds the streamed pattern
```

## Notes on the supported subset

- Each file declares `` `timescale 1ns/1ns `` explicitly. A design with no
  `` `timescale `` still runs (the simulator assumes a 1ns/1ns base and prints one
  warning to stderr).
- VCD output is RTL-driven: it appears only when the design calls `$dumpfile` /
  `$dumpvars`. There is no always-on auto-dump.
- These designs stay inside the Phase-1 MVP grammar. For the full list of what is
  and isn't supported, see [goals and scope](../docs/preview/01-goals-and-scope.md)
  and [limitations](../docs/manual/006_limitations.md).
