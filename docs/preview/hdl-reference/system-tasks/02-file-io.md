# 02 · File I/O System Tasks

## Overview

The category of tasks and functions that read and write files during simulation.
A testbench uses them to load stimulus from a file, or to save results for
post-processing. All of them are simulation-only and cannot be synthesized.

## Scope of this page

- **Write side**: `$fopen`, `$fclose`, `$fwrite`/`$fdisplay` (plus their b/o/h
  variants and the MCD broadcast form), `$fstrobe`, `$fmonitor`.
- **Read side**: `$fread`, `$fscanf`, `$fgets`, `$sscanf`, `$feof`, `$fgetc`.
- **String formatting**: `$sformat`, `$sformatf`.

These notes describe the standard. Which spellings vita accepts, in which
statement positions a file read may appear, and how each descriptor behaves are
documented in [manual/005_system-tasks.md](../../../manual/005_system-tasks.md),
[manual/003_language-reference.md](../../../manual/003_language-reference.md) and
[manual/006_limitations.md](../../../manual/006_limitations.md).

---

## mcd vs fd — the distinction to get right first

Verilog file I/O carries two file-handle schemes side by side. Confusing them is
the single most common mistake in this category.

### mcd (multi-channel descriptor) — the pre-Verilog-2001 legacy form

`$fopen("filename")` — a file name and no mode argument returns an mcd.

- A 32-bit bit field (`reg [31:0]`) with exactly one bit set
- bit 0 = stdout (always open, cannot be closed)
- bit 31 = reserved (unusable)
- at most 30 files open at once
- OR the handles together to write to several files in one call

```sv
// mcd form — write to two files at once
reg [31:0] fd_log, fd_csv;
fd_log = $fopen("sim.log");   // no mode → returns an mcd
fd_csv = $fopen("data.csv");
$fdisplay(fd_log | fd_csv, "time=%0t val=%h", $time, val);
// to send it to stdout as well: fd_log | fd_csv | 32'h1
```

### fd (file descriptor) — the modern IEEE 1364-2001 form

`$fopen("filename", "mode")` — a mode argument returns an fd.

- A positive integer handle (the C `FILE*` style)
- 0 means the open failed
- read modes, seeking and binary I/O are available
- unlike an mcd, fds cannot be OR-ed together

```sv
// fd form — the advanced I/O, reading included
integer fd;
fd = $fopen("output.txt", "w");   // a mode → returns an fd
if (fd == 0) $display("open failed");
$fdisplay(fd, "result=%d", result);
$fclose(fd);
```

**In short**: mcd for simple write-only logging, fd for reading, seeking, binary
data and anything else beyond it.

---

## Item detail

### `$fopen`

- **Signature**:
  ```sv
  // mcd form (pre-Verilog-2001)
  reg [31:0] mcd;
  mcd = $fopen("filename");

  // fd form (IEEE 1364-2001+, SV)
  integer fd;
  fd = $fopen("filename", "mode");
  ```
- **Standard**: IEEE 1800-2017 §21.3 / IEEE 1364-2005 §17.4.1
- **Mode strings**:

| Mode | Behaviour |
|------|-----------|
| `"r"` / `"rb"` | read only (fails and returns 0 if the file does not exist) |
| `"w"` / `"wb"` | write (creates the file, or truncates an existing one) |
| `"a"` / `"ab"` | append (creates the file if it does not exist) |
| `"r+"` / `"rb+"` | read and write (the file must already exist) |
| `"w+"` / `"wb+"` | read and write (creates or truncates) |
| `"a+"` / `"ab+"` | read and append |

The `b` suffix means binary mode (no text/binary distinction on Unix; on Windows
it changes line-ending handling).

- **Returns**: the mcd form a 32-bit mcd, the fd form an integer fd; 0 on failure
  either way

---

### `$fclose`

- **Signature**: `$fclose(fd_or_mcd)`
- **Standard**: IEEE 1800-2017 §21.3 / IEEE 1364-2005 §17.4.2
- **Meaning**: closes the file. Any `$fmonitor` or `$fstrobe` still active on
  that file is cancelled automatically.
- **Returns**: void
- **Example**:

