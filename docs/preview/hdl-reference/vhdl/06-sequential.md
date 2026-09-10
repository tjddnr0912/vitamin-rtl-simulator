# 06 · VHDL Sequential Statements

Per IEEE 1076-2008 §10. Usable only inside a `process`, a `function` or a `procedure`. They execute
top to bottom, in the order written.

---

## The sequential statements at a glance

| Statement | Keyword | Main use |
|------|--------|----------|
| if | `if/elsif/else` | Conditional branch |
| case | `case/when` | Multi-way selection (full coverage) |
| matching case | `case?/when` | don't-care support (VHDL-2008) |
| for loop | `for ... in ... loop` | Fixed iteration count |
| while loop | `while ... loop` | Conditional iteration |
| Infinite loop | `loop` | Left with exit |
| next | `next` | Skips the rest of the current iteration |
| exit | `exit` | Leaves the loop |
| wait | `wait` | Waits for an event, a time or a condition |
| Variable assignment | `:=` | Takes effect immediately |
| Signal assignment | `<=` | Takes effect after a delta cycle |
| assert | `assert` | Checks a condition and reports |
| report | `report` | Prints a message |
| return | `return` | Returns from a function or procedure |

---

## if / elsif / else

```vhdl
if condition1 then
  statements;
elsif condition2 then
  statements;
elsif condition3 then
  statements;
else
  statements;
end if;
```

Both the `elsif` clauses and the `else` clause are optional. There is no limit on the number of
`elsif` clauses.

### The synchronous register pattern

```vhdl
process(clk, rst_n)
begin
  if rst_n = '0' then          -- asynchronous reset
    q <= (others => '0');
  elsif rising_edge(clk) then  -- synchronous logic
    if en = '1' then
      q <= d;
    end if;
  end if;
end process;
```

### The combinational pattern

```vhdl
process(all)
begin
  -- every branch must be covered (a latch is inferred without the else)
  if sel = '0' then
    y <= a;
  else
    y <= b;
  end if;
end process;
```

---

## case

```vhdl
case expression is
  when value1 =>
    statements;
  when value2 | value3 =>    -- OR: several values in one branch
    statements;
  when value4 to value7 =>   -- a range (integer or enumeration)
    statements;
  when others =>             -- everything else (optional, but recommended)
    null;
end case;
```

- The choices must give **full coverage** (either finish with `when others` or list every value
  explicitly).
- Choices **may not overlap** — a value belonging to two branches is a compile error.
- `null` is an explicitly empty branch that does nothing.

### A 4-to-1 MUX

```vhdl
process(all)
begin
  case sel is
    when "00" => y <= a;
    when "01" => y <= b;
    when "10" => y <= c;
    when "11" => y <= d;
  end case;
end process;

-- std_logic_vector has further values ('U', 'X', ...), so when others is recommended
process(all)
begin
  case sel is
    when "00"   => y <= a;
    when "01"   => y <= b;
    when "10"   => y <= c;
    when others => y <= d;
  end case;
end process;
```

### The state-machine pattern

```vhdl
type state_t is (IDLE, RUN, DONE);
signal state : state_t;

process(clk)
begin
  if rising_edge(clk) then
    case state is
      when IDLE =>
        if start = '1' then state <= RUN; end if;
      when RUN  =>
        if done_flag = '1' then state <= DONE;
        else state <= RUN; end if;
      when DONE =>
        state <= IDLE;
    end case;
  end if;
end process;
```

---

## case? (matching case) — VHDL-2008

