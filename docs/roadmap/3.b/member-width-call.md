(§3.b member-width-call · OPEN · in Parser accept) (a package struct member width that calls a constant function):
* symptom: a package struct member width over a constant function call (`top_racl_pkg`, `prim_util_pkg::vbits`, `otbn_pkg`) is E2002 (`struct member width must be a named integer type or a constant-literal range`; `typedefs.rs:761–778`)
* the package struct's record keeps the width as an expression spelled in the package's scoped names (`p::vbits(…)`), folded once by elaborate
* oracles: iverilog and sv2v → iverilog `A s=6 e=1 bits=4 L=3`
* witness: row-9 R01b
* status: OPEN (§5.2 row 43)