```sv
integer fd;
fd = $fopen("result.txt", "w");
$fdisplay(fd, "done at time %0t", $time);
$fclose(fd);
```

---

### `$fwrite` / `$fdisplay` / `$fmonitor` / `$fstrobe`

The same tasks as the console family (`$write`/`$display`/`$monitor`/`$strobe`),
except that the first argument is a file handle (an fd or an mcd).

- **Signature**:
  ```sv
  $fdisplay(fd_or_mcd [, "format" [, args...]]);
  $fwrite(fd_or_mcd [, "format" [, args...]]);
  $fstrobe(fd_or_mcd [, "format" [, args...]]);
  $fmonitor(fd_or_mcd [, "format" [, args...]]);
  ```
- **Standard**: IEEE 1800-2017 §21.2 / IEEE 1364-2005 §17.3
- **When each one runs, and whether it appends a newline**:

| Task | When it runs | Newline appended |
|------|--------------|------------------|
| `$fdisplay` | Active/Inactive region (immediately) | ✅ |
| `$fwrite` | Active/Inactive region (immediately) | ❌ |
| `$fstrobe` | Postponed region (after nonblocking updates) | ✅ |
| `$fmonitor` | Postponed region (automatically, on an argument change) | ✅ |

The b/o/h variants exist here too: `$fdisplayh`, `$fwriteb`, `$fstrobeo`,
`$fmonitorh` and so on.

- **Returns**: void
- **Example**:

```sv
integer log_fd;
initial begin
  log_fd = $fopen("sim.log", "w");

  // console and file at once (the mcd OR form)
  reg [31:0] both;
  both = log_fd | 32'h1;   // bit0 = stdout
  $fdisplay(both, "starting simulation");
end

always @(posedge clk) begin
  $fwrite(log_fd, "t=%0t d=%b q=%b  ", $time, d, q);
  $fstrobe(log_fd, "q_final=%b", q);   // the settled value, after the NBA
end
```

---

### `$fread`

- **Signature**:
  ```sv
  integer n;
  n = $fread(reg_or_mem_target, fd);
  n = $fread(reg_or_mem_target, fd, start_addr);
  n = $fread(reg_or_mem_target, fd, start_addr, count);
  ```
- **Standard**: IEEE 1800-2017 §21.4 / IEEE 1364-2005 §17.4.4
- **Meaning**: reads binary data from a file.
  If `target` is a single reg, it reads as many bytes as that width takes
  (bit count / 8).
  If `target` is a memory array, it fills elements in order from `start_addr`.
  With `count` given, it reads that many elements and no more.
- **Returns**: the number of bytes actually read (0 or less at EOF or on error)
- **Example**:

```sv
reg [7:0] mem [0:255];
integer fd, n;
initial begin
  fd = $fopen("data.bin", "rb");
  n = $fread(mem, fd, 0, 256);   // load 256 elements starting at address 0
  $display("read %0d bytes", n);
  $fclose(fd);
end
```

---

### `$fscanf`

- **Signature**: `integer n = $fscanf(fd, "format_string", var1, var2, ...)`
- **Standard**: IEEE 1800-2017 §21.3 / IEEE 1364-2005 §17.4.3
- **Meaning**: reads a line (or a field) from the file and parses it against the
  format string, using the same conversion specifiers as C's `fscanf()`.
- **Returns**: the number of items matched successfully (negative at EOF, 0 on
  error)
- **Example**:

```sv
integer fd, addr, val, n;
initial begin
  fd = $fopen("vectors.txt", "r");
  while (!$feof(fd)) begin
    n = $fscanf(fd, "%h %h\n", addr, val);
    if (n == 2) begin
      mem[addr] = val;
    end
  end
  $fclose(fd);
end
```

---

### `$fgets`

- **Signature**: `integer n = $fgets(str_var, fd)`
- **Standard**: IEEE 1800-2017 §21.3 / IEEE 1364-2005 §17.4.3
- **Meaning**: reads one line from the file into `str_var`, up to and including a
  newline or up to EOF. The size of `str_var` sets the limit.
- **Returns**: the number of characters read (0 on error, or when EOF is reached
  immediately)
- **Example**:

