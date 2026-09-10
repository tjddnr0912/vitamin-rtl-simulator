# 01 · SystemVerilog Data Types

Per IEEE 1800-2017 §6/§7. Where Verilog offered 4-state types only, SV adds 2-state types,
aggregate types (enum/struct/union) and a handful of types meant for verification.

---

## 2-state vs 4-state

Every Verilog type was 4-state (0/1/X/Z). SV introduces 2-state types, which carry no X/Z, to
speed up simulation and to line up with software integer arithmetic.

### Type comparison

| Type | State | Size | Sign | Verilog counterpart |
|------|-------|------|------|--------------------|
| `logic` | 4 | variable | unsigned | replaces wire + reg with a single type |
| `reg` | 4 | variable | unsigned | reg (legacy) |
| `integer` | 4 | 32-bit | signed | integer (legacy) |
| `time` | 4 | 64-bit | unsigned | time (not synthesizable) |
| `bit` | 2 | variable | unsigned | — (new in SV) |
| `byte` | 2 | 8-bit | signed | — (new in SV) |
| `shortint` | 2 | 16-bit | signed | — (new in SV) |
| `int` | 2 | 32-bit | signed | — (new in SV) |
| `longint` | 2 | 64-bit | signed | — (new in SV) |

- A 2-state type initializes to `0` by default; a 4-state type initializes to `X`.
- 2-state types simulate faster, but keep `logic` wherever the RTL relies on X propagation.
- `real` (64-bit IEEE 754) and `shortreal` (32-bit) remain available in SV — neither is synthesizable.

### logic vs reg

`logic` is the 4-state variable type defined in SV §6.3.4. In Verilog a `wire` could only be
driven by `assign` or a port connection and a `reg` only from `always`/`initial`, so which one
to declare had to be memorised per context. `logic` is legal in both contexts, but it enforces
a single driver at compile time.

```systemverilog
logic       flag;           // 1 bit
logic [7:0] data;           // 8-bit vector
bit   [3:0] nibble;         // 4-bit, 2-state
int         count;          // 32-bit signed, 2-state
```

A multi-driver bus (`tri`, `wor`, …) still uses the existing net types.

---

## Enum

A user-defined enumeration type. A base type (`int`, `logic`, …) may be given.

```systemverilog
// explicit values — a member left unlabelled takes the previous value + 1
typedef enum logic [1:0] {
    IDLE  = 2'b00,
    RUN   = 2'b01,
    DONE  = 2'b10,
    ERROR             // implicitly 2'b11
} state_e;

state_e s = IDLE;
```

### Built-in methods (§6.19)

| Method | Effect |
|--------|--------|
| `.first()` | returns the value of the first member |
| `.last()` | returns the value of the last member |
| `.next(N)` | the Nth following value (N defaults to 1) |
| `.prev(N)` | the Nth preceding value (N defaults to 1) |
| `.num()` | returns the total number of members |
| `.name()` | returns the string spelling of the current value |

```systemverilog
state_e s = IDLE;
s = s.next();              // RUN
$display("%s", s.name());  // "RUN"
int n = state_e.num();     // 4
```

---

## Struct

Groups members of several types under one name.

### packed struct

The members map onto one contiguous bit vector, so the struct can be sliced. Synthesizable.

```systemverilog
typedef struct packed {
    logic [3:0]  opcode;
    logic [11:0] address;
    logic [7:0]  data;
} instr_t;   // a single 24-bit vector

instr_t ins;
ins.opcode = 4'hA;
logic [23:0] raw = ins;  // read the whole struct as a vector
```

### unpacked struct

Gaps between members are allowed, and the type is not synthesizable. For verification and
modelling.

```systemverilog
typedef struct {
    int     id;
    string  name;
    real    score;
} student_t;
```

### rand / randc fields

Marking a struct field inside a class with the `rand`/`randc` qualifier includes it in
randomization when `randomize()` is called.

---

## Union

Every member shares the same storage.

### packed union

Every member must have the same bit width. Synthesizable.

```systemverilog
typedef union packed {
    logic [31:0]       word;
    logic [3:0][7:0]   bytes;   // 4 × 8 bits
} word_u;

word_u u;
u.word = 32'hDEAD_BEEF;
$display("%h", u.bytes[3]);   // DE
```

### tagged union

The member written last is recorded in an internal tag; reading through a different member is
a runtime error. Members may differ in size, and the type is not synthesizable.

```systemverilog
typedef union tagged {
    int        a;
    byte       b;
    bit [15:0] c;
} data_t;

data_t d;
d = tagged a 32'hffff;
// d.b;  → at run time: "Invalid member usage of a tagged union"
```

---

## typedef

Declares a type alias. Naming a struct/union/enum with `typedef` is the standard pattern.

```systemverilog
typedef logic [7:0] byte_t;
typedef struct packed { logic [7:0] r, g, b; } rgb_t;
```

---

## string

A dynamically sized string. Not synthesizable.

```systemverilog
string s = "Hello";
int    n = s.len();         // length
s = {s, " World"};         // concatenation
s = s.toupper();           // to upper case
int v = s.atoi();          // string → integer
```

---

## chandle

An opaque handle type that lets SV hold and pass a pointer returned by a C/C++ function through
the DPI (Direct Programming Interface). SV code can only compare it (`==`, `!=`, a `null`
check); it cannot be dereferenced.

```systemverilog
import "DPI-C" function chandle alloc_ctx();
chandle ctx = alloc_ctx();
```

---

## virtual interface

A handle to an interface instance. A class cannot have ports, so a class-based verification
component uses one to reach hardware signals. For interfaces themselves see `04-interfaces.md`.

```systemverilog
interface axi_if(input logic clk);
    logic [31:0] addr;
    // ...
endinterface

class Driver;
    virtual axi_if vif;   // handle — the real interface instance is injected from outside
    task run();
        vif.addr = 32'h0;
    endtask
endclass
```

---

## Sources

- IEEE 1800-2017 §6 (Data types), §7 (Aggregate types)
- chipverify.com/systemverilog/systemverilog-quick-refresher
- chipverify.com/systemverilog/systemverilog-enumeration
- verilogpro.com/systemverilog-structures-unions-design/
- vlsitrainers.com/system-verilog-union-packed-unpacked-and-tagged/
- circuitcove.com/data-types-enum/
