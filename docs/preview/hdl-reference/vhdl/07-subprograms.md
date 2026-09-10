# 07 · VHDL Subprograms — Function & Procedure

Per IEEE 1076-2008 §4. A subprogram is either a **function** or a **procedure**. Subprograms are the
main tool for abstracting repeated logic, overloading operators and writing testbench utilities.

---

## Function vs procedure

| Item | function | procedure |
|------|---------|-----------|
| Return value | exactly one (`return TYPE`) | none (`return;` is an early exit only) |
| Call context | an **expression** (wherever a value is needed) | a **statement** (in statement position) |
| Parameter modes | `in` only | `in` / `out` / `inout` |
| Parameter classes | constant, signal, file (`in`) | constant, signal, variable, file |
| `wait` statement | not allowed | allowed (simulation) |
| Purity | `pure` (default) / `impure` | inherently impure |
| Concurrent call | no (it is an expression) | yes (concurrent procedure call) |

---

## Function

### Syntax

```vhdl
[pure|impure] function FUNC_NAME (
    param1 : [constant|signal|file] in type_name [:= default_val];
    param2 : in type_name [:= default_val]
) return return_type is
    -- declarative part: variables, constants, nested subprograms, ...
begin
    -- sequential statements
    return expression;
end [function] [FUNC_NAME];
```

- `pure` is the default and may be omitted. The `impure` keyword is written only to declare that the
  function is impure.
- A separate **function declaration** is optional — the body alone is enough.
- A `return` statement must be present.

### pure vs impure

| | `pure` (default) | `impure` |
|-|--------------|---------|
| Same arguments → same result | **guaranteed** | not guaranteed |
| Access to objects outside the scope | **not allowed** (shared variables, files, ...) | allowed |
| Calling an impure function | **not allowed** | allowed |
| Synthesis | no restriction | not synthesizable when it depends on external state |
| Typical use | combinational logic, type conversion, arithmetic | random-number generation, reading file stimulus |

```vhdl
-- pure function: abstracting combinational logic
pure function parity(v : std_logic_vector) return std_logic is
    variable p : std_logic := '0';
begin
    for i in v'range loop
        p := p xor v(i);
    end loop;
    return p;
end function parity;

-- impure function: reaching shared state (for a testbench)
shared variable seed1 : integer := 42;
shared variable seed2 : integer := 17;

impure function rand_int(lo, hi : integer) return integer is
    variable r : real;
begin
    uniform(seed1, seed2, r);               -- math_real uniform()
    return lo + integer(r * real(hi - lo));
end function rand_int;
```

### Parameter defaults

```vhdl
-- a parameter with a default may be omitted at the call site
function to_slv(
    val   : integer;
    width : natural := 8         -- default: 8
) return std_logic_vector is
begin
    return std_logic_vector(to_unsigned(val, width));
end function;

-- calls
signal a : std_logic_vector(7 downto 0) := to_slv(42);       -- width=8 omitted
signal b : std_logic_vector(15 downto 0) := to_slv(42, 16);  -- given explicitly
```

### Recursive functions

```vhdl
-- synthesizable only if the depth is fixed at compile time
function clog2(n : positive) return natural is
begin
    if n <= 1 then
        return 0;
    else
        return 1 + clog2((n + 1) / 2);   -- recursive call
    end if;
end function clog2;

-- use: computing an address-bus width (used only as an elaboration-time constant)
constant ADDR_WIDTH : natural := clog2(MEM_DEPTH);
```

> **Synthesis note:** a recursive function has to unroll statically during elaboration. Recursion
> whose depth is decided at run time is not synthesizable.

---

## Procedure

### Syntax

```vhdl
procedure PROC_NAME (
    signal   clk   : in    std_logic;
    variable data  : out   integer;
    signal   bus   : inout std_logic_vector(7 downto 0);
    constant LIMIT : in    integer := 255    -- defaults are supported
) is
    -- declarative part
begin
    -- sequential statements (wait allowed)
    [return;]   -- optional, an early exit
end [procedure] [PROC_NAME];
```

- **Default class per mode**: `in` → constant, `out` / `inout` → variable (signal if `signal` is
  written explicitly).
- `out` / `inout` parameters are how a procedure effectively returns several values.

### Parameter modes

```vhdl
procedure add_with_carry (
    a, b   : in  unsigned(7 downto 0);
    result : out unsigned(8 downto 0)   -- 9 bits, so the carry fits
) is
begin
    result := ('0' & a) + ('0' & b);
end procedure;

-- calling it
procedure_result : process(a, b)
    variable sum9 : unsigned(8 downto 0);
begin
    add_with_carry(a, b, sum9);
    carry  <= sum9(8);
    result <= sum9(7 downto 0);
end process;
```

### signal parameters + wait (for testbenches)

```vhdl
-- encapsulating an SPI write sequence in a testbench
procedure spi_write (
    signal sck   : out std_logic;
    signal mosi  : out std_logic;
    signal cs_n  : out std_logic;
    data         : in  std_logic_vector(7 downto 0);
    constant T   : in  time := 10 ns
) is
begin
    cs_n <= '0';
    for i in 7 downto 0 loop
        sck  <= '0';
        mosi <= data(i);
        wait for T;
        sck  <= '1';
        wait for T;
    end loop;
    sck  <= '0';
    cs_n <= '1';
    wait for T;
end procedure;

-- called from a testbench process
spi_write(sck, mosi, cs_n, X"A5");
```

