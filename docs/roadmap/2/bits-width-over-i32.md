(§2 bits-width-over-i32 · OPEN · in Constant domain (i64)) (`$bits` of a vector 2^32 bits wide prints -1):
* symptom: a width of 2^31 or more overflows the 32-bit answer of `$bits`
* witness: lf_10_pmd `logic [1:0][2147483648-1:0] q;` `$display("%0d %0d", $bits(q), $size(q, 2));`: PRE = POST `-1 -2147483648` rc 0 (about 6 GB, 11 s)
* oracles: iverilog `0 -2147483648`, verilator `0 -2147483648`, sv2v → iverilog `-4294967296 -2147483648` (W/batch2/lens-diff/oracle/lf_10_pmd/RES.txt)
* note: batch 2 lens-diff N-2 (W/batch2/RULINGS-lens-diff-r1.txt), not batch 2's; `$size` agrees with the oracles, `$bits` does not
* status: OPEN
