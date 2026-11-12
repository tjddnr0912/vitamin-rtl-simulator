(§3.b const-fn-if-x · OPEN · in Constants / parameters) (a constant function's `if` on an x condition declines):
* symptom: LOUD: a constant function whose `if` condition holds an x bit is E3009, where the same function written with `?:` folds
* witness: g02_fn_if_x `function automatic int f(input int k); if (4'b100x) return k; else return -k; endfunction`, its twin `g` returning `4'b100x ? k : -k`, `localparam int Q = g(3); localparam int P = f(3);`
* witness: PRE = POST rc 1 ``t.sv:9:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``; a16_fn_if_x (`f` alone) the same
* oracles: iverilog, sv2v → iverilog and verilator `A 3 3` (g02_fn_if_x), `A 3` (a16_fn_if_x) (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G2 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's
* status: OPEN
