# 09 · SystemVerilog Synthesizability

Based on IEEE 1800-2017 plus Vivado UG901 and Synopsys DC.
For Verilog synthesizability see `../verilog/11-synthesizability.md`.

> **Legend**:
> ✅ Synthesizable — supported by Vivado, DC and Genus alike
> ⚠️ Conditional — subject to tool or form restrictions
> ❌ Non-synthesizable — simulation and verification only

---

## ✅ Synthesizable (universal RTL subset)

| Construct | Notes |
|------|------|
| `logic` | The unified `reg`/`wire` type. Recommended over `reg` in RTL |
| `bit` | A 2-state single bit. Unlike `logic` (4-state) it has no X/Z — check tool compatibility |
| `byte` / `shortint` / `int` / `longint` | 2-state integers, signed. Bit widths are explicit; usable freely in RTL |
| `logic [N-1:0]` vectors | The basic RTL type |
| `enum` | `enum logic [1:0] {IDLE, BUSY, DONE}` — useful for FSM state encoding |
| `typedef` | A type alias. Combined with packages for reuse |
| `struct` (packed) | `typedef struct packed { logic [7:0] data; logic valid; } pkt_t;` |
| `always_comb` | Combinational logic. Sensitivity list inferred automatically, with a warning on a missing signal |
| `always_ff` | Flip-flops. Only one edge event is allowed |
| `always_latch` | Latches. Better avoided in favour of `always_comb` |
| packed multi-dim array | `logic [3:0][7:0] mat;` — 2D/3D packed arrays |
| `interface`/`modport` | A bundle of RTL ports. Synthesizable (virtual interfaces excepted) |
| `package`/`import` | Sharing types, constants and functions. Synthesizable |
| `generate`/`genvar` | Parameterized instance generation |
| `for` loop (bounded) | Compile-time constant bounds — unrolled |
| `parameter` / `localparam` | Synthesis constants |

### always_comb / always_ff usage patterns

```systemverilog
// combinational logic — always_comb recommended
always_comb begin
    unique case (state)
        IDLE:  next_state = req ? BUSY : IDLE;
        BUSY:  next_state = done ? DONE : BUSY;
        DONE:  next_state = IDLE;
        default: next_state = IDLE;
    endcase
end

// flip-flops — always_ff recommended
always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n) state <= IDLE;
    else        state <= next_state;
end
```

### The enum + typedef + struct (packed) pattern

```systemverilog
package bus_pkg;
    typedef enum logic [1:0] {
        IDLE  = 2'b00,
        WRITE = 2'b01,
        READ  = 2'b10,
        ERR   = 2'b11
    } bus_state_e;

    typedef struct packed {
        logic [31:0] addr;
        logic [31:0] data;
        logic        write;
        logic        valid;
    } bus_txn_t;
endpackage

module my_ctrl
    import bus_pkg::*;
(
    input  bus_txn_t txn,
    output bus_state_e state
);
    // ...
endmodule
```

---

## ⚠️ Conditionally synthesizable (tool and form restrictions)

| Construct | Condition | Notes |
|------|------|------|
| `foreach` | ✅ when the bounds are static (compile-time constants) | Iterating a dynamically sized array is ❌ |
| `unique case` / `priority case` | ✅ treated as a synthesis hint | The simulation-warning behaviour (the assertion) is non-synthesizable |
| `unique if` / `priority if` | ✅ a synthesis hint | Same |
| 2-state types (`bit`, `int`) | ✅ supported by most tools. Mind the absence of X/Z | Check portability against 4-state `logic` |
| `struct` (unpacked) | ⚠️ tool-dependent. Some synthesis tools do not support it | packed is recommended |
| `union` (packed) | ✅ supported by some tools | An unpacked union is non-synthesizable |
| `initial` block | **FPGA**: Vivado/Quartus → power-up initial value ✅ | **ASIC** (DC): ignored ❌ |
| dynamic array size (parameter-driven) | ✅ when a parameter fixes the size | A runtime `new[]` is non-synthesizable |
| interface (RTL restricted) | ✅ modport plus signals only | A complex interface with a `virtual interface` or tasks/functions is ⚠️ |
| typedef forward reference | ⚠️ unsupported by some tools | Declare it fully inside a package |
| `$bits()` / `$size()` / `$clog2()` | ✅ synthesizable system functions | |
| recursive function | ❌ supported by neither DC nor Vivado | Convert to a loop |

### foreach synthesis example

```systemverilog
// ✅ synthesizable — a statically sized packed array
logic [3:0][7:0] vec;
always_comb begin
    foreach (vec[i])     // i: 0..3, determined at compile time
        vec[i] = 8'hFF;
end

// ❌ non-synthesizable — a dynamic array
logic [7:0] dyn [];     // size determined at runtime
```

### How unique/priority behave under synthesis

```systemverilog
// unique case: hints to the synthesis tool that "exactly one case ever matches"
// → exploited for decoder optimization; no priority MUX needed
always_comb begin
    unique case (sel)   // synthesis: a parallel decoder may be generated
        2'b00: out = a;
        2'b01: out = b;
        2'b10: out = c;
        2'b11: out = d;
    endcase
end
```

---

## ❌ Non-synthesizable (simulation and verification only)

