# 10 · System Tasks (the Verilog view)

An overview of the standard system tasks and functions of Verilog (IEEE 1364-2005). For
the detailed per-category reference, cross-link to `../system-tasks/`.

---

## System task categories included in Verilog

| Category | Main items | Detail |
|---|---|---|
| Display & I/O | `$display`, `$write`, `$monitor`, `$strobe` plus the `b`/`o`/`h` suffixes | [../system-tasks/01-display-io.md](../system-tasks/01-display-io.md) |
| File I/O | `$fopen`, `$fclose`, `$fwrite`, `$fdisplay`, `$fread` | [../system-tasks/02-file-io.md](../system-tasks/02-file-io.md) |
| Memory load | `$readmemb`, `$readmemh` | [../system-tasks/03-memory-load.md](../system-tasks/03-memory-load.md) |
| Sim control | `$finish`, `$stop` | [../system-tasks/04-simulation-control.md](../system-tasks/04-simulation-control.md) |
| Time | `$time`, `$stime`, `$realtime` | [../system-tasks/05-time-functions.md](../system-tasks/05-time-functions.md) |
| Conversion | `$signed`, `$unsigned`, `$rtoi`, `$itor`, `$bitstoreal`, `$realtobits` | [../system-tasks/06-conversion.md](../system-tasks/06-conversion.md) |
| VCD dump | `$dumpfile`, `$dumpvars`, `$dumpon`, `$dumpoff`, `$dumpall`, `$dumpflush`, `$dumplimit` | [../system-tasks/10-vcd-dump.md](../system-tasks/10-vcd-dump.md) |
| Random | `$random`, `$dist_*` | [../system-tasks/09-random.md](../system-tasks/09-random.md) |

## SystemVerilog only (not in Verilog)

`$urandom`/`$urandom_range`, `$past`/`$rose`/`$fell`/`$stable`/`$changed` (assertion
sampling), `$bits`/`$clog2`/`$countones` and
`$value$plusargs`/`$test$plusargs` are IEEE 1800 (SystemVerilog) extensions. They are
not available in a Verilog-2005-only environment.

For the details, see the whole `../system-tasks/` folder and
`../systemverilog/08-functions-tasks.md`.

## Synthesizability

❌ Every system task and system function is **non-synthesizable**. They are for
simulation and verification only. Synthesis tools ignore system tasks such as
`$display` and `$monitor`, or warn about them. When a system task has to live inside
RTL code, the standard practice is to wrap it in `` `ifndef SYNTHESIS `` /
`` `endif ``.

## Sources

- IEEE 1364-2005 §17 (system tasks and functions)
- the whole `../system-tasks/` folder (per-category detail)
