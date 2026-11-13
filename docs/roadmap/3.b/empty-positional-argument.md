(§3.b empty-positional-argument · OPEN · in Parser accept) (an empty positional argument `f(1, , 3)` is a parse error):
* symptom: LOUD: an empty positional argument of a call whose formals have defaults is E2002 at parse
* witness: c26_default_args `function automatic int f(input int a, input int b = 4, input int c = 10); return a * 100 + b * 10 + c; endfunction` with `f(1)`, `f(1, 2)`, `f(1, , 3)`: PRE = POST rc 1 `t.sv:5:27: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected expression, found ','`
* oracles: iverilog and verilator `A 150 130 143`; sv2v `error: The function f has been called with missing/empty parameters.` (W/batch4/lens-diff/oracle/c26_default_args/RES.txt)
* note: batch 4 lens-diff (W/batch4/RULINGS-lens-diff-r1.txt), PRE = POST, not batch 4's
* status: OPEN
