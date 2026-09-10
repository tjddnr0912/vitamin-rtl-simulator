# 09 · VHDL Synthesizability

Per IEEE 1076-2008. Records whether a synthesis tool (Vivado, Quartus and the like) can turn VHDL
source into a real gate-level netlist.

**Legend:**
- ✅ Synthesizable (universal) — supported by all the mainstream tools
- ⚠️ Conditional — support depends on the tool, the version or how it is written
- ❌ Not synthesizable — unusable in RTL design (simulation and testbench only)

---

## ✅ Synthesizable (universal)

### Signal types

```vhdl
signal a : std_logic;                        -- one bit of logic
signal b : std_logic_vector(7 downto 0);     -- a vector
signal c : std_ulogic;                       -- unresolved (identical to std_logic for synthesis)
signal d : unsigned(7 downto 0);            -- the numeric_std types
signal e : signed(7 downto 0);
```

```vhdl
-- integer: a range constraint is mandatory (an unbounded range is not synthesizable)
signal cnt : integer range 0 to 255;         -- ✅
signal bad : integer;                        -- ⚠️ a warning or an error, depending on the tool
```

### Enumerations and boolean

```vhdl
type state_t is (IDLE, FETCH, DECODE, EXECUTE, WRITEBACK);
signal state : state_t;         -- encoded automatically, binary or one-hot

signal flag : boolean;          -- false/true → 0/1
```

### Process (with a complete sensitivity list)

```vhdl
-- synchronous (clock + asynchronous reset)
process(clk, rst_n)
begin
    if rst_n = '0' then
        q <= (others => '0');
    elsif rising_edge(clk) then
        q <= d;
    end if;
end process;

-- combinational (VHDL-2008: process(all))
process(all)                     -- all = every signal read is included automatically
begin
    y <= a and b;
end process;
```

### if / case

```vhdl
-- if: without an else a latch is inferred → in a combinational process the else is mandatory
if sel = '1' then
    y <= a;
else
    y <= b;                      -- the else is what avoids the latch
end if;

-- case: when others is recommended (std_logic_vector has further values such as 'U' and 'X')
case opcode is
    when "00" => exec_add;
    when "01" => exec_sub;
    when others => exec_nop;     -- covers everything else
end case;
```

### for loops (static range)

```vhdl
-- unrolled by synthesis → N pieces of parallel hardware
for i in 0 to 7 loop
    result(i) <= a(i) xor b(i);
end loop;

-- parameterised by a generic
for i in 0 to WIDTH-1 loop      -- fine as long as WIDTH is an elaboration-time constant
    sum := sum + to_integer(unsigned'(0 => vec(i)));
end loop;
```

### generate

```vhdl
-- for-generate: creates N instances automatically
gen_ff : for i in 0 to N-1 generate
    dff_i : dff port map(clk => clk, d => d(i), q => q(i));
end generate;

-- if-generate: structure conditional on a parameter
gen_rst : if HAS_RESET generate
    reset_logic : process(clk, rst_n) ...
end generate;

-- case-generate (VHDL-2008): ⚠️ partial support (see the conditional section below)
gen_sel : case ARCH_TYPE generate
    when "fast" => fast_inst : fast_module port map(...);
    when others => slow_inst : slow_module port map(...);
end generate;
```

### Functions and procedures (when the restrictions are met)

```vhdl
-- a pure function maps to combinational logic
function parity(v : std_logic_vector) return std_logic is
    variable p : std_logic := '0';
begin
    for i in v'range loop p := p xor v(i); end loop;
    return p;
end function;

-- a procedure with no wait, no file and no external state
procedure gray_encode(
    bin  : in  std_logic_vector;
    gray : out std_logic_vector
) is begin
    gray := bin xor ('0' & bin(bin'high downto 1));
end procedure;
```

### Package operators

```vhdl
-- ieee.std_logic_1164: the std_logic operations and/or/xor/...
y <= a and b;

-- ieee.numeric_std: signed/unsigned arithmetic
sum <= a + b;
diff <= a - b;
```

---

## ⚠️ Conditionally synthesizable (tool-dependent)

### record types

```vhdl
type pixel_t is record
    r, g, b : unsigned(7 downto 0);
end record;

signal px : pixel_t;          -- as a signal or variable: fine in most tools
```

| Where it is used | Support |
|----------|----------|
| signal, variable | ✅ fine in most tools |
| entity port | ⚠️ Vivado 2019+: fine; some tools: unsupported |
| generic | ⚠️ VHDL-2008+, partial support |

Using a record port in Vivado requires the `-2008` compile option and the matching project language
setting.

### Variables (and shared variables)

```vhdl
-- a process-local variable: ✅ synthesizable
process(clk)
    variable cnt : integer range 0 to 15 := 0;
begin
    if rising_edge(clk) then
        cnt := cnt + 1;
    end if;
end process;

-- a shared variable: ⚠️ handled differently by each synthesis tool
shared variable global_cnt : integer := 0;   -- careful: most tools warn
```

### protected types (VHDL-2008+)

```vhdl
type counter_t is protected
    procedure increment;
    impure function get return integer;
end protected;
```

Useful in simulation as a thread-safe counter. Synthesis: **very limited** — the mainstream tools do
not support it.

### Generic types (VHDL-2008+)

```vhdl
-- an entity parameterised by a type
entity sorter is
    generic (type T; N : natural; function "<"(a, b : T) return boolean is <>);
    port (data : in my_array_t; sorted : out my_array_t);
end entity;
```

**Partial support** — Vivado supports it to a limited extent. Check what Quartus supports.

### Recursion (static depth)

