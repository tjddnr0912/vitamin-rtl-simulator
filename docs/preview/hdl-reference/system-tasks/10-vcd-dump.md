# 10 · VCD Dump System Tasks

## Overview

This category covers the tasks that create and control a VCD (Value Change Dump) waveform file.
They record signal-value changes to a file during simulation so that a waveform viewer such as
GTKWave can analyse them afterwards. They are simulation-only and not synthesizable.

The VCD file format itself — the header, scopes, variable declarations and the value-change form —
is specified separately in [07-vcd-format.md](../../07-vcd-format.md).
This note concerns only the call signatures and the behaviour of the tasks.

## vita support

This note describes the language, not the simulator. What vita accepts today is recorded in
[docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## The core rule

A VCD is produced only when both `$dumpfile` and `$dumpvars` are called.
A `$dumpfile` with no `$dumpvars` creates no file — it is a no-op.
`$dumpvars` is what actually triggers the dump.

---

## Entries

### `$dumpfile("filename.vcd")`

- **Signature**: `$dumpfile(string_filename)`
- **Standard**: IEEE 1364-2005 §18.1
- **Meaning**: names the file path the VCD is written to.
  It has to be called before `$dumpvars`; when it is omitted the default file name `"dump.vcd"`
  is used.
  The path may be relative to the simulator's working directory or absolute.
- **Returns**: void
- **Example**:

```sv
initial begin
  $dumpfile("tb_output.vcd");
  $dumpvars(0, tb);   // $dumpvars must be called too, or no VCD appears
end
```

---

### `$dumpvars`

- **Signature** (three forms):

  ```sv
  $dumpvars;                                  // 1) no arguments
  $dumpvars(level);                           // 2) level only
  $dumpvars(level, scope1 [, scope2, ...]);   // 3) level plus a scope list
  ```

- **Standard**: IEEE 1364-2005 §18.2.1
- **Meaning**: selects which signals go into the VCD and starts the dump.
  Without `$dumpvars` no VCD is produced.

#### The arguments

| Argument | Type | Meaning |
|------|------|------|
| `level` | integer | Hierarchy depth. `0` = unlimited (the named scope and every instance beneath it). `1` = that scope only. `2` = one level below it. |
| `scope` | a module or net reference | The module or the individual signal to dump. Several may be listed. |

#### Call forms

```sv
// 1) no arguments — every signal in the whole design
$dumpvars;

// 2) level only — level 0 = the whole design (the same as no arguments)
$dumpvars(0);

// 3) a specific scope, two levels deep
$dumpvars(2, tb.dut);

// 4) a specific scope, unlimited depth
$dumpvars(0, tb);

// 5) individual signals (a signal name may stand where a scope does)
$dumpvars(0, tb.dut.sig_valid, tb.dut.data_out);

// 6) several scopes at once
$dumpvars(0, tb.ram_ctrl, tb.alu);
```

- **Returns**: void
- **Careful**: the dump does not begin the instant `$dumpvars` is called; it becomes active at the
  end of the current simulation time (the end of the current timestep).

---

### `$dumpoff`

- **Signature**: `$dumpoff`
- **Standard**: IEEE 1364-2005 §18.3
- **Meaning**: suspends the dump. Two things happen at once:
  1. an `x` value is written to the VCD for every variable currently being traced
  2. no later signal change is written to the VCD

  In a waveform viewer every signal reads as X over the interval after `$dumpoff`.
- **Returns**: void
- **Example**:

```sv
initial begin
  $dumpfile("sim.vcd");
  $dumpvars(0, tb);
  #1000;
  $dumpoff;      // skip the start-up noise
  #100;
  $dumpon;       // record only the interval of the real test
  #5000;
  $finish;
end
```

---

### `$dumpon`

- **Signature**: `$dumpon`
- **Standard**: IEEE 1364-2005 §18.3
- **Meaning**: resumes a dump suspended by `$dumpoff`.
  It writes the current value of every signal to the VCD at the point of resumption and then keeps
  tracking changes.
- **Returns**: void

---

### `$dumpall`

- **Signature**: `$dumpall`
- **Standard**: IEEE 1364-2005 §18.4
- **Meaning**: immediately forces the current value of every traced variable into the VCD at the
  current simulation time.
  A VCD normally records only changes (deltas), so `$dumpall` is what you use when a full state
  snapshot (a checkpoint) is wanted at a particular point.
  In a long simulation it helps a waveform viewer render the intermediate state correctly.
- **Returns**: void
- **Example**:

```sv
// insert a checkpoint every 1000 ns
always #1000 $dumpall;
```

---

### `$dumpflush`

- **Signature**: `$dumpflush`
- **Standard**: IEEE 1364-2005 §18.5
- **Meaning**: flushes the simulator's internal VCD buffer to the file immediately.
  Simulators buffer VCD data internally and write it in batches for I/O performance;
  `$dumpflush` empties that buffer at once so the file reflects it.

  When to use it:
  - guarding against an unexpected termination (a crash) in a long run
  - working alongside a live waveform viewer (GTKWave live reload)
- **Returns**: void

---

### `$dumplimit(byte_limit)`

- **Signature**: `$dumplimit(integer byte_limit)`
- **Standard**: IEEE 1364-2005 §18.6
- **Meaning**: sets an upper bound on the VCD file size.
  Once the file reaches `byte_limit` bytes the dump stops automatically and the following is
  inserted at the end of the VCD:

  ```vcd
  $comment Dump limit reached $end
  ```

  Use it to cap a VCD that could otherwise reach several gigabytes and exhaust the disk.
- **Returns**: void
- **Example**:

```sv
initial begin
  $dumpfile("sim.vcd");
  $dumplimit(100_000_000);   // a 100 MB cap
  $dumpvars(0, tb);
end
```

---

## Icarus / Verilator behavioural differences

| Task | Icarus Verilog | Verilator |
|--------|---------------|-----------|
| `$dumpfile` | full | supported |
| `$dumpvars` (no arguments) | full | supported |
| `$dumpvars` level argument | full | **ignored** |
| `$dumpvars` scope argument | full | **ignored** (dumps from the design top) |
| `$dumpoff` | full | **currently ignored** |
| `$dumpon` | full | **currently ignored** |
| `$dumpall` | full | **currently ignored** |
| `$dumplimit` | supported | **currently ignored** |
| `$dumpflush` | supported | unspecified |
| Concurrent trace files | no limit | **only one** may be active |

**A note on Verilator**: VCD tracing under Verilator ignores the scope and level arguments of
`$dumpvars` and traces the whole design top. Narrowing the trace scope takes a compile-time pragma
(`/* verilator tracing_off */` and `/* verilator tracing_on */`) instead.

---

## Synthesizability

❌ Not synthesizable — every task here is simulation-only.

---

## Sources

- IEEE 1364-2005 §18.1–18.6 (VCD dump system tasks — the primary standard)
- IEEE 1800-2017 §18 (SystemVerilog, which absorbs 1364 §18)
- research-log: [system-tasks-io-memory-2026-05-28.md](../../../history/research-log/system-tasks-io-memory-2026-05-28.md)
- [chipverify.com — Verilog VCD Dump](https://chipverify.com/verilog/verilog-dump-vcd)
- [peterfab.com — Verilog VCD Tasks](https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00056.htm)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
- [07-vcd-format.md](../../07-vcd-format.md) (internal document, the VCD file format specification)
