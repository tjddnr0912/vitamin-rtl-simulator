# 04 · VHDL Design Units

Per IEEE 1076-2008 §3: entity / architecture / package / library and use / configuration.

---

## The design units at a glance

| Unit | Role | Synth |
|------|------|------|
| entity | declares the external interface (generics + ports) | ✅ |
| architecture | the internal implementation (belongs to an entity) | ✅ |
| package declaration | shares type, constant, component and subprogram declarations | ✅ |
| package body | subprogram bodies, and the values of deferred constants | ✅ (the subprogram bodies) |
| configuration | binds a component to an entity-architecture pair | ❌ (most tools do not support it) |

A context clause (`library` + `use`) is not a design unit of its own; it is a header attached
in front of a design unit.

---

## entity

An entity declares the **external interface** of a hardware module. It carries no behaviour.

### Syntax

```vhdl
entity entity_name is
  generic (
    generic_name : type [:= default_value];
    ...
  );
  port (
    port_name : mode type;
    ...
  );
end entity entity_name;
```

- Both the `generic` clause and the `port` clause are optional.
- In `end entity entity_name;` the `entity` keyword and the name may be omitted, though
  spelling them out is recommended.

### generic

A parameter injected from outside at instantiation. It is like a `constant`, except that the
value arrives across the design-unit boundary.

```vhdl
entity adder is
  generic (
    WIDTH     : integer := 8;        -- with a default
    SIGNED_OP : boolean := false
  );
  port (
    a, b : in  std_logic_vector(WIDTH-1 downto 0);
    sum  : out std_logic_vector(WIDTH downto 0)
  );
end entity adder;
```

### Port modes

| Mode | Read inside | Write inside | Multiple drivers |
|------|-----------|-----------|--------------|
| `in` | ✅ | ❌ | N/A |
| `out` | ❌ (93) / ✅ (2008+) | ✅ | ❌ |
| `inout` | ✅ | ✅ | ✅ |
| `buffer` | ✅ | ✅ | ❌ |
| `linkage` | restricted | restricted | — |

Because VHDL-2008 allows an `out` port to be read inside the architecture, `buffer` is far
less often needed.

---

## architecture

An architecture holds the **internal implementation** of an entity. One entity may have
several architectures.

### Syntax

```vhdl
architecture arch_name of entity_name is
  -- declarative region: signal, constant, component, type, subtype, function, procedure, ...
begin
  -- concurrent statements
end architecture arch_name;
```

### One entity, several architectures

```vhdl
-- the RTL implementation
architecture rtl of adder is
begin
  sum <= std_logic_vector(
    ('0' & unsigned(a)) + ('0' & unsigned(b))
  );
end architecture rtl;

-- a behavioural model (for simulation)
architecture behavioral of adder is
begin
  process(a, b)
    variable s : integer;
  begin
    s   := to_integer(unsigned(a)) + to_integer(unsigned(b));
    sum <= std_logic_vector(to_unsigned(s, sum'length));
  end process;
end architecture behavioral;
```

By default a tool picks the **most recently compiled** architecture. Use a configuration when
the choice has to be explicit.

### The declarative region

```vhdl
architecture rtl of top is
  signal   s1     : std_logic;
  signal   bus8   : std_logic_vector(7 downto 0) := X"00";
  constant C_SIZE : integer := 16;
  component mux2
    port (sel : in std_logic; a, b : in std_logic; y : out std_logic);
  end component;
begin
  ...
end architecture;
```

---

## package

A package **shares** type, constant, component and subprogram declarations across several
design units.

### package declaration

```vhdl
library IEEE;
use IEEE.std_logic_1164.all;

package my_pkg is
  -- a constant with its value here
  constant DATA_WIDTH : integer := 8;

  -- a deferred constant — the value comes from the body
  constant MAX_COUNT  : integer;

  -- types and subtypes
  subtype byte_t  is std_logic_vector(7 downto 0);
  type state_t    is (IDLE, ACTIVE, DONE);

  -- a component declaration
  component fifo
    generic (DEPTH : integer := 16);
    port (clk, rst, wr_en, rd_en : in  std_logic;
          wr_data                 : in  byte_t;
          rd_data                 : out byte_t;
          full, empty             : out std_logic);
  end component;

  -- subprogram declarations (no body)
  function parity(v : byte_t) return std_logic;
  procedure swap(a, b : inout integer);
end package my_pkg;
```

- **Only the declaration is visible from outside.** Nothing in the body is.
- If there are no subprogram bodies, the package body itself may be omitted.

### package body

```vhdl
package body my_pkg is
  -- give the deferred constant its value
  constant MAX_COUNT : integer := 255;

  -- the function implementation
  function parity(v : byte_t) return std_logic is
    variable p : std_logic := '0';
  begin
    for i in v'range loop
      p := p xor v(i);
    end loop;
    return p;
  end function parity;

  -- the procedure implementation
  procedure swap(a, b : inout integer) is
    variable tmp : integer;
  begin
    tmp := a;  a := b;  b := tmp;
  end procedure swap;
end package body my_pkg;
```

