(§3.b const-fn-break-continue · OPEN · in Constants / parameters) (`break` or `continue` in a constant function declines):
* symptom: LOUD: a constant function whose loop holds `continue` (f13) or `break` (f14) is E3009
* witness: f13_continue `for (int i = 0; i < 10; i++) begin if (i % 3 == 0) continue; s += i; end return s;`, `f(3)`: PRE = POST rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``; f14 (`break`) the same
* oracles: iverilog, sv2v → iverilog and verilator `A P=27` (f13; f14 not oracled) (W/batch4/lens-diff/oracle/f13_continue/RES.txt)
* note: batch 4 lens-diff (W/batch4/RULINGS-lens-diff-r1.txt), PRE = POST, not batch 4's
* status: OPEN
