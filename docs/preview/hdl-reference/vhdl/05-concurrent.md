# 05 · VHDL Concurrent Statements

Per IEEE 1076-2008 §11. Every statement in an architecture body is active at the same time — the
order they are written in carries no meaning.

---

## The concurrent statements at a glance

| Statement | Keyword | Main use |
|------|--------|----------|
| Process | `process` | Encapsulates sequential execution: registers, state machines |
| Simple signal assignment | `<=` | Describes combinational logic |
| Conditional signal assignment | `when/else` | Priority MUX |
| Selected signal assignment | `with/select` | Parallel MUX (equivalent to a case) |
| Component instantiation | `entity` / a component label | Structural hierarchy |
| generate | `generate` | Repeated or conditional hardware |
| block | `block` | Groups concurrent statements (hierarchical readability) |
| Concurrent procedure call | the procedure name | Calls a procedure without writing a process |
| Concurrent assertion | `assert` | Checks a design constraint (simulation) |

---

## Process

The only concurrent statement that holds **sequential execution**. For what goes inside a process,
see §06-sequential.

### The sensitivity-list form

```vhdl
-- VHDL-93: listed explicitly
process(clk, rst_n)
begin
  if rst_n = '0' then
    q <= '0';
  elsif rising_edge(clk) then
    q <= d;
  end if;
end process;

-- VHDL-2008: process(all) — every signal read is included automatically
process(all)
begin
  case sel is
    when "00" => y <= a;
    when "01" => y <= b;
    when others => y <= c;
  end case;
end process;
```

`process(all)` rules out the missing-sensitivity-list bug. Some synthesis tools do not support it —
check the toolchain the project uses.

### The wait form

```vhdl
-- the testbench clock-generation pattern
process
begin
  clk <= '0';
  wait for CLK_PERIOD / 2;
  clk <= '1';
  wait for CLK_PERIOD / 2;
end process;
```

A sensitivity list and `wait` **cannot both appear in one process**.

### Sensitivity list ↔ wait equivalence

```vhdl
-- the two processes below are equivalent
process(a, b)
begin
  y <= a and b;
end process;

process
begin
  y <= a and b;
  wait on a, b;      -- wait on the sensitivity signals at the end of the process
end process;
```

### Rules at a glance

| Aspect | Sensitivity list | `wait` form |
|------|-----------|-----------|
| Synthesis | ✅ (the RTL standard) | ⚠️ (limited) |
| Simulation | ✅ | ✅ |
| Use | RTL code | testbench, simulation models |

---

## Concurrent Signal Assignment

### Simple assignment

```vhdl
y    <= a and b;
z    <= not (a or b) after 2 ns;  -- a delay may be attached
flag <= '1';                       -- constant drive
```

Re-evaluated whenever an event occurs on any of the signals on the right-hand side.

### Conditional Signal Assignment

```vhdl
-- 4-to-1 MUX
mux_out <= a when sel = "00" else
           b when sel = "01" else
           c when sel = "10" else
           d;                -- a final unconditional else is required (a latch is inferred without it)
```

- **Priority**: top to bottom (the first true condition applies).
- Overlapping conditions are allowed — the higher one wins.
- Without the final `else`, synthesis warns that a latch has been created.

A three-state bus:

```vhdl
bus_out <= data_out when oe = '1' else (others => 'Z');
```

### Selected Signal Assignment

```vhdl
with sel select
  mux_out <= a when "00",
             b when "01",
             c when "10",
             d when others;  -- full coverage is required

-- grouping several values
with opcode select
  alu_op <= OP_ADD  when X"00" | X"01",
            OP_SUB  when X"02",
            OP_AND  when X"10" to X"13",   -- a range
            OP_NOP  when others;
```

- **`when others` is required** — every case must be covered.
- Choices may not overlap (the same restriction as the case statement).
- No priority — each choice is independent (mutually exclusive).

### CSA vs SSA

| | Conditional (when/else) | Selected (with/select) |
|--|-------------------|-------------------|
| Priority | yes (the higher one wins) | none (independent branches) |
| Condition form | any boolean condition | values or ranges of one expression |
| Overlapping conditions | allowed | not allowed |
| Synthesis result | a priority MUX chain | a parallel MUX |

---

## Component instantiation

### Direct entity instantiation (recommended, VHDL-93 onwards)

```vhdl
-- Named association — recommended
u_adder : entity work.adder(rtl)
  generic map (
    WIDTH     => 16,
    SIGNED_OP => false
  )
  port map (
    clk => clk,
    a   => op_a,
    b   => op_b,
    sum => result
  );

-- omit the architecture name → the most recently compiled architecture is used
u_adder2 : entity work.adder
  port map (clk => clk, a => op_a, b => op_b, sum => result);
```

```vhdl
-- Positional association — not recommended
u_adder3 : entity work.adder(rtl)
  generic map (16, false)
  port map (clk, op_a, op_b, result);
```

Positional association breaks as soon as the port order changes, so named association is preferred.

### Component instantiation (for Verilog integration, netlists and configurations)

```vhdl
architecture rtl of top is
  -- step 1: declare the component in the architecture declarative part
  component adder
    generic (
      WIDTH     : integer := 8;
      SIGNED_OP : boolean := false
    );
    port (
      clk : in  std_logic;
      a   : in  std_logic_vector(WIDTH-1 downto 0);
      b   : in  std_logic_vector(WIDTH-1 downto 0);
      sum : out std_logic_vector(WIDTH downto 0)
    );
  end component adder;

begin
  -- step 2: instantiate it
  u_adder : adder
    generic map (WIDTH => 16, SIGNED_OP => false)
    port map (clk => clk, a => op_a, b => op_b, sum => result);
end architecture;
```

