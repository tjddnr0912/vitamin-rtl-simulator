(§2 sysfn-zero-arg-panic · OPEN · in Diagnostics / artifacts) (`$clog2()` or `$signed()` with no argument panics at run time):
* symptom: a zero-argument `$clog2()` or `$signed()` in a run-time position passes elaboration and panics the engine, `crates/sim-engine/src/eval/sysfunc.rs:393:39` / `:861:39` `index out of bounds: the len is 0 but the index is 0`, rc 101
* witness: s_clog2_0arg__rt `initial begin #1; $display("rt=%0d", $clog2()); end`, PRE = POST rc 101; 10 cells, s_clog2_0arg__ and s_signed_0arg__ in rt, rtassign, task, case and assert positions (W/batch2/lens-diff/oracle)
* oracles: iverilog `t.sv:9: error: constant function $clog2() does not support 0 arguments.` and `ERROR: t.sv:9: $clog2 requires one numeric argument.`, a segfault on `$signed()` (rc 139); verilator `%Error: t.sv:9:47: syntax error, unexpected ')', expecting IDENTIFIER-for-type`
* note: batch 2 lens-diff N-1 (W/batch2/RULINGS-lens-diff-r1.txt), not batch 2's (PRE = POST): a crash where the oracles refuse; a census of zero-argument system calls at run time is not taken
* status: OPEN
