# 09 · Verilog Compiler Directives

Per IEEE 1364-2001/2005. A compiler directive starts with a backtick (`` ` ``) and
instructs the compiler or simulator. Once declared, it stays in effect for all source
that follows, **crossing both file boundaries and module boundaries**. A directive does
not describe hardware to be synthesised; it controls how compilation happens.

---

## `define — macro definition

### Simple constant macros

```verilog
`define DATA_WIDTH 8
`define RESET_VAL  8'h00

// use — always referenced with a backtick
wire [`DATA_WIDTH-1:0] bus;
assign bus = `RESET_VAL;
```

### Function-like macros with arguments

```
`define MACRO_NAME(arg1, arg2, ...) macro_body
```

```verilog
`define MAX(a, b)   ((a) > (b) ? (a) : (b))
`define ADD3(x, y, z) ((x) + (y) + (z))

// use
assign result = `MAX(sig_a, sig_b);
assign sum    = `ADD3(p, q, r);
```

**Why every argument must be parenthesised**: an argument containing an operator runs
into precedence problems.

```verilog
`define DOUBLE(x) x * 2          // ❌ hazardous
`define DOUBLE(x) ((x) * 2)      // ✅ safe

assign y = `DOUBLE(a + b);
// ❌ expands to: a + b * 2  → wrong result
// ✅ expands to: ((a + b) * 2)
```

### Multi-line macros (backslash line continuation)

```verilog
`define LONG_EXPR(a, b, c) \
    ((a) * (b) + \
     (c))

assign result = `LONG_EXPR(p, q, r);
```

The final line must not end with a backslash.

### Token pasting

A pair of backticks (``` `` ```) pastes an argument onto the adjacent text:

```verilog
`define SIGNAL(n) sig_``n

// `SIGNAL(a) → sig_a
// `SIGNAL(7) → sig_7
```

---

## `undef — undefining a macro

```verilog
`define TEMP 100
// ... TEMP used here ...
`undef TEMP
// referencing `TEMP after this point → compile error
```

Used to limit a macro to one file, or to clean up at the end of a header file.

---

## `ifdef / `ifndef / `elsif / `else / `endif — conditional compilation

```verilog
`define SYNTHESIS

`ifdef SYNTHESIS
    // synthesis-only code (the simulator never reads this section)
    assign out = fast_path;
`elsif FPGA_TARGET
    // FPGA-only code
    assign out = fpga_path;
`else
    // everything else (simulation)
    assign out = sim_path;
`endif
```

`ifndef` is the inverse of `ifdef`:

```verilog
`ifndef GATE_SIM
initial $display("RTL simulation");
`endif
```

`ifdef / `else / `endif can be nested, but deep nesting hurts readability — keep it to
a minimum.

---

## `include — file inclusion

```verilog
`include "defs.vh"
`include "../common/params.vh"
`include "/abs/path/to/defines.vh"
```

The entire contents of the file are inserted at that point. Search paths are added
through compiler options:

```
iverilog -I ./include -I ../shared ...
vcs     +incdir+./include+../shared ...
```

A relative path is resolved against **the location of the current source file** (not
the directory the compiler was run from). Header files use the include-guard pattern to
avoid being inserted twice:

```verilog
// defs.vh
`ifndef DEFS_VH
`define DEFS_VH
`define CLK_PERIOD 10
// ...
`endif  // DEFS_VH
```

---

## `timescale — time unit and precision

```
`timescale <time_unit> / <time_precision>
```

```verilog
`timescale 1ns  / 1ps    // unit 1 ns, precision 1 ps
`timescale 10ns / 1ns    // unit 10 ns, precision 1 ns
`timescale 1us  / 100ns  // unit 1 µs, precision 100 ns
```

Permitted units: `1`, `10` or `100` combined with `s / ms / us / ns / ps / fs`. The
precision must be less than or equal to the unit (`1ns/10ns` is illegal).

### Scoping behaviour

A `timescale` applies to every module declared after it, and it crosses file
boundaries. When several files each declare a `timescale`, **the last declaration
overrides what follows it**.

```verilog
// fileA.v
`timescale 1ns/1ps
module A; ... endmodule

// fileB.v  (compiled after fileA.v)
`timescale 1us/1ns
module B; ... endmodule
// from here on, the insides of module A may be reinterpreted as 1us/1ns
```

To stop this leakage, put a `` `resetall `` at the head of each file and re-declare the
`timescale` you want:

```verilog
// fileA.v — the safe pattern
`resetall
`timescale 1ns/1ps
`default_nettype none
module A; ... endmodule
```

---

## `default_nettype — implicit net type

```verilog
`default_nettype none    // disable implicit net declarations
`default_nettype wire    // the default (implicit wires allowed)
```

### Why `default_nettype none` is worth using

Under the `wire` default, any undeclared signal name silently becomes a 1-bit `wire`. A
typo creates a net that is not connected to anything, and the simulation quietly
propagates X:

```verilog
// ❌ default_nettype wire (the default) — hazardous
module buggy(output y, input a, b);
    assign y = aaaa & b;   // typo: 'a' → 'aaaa'
    // 'aaaa' is created as an implicit wire → disconnected from a, y is always 0
    // no compile error → very hard to debug
endmodule
```

