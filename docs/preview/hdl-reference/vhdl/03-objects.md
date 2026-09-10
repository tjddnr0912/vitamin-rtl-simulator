# 03 · VHDL Objects

Per IEEE 1076-2008 §6: signal / variable / constant / generic / file, plus the port modes.

---

## The object classes at a glance

| Object | Keyword | When it updates | Where it is valid | Synth |
|----------|--------|------------|------------|------|
| signal | `signal` | after a delta cycle | architecture, package, port | ✅ |
| variable | `variable` | immediately | inside a process or subprogram | ✅ |
| shared variable | `shared variable` | immediately | architecture, package | ⚠️ |
| constant | `constant` | never (fixed at design time) | any declarative region | ✅ |
| generic | `generic` | never (fixed at instantiation) | entity, component | ✅ |
| file | `file` | N/A | process, subprogram | ❌ |

---

## signal

A signal models a hardware net. Its value is driven by a driver, and an assignment takes
effect one **delta cycle** later.

### Declaration

```vhdl
signal clk      : std_logic := '0';
signal data_bus : std_logic_vector(7 downto 0);
signal count    : integer range 0 to 255 := 0;
```

### Assignment — the delta-cycle delay

```vhdl
process(clk)
begin
  if rising_edge(clk) then
    s <= '1';          -- scheduled: takes effect in the next delta
    -- reading s on this line still gives the old value
    out_val <= s;      -- the old value of s is what propagates
  end if;
end process;
```

When the process suspends — at a `wait`, or at the end of a process with a sensitivity list —
the signal assignments it accumulated all take effect together in delta cycle 1. If a signal's
change wakes another process, delta cycle 2 begins, and this repeats until it converges,
without physical time advancing.

### Delta cycles, informally

```
physical time T = 10 ns:
  delta 1: process A runs → schedules signal a <= '1'
  delta 2: a = '1' takes effect → process B (sensitive to a) wakes
  delta 3: process B runs → schedules b <= a
  delta 4: b = '1' takes effect → no events left → T = 10 ns is complete
```

### Multiple drivers

A resolved type (`std_logic`) permits multiple drivers.
`std_ulogic` and `bit` permit a single driver only — a violation is a simulation error.

### Attributes

```vhdl
s'event       -- did the value change in this delta? (boolean)
s'active      -- was an assignment made in this delta?
s'last_value  -- the value before the current one
s'last_event  -- the time elapsed since the last event
s'stable(t)   -- has it been unchanged for t? (boolean)
s'quiet(t)    -- has there been no assignment for t?

-- the idiomatic clock-edge tests
rising_edge(clk)   -- clk'event and clk = '1'
falling_edge(clk)  -- clk'event and clk = '0'
```

---

## variable

Storage local to a process or a subprogram. An assignment takes effect immediately — the
semantics of a software variable, not of a hardware register.

### Declaration

```vhdl
process
  variable v     : integer := 0;
  variable temp  : std_logic_vector(7 downto 0);
begin
  v := v + 1;        -- takes effect immediately
  temp := X"FF";
  out_sig <= temp;   -- the current value of temp is what the signal assignment carries
  wait for CLK_PERIOD;
end process;
```

### signal vs variable — the essential difference

```vhdl
-- signal: the old value is read
signal s : std_logic := '0';        -- declared in the architecture
...
process
begin
  s <= '1';
  out1 <= s;   -- '0' (not updated yet)
  wait;
end process;

-- variable: updated immediately
process
  variable v : std_logic := '0';
begin
  v := '1';
  out2 <= v;   -- '1' (updated immediately)
  wait;
end process;
```

### Variables under synthesis

```vhdl
-- combinational logic: a variable inside the process synthesizes to a wire
process(all)
  variable temp : integer;
begin
  temp := a + b;
  result <= temp * c;   -- temp is a wire
end process;

-- a register: a variable in a clocked process that must hold until the next clock
process(clk)
  variable acc : integer := 0;
begin
  if rising_edge(clk) then
    acc := acc + input;   -- acc synthesizes to a flip-flop
    output <= acc;
  end if;
end process;
```

### shared variable

Reachable from several processes. VHDL-2008 recommends that a shared variable be of a
`protected type` only.

```vhdl
shared variable counter : integer := 0;
```

Beware of race conditions — the result of concurrent access is undefined.

---

## constant

An immutable object whose value is settled at design time.

### Declaration

```vhdl
constant CLK_PERIOD : time    := 10 ns;
constant DATA_WIDTH : integer := 8;
constant RESET_VAL  : std_logic_vector(7 downto 0) := X"00";
```

### Deferred constants

The package declaration gives the type only; the package body supplies the value.

