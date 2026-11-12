(§3.b const-fn-wide-local-truth · OPEN · in Constants / parameters) (a function local over 64 bits as a `?:` condition declines):
* symptom: LOUD: a constant function whose `?:` condition is a local wider than 64 bits is E3009
* witness: a17_fn_wide_local_tern `function automatic int f(input int k); logic [79:0] w; w = 80'h1 << 70; return w ? k : -k; endfunction` `localparam int P = f(3);`
* witness: PRE = POST rc 1 ``t.sv:7:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
* oracles: iverilog, sv2v → iverilog and verilator `A 3` (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G3 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's; the closest line, §2 🆕 H ⓔ, names reductions only
* status: OPEN