### Concurrent procedure call

```vhdl
-- may be called directly at architecture-body level (the concurrent region)
-- it is wrapped in a process automatically
LABEL : PROC_NAME(port_or_signal_list);

-- example: running a bus monitor continuously in the same architecture
bus_mon : monitor_bus(clk, addr, data, wr_en);
```

---

## Subprogram overloading

Several subprograms may share one name as long as their type signatures differ. The compiler picks
the right version from the argument types.

```vhdl
-- overloaded by type
function to_slv(val : integer;  width : natural) return std_logic_vector;
function to_slv(val : unsigned)                  return std_logic_vector;
function to_slv(val : boolean)                   return std_logic_vector;

-- selected automatically at the call site
signal a : std_logic_vector(7 downto 0) := to_slv(42, 8);   -- the first
signal b : std_logic_vector(7 downto 0) := to_slv(u_val);   -- the second
signal c : std_logic_vector(0 downto 0) := to_slv(true);    -- the third
```

---

## Operator overloading

Naming a function after an **operator string** defines that operator for a new type.

### Overloadable operators

| Class | Operators |
|------|--------|
| Arithmetic | `+` `-` `*` `/` `**` `mod` `rem` `abs` |
| Relational | `=` `/=` `<` `<=` `>` `>=` |
| Logical | `and` `or` `nand` `nor` `xor` `xnor` `not` |
| Shift | `sll` `srl` `sla` `sra` `rol` `ror` |
| Concatenation | `&` |

### Definition syntax

```vhdl
-- an operator function: the name is the operator symbol in double quotes
function "+" (L, R : my_vec_t) return my_vec_t is
begin
    return my_vec_t(unsigned(L) + unsigned(R));
end "+";

function "<" (L, R : my_rec_t) return boolean is
begin
    return L.value < R.value;
end "<";
```

### Defining them in a package (recommended)

```vhdl
package my_types_pkg is
    type q16_t is array (15 downto 0) of std_logic;  -- 16-bit fixed point

    function "+" (L, R : q16_t) return q16_t;
    function "-" (L, R : q16_t) return q16_t;
    function "*" (L, R : q16_t) return q16_t;
end package;

package body my_types_pkg is
    function "+" (L, R : q16_t) return q16_t is
    begin
        return q16_t(signed(L) + signed(R));
    end "+";
    -- ... the rest of the implementations
end package body;
```

This is exactly how `ieee.std_logic_1164` and `ieee.numeric_std` define their std_logic, unsigned
and signed operations internally.

---

## Subprogram declarations vs bodies

```vhdl
package util_pkg is
    -- declaration: the signature only
    function parity(v : std_logic_vector) return std_logic;
    procedure check_range(val, lo, hi : in integer);
end package;

package body util_pkg is
    -- body: the actual implementation
    function parity(v : std_logic_vector) return std_logic is
        variable p : std_logic := '0';
    begin
        for i in v'range loop p := p xor v(i); end loop;
        return p;
    end function;

    procedure check_range(val, lo, hi : in integer) is
    begin
        assert val >= lo and val <= hi
            report "Value " & integer'image(val) & " out of range"
            severity ERROR;
    end procedure;
end package body;
```

- Declare in the **package spec**, implement in the **package body**.
- Inside a single entity/architecture the body alone is enough (the declaration is optional).
- Splitting the declaration from the body is what makes a forward reference possible.

---

## Synthesis checklist

| Pattern | Synthesis result | Note |
|------|----------|------|
| pure function, no wait | ✅ synthesizable | becomes combinational logic |
| impure function (no external state) | ✅ synthesizable | effectively pure |
| impure function (touches a shared variable) | ❌ not synthesizable | global state = not synthesizable |
| function with wait | ❌ not synthesizable | wait = not synthesizable |
| procedure (no wait, no signal parameter) | ✅ synthesizable | |
| procedure with wait | ❌ not synthesizable | testbench only |
| recursion (static depth) | ✅ synthesizable | unrolled during elaboration |
| recursion (dynamic depth) | ❌ not synthesizable | a run-time depth is impossible |
| operator overloading | ✅ synthesizable | provided the implementing function is |
| overloaded operator with file I/O | ❌ not synthesizable | because of the file I/O |

---

## Sources

- IEEE 1076-2008 §4 (Subprograms and packages)
- Azimuth — VHDL Function vs Procedure: https://azimuth.tech/2025/02/23/whats-the-difference-between-a-vhdl-function-or-procedure/ (WebFetch ✓)
- VHDL-Online — Subprograms: https://www.vhdl-online.de/courses/system_design/vhdl_language_and_syntax/subprograms (WebFetch ✓)
- HDLworks — Function reference: https://www.hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/Function.htm (WebFetch ✓)
- HDLworks — Operator Overloading: https://www.hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/OperatorOverloading.htm
- Research log: [vhdl-subprograms-pkg-synth-2026-05-28.md](../../../history/research-log/vhdl-subprograms-pkg-synth-2026-05-28.md)
