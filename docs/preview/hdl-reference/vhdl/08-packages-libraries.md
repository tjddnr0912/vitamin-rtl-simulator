# 08 · VHDL Packages & Standard Libraries

Per IEEE 1076-2008 §4 (package syntax) and §16 (standard packages).

---

## Package syntax

A package bundles types, constants, subprograms and component declarations into one namespace.

```vhdl
-- package declaration (spec)
package PKG_NAME is
    -- type declaration
    type my_state_t is (IDLE, RUN, DONE);

    -- constant
    constant CLK_FREQ : integer := 100_000_000;

    -- subprogram declaration (the signature only)
    function to_slv(val : my_state_t) return std_logic_vector;

    -- component declaration (optional — VHDL-2008 prefers direct instantiation)
    component uart_rx
        port (clk, rx : in std_logic; data : out std_logic_vector(7 downto 0));
    end component;
end package PKG_NAME;

-- package body — needed when a subprogram has to be implemented
package body PKG_NAME is
    function to_slv(val : my_state_t) return std_logic_vector is
    begin
        return std_logic_vector(to_unsigned(my_state_t'pos(val), 2));
    end function;
end package body PKG_NAME;
```

### library and use clauses

```vhdl
library WORK;                   -- the current work library (included by default)
use work.PKG_NAME.all;          -- make the whole package visible

library IEEE;
use ieee.std_logic_1164.all;    -- the IEEE packages
use ieee.numeric_std.all;
```

---

## The standard packages

| Package | Synthesis | Summary |
|--------|------|----------|
| `std.standard` | ✅ | all the base types (implicit — no use clause needed) |
| `std.textio` | ❌ | file I/O (testbench) |
| `ieee.std_logic_1164` | ✅ | 9-value logic, the std_logic family |
| `ieee.numeric_std` | ✅ | signed/unsigned arithmetic **[recommended]** |
| `ieee.std_logic_arith` | ⚠️ | **DEPRECATED — do not use** |
| `ieee.std_logic_unsigned` | ⚠️ | **DEPRECATED — do not use** |
| `ieee.std_logic_signed` | ⚠️ | **DEPRECATED — do not use** |
| `ieee.numeric_std_unsigned` | ✅ | arithmetic directly on std_logic_vector (2008+) |
| `ieee.math_real` | ❌ | maths functions (testbench / elaboration) |
| `ieee.fixed_pkg` | ✅⚠️ | fixed point (2008+, check tool support) |
| `ieee.float_pkg` | ✅⚠️ | IEEE 754 floating point (2008+, mind the area) |

---

## std.standard — implicit, no use clause

Included automatically in every VHDL design unit. The types below are available without any explicit
`use` clause.

```vhdl
-- usable without a use clause
signal flag  : boolean;                         -- false / true
signal bit0  : bit;                             -- '0' / '1'
signal vec   : bit_vector(7 downto 0);
signal ch    : character;
signal str   : string(1 to 8);
signal n     : integer range 0 to 2**16-1;
signal nat   : natural;                         -- 0 to integer'high
signal pos   : positive;                        -- 1 to integer'high
signal r     : real;                            -- (not synthesizable)
signal t     : time;                            -- (not synthesizable)
```

**Predefined subtypes:**

| Name | Definition |
|------|------|
| `natural` | `integer range 0 to integer'high` |
| `positive` | `integer range 1 to integer'high` |

---

## std.textio — file I/O, testbench only

```vhdl
use std.textio.all;
```

```vhdl
-- reading a file (testbench)
file input_file : text open READ_MODE is "stimulus.txt";
variable line_buf : line;
variable val      : integer;

process
begin
    while not endfile(input_file) loop
        readline(input_file, line_buf);     -- read one line
        read(line_buf, val);                -- parse it as an integer
        data_in <= std_logic_vector(to_signed(val, 8));
        wait until rising_edge(clk);
    end loop;
    wait;
end process;
```

> **VHDL-2008:** what `std_logic_textio` provided (the std_logic read/write procedures) has been
> folded into `ieee.std_logic_1164`. `std_logic_textio` is now a stub, but it can still be named for
> backward compatibility.

---

## ieee.std_logic_1164 — the 9-value logic standard

```vhdl
library ieee;
use ieee.std_logic_1164.all;
```

### The nine values (std_ulogic)

| Value | Meaning |
|----|------|
| `'U'` | Uninitialized |
| `'X'` | Unknown (strong) |
| `'0'` | Logic 0 (strong) |
| `'1'` | Logic 1 (strong) |
| `'Z'` | High impedance |
| `'W'` | Unknown (weak) |
| `'L'` | Logic 0 (weak) |
| `'H'` | Logic 1 (weak) |
| `'-'` | Don't care |

