(§3.b const-fn-bits-of-local · OPEN · in Constants / parameters) (`$bits` of a constant-function local declines):
* symptom: LOUD: `$bits` of a constant-function local is E3009, also for a local shadowing a module parameter or sized by a constant function
* witness: f11_bits_local `logic [12:0] x; x = a; return $bits(x);`; f43 (`logic [20:0] W` shadowing `localparam int W = 6`), b14 (`logic [12:0] X` beside a 6-bit `X`, `$bits(X) + X`), c22 (`logic [g(6)-1:0] t`): PRE = POST rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
* oracles: iverilog, sv2v → iverilog and verilator `A P=13` (f11), `A P=21 W=6` (f43), `A P=15 bX=6` (b14), `A P=74` (c22) (W/batch4/lens-diff/oracle/)
* note: c22 sizes its local by a constant function, a source IEEE 1800-2017 §13.4.3 forbids (§2 const-fn-local-unbound-width)
* note: batch 4 lens-diff (W/batch4/RULINGS-lens-diff-r1.txt), PRE = POST, not batch 4's
* status: OPEN
