# 08 · Verilog Gate-Level Modeling

Per IEEE 1364-2001/2005. Covers the 26 built-in primitives, which can be instantiated
directly without a module declaration, and the UDP (User-Defined Primitive) that a user
writes.

---

## Built-in gate primitives, by category

| Category | Primitives | Port structure | Count |
|------|--------------|----------|------|
| Logic gates (multi-input) | `and` `or` `nand` `nor` `xor` `xnor` | 1 output + N inputs | 6 |
| Buffer / inverter (single-input) | `buf` `not` | N outputs + 1 input | 2 |
| Tri-state buffers | `bufif0` `bufif1` `notif0` `notif1` | 1 output + 1 input + 1 control | 4 |
| MOS switches (unidirectional) | `nmos` `pmos` `rnmos` `rpmos` | output + data + control | 4 |
| CMOS switches (unidirectional) | `cmos` `rcmos` | output + data + n-control + p-control | 2 |
| Bidirectional switches | `tran` `rtran` `tranif0` `tranif1` `rtranif0` `rtranif1` | 2 × inout (+ control) | 6 |
| Pull sources | `pullup` `pulldown` | one or more nets | 2 |
| **Total** | | | **26 primitives** (including the six `tran` forms) |

> `xor` / `xnor` accept two or more inputs in principle, but most synthesis tools
> support exactly two. Keep them to two inputs to stay safe.

---

## Logic gates

The first port is the output; every port after it is an input. There is no limit on the
number of inputs.

```verilog
// basic instantiation
and  g1  (y, a, b);             // 2-input AND
or   g2  (y, a, b, c);          // 3-input OR
nand g3  (y, a, b);             // NAND
nor       (y, a, b);            // the instance name may be omitted
xor  g5  (y, a, b);             // XOR (two inputs recommended)
xnor g6  (y, a, b);             // XNOR

// array instantiation — four identical gates across a 4-bit bus
and [3:0] ga (y_bus, a_bus, b_bus);
```

Truth table:

| a | b | and | nand | or | nor | xor | xnor |
|---|---|-----|------|-----|-----|-----|------|
| 0 | 0 | 0 | 1 | 0 | 1 | 0 | 1 |
| 0 | 1 | 0 | 1 | 1 | 0 | 1 | 0 |
| 1 | 0 | 0 | 1 | 1 | 0 | 1 | 0 |
| 1 | 1 | 1 | 0 | 1 | 0 | 0 | 1 |

An `x` on an input can make the output `x` as well (whenever the result is not
determinable).

---

## Buffers and inverters

`buf` may have several outputs (fan-out). It must have exactly one input:

```verilog
buf  b1 (y1, y2, y3, in);   // three outputs, one input (the last port is the input)
not  n1 (y, in);
```

---

## Tri-state buffers

When the control signal (ctrl) is asserted the output is driven; when it is deasserted
the output goes to high impedance (Z).

| Primitive | ctrl asserted | ctrl deasserted | Notes |
|-----------|---------|-----------|------|
| `bufif1` | ctrl = 1 → `out = in` | ctrl = 0 → `out = Z` | active high |
| `bufif0` | ctrl = 0 → `out = in` | ctrl = 1 → `out = Z` | active low |
| `notif1` | ctrl = 1 → `out = ~in` | ctrl = 0 → `out = Z` | inverting, active high |
| `notif0` | ctrl = 0 → `out = ~in` | ctrl = 1 → `out = Z` | inverting, active low |

```verilog
bufif1 tb1 (out, in, ctrl);    // buffer when ctrl=1, Z when ctrl=0
bufif0 tb2 (out, in, oe_n);    // buffer when oe_n=0 (active low)
notif1 ti1 (out, in, ctrl);    // inverter when ctrl=1, Z when ctrl=0
```

---

## MOS switches (unidirectional)

For analog, CMOS-level simulation. Not a target for RTL synthesis.

### nmos / pmos / rnmos / rpmos — three ports

```
(output, data_input, control)
```

| Primitive | Conducts when | Resistive |
|-----------|---------|--------|
| `nmos` | control = 1 | ❌ |
| `pmos` | control = 0 | ❌ |
| `rnmos` | control = 1 | ✅ (passes a weakened signal) |
| `rpmos` | control = 0 | ✅ |