```vhdl
-- recursion that unrolls completely during elaboration: ✅ (confirm tool support)
function clog2(n : positive) return natural is
begin
    if n <= 1 then return 0;
    else return 1 + clog2((n + 1) / 2);
    end if;
end function;

constant ADDR_W : natural := clog2(MEM_SIZE);  -- used only to compute a constant
```

Vivado: static-depth recursion is fine. Run-time-depth recursion: ❌.

### case-generate (VHDL-2008)

```vhdl
gen_arch : case IMPLEMENTATION generate
    when 0 => ...    -- LUT-based
    when 1 => ...    -- DSP-based
    when others => ...
end generate;
```

**Partial support** — Vivado 2019+: fine. Older versions and some tools: unsupported.

### fixed_pkg / float_pkg

- **Synthesizable, but the area cost is large.**
- `fixed_pkg`: fine in Vivado. Older tools need manual expansion.
- `float_pkg`: Vivado synthesizes it, but one single-precision FP operation costs hundreds of LUTs.
  → For a complex FP datapath, use the Vivado Floating Point IP core instead.

---

## ❌ Not synthesizable (simulation/testbench only)

### file types / textio

```vhdl
-- hardware has no file system → not synthesizable
file stim_file : text open READ_MODE is "input.txt";
use std.textio.all;
```

### access types (pointers / dynamic memory)

```vhdl
-- access = the VHDL pointer. Dynamic allocation cannot be mapped to hardware
type int_ptr is access integer;
variable ptr : int_ptr;
ptr := new integer'(42);            -- ❌ the new keyword itself is not synthesizable
deallocate(ptr);
```

### wait for (an explicit time)

```vhdl
wait for 10 ns;                     -- ❌ physical time is a simulator concept
wait for CLK_PERIOD / 2;            -- ❌ testbench only

-- Quartus: a single wait until is accepted; several of them, or a wait for, are rejected
wait until rising_edge(clk);        -- ✅ (a single wait until, in some tools)
```

### after delays on a signal assignment

```vhdl
-- the after keyword is either ignored by synthesis or treated as an error
y <= a after 5 ns;                  -- ❌ ignored by synthesis (waveform modelling only)
clk <= not clk after 5 ns;         -- ❌ testbench clock generation only
```

### real arithmetic / math_real

```vhdl
signal r : real;                    -- ❌ rejected by synthesis tools
r := 3.14 * 2.0;

use ieee.math_real.all;
x := sqrt(2.0);                     -- ❌ not synthesizable
```

> Exception: some tools allow a `real` value when it is used only to compute an **elaboration-time
> constant**. Declaring it as a signal or a variable is rejected.

### Infinite loops (with no static exit)

```vhdl
-- the testbench clock-generation pattern (forbidden in RTL)
loop
    clk <= '0'; wait for 5 ns;
    clk <= '1'; wait for 5 ns;
end loop;

-- while true with no static exit: ❌
while true loop
    ...         -- a synthesis tool cannot decide the termination condition statically
end loop;
```

### An impure function that depends on global state

```vhdl
shared variable global_state : integer := 0;

impure function get_state return integer is
begin
    return global_state;           -- reads mutable global state
end function;
```

Synthesis has no way to express the side effect in hardware.

---

## Synthesizable-design checklist

```
Signal types
  [ ] use std_logic / std_logic_vector
  [ ] give every integer a range constraint
  [ ] for record ports, confirm the tool is in VHDL-2008 mode

Process
  [ ] synchronous process: only clk and rst_n in the sensitivity list
  [ ] combinational process: process(all) or a complete sensitivity list
  [ ] check for wait statements (no wait in a process meant for synthesis)

Branches and loops
  [ ] every if has an else (to avoid a latch)
  [ ] every case has when others
  [ ] every for-loop range is an elaboration constant
  [ ] no while or infinite loops left in RTL

Packages
  [ ] use ieee.numeric_std (never std_logic_arith)
  [ ] keep math_real and textio out of synthesizable files
  [ ] if fixed_pkg/float_pkg is used, check the area budget

Subprograms
  [ ] no wait and no file I/O in a function meant for synthesis
  [ ] every recursive function's depth is an elaboration constant
  [ ] no testbench-only procedure mixed into a synthesizable file
```

---

## Non-synthesizable patterns → synthesizable replacements

| Non-synthesizable pattern | Synthesizable replacement |
|------------|----------|
| `wait for N ns` | a counter plus clock-edge timing |
| `after N ns` | a register pipeline delay |
| a `real` variable | `integer` (fixed point) or `sfixed`/`ufixed` |
| an `access` type | a static array plus an integer index |
| `math_real.sqrt(x)` | Newton-Raphson iteration, CORDIC, or a LUT |
| `file` stimulus | a ROM (an initialised array constant) |
| a `shared variable` | refactor into an FSM or separate signals |

---

## Sources

- AMD Vivado UG901 — VHDL Constructs Support Status: https://docs.amd.com/r/en-US/ug901-vivado-synthesis/VHDL-Constructs-Support-Status
- AMD Vivado UG901 — Supported/Unsupported VHDL Data Types: https://docs.amd.com/r/en-US/ug901-vivado-synthesis/Supported-and-Unsupported-VHDL-Data-Types
- Intel/Altera — Quartus wait constructs: https://www.intel.com/content/www/us/en/support/programmable/articles/000076012.html
- EDAboard — real data type synthesis: https://www.edaboard.com/threads/vhdl-real-data-type-error-10414.353096/
- HDL Factory — VHDL IEEE Libraries: https://www.hdlfactory.com/post/2025/06/29/vhdl-ieee-libraries-and-numeric-type-conversion-a-definitive-reference/ (WebFetch ✓)
- Research log: [vhdl-subprograms-pkg-synth-2026-05-28.md](../../../history/research-log/vhdl-subprograms-pkg-synth-2026-05-28.md)