An extension of the ordinary `case`. It uses the `?=` matching operator, so it handles `'-'`
(don't care) and the weak `std_logic` values (`'H'` = `'1'`, `'L'` = `'0'`).

```vhdl
case? opcode is
  when "1---" => execute_branch;    -- '-' = don't care: matches all of 1xxx
  when "01--" => execute_load;
  when "001-" => execute_store;
  when "0001" => execute_halt;
  when others => execute_nop;
end case?;
```

### case vs case?

| Item | `case` | `case?` |
|------|--------|---------|
| `'-'` | compared as the literal value `'-'` | don't care (matches every value) |
| `'H'` / `'L'` | compared literally | match `'1'` / `'0'` |
| Operator | `=` | `?=` |
| Keyword | `case` / `end case;` | `case?` / `end case?;` |
| Introduced in | VHDL-87 onwards | VHDL-2008 |

### Watch for overlapping patterns

```vhdl
-- wrong: "1-" and "11" overlap
case? sel is
  when "1-" => ...   -- "11" matches too
  when "11" => ...   -- overlap — a compiler/tool error
  when others => ...
end case?;
```

Where patterns contain don't cares, it is the designer who must guarantee there is no overlap.

### Tool support

- GHDL: implementation issues reported (issue #1940). Check the version.
- Vivado / Quartus: needs the `-2008` compile option or a VHDL-2008 project setting.
- ModelSim: needs `vcom -2008`.

---

## Loops

### for loop

```vhdl
for i in 0 to 7 loop          -- ascending (0, 1, ..., 7)
  result(i) := data(i) xor mask(i);
end loop;

for i in 7 downto 0 loop      -- descending
  sum := sum + to_integer(unsigned'(0 => vec(i)));
end loop;

-- with labels
outer : for i in 0 to N-1 loop
  inner : for j in 0 to M-1 loop
    matrix(i, j) := i * M + j;
  end loop inner;
end loop outer;
```

- The index variable (`i`) is declared implicitly by the loop — no separate declaration is needed.
- The index variable is **read-only** — it cannot be modified inside the loop.
- The range expression must be constant (for synthesis).

### while loop

```vhdl
while count < LIMIT loop
  count := count + 1;
  data(count) := some_val;
end loop;

-- if the condition is false to begin with, the body never runs
while false loop   -- never executes
  ...
end loop;
```

### Infinite loop

```vhdl
-- an infinite loop left with exit
clock_gen : loop
  clk <= '0';  wait for CLK_PERIOD/2;
  clk <= '1';  wait for CLK_PERIOD/2;
end loop clock_gen;

-- conditional exit
scan : loop
  read_byte(byte_val);
  exit scan when byte_val = STOP_BYTE;
end loop scan;
```

---

## next / exit

### next — skip the current iteration

```vhdl
for i in 0 to 15 loop
  next when data(i) = '0';    -- if '0', skip this iteration
  process_bit(i);
end loop;

-- naming an outer loop from a nested one
outer : for i in 0 to N-1 loop
  for j in 0 to M-1 loop
    next outer when skip_row(i) = '1';   -- on to the next iteration of outer
    matrix(i, j) := compute(i, j);
  end loop;
end loop outer;
```

### exit — leave the loop

```vhdl
for i in 0 to 255 loop
  exit when found = '1';     -- end the loop once the condition holds
  search(i, found);
end loop;

-- leaving a nested loop
outer : for i in 0 to N-1 loop
  for j in 0 to M-1 loop
    exit outer when matrix(i, j) = TARGET;  -- leaves both loops
  end loop;
end loop outer;
```

### next vs exit

| | `next` | `exit` |
|--|--------|--------|
| Effect | skips the rest of this iteration and starts the next | leaves the loop entirely |
| Without a label | applies to the innermost loop | applies to the innermost loop |
| With a label | on to the next iteration of that loop | leaves that loop |

---

## The wait statement

`wait` suspends the process. It **cannot be used in a process that has a sensitivity list**
(LRM §11.3).

### The four forms

```vhdl
wait;                                  -- (1) wait forever
wait on sig1, sig2;                    -- (2) wait for an event
wait until condition;                  -- (3) wait for a condition
wait for time_expression;             -- (4) wait for a time
```

They can be combined:

```vhdl
wait on clk until clk = '1';          -- event + condition
wait until clk = '1' for 100 ns;      -- condition + timeout
wait on sig1, sig2 until cond for 50 ns;
```

### Examples

```vhdl
-- a testbench reset sequence
process
begin
  rst_n <= '0';
  wait for 20 ns;               -- hold reset for 20 ns
  rst_n <= '1';
  wait;                         -- then wait forever (runs only once)
end process;

-- waiting for a clock edge
process
begin
  wait until rising_edge(clk);  -- wait for the rising edge
  data <= test_vector;
  wait until rising_edge(clk);
  check_output(expected, actual);
end process;

-- waiting with a timeout (handshake)
process
begin
  req <= '1';
  wait until ack = '1' for TIMEOUT;
  if ack /= '1' then
    report "Handshake timeout" severity ERROR;
  end if;
  req <= '0';
  wait;
end process;
```

---

## Variable assignment `:=` vs signal assignment `<=`

### Immediate vs after a delta cycle

```vhdl
process
  variable v : std_logic_vector(7 downto 0) := X"00";
begin
  v := X"FF";          -- takes effect immediately
  result1 <= v;        -- passes X"FF" (the current value of v)

  sig <= X"AA";        -- scheduled: takes effect in the next delta
  result2 <= sig;      -- passes the old value of sig (not X"AA" yet)

  wait for 10 ns;
end process;
```

### Which to use when

| Situation | Recommended |
|------|------|
| Holding an intermediate result | `variable` + `:=` |
| Driving the output after a combinational computation | `<=` at the end |
| A register (flip-flop) | `signal` + `<=` |
| A loop counter | `variable` + `:=` |
| Shared state (careful) | `shared variable` + `:=` |

### Signal assignment delay models

```vhdl
-- inertial (the default): filters pulses shorter than the delay
y <= a after 5 ns;
y <= inertial a after 5 ns;   -- explicitly the same thing

-- transport: passes every transition on (transmission-line model)
y <= transport a after 5 ns;

-- reject N ns inertial: sets the minimum pulse width to N ns
y <= reject 2 ns inertial a after 5 ns;   -- filters pulses under 2 ns

-- a waveform: several transitions scheduled at once
clk <= '1', '0' after 5 ns, '1' after 10 ns, '0' after 15 ns;
```

---

## assert / report / severity

### assert

```vhdl
-- the basic form
assert boolean_condition
  [report string_expression]
  [severity severity_level];

-- example: check the counter's initial value after reset
assert counter = 0
  report "Counter not zero after reset, got: " & integer'image(counter)
  severity ERROR;

-- without report
assert a /= b severity WARNING;

-- without severity (defaults to ERROR)
assert valid = '1'
  report "Data not valid";
```

### report

```vhdl
-- print a message without an assertion
report string_expression [severity severity_level];

report "Simulation started at " & time'image(now);
report "Test case 1 passed" severity NOTE;
report "Unexpected condition" severity FAILURE;  -- stops immediately
```

### Severity levels

| Level | Value | Default behaviour | Use |
|------|----|----------|------|
| `NOTE` | 0 | prints the message, continues | informational messages, progress |
| `WARNING` | 1 | prints the message, continues | abnormal but not fatal |
| `ERROR` | 2 | prints the message, continues | a design error; the default severity |
| `FAILURE` | 3 | stops the simulation immediately | unrecoverable error |

- The default severity of `assert`: `ERROR` (when both report and severity are omitted).
- The default severity of `report`: `NOTE`.
- Tool settings can change the severity threshold at which simulation stops.

### A practical pattern

```vhdl
-- a self-checking testbench
process
  variable pass_count : integer := 0;
  variable fail_count : integer := 0;
begin
  -- run the test case
  apply_stimulus(X"AA");
  wait until rising_edge(clk);
  if output = X"55" then
    pass_count := pass_count + 1;
    report "TC1 PASS" severity NOTE;
  else
    fail_count := fail_count + 1;
    report "TC1 FAIL: expected 0x55, got " & to_hstring(output)
      severity ERROR;
  end if;

  -- the final result
  report "Total: " & integer'image(pass_count) & " pass, "
       & integer'image(fail_count) & " fail";
  assert fail_count = 0
    report "Test FAILED"
    severity FAILURE;

  wait;
end process;
```

---

## return

```vhdl
-- procedure: returns no value (an early exit only)
procedure check_range(val : integer; lo, hi : integer) is
begin
  if val < lo or val > hi then
    report "Out of range" severity ERROR;
    return;             -- early return
  end if;
  -- carry on with the in-range handling
end procedure;

-- function: must return a value
function max_val(a, b : integer) return integer is
begin
  if a >= b then
    return a;
  else
    return b;
  end if;
end function max_val;
```

---

## Sequential-statement synthesis checklist

| Pattern | Synthesis result | Watch out for |
|------|----------|------|
| `if rising_edge(clk)` | flip-flop | the standard clock-edge pattern |
| `if` (no else) | latch (in a combinational process) | every branch must be covered |
| `case` (no others) | some values may latch | `when others` recommended |
| `for i in 0 to N-1` | unrolled N times | N must be a compile-time constant |
| `while` | limited synthesis — termination cannot be guaranteed | avoid in RTL |
| `wait` | limited synthesis | testbench only |
| `variable :=` | a wire (combinational) or a flip-flop (synchronous) | decided by context |
| `assert` / `report` | ignored by synthesis tools | simulation only |

---

## Sources

- IEEE 1076-2008 §10 (Sequential statements)
- portal.cs.umbc.edu/help/VHDL/sequential.html ✓
- fpgatutorial.com/vhdl-for-while-loop-if-case-statement/ ✓
- vhdlwhiz.com/sensitivity-list/ ✓ (the wait equivalence)
- Doulos, VHDL-2008 small changes (case? — 403, verified from the snippet)
- GHDL issue #1940 (case? implementation notes)
- Research log:
  [vhdl-design-units-statements-2026-05-28.md](../../../history/research-log/vhdl-design-units-statements-2026-05-28.md)
