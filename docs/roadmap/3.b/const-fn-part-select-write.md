(§3.b const-fn-part-select-write · OPEN · in Constants / parameters) (a constant `[m:l]` part-select write to a function local declines):
* symptom: LOUD: a constant part-select write (`x[7:4] = 4'hA`) to a constant-function local is E3009, where its `+:` and `-:` twins fold (f02, f03)
* witness: f01_part_write_const `logic [15:0] x; x = 0; x[7:4] = 4'hA; return x;`, `localparam int P = f(3);`: PRE = POST rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
* oracles: iverilog, sv2v → iverilog and verilator `A P=160` (W/batch4/lens-diff/oracle/f01_part_write_const/RES.txt)
* note: batch 4 lens-diff (W/batch4/RULINGS-lens-diff-r1.txt), PRE = POST, not batch 4's
* status: OPEN
