(§3.b const-fn-local-select-read · OPEN · in Constants / parameters) (a select read of a constant-function local declines):
* symptom: LOUD: a part-select or bit-select read of a constant-function local (`b[32:1]`, `x[11:4] + x[a]`) is E3009
* witness: f04_w33 `logic [32:0] b; b = -a; return b[32:1];`, `f(3)`, and f39_select_read_local `logic [15:0] x; x = 16'hABCD; return x[11:4] + x[a];`, `f(0)`: PRE = POST rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
* oracles: iverilog, sv2v → iverilog and verilator `A P=-2` (f04), `A P=189` (f39) (W/batch4/lens-diff/oracle/f04_w33, f39_select_read_local)
* note: batch 4 lens-diff (W/batch4/RULINGS-lens-diff-r1.txt), PRE = POST, not batch 4's
* status: OPEN