```sv
reg [255*8-1:0] line;
integer fd, n;
initial begin
  fd = $fopen("input.txt", "r");
  n = $fgets(line, fd);
  while (n > 0) begin
    $display("line: %s", line);
    n = $fgets(line, fd);
  end
  $fclose(fd);
end
```

---

### `$sscanf`

- **Signature**: `integer n = $sscanf(source_string, "format_string", var1, var2, ...)`
- **Standard**: IEEE 1800-2017 §21.3
- **Meaning**: parses a **string** instead of a file — the string counterpart of
  `$fscanf`.
- **Returns**: the number of items matched successfully
- **Example**:

```sv
string line = "addr=FF data=AB";
integer addr_v, data_v, n;
n = $sscanf(line, "addr=%h data=%h", addr_v, data_v);
// n=2, addr_v=8'hFF, data_v=8'hAB
```

---

### `$sformat`

- **Signature**: `$sformat(output_reg, "format_string" [, arg1, arg2, ...])`
- **Standard**: IEEE 1800-2017 §20.9 / IEEE 1364-2005 §17.1.3
- **Meaning**: renders the format string and its arguments and stores the result
  as a string in `output_reg`.
  It is a **task** — it returns nothing.
  The first argument `output_reg` is the `reg` or `string` variable that receives
  the result.
- **Returns**: void (task)
- **Example**:

```sv
reg [255*8-1:0] msg;
$sformat(msg, "error: addr=%h expected=%h got=%h", addr, exp, got);
$display("%s", msg);
```

---

### `$sformatf`

- **Signature**: `string s = $sformatf("format_string" [, arg1, arg2, ...])`
- **Standard**: IEEE 1800-2017 §20.9.1 (SystemVerilog only)
- **Meaning**: the same rendering as `$sformat`, but as a **function** — it
  returns the string directly.
  The difference: `$sformat` is a task that stores its result in its first
  argument, `$sformatf` is a function whose return value is the string. In
  SystemVerilog, `$sformatf` is the preferred form.
- **Returns**: `string` (function)
- **Example**:

```sv
// $sformatf can be used directly in an expression position
$display($sformatf("val=0x%08X tick=%0d", val, $time));

// convenient for building up strings
string prefix = "ERROR";
string msg = $sformatf("[%s] mismatch at addr=%h", prefix, addr);
```

---

## $sformat vs $sformatf

| Item | `$sformat` | `$sformatf` |
|------|-----------|------------|
| Kind | task (void) | function (returns a value) |
| How the result arrives | stored in the first argument | used directly as the return value |
| Available from | IEEE 1364-2001+ | IEEE 1800-2017 (SystemVerilog only) |
| Where it fits | when Verilog compatibility is needed | SystemVerilog (a modern testbench) |

---

## Icarus / Verilator differences

| Task | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$fopen` (both mcd and fd forms) | fully supported | generally supported |
| `$fclose` | fully supported | generally supported |
| `$fdisplay` / `$fwrite` | fully supported | generally supported |
| `$fstrobe` / `$fmonitor` | fully supported | generally supported |
| `$fread` | fully supported | unstated (could not be confirmed) |
| `$fscanf` | fully supported | generally supported |
| `$fgets` / `$fgetc` | fully supported | generally supported |
| `$sscanf` | fully supported | generally supported |
| `$sformat` | fully supported | unstated |
| `$sformatf` | fully supported | unstated |

---

## Synthesizability

❌ Not synthesizable — every task and function here is simulation-only.

---

## Sources

- IEEE 1800-2017 §21 (File I/O system functions/tasks)
- IEEE 1364-2005 §17.4 (Verilog file I/O)
- research-log: [system-tasks-io-memory-2026-05-28.md](../../../history/research-log/system-tasks-io-memory-2026-05-28.md)
- [chipverify.com — Verilog File IO Operations](https://chipverify.com/verilog/verilog-file-io-operations)
- [circuitcove.com — File I/O Tasks](https://circuitcove.com/system-tasks-file-io/)
- [hdlworks.com — System File I/O Tasks](https://www.hdlworks.com/hdl_corner/verilog_ref/items/SystemFileTasks.htm)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