```verilog
nmos  nm1 (out, data, ctrl);    // data → out when ctrl=1
pmos  pm1 (out, data, ctrl);    // data → out when ctrl=0
rnmos rnm (out, data, ctrl);    // resistive nmos (strength attenuated)
rpmos rpm (out, data, ctrl);    // resistive pmos
```

### cmos / rcmos — four ports

```
(output, data_input, n_control, p_control)
```

An nmos and a pmos bundled into one. Conducts when n_control=1 and p_control=0:

```verilog
cmos  cm1 (out, data, nctrl, pctrl);
rcmos rcm (out, data, nctrl, pctrl);
```

---

## Bidirectional switches

Both ports are inout. Signal flows in both directions.
**Drive strength cannot be specified. No synthesis support.**

| Primitive | Control | Conducts when | Resistive |
|-----------|------|---------|--------|
| `tran` | none | always | ❌ |
| `rtran` | none | always | ✅ |
| `tranif1` | ctrl | ctrl = 1 | ❌ |
| `tranif0` | ctrl | ctrl = 0 | ❌ |
| `rtranif1` | ctrl | ctrl = 1 | ✅ |
| `rtranif0` | ctrl | ctrl = 0 | ✅ |

```verilog
tran     tr1 (inout1, inout2);              // always conducting (a plain pass-gate connection)
rtran    rtr (inout1, inout2);              // resistive, always conducting
tranif1  ti1 (inout1, inout2, ctrl);        // conducts when ctrl=1
tranif0  ti0 (inout1, inout2, ctrl);        // conducts when ctrl=0
rtranif1 ri1 (inout1, inout2, ctrl);
rtranif0 ri0 (inout1, inout2, ctrl);
```

---

## Pull sources

Weakly pull a net up or down. Used to give a net a default value when nothing else
drives it:

```verilog
pullup  pu1 (net_a);        // net_a → logic 1 (pull strength)
pulldown pd1 (net_b);       // net_b → logic 0 (pull strength)
pullup  (pull1) pu2 (sda);  // strength stated explicitly
```

---

## Drive Strength

Applies to logic gates and UDPs. Cannot be used on MOS switches or the `tran` family.

### Strength levels (lowest to highest)

| Level | Name | Keyword (strength on 1) | Keyword (strength on 0) | Applies to |
|------|------|--------------|--------------|----------|
| 0 | High-Z | `highz1` | `highz0` | gates / nets |
| 1 | Small | `small` | — | `trireg` only |
| 2 | Medium | `medium` | — | `trireg` only |
| 3 | Weak | `weak1` | `weak0` | gates / nets |
| 4 | Large | `large` | — | `trireg` only |
| 5 | Pull | `pull1` | `pull0` | gates / nets |
| 6 | Strong | `strong1` | `strong0` | gates / nets (default) |
| 7 | Supply | `supply1` | `supply0` | gates / nets |

The default is `(strong1, strong0)`. The combinations `(highz0, highz1)` and
`(highz1, highz0)` are illegal.

### Syntax

```verilog
// gate (strength1, strength0) #(delay) instance_name (ports);
and  (strong1, weak0)   g1 (out, a, b);
or   (supply1, pull0)   g2 (out, a, b);
buf  (weak1,   weak0)   b1 (out, in);

// applies to assign as well
assign (weak1, strong0) net_q = data;
```

### Synthesis caveat

Drive strength is a **simulation-only** concept. Most synthesis tools ignore `supply` /
`weak` strength specifications or warn about them. RTL logic that depends on drive
strength produces a pre/post-synthesis simulation mismatch.

---

## UDP (User-Defined Primitive)

Defined with a `primitive...endprimitive` block. It sits at the same level as a module
(not inside one).

**Constraints common to all UDPs**:
- exactly one output port; every port is a 1-bit scalar
- the output cannot be Z (only 0 / 1 / x)
- instantiated the same way a module is

### Combinational UDP

The output is determined purely by the logical combination of the inputs. Up to 10
inputs. Each table row is `inputs : output;`:

