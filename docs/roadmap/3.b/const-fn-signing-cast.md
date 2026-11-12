(§3.b const-fn-signing-cast · OPEN · in Constants / parameters) (a signing or `int'` cast in a constant function declines):
* symptom: LOUD: a signing cast or a type cast in a constant function's body is E3009, where a size cast there folds
* witness: g06_fn_signing_cast `f1` `return signed'(4'(a));`, `f2` `return 4'(a);`, `f3` `return int'(a);`, each `longint`, called with -1
* witness: PRE = POST rc 1 ``t.sv:6:26: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: `f1(…)` has no constant-fold arm [in top]`` and the same for `C` (`f3`); `f2` folds
* oracles: iverilog, sv2v → iverilog and verilator `A -1 -1 -1` (g06), and on c_w32_fn `A -1 4294967295 -1` (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G6 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's; `value/arms.rs`'s head note names it a kept ENV lane (§5.2 VAL-1)
* status: OPEN