```verilog
// ✅ default_nettype none — safe
`default_nettype none
module safe(output y, input a, b);
    assign y = aaaa & b;   // 'aaaa' undeclared → immediate compile error
endmodule
```

Restore the default at the end of the file so other files are not affected:

```verilog
`default_nettype none
// ... module declarations ...
`default_nettype wire   // restore (or `resetall)
```

---

## `begin_keywords / `end_keywords — selecting the reserved-word set

```verilog
`begin_keywords "1364-2001"
// inside this region only Verilog-2001 keywords are reserved
// the SV additions (interface, program, ...) may be used as identifiers
module old_code;
    wire interface;   // reserved in SV, but allowed inside this region
endmodule
`end_keywords
```

### Valid version strings

| Version string | Keyword set |
|-----------|-----------|
| `"1364-1995"` | Verilog-95 keywords |
| `"1364-2001"` | Verilog-2001 keywords |
| `"1364-2005"` | Verilog-2005 keywords |
| `"1800-2005"` | SystemVerilog-2005 keywords |
| `"1800-2009"` | SystemVerilog-2009 keywords |
| `"1800-2012"` | SystemVerilog-2012 keywords |
| `"1800-2017"` | SystemVerilog-2017 keywords (default) |

It may only be declared **outside** a module, primitive, interface, program or package.
Its main use: processing older Verilog code with a SystemVerilog toolchain, where the
newer keywords would otherwise collide with existing identifiers.

---

## `resetall — reset every directive

Returns every directive that has a default to its initial value. Directives affected:
- `` `timescale `` (removed)
- `` `default_nettype `` → `wire`
- `` `unconnected_drive `` (removed)
- `` `celldefine `` / `` `endcelldefine `` (removed)

```verilog
// the convention is to put this at the head of the file — it clears settings that
// leaked in from another file
`resetall
`timescale 1ns/1ps
`default_nettype none

module my_module;
    // ...
endmodule

`resetall   // restore at the end of the file (optional)
```

---

## `celldefine / `endcelldefine — marking library cells

Marks the modules declared between them as library cells. SDF (Standard Delay Format)
back-annotation tools and timing analysers read the flag and treat the contents as a
black box:

```verilog
`celldefine
module AND2X1 (output Y, input A, B);
    assign Y = A & B;
endmodule
`endcelldefine

`celldefine
module DFFX1 (output Q, input D, CK);
    always @(posedge CK) Q <= D;
endmodule
`endcelldefine
```

Apply it to every cell module when writing a standard cell library.

---

## `pragma — tool-specific hints

The standard defines the `pragma syntax, but the meaning of the keywords differs from
tool to tool:

```verilog
// synthesis translate_off (the Synopsys/Xilinx convention)
initial $display("debug: x=%0h", x);
// synthesis translate_on

`pragma protect begin    // start of IP encryption (Xilinx/Cadence)
// ... code to be encrypted ...
`pragma protect end
```

`pragma is not standardised, so it does not port between compilers. When conditional
compilation is what you actually want, prefer the `` `ifdef SYNTHESIS `` pattern.

> **Interpretation is left to the tool.** IEEE 1800 §22.11 leaves the meaning of a
> pragma to the implementation, so no pragma keyword carries portable semantics. For
> what vita does with the directive, see
> [docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## `line — inserting source-position information

Inserted by code generators and preprocessors so that error messages name the original
file and line number:

```
`line <line_number> "<filename>" <level>
```

- `level = 0`: ordinary (within the current file)
- `level = 1`: entering an include file
- `level = 2`: returning from an include file

```verilog
`line 42 "original_source.v" 0
// compile errors after this point are reported as "original_source.v:42"
```

Rarely used in hand-written RTL. It appears in the output of code generators and
macro-expansion tools.

---

## Directive scope and file patterns, summarised

| Directive | Default | Confined to one file | Recommended pattern |
|--------|-------|--------------|----------|
| `` `define `` | none | ❌ (leaks past the file) | header file + include guard |
| `` `timescale `` | none | ❌ | state it at the head of every file, with the `` `resetall `` pattern |
| `` `default_nettype `` | `wire` | ❌ | declare `none`, then `` `resetall `` at the end of the file |
| `` `celldefine `` | inactive | ❌ | apply across a whole cell-library file |
| `` `begin_keywords `` | "1800-2017" | ✅ (bounded by `` `end_keywords ``) | limit it to the legacy-code region |
| `` `resetall `` | — | — | insert at the head of the file |

---

## Sources

- IEEE 1364-2001 §19 (compiler directives)
- IEEE 1800-2017 §22 (compiler directives)
- chipverify.com/verilog/verilog-compiler-directives (verified by WebFetch ✓)
- chipverify.com/verilog/verilog-define-macros (verified by WebFetch ✓, macro argument syntax)
- hdlworks.com/hdl_corner/verilog_ref/items/CompilerDirectives.htm (verified by WebFetch ✓, `resetall / `line / `celldefine)
- vlsiverify.com/verilog/compiler-directives/ (conditional compilation examples)
- accellera.org P1800 keyword compatibility directive proposal (cross-check of the `begin_keywords version strings)
- front-end-verification.blogspot.com (verification that default_nettype none catches typos)
- analogcircuitdesign.com/verilog-compiler-directives/ (`timescale scoping behaviour)