```verilog
// 2-to-1 MUX UDP
primitive mux2 (out, sel, a, b);
    output out;
    input  sel, a, b;
    table
        //  sel  a  b  :  out
            0    0  ?  :  0;     // sel=0, a=0 → out=0 (? = b is irrelevant)
            0    1  ?  :  1;     // sel=0, a=1 → out=1
            1    ?  0  :  0;     // sel=1, b=0 → out=0
            1    ?  1  :  1;     // sel=1, b=1 → out=1
            x    0  0  :  0;     // sel=x, but a=b=0 → determinable
            x    1  1  :  1;     // sel=x, but a=b=1 → determinable
    endtable
endprimitive
```

### Sequential UDP

The output is declared `reg`. Up to 9 inputs. Each table row is
`inputs : current_state : next_state;`.

**Level-sensitive (latch)**:

```verilog
// D latch UDP
primitive dlatch (q, clk, d);
    output reg q;
    input  clk, d;
    initial q = 0;              // initial value (optional)
    table
        //  clk  d  :  q(current)  :  q(next)
            1    0  :  ?           :  0;     // clk=1 and d=0 → q=0
            1    1  :  ?           :  1;     // clk=1 and d=1 → q=1
            0    ?  :  ?           :  -;     // clk=0 → hold (- = no change)
    endtable
endprimitive
```

**Edge-sensitive (flip-flop)**:

```verilog
// rising-edge D flip-flop UDP
primitive dff (q, clk, d);
    output reg q;
    input  clk, d;
    initial q = 0;
    table
        //  clk  d  :  q  :  q_next
            r    0  :  ?  :  0;     // rising edge with d=0
            r    1  :  ?  :  1;     // rising edge with d=1
            f    ?  :  ?  :  -;     // falling edge — no change
            ?    *  :  ?  :  -;     // a change on a non-clock input — no change
    endtable
endprimitive
```

### Table symbols

| Symbol | Meaning | Where it may appear |
|------|------|------|
| `0` | logic 0 | input / current state / next state |
| `1` | logic 1 | input / current state / next state |
| `x` | unknown | input / current state / next state |
| `?` | any of 0 · 1 · x | input / current state only |
| `b` | 0 or 1 | input only |
| `*` | any change (`??`) | input only |
| `-` | output unchanged | next state only |
| `r` | rising edge (0→1) | input only (edge-sensitive) |
| `f` | falling edge (1→0) | input only (edge-sensitive) |
| `p` | potential rise (01, 0x, x1) | input only |
| `n` | potential fall (10, 1x, x0) | input only |

### Instantiating a UDP

```verilog
module top;
    wire out_mux, out_q;
    wire sel, a, b, clk, d;

    mux2  u_mux (.out(out_mux), .sel(sel), .a(a), .b(b));
    dff   u_ff  (.q(out_q), .clk(clk), .d(d));
endmodule
```

Named and positional port connections work exactly as they do for a module.

---

## Gate-Level Delay

A delay may be given for every gate and UDP:

```verilog
and #(2)         g1 (y, a, b);         // 2 time units on every transition
and #(2, 3)      g2 (y, a, b);         // rise=2, fall=3
and #(2, 3, 4)   g3 (y, a, b);         // rise=2, fall=3, turn-off=4
and #(1:2:3)     g4 (y, a, b);         // a single min:typ:max
and #(1:2:3, 1:3:5) g5 (y, a, b);      // rise min:typ:max, fall min:typ:max
```

---

## Sources

- IEEE 1364-2001 §7 (gate and switch level modeling), §7.6 (UDP)
- IEEE 1800-2017 §28 (gate and switch level modeling)
- vlsiverify.com/verilog/gate-level-modeling/ (verified by WebFetch ✓)
- vlsiverify.com/verilog/user-defined-primitives/ (verified by WebFetch ✓, symbol list)
- vlsiverify.com/verilog/strength-in-verilog/ (verified by WebFetch ✓, the eight drive-strength levels)
- peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00003.htm (verified by WebFetch ✓, complete primitive list)
- chipverify.com/verilog/verilog-gate-level-modeling (gate categories and truth tables)
- chipverify.com/verilog/verilog-udp (UDP structure, verified by WebFetch ✓)
