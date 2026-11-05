(§3.b empty-param-list · DEFERRED · in Parser accept) (an empty parameter port list `#()` is rejected):
* symptom: an empty parameter port list `module m #() ();` is E2002 (`expected identifier, found ')'`)
* IEEE 1800-2017 A.1.3 allows `#( )`
* oracles: verilator and sv2v `m ok` (iverilog syntax error)
* witness: row-9 O3
* status: DEFERRED (§7 row 56)
