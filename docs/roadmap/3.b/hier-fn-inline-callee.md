(§3.b hier-fn-inline-callee · OPEN · in Subroutine / frame) (a hierarchical call to an inline-lowered function is refused):
* symptom: a hierarchical call to an inline-lowered callee is E3009 (`hier_defer/func_call.rs` reads framed `hier_funcs` only)
* frame the callee or inline it in the instance scope
* oracles: 2 oracles
* status: OPEN
