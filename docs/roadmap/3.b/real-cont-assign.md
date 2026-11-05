(§3.b real-cont-assign · OPEN · in Loud shapes surfaced by §4.5.493–495) (a continuous assign of a function call to a `real` variable):
* symptom: `assign w = fr();` to a `real` — to any variable that is not `logic` — is E3018 when the right-hand side calls a function (§4.5.600's call gate, `cont_var.rs`): a call-free `assign` runs, but a callee body's read of a variable that is not an argument re-evaluates the `assign` in vita (its `logic` twin today) and verilator (`real w; real k; function real fr(); fr = k * 2.0; endfunction`: `t2 w=6.000000`), not in iverilog (`t2 w=2.500000`; IEEE 1800 §10.3.2 re-evaluates on an operand)
* a callee reading only its formals agrees in all three (`assign w = f(k);` `t2 w=10`)
* fix: lift for that callee, the body-read case being a split
* oracles: 2 oracles
* status: OPEN