```vhdl
-- package header
package my_pkg is
  constant MAX_COUNT : integer;
end package;

-- package body
package body my_pkg is
  constant MAX_COUNT : integer := 255;
end package body;
```

---

## generic

A parameter of an entity or a component; its value is passed in at instantiation. It behaves
much like a `constant`, except that the value is injected across the design-unit boundary.

### Declaration and use

```vhdl
entity adder is
  generic (
    WIDTH : integer := 8;          -- with a default
    SIGNED_OP : boolean := false
  );
  port (
    a, b : in  std_logic_vector(WIDTH-1 downto 0);
    sum  : out std_logic_vector(WIDTH downto 0)
  );
end entity;

architecture rtl of adder is
begin
  sum <= std_logic_vector(
    ('0' & unsigned(a)) + ('0' & unsigned(b))
  );
end architecture;
```

### Passing the value at instantiation

```vhdl
u_add16 : entity work.adder
  generic map (WIDTH => 16, SIGNED_OP => false)
  port map (a => a16, b => b16, sum => s17);
```

After synthesis a generic is inlined, so the parameter becomes concrete circuitry.

### Package generics (VHDL-2008+)

```vhdl
package generic_fifo is
  generic (type ITEM_TYPE; DEPTH : integer := 16);
  -- ...
end package;
```

---

## file objects

File I/O for simulation. **Not synthesizable.**

### Declaration

```vhdl
file input_file  : text open READ_MODE   is "stimulus.txt";
file output_file : text open WRITE_MODE  is "results.txt";
file log_file    : text open APPEND_MODE is "sim.log";
```

### The main procedures (the textio package)

```vhdl
use std.textio.all;

variable line_buf : line;
variable val      : integer;

readline(input_file, line_buf);    -- read one line
read(line_buf, val);               -- parse a value out of it

write(line_buf, val);              -- write a value into the buffer
writeline(output_file, line_buf);  -- emit it
```

---

## Port Modes

The interface direction given in an entity port declaration.

```vhdl
entity entity_name is
  port (
    clk      : in  std_logic;
    data_out : out std_logic_vector(7 downto 0);
    data_bus : inout std_logic_vector(7 downto 0);
    feedback : buffer std_logic
  );
end entity;
```

### What each mode permits

| Mode | Read inside the architecture | Write inside the architecture | Multiple drivers | When it is used |
|------|----------------------|----------------------|--------------|--------------|
| `in` | ✅ | ❌ | N/A | inputs: clock, reset, incoming data |
| `out` | ❌ (93) / ✅ (2008+) | ✅ | ❌ | outputs: results, status |
| `inout` | ✅ | ✅ | ✅ | bidirectional buses, I/O pins |
| `buffer` | ✅ | ✅ | ❌ (single) | an output that has to be fed back |
| `linkage` | restricted | restricted | — | almost never used (for linkage declarations only) |

**The VHDL-2008 improvement to `out`**: under the earlier standards an `out` port could not be
read inside the architecture, which forced the use of `buffer`. From VHDL-2008 an `out` port
can be read internally, so `buffer` is far less often needed.

```vhdl
-- VHDL-93: buffer is required when an out port has to be fed back
port (count : buffer integer range 0 to 255);

-- VHDL-2008: an out port can be read directly
port (count : out integer range 0 to 255);
architecture rtl of ...
begin
  process(clk) begin
    if rising_edge(clk) then
      count <= count + 1;  -- reading an out port is legal in 2008
    end if;
  end process;
end architecture;
```

**`inout` vs `buffer`**:
- `inout`: multiple drivers are allowed. Use it for a genuinely bidirectional pin (I²C SDA,
  for instance) or a bus.
- `buffer`: a single driver. Use it when only feedback is needed and there is no external
  driver.

**`linkage`**: the port may connect only to another port of mode `linkage`. It exists in the
standard, but it has almost no practical use.

---

## Where each object is declared

```vhdl
architecture rtl of my_entity is
  signal   s1 : std_logic;       -- architecture declarative region
  constant C1 : integer := 8;    -- architecture declarative region
  shared variable sv : integer;  -- architecture declarative region

begin
  process
    variable v1 : integer := 0;  -- process declarative region
    file f1 : text open READ_MODE is "x.txt";
  begin
    -- ...
  end process;
end architecture;
```

A generic can be declared only in the entity declaration.

---

## Sources

- IEEE 1076-2008 §6 (Objects, classes, and associated operations)
- IEEE 1076-2008 §9 (Concurrent statements) — process semantics
- vhdlwhiz.com/delta-cycles-explained/ ✓
- emlogic.no/2024/01/using-variables-as-registers-in-vhdl/ ✓
- piembsystech.com/ports-and-port-modes-in-vhdl/ ✓
- hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/Port.htm ✓
