(§3.b cg-block-local · DEFERRED · in Parser accept) (a covergroup block local declared with an initializer is rejected):
* symptom: a covergroup-typed block-local declaration with an initializer (`cg_t cg = new();`, EL2 `el2_veer_lockstep_cov_if.sv:317`) is E2002 (`expected '=' or '<=' after lvalue`)
* oracles: verilator `ok`
* oracles: iverilog and sv2v cannot parse the covergroup: verilator + hand-IEEE (clause 19)
* witness: row-9 O2
* status: DEFERRED (§7 row 55)