Note: a constant or type newly declared inside the body is not visible from outside — a common
point of confusion.

---

## library and use clauses

### Syntax

```vhdl
library library_name;
use library_name.package_name.item_or_all;
```

### The idiomatic patterns

```vhdl
-- IEEE standard packages
library IEEE;
use IEEE.std_logic_1164.all;   -- std_logic, std_logic_vector
use IEEE.numeric_std.all;      -- unsigned, signed

-- the current project's own package
library work;                  -- implicitly always in effect (the clause may be omitted)
use work.my_pkg.all;           -- the whole package
use work.my_pkg.parity;        -- a selective import
```

### The rules

- A `library` clause adds a library to the current context.
- `work` is the default compilation target of the current project — it is always available
  even without `library work;`.
- A context clause sits **in front of** a design unit and applies only to that design unit.
- Changing a package requires recompiling every design unit that `use`s it.

### The standard libraries

| Library | Package | Main content |
|-----------|--------|----------|
| `IEEE` | `std_logic_1164` | `std_logic`, `std_logic_vector`, conversion functions |
| `IEEE` | `numeric_std` | `unsigned`, `signed`, arithmetic operators |
| `IEEE` | `math_real` | `sqrt`, `log`, trigonometry (simulation only) |
| `STD` | `standard` | `integer`, `boolean`, `bit` and the other base types (always implicit) |
| `STD` | `textio` | file I/O (simulation only) |

---

## configuration

A configuration **binds a component instance** inside an architecture to a particular
entity-architecture pair.

### configuration declaration

```vhdl
configuration cfg_name of entity_name is
  for architecture_name
    -- bind a component instance
    for instance_label : component_name
      use entity lib_name.entity_name(arch_name);
      generic map (generic_name => value);
      port map    (comp_port    => entity_port);
    end for;
  end for;
end configuration cfg_name;
```

### A worked example: swapping two implementations

```vhdl
-- pick the fast implementation
configuration cfg_fast of top is
  for rtl
    for u_alu : alu_comp
      use entity work.alu(fast_rtl);
    end for;
  end for;
end configuration cfg_fast;

-- pick the behavioural model, for verification
configuration cfg_behav of top is
  for rtl
    for u_alu : alu_comp
      use entity work.alu(behavioral);
    end for;
  end for;
end configuration cfg_behav;
```

### Binding down the hierarchy

```vhdl
configuration cfg_full of system is
  for struct
    for u_cpu : cpu_comp
      use entity work.cpu(rtl);
      for rtl
        for u_alu : alu_comp
          use entity work.alu(fast_rtl);
        end for;
      end for;
    end for;
  end for;
end configuration cfg_full;
```

### configuration specification (binding inline in the architecture)

```vhdl
architecture rtl of top is
  component alu_comp
    port (a, b : in std_logic_vector(7 downto 0); result : out std_logic_vector(7 downto 0));
  end component;

  -- bound in the declarative region (a configuration specification)
  for u_alu : alu_comp
    use entity work.alu(rtl);
  end for;
begin
  u_alu : alu_comp port map (a => op_a, b => op_b, result => res);
end architecture;
```

### The synthesis restriction

Most synthesis tools (Vivado, Quartus and the rest) do not support configurations. In practice:

- use them in a simulation or verification environment, to swap implementations;
- for RTL synthesis, the alternative is direct entity instantiation with the architecture name
  spelled out.

---

## The conventional file layout of a design unit

```vhdl
-- my_module.vhd

-- 1. the context clause (in front of every design unit)
library IEEE;
use IEEE.std_logic_1164.all;
use IEEE.numeric_std.all;

-- 2. the entity
entity my_module is
  generic (WIDTH : integer := 8);
  port (
    clk   : in  std_logic;
    rst_n : in  std_logic;
    data  : in  std_logic_vector(WIDTH-1 downto 0);
    valid : out std_logic
  );
end entity my_module;

-- 3. the architecture (same file, or a separate one)
architecture rtl of my_module is
  signal count : unsigned(3 downto 0) := (others => '0');
begin
  process(clk, rst_n)
  begin
    if rst_n = '0' then
      count <= (others => '0');
      valid <= '0';
    elsif rising_edge(clk) then
      count <= count + 1;
      valid <= '1' when count = X"F" else '0';
    end if;
  end process;
end architecture rtl;
```

---

## Sources

- IEEE 1076-2008 §3 (Design entities and configurations)
- IEEE 1076-2008 §4 (Subprograms and packages)
- vhdlwhiz.com/entity-instantiation-and-component-instantiation/ ✓
- hdlworks.com/hdl_corner/vhdl_ref/VHDLContents/Package.htm ✓
- kindatechnical.com/vhdl-guide/package-body-and-declaration.html ✓
- peterfab.com/ref/vhdl/vhdl_renerta/mobile/source/vhd00020.htm ✓ (configuration)
- Research log: vhdl-design-units-statements-2026-05-28.md
