# 04 · Verilog Modules and Hierarchy

Per IEEE 1364-2001/2005. The module is the basic unit of a Verilog design. Ports,
parameters, instances and generate blocks together build a hierarchical design.

---

## Module declaration — ANSI vs. non-ANSI port style

### non-ANSI style (since Verilog-1995)

The header lists port names only; direction and type are declared separately in the
body.

```verilog
module adder (a, b, cin, sum, cout);
    input  [3:0] a, b;
    input        cin;
    output [3:0] sum;
    output       cout;

    assign {cout, sum} = a + b + cin;
endmodule
```

### ANSI style (added by Verilog-2001 — recommended)

Direction, type and width are declared once, in the port list. Nothing is repeated,
so there is less to get wrong.

```verilog
module adder #(
    parameter WIDTH = 4
)(
    input  [WIDTH-1:0] a,
    input  [WIDTH-1:0] b,
    input              cin,
    output [WIDTH-1:0] sum,
    output             cout
);
    assign {cout, sum} = a + b + cin;
endmodule
```

### Side by side

| Aspect | non-ANSI | ANSI |
|------|---------|------|
| Introduced by | IEEE 1364-1995 | IEEE 1364-2001 |
| Where the direction lives | a separate declaration in the module body | inline in the port list |
| Readability | declarations are scattered | everything in one place |
| When to use it | reading legacy code | recommended for new code |
| interface ports | not possible | possible (SystemVerilog) |

---

## Port directions

| Keyword | Direction | Default net type | Notes |
|--------|------|-------------|------|
| `input` | outside → module | wire | read-only; cannot be assigned as a reg |
| `output` | module → outside | wire (may be declared reg) | driven from inside |
| `inout` | bidirectional | wire | tri-state bus; must be driven to `z` when idle |

```verilog
// inout in use (a bidirectional bus driver)
module bus_driver (
    inout  [7:0] data_bus,
    input        oe,        // output enable
    input  [7:0] tx_data,
    output [7:0] rx_data
);
    assign data_bus = oe ? tx_data : 8'bz;
    assign rx_data  = data_bus;
endmodule
```

---

## Parameters (parameter / localparam)

### parameter — overridable at instantiation

```verilog
module fifo #(
    parameter DEPTH = 16,
    parameter WIDTH = 8
)(
    input              clk, rst,
    input  [WIDTH-1:0] din,
    output [WIDTH-1:0] dout
);
    // ...
endmodule
```

### localparam — an internal constant (no external override)

```verilog
module fsm (input clk, rst, input go, output done);
    localparam IDLE  = 2'b00;
    localparam RUN   = 2'b01;
    localparam DONE  = 2'b10;

    reg [1:0] state;
    // ...
endmodule
```

### defparam — deprecated ⚠️

`defparam` could force a parameter to a new value from somewhere entirely apart
from the instance declaration. IEEE 1800-2017 §23.10 deprecates it explicitly.

```verilog
// ❌ defparam — do not use
defparam u_fifo.DEPTH = 32;
defparam u_fifo.WIDTH = 16;

// ✅ instead: a named parameter override (recommended since Verilog-2001)
fifo #(.DEPTH(32), .WIDTH(16)) u_fifo (.clk(clk), .rst(rst), ...);
```

**Why it was deprecated**: a `defparam` statement applies even from a different
position — or a different file — than its instance, which hurts readability and
complicates tools. Most modern lint and synthesis tools warn or error on it.

---

## Module instantiation

### Named connections (recommended)

```verilog
adder #(.WIDTH(8)) u_adder (
    .a   (op_a),
    .b   (op_b),
    .cin (carry_in),
    .sum (result),
    .cout(carry_out)
);
```

Because each port is named, reordering the module's ports does not break the
connection.

### Positional connections (discouraged)

```verilog
adder u_adder (op_a, op_b, carry_in, result, carry_out);
```

This depends on the order the ports were declared in — reorder them and the design
silently mis-connects.

### Port connection rules

