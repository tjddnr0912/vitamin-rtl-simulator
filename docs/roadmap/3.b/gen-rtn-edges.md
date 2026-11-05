(§3.b gen-rtn-edges · OPEN · in Constants / parameters) (calls into generate-block routines by hierarchy or in a constant):
* symptom: `u.g.f()`, `gi.f()` and a constant-expression call of a generate routine are E3009 (`frames_reserve.rs` hier gate, `const_func_table`, `frames_classify.rs`)
* compose the hier key, register per scope (the constant-call half shares §2 🆕 AG's root: one slice)
* oracles: iverilog
* status: OPEN
