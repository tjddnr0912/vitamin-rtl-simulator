(§2 xz-decimal-bound · OPEN · in Ranges / bounds / selects) (a malformed decimal x / z literal in a range bound is accepted):
* symptom: a decimal literal that mixes an x or z with digits is accepted in a range bound and folds silently
* witness: c1_dmal `logic [4'd1x : 0] w;` PRE vita-main 8c970e9b rc 0 `A bw=1` (W/s1v/plan2/cells)
* oracles: iverilog `t.sv:2: syntax error`, sv2v `t.sv:2:14: Parse error: unexpected token 'x' (Id_simple)`, verilator `%Error: t.sv:2:10: Mixing X/Z/? with digits not legal in decimal constant: 1x` (W/s1v/plan2/filings.txt F-LIT)
* note: the S1v-R prototype pR2 refused it with a false C1 cause (x / z bits of a literal), and pRp kept PRE's value
* status: OPEN