| Case | Result |
|------|------|
| output → wire | connects directly to the net |
| output → reg | the port is still a wire; an internal reg drives it through an assign |
| input left unconnected | treated as high impedance (z) |
| output left unconnected | allowed (with a warning) |

---

## generate blocks (Verilog-2001)

A generate block conditionally expands constructs, or instantiates them repeatedly,
at **elaboration time**. `genvar` is an elaboration-time-only integer variable — it
does not exist during simulation.

### generate-for: repeated instantiation

```verilog
module ripple_carry #(parameter N = 4)(
    input  [N-1:0] a, b,
    input          cin,
    output [N-1:0] sum,
    output         cout
);
    wire [N:0] carry;
    assign carry[0] = cin;

    genvar i;
    generate
        for (i = 0; i < N; i = i + 1) begin : gen_fa
            full_adder u_fa (
                .a   (a[i]),
                .b   (b[i]),
                .cin (carry[i]),
                .sum (sum[i]),
                .cout(carry[i+1])
            );
        end
    endgenerate

    assign cout = carry[N];
endmodule
```

`begin : gen_fa` — with a label, the instances are reachable by the hierarchical
names `gen_fa[0].u_fa`, `gen_fa[1].u_fa`. Some tools warn about an unlabelled
generate loop, so it is best to always name one.

### generate-if: choosing an implementation

```verilog
module adder_impl #(parameter USE_CARRY_LOOKAHEAD = 0)(
    input  [7:0] a, b,
    input        cin,
    output [7:0] sum,
    output       cout
);
    generate
        if (USE_CARRY_LOOKAHEAD) begin : gen_cla
            cla_adder u_add (.a(a), .b(b), .cin(cin), .sum(sum), .cout(cout));
        end else begin : gen_rca
            rca_adder u_add (.a(a), .b(b), .cin(cin), .sum(sum), .cout(cout));
        end
    endgenerate
endmodule
```

### generate-case: selecting among several implementations

```verilog
module encoder #(parameter TYPE = 0)( ... );
    generate
        case (TYPE)
            0: priority_enc u_enc ( ... );
            1: onehot_enc   u_enc ( ... );
            default: $error("Unknown encoder type");
        endcase
    endgenerate
endmodule
```

### What a generate block may and may not contain

| Allowed | Forbidden |
|------|------|
| module instance | port declaration (input/output) |
| gate primitive | specify block |
| continuous assign | parameter/localparam declaration |
| initial / always block | |
| data types (net, reg, integer, real) | |
| task/function (only inside an if/case generate) | |

---

## Hierarchical names

A dot-separated path reaches any signal anywhere in the design.

```verilog
// hierarchy: tb → dut → core → alu
$display("ALU result: %h", tb.dut.core.alu.result);

// with a labelled generate block
$display("FA cout[2]: %b", tb.dut.u_rca.gen_fa[2].u_fa.cout);
```

Where they are used:

- observing internal signals from a testbench (`$monitor`, `$display`, force/release)
- naming the target of a `defparam` (discouraged — see above)
- reading a specific register directly while debugging

**Note**: hierarchical references are not synthesizable — they are for simulation
and verification only.

---

## Sources

- IEEE 1364-2001 §12 (module declaration, ANSI port syntax)
- IEEE 1364-2005 §12, §14 (parameter, generate)
- IEEE 1800-2017 §23 (module definitions and hierarchy), §23.10 (defparam deprecated)
- sigasi.com/tech/ansi-vs-non-ansi/ (ANSI vs. non-ANSI ports — verified by WebFetch)
- chipverify.com/verilog/verilog-parameters (parameter/defparam — verified by WebFetch)
- chipverify.com/verilog/verilog-generate-block (generate syntax — verified by WebFetch)
- vlsiverify.com/verilog/generate-blocks-in-verilog/ (genvar, hierarchical names — verified by WebFetch)
- sutherland-hdl.com/pdfs/verilog_2001_ref_guide.pdf (Verilog-2001 Quick Reference)
