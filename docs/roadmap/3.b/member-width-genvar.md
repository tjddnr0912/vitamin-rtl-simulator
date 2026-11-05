(§3.b member-width-genvar · OPEN · in Parser accept) (a struct member width over a genvar in a generate-block typedef):
* symptom: a struct member width over a genvar in a generate-block typedef (`logic [g+1:0] x;`, reached through nested anonymous members in §4.5.604's review) is E2002 `struct member width must be a named integer type or a constant-literal range in v1`
* oracles: iverilog, sv2v → iverilog and verilator `G0 x=11 bits=3` / `G1 x=111 bits=4`
* status: OPEN
