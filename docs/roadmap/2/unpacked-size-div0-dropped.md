(§2 unpacked-size-div0-dropped · OPEN · in Ranges / bounds / selects) (an unpacked size dimension that divides by zero is dropped):
* symptom: SILENT: an unpacked size dimension `[8/0]` or `[0/0]` is dropped at rc 0, so `$size` and `$bits` read the element alone
* witness: g12_size_x `logic [1:0] b [8/0]; int c [0/0];` `$display("A %0d %0d %0d", $size(b), $bits(b), $size(c));`: PRE = POST `A 2 2 32` rc 0; f36_md_shared_span `A 48 2` the same way
* oracles: iverilog `t.sv:2: error: Dimension size must be greater than zero.`, verilator `%Error: t.sv:2:19: right side of bit range isn't a two-state constant (IEEE 1800-2023 6.9.1)`, sv2v → iverilog `A x x x` (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G12 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's; main's re-measure on the frozen binaries confirmed it (R4)
* status: OPEN
