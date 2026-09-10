# 13 · Miscellaneous Tasks (Plusargs · System · Severity · Exit)

## Overview

This category covers four groups of utilities that make up the test infrastructure.

- **Plusargs** (`$test$plusargs`, `$value$plusargs`): the channel that carries command-line
  arguments into simulation code
- **Shell execution** (`$system`): running an OS shell command from inside the simulation
- **Severity tasks** (`$fatal`, `$error`, `$warning`, `$info`): structured diagnostic output usable
  from both elaboration and simulation
- **Test termination** (`$exit`): phase control for a program-block testbench

## vita support

This note describes the language, not the simulator. What vita accepts today is recorded in
[docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Plusargs — reading values from the command line

### `$test$plusargs(user_string)` — detecting a boolean flag

- **Standard**: IEEE 1800-2017 §20.10
- Returns non-zero (1) when any plusarg supplied on the command line starts with `user_string`, and
  0 otherwise.
- The match is a **prefix match**: an argument `+VERBOSE_MODE` matches
  `"V"`, `"VERBOSE"` and `"VERBOSE_MODE"` alike.
- It is **case-sensitive**.
- It suits boolean flags, where no value is needed.

```sv
// the simulation code
initial begin
  if ($test$plusargs("VERBOSE"))
    $display("[DEBUG] verbose logging enabled");
  if ($test$plusargs("WAVE"))
    $dumpvars();
end
```

The command line:
```sh
# Icarus Verilog
vvp sim.vvp +VERBOSE +WAVE

# Verilator
./obj_dir/Vtop +VERBOSE +WAVE
```

---

### `$value$plusargs(user_string, variable)` — extracting a value

- **Standard**: IEEE 1800-2017 §20.10
- Matches against a string that carries a format specifier and, on a match, stores the value into
  `variable`.
- Returns non-zero (1) when the match and the store both succeed, 0 otherwise. **On failure the
  variable is left unchanged.**
- The format string looks like `"ARGNAME=%fmt"` — no spaces between the name, the `=` and the
  format code.

#### The supported format codes

| Code | Conversion |
|------|------|
| `%d` | decimal integer |
| `%o` | octal integer |
| `%h` / `%x` | hexadecimal integer |
| `%b` | binary integer |
| `%e` | real (exponential notation) |
| `%f` | real (decimal notation) |
| `%g` | real (decimal or exponential — whichever is shorter) |
| `%s` | string (no conversion) |

```sv
int    seed;
string testname;
real   timeout_ns;
logic  [7:0] mask;

// each plusarg is queried independently
if ($value$plusargs("SEED=%d",    seed))
  $display("seed=%0d", seed);
if ($value$plusargs("TEST=%s",    testname))
  $display("test=%s", testname);
if ($value$plusargs("TIMEOUT=%f", timeout_ns))
  $display("timeout=%.1f ns", timeout_ns);
if ($value$plusargs("MASK=%h",    mask))
  $display("mask=0x%02h", mask);
```

The command line:
```sh
./sim +SEED=42 +TEST=axi_burst +TIMEOUT=1000.0 +MASK=ff
```

**Edge cases worth noting**:
- `+KEY=` (an empty value): `%s` receives the empty string; with an integer format it is
  implementation defined.
- A format mismatch (`%d` against something that is not an integer, say): a run-time error, or
  implementation-defined behaviour.
- When the same plusarg name is supplied several times the last value is used (including in the
  implementation-defined cases).

**Icarus / Verilator**: fully supported.

---

## Shell execution — `$system("command")`

- **Standard**: IEEE 1800-2017 §21.3
- Passes the argument string to the OS shell and runs it.
- **Function form**: returns the shell process's exit status as an int.
  In a POSIX environment 0 = success, non-zero = failure.
- **Task form**: the return value is discarded.

```sv
// the function form — check the return value
int ret;
ret = $system("cp golden.mem /tmp/golden.mem");
if (ret != 0)
  $error("file copy failed with code %0d", ret);

// the task form
$system("mkdir -p /tmp/sim_out");
$system("date > /tmp/sim_out/timestamp.txt");
```

**Security and portability warnings**:

1. **OS dependence**: POSIX commands such as `ls`, `cp` and `mkdir` do not work under Windows
   cmd.exe. Where a cross-platform test environment matters, abstract the call behind a Makefile or
   a shell wrapper.

2. **Command-injection risk**: passing a string that came from an external input — a plusarg, for
   instance — straight into `$system` allows arbitrary command execution.
   Pass only literal strings, or values that have been validated thoroughly.

3. **File-descriptor inheritance**: the spawned process may inherit the simulator's file
   descriptors.

4. **Restricted use**: `$system` belongs to the test infrastructure.
   Calling it from RTL simulation logic or from an assertion is an anti-pattern.

**Icarus**: supported.
**Verilator**: supported (works correctly on Linux and macOS).

---

## Severity tasks — `$fatal` / `$error` / `$warning` / `$info`

- **Standard**: IEEE 1800-2017 §20.11 (elaboration), §20.12 (the simulation assertion context)

These four tasks behave differently in **two contexts under the same names**.

| Context | When it runs | Where it sits |
|---------|---------|------|
| **Elaboration** | module parameter validation, generate blocks | **outside** procedural code |
| **Simulation** | run-time assertions, test logic | **inside** procedural code |

Code inside a procedural block (`initial`, `always`, a `task` and so on) automatically takes the
simulation-time behaviour.

### Signatures

```sv
$fatal   [(finish_number [, list_of_arguments])];
$error   [(list_of_arguments)];
$warning [(list_of_arguments)];
$info    [(list_of_arguments)];
```

- `finish_number` (`$fatal` only): the same finish_number as Verilog's `$finish`.
  - `0`: no diagnostic output
  - `1`: print the simulation time (the default)
  - `2`: print the simulation time and memory usage (implementation defined)
- `list_of_arguments`: the same format-string syntax as `$display` (`%0d`, `%s`, `%0t` and so on).

### What each task does

| Task | Elaboration | Simulation |
|--------|-------------|------------|
| `$fatal` | aborts elaboration at once; no simulation is built | a run-time error → the simulation is terminated |
| `$error` | prints the error, elaboration continues | prints the error, the simulation continues |
| `$warning` | prints a warning, suppressible per tool | prints a warning, suppressible per tool |
| `$info` | prints information | prints information |

### The information the tool adds

Every severity task produces a message into which the tool automatically folds:
- the file name and line number
- the hierarchical path of the calling scope (`tb.dut.alu`, say)
- the simulation time, in the simulation context

### The elaboration context — validating parameters

Validating parameters at module level or inside a generate block is the representative pattern:

```sv
module mac #(
  parameter int IN_W  = 8,
  parameter int OUT_W = 16,
  parameter int LATENCY = 2
) (...);

  // parameter range checks — these run at elaboration time
  if (IN_W < 1 || IN_W > 64)
    $fatal(1, "IN_W=%0d out of range [1,64]", IN_W);
  if (OUT_W < IN_W * 2)
    $fatal(1, "OUT_W=%0d too small for IN_W=%0d", OUT_W, IN_W);
  if (LATENCY < 1)
    $warning("LATENCY=%0d is unusually small", LATENCY);

endmodule
```

The `$fatal` and `$warning` above sit outside any procedural block, so they run at elaboration time.
When a parameter is invalid, no simulation executable is produced at all.

### The simulation context — assertions and test logic

```sv
// an SVA action block
assert property (@(posedge clk) req |-> ##[1:5] ack)
  else $error("ack missing after req at time %0t, req=%0b", $time, $sampled(req));

// procedural code
initial begin
  if (dut_result !== expected)
    $fatal(1, "MISMATCH: got=%0h expected=%0h at %0t",
           dut_result, expected, $time);
  else
    $info("test PASSED");
end
```

**Icarus**: `$error`, `$warning` and `$info` are supported.
Elaboration-time `$fatal` is partially supported — it may fall back to simulation time.
**Verilator**: elaboration tasks are officially supported (since the fix for issue #1429).

---

## Test termination — `$exit`

- **Standard**: IEEE 1800-2017 §21 (program block phase control)
- Waits until every **program block** has completed and then calls `$finish`.

`$finish` ends the simulation immediately. `$exit`, by contrast, only raises the termination signal
and waits for the running program blocks to finish their own cleanup — tearing down, aggregating
coverage and so on.
It is the idiomatic way to let a UVM/OVM or class-based testbench wind down naturally.

```sv
program automatic test;
  initial begin
    // the test sequence
    fork
      run_stimulus();
      check_outputs();
    join

    // wait for every program block to complete, then call $finish
    $exit;
  end
endprogram
```

Compared with `$finish`:

| | `$finish` | `$exit` |
|-|-----------|---------|
| When it ends | immediately | after every program block completes |
| Program-block cleanup | none | yes |
| Where it is mainly used | a module-based TB | a program-based TB |

**Icarus**: `program` block support is limited, so `$exit` behaves in a limited way too.
**Verilator**: `program` blocks are unsupported, so `$exit` cannot be used.
A Verilator-based testbench calls `$finish` directly.

---

## The tasks and functions side by side

| Name | Returns | Purpose |
|------|------|------|
| `$test$plusargs(str)` | int (0/non-0) | detecting a boolean flag on the command line |
| `$value$plusargs(str, var)` | int (0/non-0) | reading a plusarg that carries a value |
| `$system(cmd)` | int (exit code) | running an OS shell command |
| `$fatal(n, ...)` | — | terminate at once (elaboration or simulation) |
| `$error(...)` | — | print an error and keep going |
| `$warning(...)` | — | print a warning, suppressible |
| `$info(...)` | — | print information |
| `$exit` | — | $finish once the program blocks complete |

---

## Icarus / Verilator support

| Task | Icarus Verilog | Verilator |
|--------|---------------|-----------|
| `$test$plusargs` | full | full |
| `$value$plusargs` | full | full |
| `$system` | supported | supported |
| `$error`/`$warning`/`$info` | supported | supported |
| `$fatal` (simulation) | supported | supported |
| `$fatal` (elaboration) | partial | supported (issue #1429 fix) |
| `$exit` | limited (program-block limits) | unsupported (use `$finish`) |

---

## Synthesizability

❌ Nothing here is synthesizable — it is all simulation and test infrastructure.
The severity tasks (`$fatal`, `$error` and the rest) are conventionally placed inside a
`/* synthesis translate_off */` guard.

---

## Sources

- IEEE 1800-2017 §20.10 ($value$plusargs, $test$plusargs), §20.11 (elaboration severity),
  §20.12 (simulation severity assertion control), §21.3 ($system)
- research-log: [system-tasks-introspection-misc-2026-05-28.md](../../../history/research-log/system-tasks-introspection-misc-2026-05-28.md)
- [chipverify.com — Command Line Input](https://chipverify.com/systemverilog/systemverilog-command-line-input) (WebFetch ✓)
- [theartofverification.com — Plusargs](https://theartofverification.com/plusargs-in-systemverilog/) (WebFetch ✓)
- [accellera.org sv-bc — Severity Tasks](https://www.accellera.org/images/eda/sv-bc/att-5678/severity_tasks_3.htm) (WebFetch ✓)
- [circuitcove.com — Simulation Control Tasks](https://circuitcove.com/system-tasks-simulation-control/) (WebFetch ✓)
- [github.com/verilator — Elaboration tasks issue #1429](https://github.com/verilator/verilator/issues/1429) (WebFetch ✓ — closed/fixed)
