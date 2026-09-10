# 11 · Verilog Synthesizability

## Overview

vita is a simulator — synthesis is a non-goal. These reference notes still mark
synthesizability, for two reasons.

1. It helps a reader writing RTL tell synthesis-friendly constructs apart from the
   simulation-only ones.
2. It gives implementation work a priority ordering: the constructs real designs must
   be able to synthesise are the ones a simulator is asked for first.

The marks describe the synthesis tools, not this simulator. For what vita actually
supports, see
[docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

Legend: [../01-synthesizability-legend.md](../01-synthesizability-legend.md) — the
cross-tool criteria (Synopsys DC, Xilinx Vivado, Cadence Genus).

---

## Mapping by category

### ✅ Synthesizable (universal)

All three tools convert these to gates.

**Modules / structure**

- module declarations, ports, `parameter`, `localparam`
- module instantiation (named and positional port connections)
- `generate for` / `generate if` — the recommended way to build parameterised structure

**Data types**

- `wire`, `reg` — the core synthesis types
- `reg [N:0]` vectors with an explicit width, including the `signed` qualifier
- `integer` — 32-bit signed; synthesis tools trim the width down to the range actually
  used (an explicit `reg [N:0]` declaration is safer — see the ⚠️ notes)

**Procedural blocks / control flow**

- `always @(posedge clk)` / `always @(posedge clk, posedge rst)` — infers clocked flops
- `always @(*)` — infers combinational logic
- `if/else` — infers a priority MUX
- `case` / `casez` (with `?` wildcards only) — synthesizable
- `for` loop (bounded) — compile-time constant bounds → unrolled
- `assign` (continuous assignment) — combinational logic

**Tasks / functions**

- `function` — zero-time by construction, `input` ports only, synthesizable
- `task` (static, consuming no time) — synthesizable, in the form without `@` / `#` /
  `wait`

**Gate primitives**

- combinational built-in gates: `and`, `or`, `nand`, `nor`, `xor`, `xnor`, `buf`, `not`
- tri-state buffers (`bufif1`, `bufif0`, ...) — synthesizable at FPGA top-level I/O

---

### ⚠️ Conditionally synthesizable

Constructs whose support differs between tools, that synthesise only in a particular
form, or that do synthesise but are not recommended.

**`initial` — fine on FPGA / careful on ASIC**

FPGA flows (Vivado/Quartus/XST) turn assignments inside an `initial` block into
power-up initial values. Synopsys DC (ASIC) emits `VER-708: The construct 'declaration
initial assignment' is not supported in synthesis; it is ignored` and ignores it.
Cadence Genus generally does not support it in an ASIC flow either.

```verilog
// FPGA: synthesizable — treated as a power-up initial value
// ASIC (DC/Genus): warned about, then ignored
initial begin
    state <= 3'd0;
    count <= 8'hFF;
end
```

**`casex` — synthesizable, but do not use it**

Synthesis tools handle it, but `x` acts as a wildcard, so the result can differ from
the X-propagation the simulation performs. The lowRISC style guide forbids it outright
and recommends `casez` (Verilog-2001) or SV `case inside` instead.

**`integer` type — conditional**

Synthesizable, but the tool trims it to fewer than 32 bits according to the range in
use. In RTL it is safer to state the width explicitly with `reg [N:0]`.

**`defparam` — deprecated, do not use**

Some synthesis tools still process it, but IEEE 1800-2012 announced its deprecation and
the lowRISC style guide states `Do not use defparam`. Override parameters with the
`#(...)` instance-parameter form instead.

```verilog
// ❌ defparam — do not use
defparam u_adder.WIDTH = 8;

// ✅ recommended: instance parameter override
adder #(.WIDTH(8)) u_adder (...);
```

**Tri-state (`1'bz`) — fine at FPGA top level / restricted on-chip**

Tri-state buffers on FPGA top-level I/O pins are synthesizable. Using Z for on-chip
internal muxing is not recommended — lowRISC forbids it explicitly.

**Combinational UDP — supported by some tools**

A combinational `primitive...endprimitive` UDP is supported by some synthesis tools,
but portability is poor. The standard practice is to write it as `assign` or
`always @(*)` instead.

**`full_case` / `parallel_case` pragmas — do not use**

```verilog
// ❌ pragma — risk of a simulation/synthesis mismatch
case (sel) // synthesis full_case parallel_case
```

Synthesis tools process them, but the simulation infers a latch while synthesis builds
combinational logic — a sim/synth mismatch. lowRISC: "Never use either pragma."
Alternatives: SV `unique case` / `priority case`, or an explicit assignment on every
path.

---

### ❌ Non-synthesizable

No synthesis tool converts these to gates. Simulation and verification only.

| Construct | Reason |
|------|------|
| `real`, `time` data types | floating point / 64-bit, simulation only |
| `#delay` inside `always` | ignored or rejected by synthesis — "FPGA has no concept of time" |
| `$display`, `$monitor`, `$finish` and every other system task | simulation I/O only |
| `fork-join` | parallel-execution semantics — not synthesizable; testbench only |
| Sequential UDPs (edge-sensitive / level-sensitive latch UDPs) | not supported by synthesis tools |
| `force` / `release` | testbench only |
| Recursive `function` / `task` | neither DC nor Vivado synthesises recursion; absent from IEEE 1364.1-2002 as well |
| `while` / `forever` loops (unbounded) | no compile-time termination condition |
| Gate delays (`and #(2) g(...)`) | timing simulation only; ignored by synthesis |
| Drive strength (`strong1`, `weak0`, ...) | simulation only; ignored or warned about by synthesis |
| MOS and bidirectional switches (`nmos`, `pmos`, `tran`, ...) | analog / switch-level simulation only |

---

## Synthesis-friendly RTL patterns

**Avoid latches**: assign explicitly on every path of a `case` / `if-else`. An empty
path infers a latch.

```verilog
// ❌ infers a latch: the sel=2 path is missing
always @(*) begin
    case (sel)
        2'd0: out = a;
        2'd1: out = b;
        // sel=2 and sel=3 missing → latch
    endcase
end

// ✅ default covers every path
always @(*) begin
    case (sel)
        2'd0: out = a;
        2'd1: out = b;
        default: out = '0;
    endcase
end
```

**Separate clocked from combinational**: put clocked logic and combinational logic in
separate `always` blocks.

```verilog
// ✅ the recommended pattern
always @(posedge clk or posedge rst) begin
    if (rst) q <= '0;
    else     q <= d_next;
end

always @(*) begin
    d_next = (en) ? data_in : q;
end
```

**Bounded for-loops**: fix the loop bound with a `parameter` or a constant.

```verilog
parameter N = 8;
integer i;
always @(*) begin
    for (i = 0; i < N; i = i + 1)  // N is constant → unrolled
        out[i] = in[N-1-i];
end
```

**Isolate non-synthesizable code**: use `` `ifndef SYNTHESIS `` to mark system tasks
and `initial` blocks as simulation-only.

```verilog
`ifndef SYNTHESIS
    initial $display("Testbench: reset applied");
`endif
```

---

## Sources

- IEEE 1364.1-2002 "IEEE Standard for Verilog Register Transfer Level Synthesis"
  (IEEE Xplore, withdrawn but remains the formal RTL synthesis subset definition)
- Vivado Design Suite User Guide Synthesis UG901 — Verilog Language Support
  (docs.amd.com/r/en-US/ug901-vivado-synthesis)
- Synopsys Design Compiler — VER-708 warning (initial block ignored in ASIC flow)
- lowRISC Verilog Coding Style Guide (github.com/lowRISC/style-guides) — the basis for
  banning defparam / casex / full_case / parallel_case (verified by WebFetch ✓)
- billauer.se/blog/2018/02/verilog-initial-xst-quartus-vivado/ — how FPGA flows treat
  the initial block (verified by WebFetch ✓)
- asic-soc.blogspot.com/2013/06/synthesizable-and-non-synthesizable.html — the
  construct classification table (verified by WebFetch ✓)
- [../01-synthesizability-legend.md](../01-synthesizability-legend.md) (shared legend,
  tool criteria)
- Research log:
  [verilog-synthesizability-2026-05-28.md](../../../history/research-log/verilog-synthesizability-2026-05-28.md)
  (the full investigation)