| Construct | Notes |
|------|------|
| `class` and all OOP | `extends`, `virtual`, `super`, `new`, destructors. A synthesis tool errors out |
| `rand` / `randc` / `constraint` | Randomization only. Rejected outright by synthesis tools |
| dynamic memory (`new[]`, `delete`) | Runtime memory allocation. Not synthesizable |
| `queue` (`$q`) — general use | An RTL queue is built as a parameterized array or a FIFO IP |
| `string` — complex operations | `$sformat`, `str.len()`, `str.substr()` and the like are non-synthesizable. Constant strings are simulation-only |
| `chandle` | A C DPI pointer. Not synthesizable |
| `real` / `shortreal` | Floating point. Unsupported by synthesis tools |
| `virtual interface` | The class-based verification bridge. Not synthesizable |
| `program` block | Verification only. A synthesis error |
| immediate assertions (`assert`, `assume`, `cover`) | Ignored by synthesis tools, or a warning |
| concurrent assertions (`assert property`) | Ignored by synthesis tools, or a warning |
| `fork-join` | Parallel-execution semantics. Not synthesizable |
| simulation tasks such as `$display` / `$monitor` / `$finish` | Ignored by synthesis tools |
| DPI-C import (in part) | A `context` import is non-synthesizable; `pure`/`DPI-C` is tool-dependent |
| `mailbox` / `semaphore` / `event` (process sync) | Process-synchronization objects. Not synthesizable |

### class + assertions → the pattern that keeps the RTL boundary clear

```systemverilog
// ❌ never mix a class into an RTL file
// ✅ keep it in a verification-only file

// rtl/my_ctrl.sv — synthesizable RTL
module my_ctrl (input clk, input logic req, output logic ack);
    always_ff @(posedge clk) ack <= req;
endmodule

// tb/my_ctrl_tb.sv — non-synthesizable, verification only
module my_ctrl_tb;
    // assertions (non-synthesizable) and classes (non-synthesizable) live only here
    assert property (@(posedge clk) req |-> ack);

    class Checker;
        // ...
    endclass
endmodule
```

---

## Synthesizability summary — the main SV extensions

| SV construct | ✅/⚠️/❌ | Verilog equivalent |
|---------|---------|----------------|
| `logic` | ✅ | `reg`/`wire` |
| `always_comb` | ✅ | `always @(*)` |
| `always_ff` | ✅ | `always @(posedge clk)` |
| `enum` | ✅ | `parameter` + integer |
| `struct` packed | ✅ | packed vector |
| `struct` unpacked | ⚠️ | separate signals |
| `interface`/`modport` | ✅ | port list |
| `package`/`import` | ✅ | `include` + global |
| `unique/priority case` | ⚠️ (hint) | `full_case`/`parallel_case` pragma (do not use) |
| `foreach` (static) | ✅ | `for (genvar i=...)` |
| `class` | ❌ | — |
| `rand`/constraint | ❌ | — |
| `virtual interface` | ❌ | — |
| `queue` | ❌ | FIFO IP / parameterized array |
| `dynamic array` (new[]) | ❌ | parameterized fixed array |
| assertions (any) | ❌ | — |
| `program` | ❌ | — |
| `string` ops | ❌ | — |

---

## Differences between tools

| Item | Vivado (FPGA) | Synopsys DC (ASIC) |
|------|-------------|-------------------|
| `initial` block | ✅ converted to a power-up initial value | ❌ VER-708 warning, then ignored |
| `interface` | ✅ modport supported | ✅ (with some restrictions) |
| `unique/priority` | ✅ an optimization hint | ✅ |
| `assertion` | ❌ ignored | ❌ ignored |
| `class` | ❌ error | ❌ error |
| packed struct | ✅ | ✅ |
| unpacked struct | ⚠️ (depends on the tool version) | ⚠️ |

---

## Related documents

- `../verilog/11-synthesizability.md` — Verilog-2005 synthesizability (the supertype relation)
- [01-data-types.md](01-data-types.md) — logic/bit/int/enum/struct types in detail
- [03-procedural.md](03-procedural.md) — always_comb/_ff/_latch semantics
- [04-interfaces.md](04-interfaces.md) — interface/modport RTL usage patterns
- [06-classes-oop.md](06-classes-oop.md) — class OOP (❌ non-synthesizable only)
- [07-assertions-sva.md](07-assertions-sva.md) — assertions (❌ non-synthesizable only)

---

## Sources

- IEEE 1800-2017 §13/§16 + training-data grounding
- vlsi.pro/sva-sequences-repetition-operators/ (WebFetch ✓)
- chipverify.com/systemverilog/systemverilog-quick-refresher — synthesizable constructs (WebSearch)
- chipverify.com/systemverilog/systemverilog-always — always_comb/_ff synthesis (WebSearch)
- systemverilog.dev/2.html, systemverilog.dev/3.html — RTL modelling patterns (WebSearch)
- Vivado UG901 HTML — confirmed the "SystemVerilog Support" section exists (table-of-contents level; the detailed PDF was not reachable)
- Synopsys DC VER-708 warning — initial-block handling for ASIC (carried over from earlier research; copyprogramming.com WebFetch ✓)