### std_ulogic vs std_logic

```vhdl
-- std_ulogic: a single driver only (unresolved)
signal u : std_ulogic;

-- std_logic: carries a resolution function
-- → several drivers may be connected, so buses and tri-states can be modelled
signal s : std_logic;                  -- the default port type
signal v : std_logic_vector(7 downto 0);
```

For synthesis, std_logic and std_ulogic are treated identically.

### Conversion functions

```vhdl
-- std_logic ↔ bit conversion (interworking with the legacy bit type)
b  := to_bit(sl);                      -- std_logic → bit ('H'→'1', 'L'→'0')
bv := to_bitvector(slv);               -- std_logic_vector → bit_vector
sl := to_stdulogic(b);                 -- bit → std_ulogic
sl := to_stdlogic(b);                  -- bit → std_logic (VHDL-2008+)
sv := to_stdlogicvector(bv);           -- bit_vector → std_logic_vector
```

### The main (overloaded) operators

`and`, `or`, `nand`, `nor`, `xor`, `xnor`, `not` — defined for std_logic and std_logic_vector.

```vhdl
-- reduction operators (VHDL-2008+)
result <= and  slv;    -- AND of every bit
result <= or   slv;    -- OR of every bit
result <= xor  slv;    -- XOR of every bit (odd parity)
result <= nand slv;
result <= nor  slv;
result <= xnor slv;    -- XNOR of every bit (even parity)
```

---

## ieee.numeric_std — the arithmetic standard **[required for new designs]**

```vhdl
library ieee;
use ieee.numeric_std.all;
```

Defines the two types `signed` and `unsigned`, both built on arrays of std_logic.

```vhdl
signal u : unsigned(7 downto 0) := to_unsigned(200, 8);  -- unsigned
signal s : signed(7 downto 0)  := to_signed(-100, 8);    -- two's complement
```

### Arithmetic and comparison

```vhdl
-- mind the overflow: you manage the result width yourself
signal a, b : unsigned(7 downto 0);
signal sum9 : unsigned(8 downto 0);

sum9 <= ('0' & a) + ('0' & b);         -- 9 bits, so the carry fits

-- comparison (numeric interpretation)
if unsigned(addr) < to_unsigned(BASE, 16) then ...
```

### Conversion functions

```vhdl
-- signed/unsigned → integer
n := to_integer(u_val);               -- unsigned → integer (always >= 0)
n := to_integer(s_val);               -- signed → integer (negatives included)

-- integer → signed/unsigned (the size must be given)
u := to_unsigned(42, 8);             -- 8-bit unsigned
s := to_signed(-5,  8);              -- 8-bit signed

-- changing the size
u2 := resize(u, 16);                 -- unsigned: zero-extend
s2 := resize(s, 16);                 -- signed: sign-extend
```

### Shift and rotate

```vhdl
-- shifts (vacated positions filled with 0)
u_shifted := shift_left (u, 3);       -- logical left shift (*8)
u_shifted := shift_right(u, 3);       -- logical right shift, unsigned: fills with 0
s_shifted := shift_right(s, 3);       -- arithmetic right shift, signed: replicates the MSB

-- rotates
u_rot := rotate_left (u, 2);
u_rot := rotate_right(u, 2);
```

### Type-conversion patterns

```vhdl
-- casting between std_logic_vector and unsigned/signed
u_val := unsigned(slv);               -- a reinterpretation (no bits are copied)
s_val := signed(slv);
slv   := std_logic_vector(u_val);
```

---

## ieee.std_logic_arith — ⚠️ DEPRECATED, do not use

```vhdl
-- never use these packages in a new design
-- use ieee.std_logic_arith.all;      -- ❌ DEPRECATED
-- use ieee.std_logic_unsigned.all;   -- ❌ DEPRECATED
-- use ieee.std_logic_signed.all;     -- ❌ DEPRECATED
```

**Why they are dangerous:**

1. **Non-standard** — written by Synopsys. The IEEE never standardised them.
2. **They break portability** — the Synopsys, Cadence and Mentor implementations differ from one
   another, defining incompatible types in the same IEEE namespace.
3. **They collide** — the `SIGNED` / `UNSIGNED` of `std_logic_arith` are **different types** from
   those in `numeric_std`. Using both packages together is a compile error.
4. **Mutually exclusive** — `std_logic_signed` and `std_logic_unsigned` cannot be used in the same
   design unit.

**Use instead:**

```vhdl
-- the standard combination for new designs
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;              -- all arithmetic comes from here
```

---

## ieee.numeric_std_unsigned — VHDL-2008+

```vhdl
use ieee.numeric_std_unsigned.all;
```

Lets `std_logic_vector` take part in arithmetic directly, without a cast. This is the standard
replacement for `std_logic_unsigned`.

