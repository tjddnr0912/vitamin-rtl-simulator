(§3.b const-fn-negate-min · OPEN · in Constants / parameters) (a constant function's minus of the 64-bit minimum declines):
* symptom: LOUD: unary minus of the 64-bit minimum inside a constant function is E3009, where module scope folds it (b01)
* witness: b02_neg_min_fn `function automatic longint f(input longint a); return -a; endfunction` `localparam longint A = f(64'sh8000_0000_0000_0000);`
* witness: PRE = POST rc 1 ``t.sv:3:26: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: `f(…)` has no constant-fold arm [in top]``; b03 (an 8-bit return) and b19 the same
* oracles: iverilog, sv2v → iverilog and verilator `A -9223372036854775808` (b02), `A 0` (b03), `A 0 8000000000000000` (b19) (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G4 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's; `checked_neg` declines there (§5.2 VAL-1, M-UNARY)
* status: OPEN
