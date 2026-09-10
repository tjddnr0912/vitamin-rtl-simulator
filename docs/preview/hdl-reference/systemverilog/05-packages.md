# 05 · SystemVerilog Packages

Based on IEEE 1800-2017 §26. A package is the construct that shares types, constants,
functions, tasks and classes outside a compilation unit, together with a namespace of its own.
It solves the name-collision and portability problems of the Verilog-era `` `include `` +
global-declaration approach.

---

## Package declaration and body (§26.2)

```systemverilog
package bus_pkg;
    // type definitions
    typedef enum logic [1:0] {
        IDLE  = 2'b00,
        WRITE = 2'b01,
        READ  = 2'b10,
        RESP  = 2'b11
    } bus_state_e;

    // parameters
    parameter int DATA_W = 32;
    parameter int ADDR_W = 32;

    // function
    function automatic logic [DATA_W-1:0] swap32(
        input logic [DATA_W-1:0] d
    );
        return {d[7:0], d[15:8], d[23:16], d[31:24]};
    endfunction

    // task
    task automatic print_state(input bus_state_e s);
        $display("state = %s", s.name());
    endtask
endpackage
```

A package is the central store for the types and subroutines shared by modules, interfaces and
program blocks. By default an item inside a package is scoped under the package name.

---

## import — explicit and wildcard (§26.3)

### Explicit import (recommended)

Brings only the named items into the current scope. The code says plainly where each name
came from.

```systemverilog
import bus_pkg::bus_state_e;  // the type only
import bus_pkg::DATA_W;        // the parameter only
import bus_pkg::swap32;        // the function only
```

After an explicit import the name can be used directly, with no scope resolution operator:

```systemverilog
module foo
    import bus_pkg::bus_state_e;
    (
        input  logic         clk,
        input  bus_state_e   state_in,   // a package type used in a port declaration
        output bus_state_e   state_out
    );
    always_ff @(posedge clk)
        state_out <= state_in;
endmodule
```

Placing `import bus_pkg::bus_state_e;` in the module declaration itself is syntax permitted
since IEEE 1800-2009. The import is processed ahead of the port declarations, so a package
type can be used directly as a port type.

### Wildcard import

Brings a whole package in at once. It carries a name-collision risk, so it suits the
convenience of a small design or a testbench.

```systemverilog
import bus_pkg::*;  // every public identifier of bus_pkg
```

A wildcard import does not **reserve** the names — if a local declaration uses the same name,
the local one wins (shadowing). If two wildcard imports bring in the same name, the result is
a compile error (ambiguous identifier).

```systemverilog
import pkg_a::*;  // pkg_a::foo exists
import pkg_b::*;  // pkg_b::foo exists
// using foo is a compile error: ambiguous
```

---

## export (§26.5)

Used when a package re-exposes to its own users an item it imported from another package.

```systemverilog
package low_pkg;
    typedef int my_type;
endpackage

package mid_pkg;
    import low_pkg::my_type;   // brought in from low_pkg
    export low_pkg::my_type;   // re-exposed to users of mid_pkg
    // or re-export everything: export low_pkg::*;
endpackage

// user side: importing mid_pkg alone is enough to reach my_type
module foo;
    import mid_pkg::my_type;
    my_type x;
endmodule
```

With `import` alone and no `export`, `import low_pkg::my_type` is visible only inside
`mid_pkg`; code that imports `mid_pkg` from outside does not see `my_type`.

---

## $unit — the compilation-unit scope and its hazards (§3.12)

`$unit` is the file top-level scope that precedes any module, package or interface
declaration. It is the formalisation of Verilog's traditional global-declaration style.

```systemverilog
// file top level — the $unit scope (above any package declaration)
typedef logic [7:0] byte_t;   // declared in $unit
parameter int TOP_W = 8;

module foo;
    byte_t x;    // references byte_t from $unit
    // ...
endmodule
```

### Why $unit is hazardous

**1. Tool-dependent compilation-unit boundary**

Tools differ in how they define a "compilation unit".
- Simulator A: one file = one compilation unit → a $unit declaration is visible only in that file
- Simulator B: all files treated as a single compilation unit → a $unit declaration is visible in every file
- If the EDA tool compiles each file separately, a $unit declaration is not visible from another file

**2. Dependence on file compilation order**

Visibility changes with which file a $unit declaration is compiled in first. Reorder the
files in the build script and the behaviour changes.

**3. Lowest search priority**

Identifier search order:
```
local declaration (highest)
  ↓
explicit import (import pkg::item)
  ↓
wildcard import (import pkg::*)
  ↓
$unit scope (lowest)
```

**Recommendation**: do not use `$unit`. When a shared declaration is needed, always use a
package.

---

## import precedence rules

Which one the compiler picks when the same name exists in several scopes:

```systemverilog
package pkg_a;
    parameter int X = 1;
endpackage

package pkg_b;
    parameter int X = 2;
endpackage

module test;
    import pkg_a::*;   // X = 1 (wildcard)
    import pkg_b::X;   // X = 2 (explicit)

    int X = 99;        // local declaration

    initial $display(X);  // 99 — the local declaration wins
endmodule
```

Precedence, summarised:

| Precedence | Item | Description |
|---------|------|------|
| 1 (highest) | Local declaration | Declared directly inside that module or function |
| 2 | Explicit import | `import pkg::item` form |
| 3 | Wildcard import | `import pkg::*` form |
| 4 (lowest) | $unit | Compilation-unit top-level scope |

**Wildcard collision rule**: when the same name is brought in by two wildcard imports, the
compile error is raised **at the point where that name is actually used** (importing it is
not itself an error — the ambiguous error comes at use).

---

## Practical patterns

### A common package structure

```systemverilog
// types_pkg.sv — base type definitions
package types_pkg;
    typedef logic [31:0] word_t;
    typedef logic [7:0]  byte_t;
    typedef enum {RD, WR, NOP} op_e;
endpackage

// params_pkg.sv — design parameters
package params_pkg;
    parameter int CLK_PERIOD = 10;
    parameter int FIFO_DEPTH = 16;
endpackage

// utils_pkg.sv — shared functions
package utils_pkg;
    import types_pkg::word_t;
    export types_pkg::word_t;  // importing utils_pkg alone is enough to reach word_t

    function automatic word_t byteswap(input word_t d);
        return {d[7:0], d[15:8], d[23:16], d[31:24]};
    endfunction
endpackage
```

---

## Sources

- IEEE 1800-2017 §26 (Packages), §3.12 (Compilation units), §26.3 (Import precedence)
- chipverify.com/systemverilog/systemverilog-package
- blogs.sw.siemens.com/verificationhorizons/2009/09/25/unit-vs-root/ (Dave Rich)
- blogs.sw.siemens.com/verificationhorizons/2010/07/13/package-import-versus-include/
- bradpierce.wordpress.com (package import vs $unit)
- dvcon-proceedings.org/using-systemverilog-packages-in-real-verification-proj.pdf
