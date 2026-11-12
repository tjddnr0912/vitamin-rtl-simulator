(§3.b ctx-guard-wide-param · OPEN · in Constants / parameters) (a wide parameter beside a function local declines the truth fold):
* symptom: LOUD: in a constant function, a `!` operand or `?:` condition mixing a module parameter over 64 bits with a frame local is E3009; the parameter alone folds (h3, h4)
* witness: h1_ctx_guard_lognot_local_wide `localparam logic [69:0] W = 70'h10_0000_0000_0000_0000;` and in `f`: `int t; t = k; return !(W | t) ? 1 : 2;`, `localparam int P = f(3);`; h2 the same with `(W + t) ? t : -t`
* witness: PRE = POST rc 1 ``t.sv:8:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]`` on both
* oracles: iverilog, sv2v → iverilog and verilator `A 2` (h1), `A 3` (h2) (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G13 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's; the Ctx lane's local-name guard of `selfdet_truth_arm` declines it (§5.2 VAL-1)
* status: OPEN
