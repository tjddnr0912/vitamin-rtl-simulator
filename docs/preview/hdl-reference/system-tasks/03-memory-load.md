# 03 · Memory Load / Store System Tasks

## Overview

The category of tasks that initialise a memory array from a text file, or dump
one back out to a file. They are used for ROM initialisation, for loading test
vectors and for dumping simulation memory. Synthesizability is tool-dependent —
some synthesis tools do accept a `$readmem*` call as a ROM initialiser.

## Scope of this page

- **Load**: `$readmemh`, `$readmemb`, and the file format they both read —
  `@<hex>` address directives, `//` and `/* */` comments, `_` separators, `x`/`z`
  digits, and the IEEE 1364-2005 lowest-address-ascending fill rule.
- **Dump**: `$writememh`, `$writememb`, which write a file the load tasks can
  read back.

These notes describe the standard. For what vita accepts, and for its exact
warning behaviour on a short file or an out-of-range address, see
[manual/005_system-tasks.md](../../../manual/005_system-tasks.md) and
[manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Item detail

### `$readmemh` / `$readmemb`

- **Signature**:
  ```sv
  // four forms — the argument count sets the range
  $readmemh("filename", mem_array);
  $readmemh("filename", mem_array, start_addr);
  $readmemh("filename", mem_array, start_addr, end_addr);

  $readmemb("filename", mem_array);
  $readmemb("filename", mem_array, start_addr);
  $readmemb("filename", mem_array, start_addr, end_addr);
  ```

- **Standard**: IEEE 1364-2005 §17.2.8 / IEEE 1800-2017 §21.4

- **Meaning**:
  - `$readmemh` — reads a hexadecimal data file
  - `$readmemb` — reads a binary data file
  - `start_addr` / `end_addr`: bound the addresses of the memory array that may
    be written. Omitted, the array's whole declared range is used.

- **Returns**: void
- **Example**:

```sv
// the plain form — initialise the whole array
reg [7:0] rom [0:255];
initial $readmemh("rom_contents.hex", rom);

// bounded — load only the range [0x10, 0x1F]
reg [15:0] sram [0:4095];
initial $readmemh("patch.hex", sram, 16'h10, 16'h1F);

// a binary file
reg [3:0] lut [0:15];
initial $readmemb("lut_init.bin", lut);
```

---

## Memory file format rules (IEEE 1364-2005 §17.2.8)

The rules a memory file (`.hex`, `.mem`, …) has to follow.

### Data separators

Spaces, tabs and newlines all separate data. Any number of them in a row is
fine.

```text
AA BB CC DD
EE FF
```

### Comments

Both Verilog comment forms are accepted.

```text
// line comment: ignored to the end of this line
AA BB CC   // the first three bytes

/* block comment:
   ignored across several lines */
DD EE FF
```

### The `@<hex>` address directive

Changes the address the load continues from. The value after `@` is **always
hexadecimal**, with no `0x` prefix.

```text
@00 AA BB CC DD      // from address 0x00: AA, BB, CC, DD
// addresses 0x04..0x0F are skipped — left unchanged
@10 00 11 22 33      // from address 0x10: 00, 11, 22, 33
@FF EE               // address 0xFF: EE
```

With no `@addr` directive the load starts at the array's first address (or at the
`start_addr` argument) and fills upwards in order.

### Underscores (`_`) — readability separators

An `_` may be placed inside a numeric value to make it readable. It does not
affect the value.

```text
// in a $readmemh file: 32-bit values
DEAD_BEEF   // same as 0xDEADBEEF
1234_5678

// in a $readmemb file: 8-bit values
1111_0000   // same as 8'b11110000
```

### 4-state values (`x`, `z`)

`x` (unknown) and `z` (high-impedance) digits may appear in the file too.

```text
// $readmemh
xX   // unknown (an x in a hex digit position)
zZ   // high impedance

// $readmemb
xxxx_0000   // top 4 bits x, low 4 bits 0
```

---

## When the file and the array are different sizes

### File shorter than the array

Only the data present in the file is loaded, starting at `start_addr`. The
remaining elements of the array are left alone — they keep whatever they held
before (x, if they were never initialised). The same holds for the gaps an
`@addr` directive skips over.

```sv
// a 256-element array, but only 64 values in the file
// → [0]..[63] take the file's values; [64]..[255] stay x (uninitialised)
reg [7:0] mem [0:255];
initial $readmemh("partial.hex", mem);
```