```vhdl
-- with numeric_std_unsigned
signal a, b, c : std_logic_vector(7 downto 0);
c <= a + b;                            -- unsigned arithmetic applied directly
c <= a + "00000001";                   -- constants work too
if a > b then ...                      -- numeric comparison
```

> Using it together with `ieee.numeric_std` can make operators such as `+` and `<` ambiguous. Be
> careful about mixing them in one design unit.

---

## ieee.math_real — maths functions, testbench only

```vhdl
use ieee.math_real.all;
```

```vhdl
-- constants
MATH_PI        -- 3.14159...
MATH_E         -- 2.71828...
MATH_SQRT2     -- 1.41421...
MATH_LOG2E     -- log2(e)

-- functions
sqrt(x)        ceil(x)    floor(x)   round(x)
log(x)         log2(x)    log10(x)   exp(x)
sin(x)         cos(x)     tan(x)
arcsin(x)      arccos(x)  arctan(x)  arctan2(y, x)
uniform(s1, s2, r)  -- uniformly distributed random number in [0.0, 1.0)
```

**Not synthesizable.** Use it only for generating testbench stimulus and computing
elaboration-time constants.

```vhdl
-- use: deriving a parameter (an elaboration constant)
constant SAMPLES : integer := integer(ceil(MATH_PI * real(N)));

-- random stimulus (testbench)
impure function rand_slv(len : natural) return std_logic_vector is
    variable r    : real;
    variable s1, s2 : integer := 47;
    variable result : std_logic_vector(len-1 downto 0);
begin
    for i in result'range loop
        uniform(s1, s2, r);
        result(i) := '1' when r > 0.5 else '0';
    end loop;
    return result;
end function;
```

---

## ieee.fixed_pkg — fixed point (VHDL-2008+)

```vhdl
use ieee.fixed_pkg.all;
```

### The types and their index notation

```vhdl
-- sfixed(integer_part_MSB downto fractional_part_LSB)
-- the binary point sits between index 0 and index -1
signal x : sfixed( 7 downto -8);   -- 8.8 format: 16 bits, ±127.996
signal y : ufixed( 7 downto -8);   -- 8.8 format: 16 bits, 0 .. 255.996
signal z : sfixed(15 downto -16);  -- 16.16 format: 32 bits
```

```vhdl
-- arithmetic
signal a, b : sfixed(7 downto -8);
signal c    : sfixed(8 downto -8);   -- one extra bit: prevents overflow

c <= a + b;                           -- the binary points are aligned automatically
c <= resize(a + b, c'high, c'low);    -- resized explicitly
```

Synthesizable (Vivado supports it). Some older tools support it only in part — check the tool before
using it.

---

## ieee.float_pkg — IEEE 754 floating point (VHDL-2008+)

```vhdl
use ieee.float_pkg.all;
```

```vhdl
signal f32 : float32;                     -- IEEE 754 single precision
signal f64 : float64;                     -- IEEE 754 double precision

-- integer/real ↔ float conversion
f32 <= to_float(42, f32);                 -- integer → float32
f32 <= to_float(3.14, f32);              -- real → float32
n   := to_integer(f32);
r   := to_real(f32);                      -- (elaboration time only)
```

Synthesizable, but it consumes a **substantial amount of LUT area**. In FPGA designs, look at an IP
core (Vivado's Floating Point IP, for example) first.

---

## Recommended use-clause combinations

```vhdl
-- RTL design (to be synthesized)
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

-- testbench
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
use ieee.math_real.all;
use std.textio.all;

-- fixed-point RTL (VHDL-2008+)
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
use ieee.fixed_pkg.all;
```

---

## Sources

- IEEE 1076-2008 §4 (Packages and package bodies), §16 (Predefined packages)
- HDL Factory — VHDL IEEE Libraries and Numeric Type Conversion: https://www.hdlfactory.com/post/2025/06/29/vhdl-ieee-libraries-and-numeric-type-conversion-a-definitive-reference/ (WebFetch ✓)
- Sigasi — Deprecated IEEE Libraries: https://www.sigasi.com/tech/deprecated-ieee-libraries/ (WebFetch ✓)
- Doulos — VHDL-2008: Incorporates existing standards: https://www.doulos.com/knowhow/vhdl/vhdl-2008-incorporates-existing-standards/
- VHDL-2008 Support Library (fphdl ReadTheDocs): https://fphdl.readthedocs.io/en/docs/
- HDLworks — Std_Logic_1164: https://www.hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/StdLogic1164.htm
- Research log: [vhdl-subprograms-pkg-synth-2026-05-28.md](../../../history/research-log/vhdl-subprograms-pkg-synth-2026-05-28.md)