### The open keyword

Used when an optional port is left unconnected:

```vhdl
u_ff : entity work.dff
  port map (
    clk   => clk,
    d     => data_in,
    q     => data_out,
    q_bar => open      -- unused
  );
```

### Which one to use

```
Use direct entity instantiation:
  → wiring ordinary VHDL modules together (most cases)

Use component instantiation:
  → integrating a Verilog module in Vivado
  → netlists, hard macros
  → when a configuration has to swap the implementation dynamically
```

---

## The generate statement

Creates hardware repeatedly or conditionally. Any concurrent statement may appear inside a generate
(processes, signal assignments and nested generates included).

### for-generate

```vhdl
-- create N RAM banks
gen_ram : for i in 0 to N_BANKS-1 generate
  u_ram : entity work.ram_sp
    port map (
      clk     => clk,
      en      => enable(i),
      we      => we,
      addr    => addr,
      wr_data => wr_data,
      rd_data => rd_data(i)
    );
end generate gen_ram;

-- a process may be placed inside
gen_regs : for i in 0 to 7 generate
  process(clk)
  begin
    if rising_edge(clk) then
      reg(i) <= data_in(i);
    end if;
  end process;
end generate gen_regs;
```

### if-generate

**VHDL-93**: no `elsif` / `else`.

```vhdl
-- VHDL-93
gen_dbg_93 : if DEBUG generate
  u_probe : entity work.ila port map (...);
end generate gen_dbg_93;
```

**VHDL-2008**: `elsif` / `else` added.

```vhdl
-- VHDL-2008
gen_impl : if IMPLEMENTATION = "FAST" generate
  u_fast : entity work.alu_fast port map (...);
elsif IMPLEMENTATION = "AREA" generate
  u_area : entity work.alu_area port map (...);
else generate
  u_default : entity work.alu_rtl port map (...);
end generate gen_impl;
```

### case-generate (new in VHDL-2008)

Selects one of several options by condition. Reads better than a chain of if-generates.

```vhdl
gen_bus : case BUS_WIDTH generate
  when 8 =>
    u_comp : entity work.comp_8b
      port map (clk => clk, data => data(7 downto 0));
  when 16 =>
    u_comp : entity work.comp_16b
      port map (clk => clk, data => data(15 downto 0));
  when others =>
    u_comp : entity work.comp_32b
      port map (clk => clk, data => data(31 downto 0));
end generate gen_bus;
```

### generate restrictions

- A variable declared inside a generate cannot be used outside it.
- Labels are required in VHDL-2008 — how strictly this is enforced varies by tool version.
- The generate parameter (`i`) is a constant integer and, unlike a loop variable, cannot be modified.

---

## The block statement

**Groups concurrent statements hierarchically** inside an architecture.

```vhdl
-- unguarded block: for readability. Synthesis tools generally see through it.
blk_datapath : block
  signal pipe_reg : std_logic_vector(7 downto 0);
begin
  pipe_reg <= data_in when rising_edge(clk) else pipe_reg;
  data_out <= pipe_reg;
end block blk_datapath;
```

```vhdl
-- guarded block: a guard signal is created automatically. Not synthesizable.
blk_tri : block (oe = '1')
begin
  bus_pin <= guarded data_out;   -- driven only while oe = '1'
end block blk_tri;
```

| Kind | Synthesis | Use |
|------|------|------|
| Unguarded block | ✅ (handled transparently by the tool) | structuring a large architecture for readability |
| Guarded block | ❌ | simulation only, tri-state modelling |

---

## Concurrent procedure call

```vhdl
-- call a procedure directly from the architecture body
-- equivalent to: process(all) begin check_parity(data, parity_ok); end process;
check_parity(data, parity_ok);

-- a label may be attached
chk : check_parity(data_in, err_flag);
```

The signals read inside the called procedure form the sensitivity list.

---

## Concurrent assertion

```vhdl
-- without a label
assert not (wr_en = '1' and rd_en = '1')
  report "Simultaneous read and write"
  severity ERROR;

-- with a label
chk_setup : assert setup_time >= T_SETUP
  report "Setup time violation"
  severity WARNING;
```

Equivalent to `process(all) begin assert ...; end process;`. Synthesis tools ignore it; it runs in
simulation only.

---

## How concurrent statements interact

Concurrent statements communicate through **signals**. All of them are evaluated in the same
simulation delta cycle.

```vhdl
architecture rtl of example is
  signal a, b, c : std_logic;
begin
  -- the three statements are active at once — their order carries no meaning
  a <= in1 and in2;               -- simple assignment
  b <= a or in3;                  -- reads the previous value of a (one delta cycle behind)

  p1 : process(all)               -- a changes → runs in the next delta
  begin
    c <= not a;
  end process;
end architecture;
```

---

## Sources

- IEEE 1076-2008 §11 (Concurrent statements)
- vhdlwhiz.com/sensitivity-list/ ✓
- vhdlwhiz.com/entity-instantiation-and-component-instantiation/ ✓
- vhdl-online.de/concurrent_statements ✓
- fpgaer.wordpress.com VHDL-2008 quick reference, if/case generate ✓
- fpgatutorial.com/vhdl-generic-generate/ ✓
- Research log:
  [vhdl-design-units-statements-2026-05-28.md](../../../history/research-log/vhdl-design-units-statements-2026-05-28.md)
