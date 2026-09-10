# 04 · Simulation Control

## Overview

The category of tasks that end a simulation or suspend it. Even the simplest
testbench needs one of them: without `$finish` the simulation never ends, which
is what makes this the first category any simulator has to implement.

## Scope of this page

- `$finish` and `$stop`, and the `severity` argument both take.
- `$exit`, the SystemVerilog task that ends a `program`-based testbench.

These notes describe the standard. For what vita does with the severity
argument, which exit code each ending produces, and where `$finish` is refused,
see [manual/005_system-tasks.md](../../../manual/005_system-tasks.md),
[manual/004_cli-reference.md](../../../manual/004_cli-reference.md) and
[manual/006_limitations.md](../../../manual/006_limitations.md).

---

## Item detail

### `$finish[(severity)]`

- **Signature**: `$finish;` or `$finish(0|1|2);`
- **Standard**: IEEE 1800-2017 §20.2 / IEEE 1364-2005 §17.8.1
- **Meaning**: **ends the simulation** for good and returns control to the
  operating system. A commercial simulator releases its license here; the run
  cannot be resumed.
  The `severity` argument selects how much diagnostic information is printed on
  the way out. The default is 1.
- **Returns**: void (it does not return — the simulation is over)

#### Severity levels

| Level | What is printed |
|-------|-----------------|
| `0` | nothing |
| `1` | simulation time + source file and line (the default) |
| `2` | simulation time + location + memory usage + CPU time statistics |

```sv
// the common pattern
initial begin
  // ... drive the stimulus ...
  #100;
  $finish;          // severity=1 (default): print time and location, then end
end

// a quiet ending (to keep a CI log clean)
$finish(0);

// with the full profiling information
$finish(2);
```

#### iverilog vvp exit codes

| How vvp is run | What `$finish` does | Exit code |
|----------------|---------------------|-----------|
| `vvp sim.vvp` (default) | ends normally | 0 |
| `vvp -n sim.vvp` | `$stop` behaves as `$finish` | 0 |
| `vvp -N sim.vvp` | `$stop` returns exit 1 | **1** (if `$stop` was called) |
| `vvp -q sim.vvp` | suppresses `$display` output | 0 |

With `-N`, a `$stop` call makes vvp exit with code 1. That is the pattern to use
when a CI pipeline has to detect a failing test from the exit code:

```bash
# use $stop as an error signal in CI
vvp -N sim.vvp
if [ $? -ne 0 ]; then echo "SIMULATION FAILED"; fi
```

#### Verilator behaviour

Verilator **ignores** the severity argument of `$finish`/`$stop`. Its default
exit code is 0. A custom shutdown handler can be registered through the
`VL_FINISH_CALLBACK` macro, which is beyond the scope of these notes.

---

### `$stop[(severity)]`

- **Signature**: `$stop;` or `$stop(0|1|2);`
- **Standard**: IEEE 1800-2017 §20.2 / IEEE 1364-2005 §17.8.2
- **Meaning**: **suspends** the simulation and enters interactive debug mode. In
  a commercial simulator a waveform viewer or a console prompt comes up. The
  severity levels follow the same rules as `$finish`.
- **In a non-interactive environment**: where there is no interactive mode — CI,
  for instance — behaviour differs from simulator to simulator.
  - iverilog, by default: enters the interactive prompt (and can wait forever)
  - `vvp -n`: turns `$stop` into `$finish`
  - `vvp -N`: `$stop` becomes `$finish` plus exit code 1
  - Verilator: ends immediately

```sv
// the error-detection pattern
always @(posedge clk) begin
  if (error_flag) begin
    $display("ERROR at time %0t: error_flag asserted", $time);
    $stop;          // stop here to debug
  end
end

// the CI-friendly pattern (use $stop as an explicit failure)
// → run under vvp -N and it returns exit code 1
```

---

### `$exit` (SystemVerilog only)

- **Signature**: `$exit;`
- **Standard**: IEEE 1800-2017 §20.2
- **Meaning**: only meaningful inside a `program` block.
  It waits for every program block to finish and then calls `$finish`
  implicitly.
  Used directly in a `module` block its behaviour is simulator-defined; most
  simulators treat it as an alias for `$finish`.
- **Verilator**: implements `$exit` as an alias for `$finish`.

```sv
// the SV program-block pattern
program automatic tb;
  initial begin
    // ... the test logic ...
    $exit;   // end the simulation once every program block is done
  end
endprogram
```

---

## Synthesizability

❌ Not synthesizable — every task here is simulation-only.
Synthesis tools ignore calls to `$finish`, `$stop` and `$exit`.

---

## Sources

- IEEE 1800-2017 §20.2
- IEEE 1364-2005 §17.8
- research-log: [system-tasks-display-time-2026-05-28.md](../../../history/research-log/system-tasks-display-time-2026-05-28.md)
- [chipverify.com $stop $finish](https://chipverify.com/verilog/verilog-stop-finish)
- [circuitcove.com Simulation Control Tasks](https://circuitcove.com/system-tasks-simulation-control/)
- [steveicarus.github.io VVP Flags](https://steveicarus.github.io/iverilog/usage/vvp_flags.html)
- [verilator.org Input Languages](https://verilator.org/guide/latest/languages.html)
