# 00 · System Tasks · Functions Index

The standard built-ins whose names begin with `$`, grouped by category. One file
per category; each file gives the signature, the governing IEEE clause, the
semantics, the tool-to-tool differences and the synthesizability of every item.

These notes document the languages, not this simulator. For what vita itself
accepts, refuses or simplifies, the authority is
[manual/003_language-reference.md](../../../manual/003_language-reference.md),
with the system-task detail in
[manual/005_system-tasks.md](../../../manual/005_system-tasks.md).

## Category index

| # | File | Key items |
|---|---|---|
| 01 | [display-io](01-display-io.md) | `$display`/`$write`/`$monitor`/`$strobe` + the b/o/h variants |
| 02 | [file-io](02-file-io.md) | WRITE: `$fopen`/`$fclose`/`$fwrite`/`$fdisplay`(+b/o/h, MCD)/`$sformat`/`$sformatf`; READ: `$fread`/`$fscanf`/`$fgets`/`$sscanf`(+`$feof`/`$fgetc`); deferred-print: `$fmonitor`/`$fstrobe` |
| 03 | [memory-load](03-memory-load.md) | `$readmemb`/`$readmemh`; `$writememb`/`$writememh` |
| 04 | [simulation-control](04-simulation-control.md) | `$finish`/`$stop`; `$exit` |
| 05 | [time-functions](05-time-functions.md) | `$time`/`$realtime`/`$stime` |
| 06 | [conversion](06-conversion.md) | `$signed`/`$unsigned`/`$rtoi`/`$itor`/`$bitstoreal`/`$realtobits` (plus the `shortreal` pair `$shortrealtobits`/`$bitstoshortreal`) |
| 07 | [bit-vector](07-bit-vector.md) | `$bits`/`$clog2`/`$countones`/`$countbits`/`$onehot`/`$onehot0`/`$isunknown` |
| 08 | [math](08-math.md) | `$pow`/`$ln`/`$log10`/`$exp`/`$sqrt`/`$sin`/`$cos`/`$tan` and the rest |
| 09 | [random](09-random.md) | `$random`/`$urandom`/`$urandom_range`/`$dist_*` |
| 10 | [vcd-dump](10-vcd-dump.md) | `$dumpfile`/`$dumpvars`/`$dumpon`/`$dumpoff`/`$dumpall`/`$dumpflush`/`$dumplimit` |
| 11 | [assertion-sampling](11-assertion-sampling.md) | `$past`/`$rose`/`$fell`/`$stable`/`$changed`/`$sampled`/`$assertoff`/`$asserton`/`$assertkill` |
| 12 | [introspection](12-introspection.md) | `$typename`/`$cast`/`$isunbounded`/`$size`/`$left`/`$right`/`$low`/`$high`/`$increment` |
| 13 | [misc](13-misc.md) | `$value$plusargs`/`$test$plusargs`/`$system` and the rest |

## The minimum set

These are the tasks the simplest possible RTL testbench cannot do without.

- **display**: `$display`, `$write`, `$monitor`, `$strobe`
- **time**: `$time`, `$realtime`
- **control**: `$finish`, `$stop`
- **dump**: `$dumpfile`, `$dumpvars`, `$dumpon`, `$dumpoff`, `$dumpall`

Without them even a Hello-World simulation cannot be confirmed to have run:
without `$finish` the simulation never ends, and without `$display` there is
nothing to look at.

## Beyond the minimum

The next tier of a working testbench is file I/O (the write family and the read
family), memory initialisation (`$readmem*`, `$writemem*`), the bit-vector query
functions (`$bits`, `$countones`, `$countbits`, `$onehot`, `$isunknown`), the
random and stochastic functions (`$random`, whose bit stream IEEE 1364-2005
Annex N pins exactly, plus `$urandom`, `$urandom_range` and the `$dist_*`
family), string formatting (`$sformat`, `$sformatf`), monitor control
(`$monitoron`, `$monitoroff`), the introspection functions (`$typename`,
`$cast`, `$size`, `$left`, `$right`) and the real-math transcendentals (`$ln`,
`$exp`, `$sqrt`, `$sin` and the rest of IEEE 1800-2017 §20.8.2).

## Verification-only categories

- **SV sampled-value functions and assertion control**: `$past`, `$rose`,
  `$fell`, `$stable`, `$changed`, `$sampled` sample a value in the Preponed
  region and are written against concurrent assertions;
  `$assertoff`/`$asserton`/`$assertkill` (and the newer `$assertcontrol`) gate
  whether an assertion may fire at all. See
  [11-assertion-sampling](11-assertion-sampling.md).
- **Extended dump tasks**: `$dumpports*` and the non-standard waveform formats
  sit outside the core VCD task set; see [10-vcd-dump](10-vcd-dump.md).

Both categories are verification-only: they reach well past the synthesizable
RTL subset that the rest of these notes centre on.

## Sources

- This project's spec, §9 (system tasks, by phase)
- IEEE 1800-2017 §20, IEEE 1364-2005 §17
- research-log: [system-tasks-display-time-2026-05-28.md](../../../history/research-log/system-tasks-display-time-2026-05-28.md)
