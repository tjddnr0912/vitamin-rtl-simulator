# 02 · Verilog Data Types

Per IEEE 1364-2005 §4. Verilog data types fall into two broad categories, **nets**
and **variables**. A net expresses a structural connection; a variable stores a value.

---

## Net vs. variable

| Category | Typical types | How it is assigned | Value storage |
|------|----------|----------|---------|
| Net | wire, tri, wand, … | `assign` (continuous), port connections | decided by its drivers — it holds nothing itself |
| Variable | reg, integer, real | `always`, `initial` (procedural) | keeps its value until the next assignment |

- Assigning to a `wire` from inside an `always` block is a syntax error.
- Driving a `reg` with `assign` is equally illegal. (SystemVerilog's `logic` allows
  both.)

---

## The nine net types

IEEE 1364-2005 defines these nine plus `uwire` (added in 2005).

| Type | Undriven default | Multiple-driver resolution | Main use |
|------|-------------|----------------------|----------|
| `wire` | `z` | `x` (unknown) | ordinary interconnect — by far the most used |
| `tri` | `z` | `x` | multi-driver bus (identical to wire; the name documents intent) |
| `wand` | `z` | AND resolution (0 wins) | wired-AND |
| `triand` | `z` | AND resolution (0 wins) | multi-driver wired-AND |
| `wor` | `z` | OR resolution (1 wins) | wired-OR |
| `trior` | `z` | OR resolution (1 wins) | multi-driver wired-OR |
| `tri0` | `0` (pull strength) | `x` | models a built-in pull-down resistor |
| `tri1` | `1` (pull strength) | `x` | models a built-in pull-up resistor |
| `supply0` | `0` (supply strength) | — | GND / supply pin |
| `supply1` | `1` (supply strength) | — | VCC / supply pin |
| `trireg` | holds its last value | `x` | capacitive node (charge storage) |
| `uwire` | `z` | compile error | enforces a single driver (added in 1364-2005) |

### trireg in detail

`trireg` is the only Verilog net that stores a value. While a driver is active it
follows the driver's value (0/1/x); once every driver goes to `z`, it holds its
previous value at one of three charge strengths — `small`, `medium` or `large`.

```verilog
trireg (small)  cap_node;   // weak charge retention
trireg          bus_node;   // default (medium) charge
trireg (large)  strong_cap; // strong charge retention
```

### supply0 / supply1

Driver conflicts are meaningless on a supply pin — it is always driven at the
strongest (supply) strength. Use these to make VCC/GND explicit in a gate-level
circuit.

```verilog
supply1 vcc;
supply0 gnd;
```

### wand / wor (wired logic)

Use these to express a circuit that ties several open-drain outputs together.

```verilog
wand  pull_low;  // 0 if any driver drives 0
wor   bus_req;   // 1 if any driver drives 1
```

### Net declaration syntax

```
net_type [signed] [drive_strength] [vectored|scalared] [range] [delay] identifier [= expression];
```

```verilog
wire                data;           // 1-bit wire
wire [7:0]          data_bus;       // 8-bit wire vector
wire signed [15:0]  offset;         // signed wire (1364-2001+)
tri  (strong1, weak0) [3:0] bus;    // explicit drive strengths
wand #(5, 3)        w;              // rise=5, fall=3 delay
```

---

## Variable types

### reg

The basic value-storing variable. It does not necessarily correspond to a hardware
register — combinational logic is modelled with `reg` too.

```verilog
reg         flag;           // 1 bit, unsigned
reg [7:0]   data;           // 8 bits, unsigned
reg signed [7:0]  acc;      // 8 bits, signed (1364-2001+)
```

### integer

A 32-bit signed integer. In RTL it serves as a loop counter or for integer
arithmetic. Synthesis may infer a register from it.

```verilog
integer i;        // loop counter
integer count;    // 32-bit signed
```

### real / realtime

64-bit IEEE 754 double precision floating point. Simulation only — **not
synthesizable**.

```verilog
real     tau = 1.5e-9;
realtime current_time;
```

### time

A 64-bit unsigned integer, used mostly to hold a `$time` result. **Not
synthesizable**.

```verilog
time     start_time;
time     elapsed;
```

> These notes describe the language standard. For which of these types vita
> supports, and with what restrictions, see
> [docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Vector declarations

```verilog
// declaration syntax
data_type [msb:lsb] identifier;

// examples
wire  [7:0]   byte_bus;    // 8 bits, MSB=7, LSB=0
reg   [15:0]  word_reg;    // 16 bits
reg   [0:7]   rev_byte;    // reversed (MSB=0, LSB=7) — synthesizable but discouraged
```

- The convention is msb ≥ lsb in `[msb:lsb]` (big-endian bit order)
- The reversed form `[0:N-1]` is legal syntax, but it flips the direction of a slice

### Bit and part selects

```verilog
data_bus[3]        // single bit select
data_bus[5:2]      // 4-bit part select (the bounds must be constant)
data_bus[base+:4]  // 4 bits upward from base (1364-2001+)
data_bus[base-:4]  // 4 bits downward from base (1364-2001+)
```

### The scalared / vectored keywords

```verilog
reg  scalared [7:0] a;   // permits bit-level access
wire vectored [7:0] b;   // hints that the vector is used as a whole
```

They are optimization hints to the simulator; the semantics are identical either way.

---

## parameter / localparam

### parameter

A constant that can be overridden from outside the module, at instantiation.

```verilog
module fifo #(
    parameter DATA_W  = 8,
    parameter DEPTH   = 16
) (
    input  wire [DATA_W-1:0] din,
    output wire [DATA_W-1:0] dout
);
    // ...
endmodule

// overridden at instantiation
fifo #(.DATA_W(16), .DEPTH(256)) u_fifo (…);
```

With an explicit type or range:

```verilog
parameter signed [7:0]  OFFSET = -1;
parameter integer       MAX_COUNT = 100;
parameter real          CLK_PERIOD = 10.0;
```

### localparam

A module-internal constant — neither `defparam` nor a `#()` override can change it.

```verilog
localparam HALF_W  = DATA_W / 2;
localparam NUM_SEL = $clog2(DEPTH);  // usable in a synthesis flow
```

---

## Memory (array) declarations

```verilog
// basic syntax
reg [width-1:0] mem_name [0:depth-1];

// examples
reg [7:0]    ram  [0:255];      // 256 × 8-bit RAM (2048 bits)
reg [31:0]   rom  [0:1023];     // 1K × 32-bit ROM
reg [N-1:0]  lut  [0:M-1];     // parameterized LUT

// access
ram[addr]         = data;       // word write
data = ram[addr];               // word read
// reading the whole array at once, or writing it as a vector, is not allowed
// nor is a bit select on an element: ram[addr][3] (tool-dependent — avoid it)
```

A Verilog-2005 memory is **one-dimensional** only. Multidimensional arrays arrived
with SystemVerilog.

---

## Sources

- IEEE 1364-2005 §4 (Declarations)
- IEEE 1800-2017 §6 (Data types — Verilog-compat)
- chipverify.com/verilog/verilog-net-types
- peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00030.htm
- verilogpro.com/verilog-reg-verilog-wire-systemverilog-logic/
- circuitcove.com/data-types-net-types/