### File longer than the array

Data beyond the end of the array makes the simulator issue a warning or an
error; which one is implementation-dependent — Icarus warns, some tools stop
with an error.

---

## `$writememh` / `$writememb`

- **Signature**:
  ```sv
  $writememh("filename", mem_array);
  $writememh("filename", mem_array, start_addr);
  $writememh("filename", mem_array, start_addr, end_addr);

  $writememb("filename", mem_array);
  $writememb("filename", mem_array, start_addr);
  $writememb("filename", mem_array, start_addr, end_addr);
  ```

- **Standard**: IEEE 1800-2017 §21.4 (added by SystemVerilog)
- **Meaning**: writes the contents of a memory array to a file in a format that
  `$readmemh`/`$readmemb` can read back.
  With `start_addr`/`end_addr`, only that range is written.
- **Output format**: the data, with an `@<hex_addr>` directive marking the
  address it belongs to. The resulting file can be loaded again by
  `$readmemh`/`$readmemb` as it stands.
- **Returns**: void
- **Example**:

```sv
// save the memory contents after the run
reg [7:0] ram [0:255];
// ... simulation runs ...
initial begin
  // the whole array
  $writememh("ram_dump.hex", ram);

  // one range only
  $writememh("ram_patch.hex", ram, 8'h10, 8'h1F);
end
```

**Illustrative `$writememh` output** (`ram[0]=8'hAA`, `ram[1]=8'hBB`). What the
standard requires is only that the load tasks can read the file back; the exact
layout — where address directives are emitted, how many words go on a line, and
whether a leading comment is written — differs between simulators:

```text
@00
AA
@01
BB
```

---

## A complete example

```sv
module tb_rom;
  // 256 x 8-bit ROM
  reg [7:0] rom [0:255];
  reg [7:0] addr;
  wire [7:0] data_out;

  // ROM initialisation
  initial begin
    // the hex file: @00 DE AD BE EF ... (addresses and data)
    $readmemh("rom_init.hex", rom);
    $display("rom[0]=%h rom[1]=%h", rom[0], rom[1]);
  end

  // dump after the run
  final begin
    $writememh("rom_verify_dump.hex", rom);
  end

endmodule
```

**What `rom_init.hex` might contain**:

```text
// ROM initialisation data (the IEEE 1364 §17.2.8 format)
@00
DE AD BE EF   // addresses 0..3
@10
00 11 22 33   // addresses 16..19, set explicitly
// addresses 4..15 in between stay uninitialised
```

---

## Icarus / Verilator differences

| Task | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$readmemh` | fully supported | supported (one dimension only) |
| `$readmemb` | fully supported | supported (one dimension only) |
| `@addr` directive | supported | supported |
| `//`, `/* */` comments | supported | supported |
| `_` underscore | supported | unclear whether supported |
| `$writememh` | supported | unstated (unclear) |
| `$writememb` | supported | unstated (unclear) |
| Multidimensional arrays | supported | **not supported** |

**Verilator restriction**: `$readmemh`/`$readmemb` handle a one-dimensional array
(`reg [N:0] mem [0:M]`) only. They cannot be used on a multidimensional array of
the `reg mem [N][M]` shape.

---

## Synthesizability

Synthesizability is tool-dependent:
- Some FPGA synthesis tools (Xilinx Vivado, Intel Quartus) recognise a
  `$readmemh` inside an `initial` block as a ROM initialiser and synthesize it.
- General-purpose ASIC synthesis tools mostly do not.
- `$writememh`/`$writememb` are never synthesizable.

---

## Sources

- IEEE 1364-2005 §17.2.8 (readmem tasks — primary standard)
- IEEE 1800-2017 §21.4 (writemem tasks, the SystemVerilog extension)
- research-log: [system-tasks-io-memory-2026-05-28.md](../../../history/research-log/system-tasks-io-memory-2026-05-28.md)
- [projectf.io — Initialize Memory in Verilog](https://projectf.io/posts/initialize-memory-in-verilog/)
- [peterfab.com — File I/O Functions](https://peterfab.com/ref/verilog/verilog_renerta/mobile/source/vrg00016.htm)
- [ovisign.com — Verilog Write/Read File Operations](https://ovisign.com/verilog-verification/verilog-write-read-file-operations/)
- [Verilog::Readmem — metacpan.org](https://metacpan.org/pod/Verilog::Readmem)
- [verilator.org — Input Languages](https://verilator.org/guide/latest/languages.html)
