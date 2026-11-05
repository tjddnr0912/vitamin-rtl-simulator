(§3.b pattern-repl · OPEN · in Parser accept) (a replication `N{…}` inside an assignment pattern is rejected):
* symptom: a replication inside an assignment pattern (`'{NumRegs{0}}`, a header array parameter's default) is E2002 (`expected '}' closing an assignment pattern, found '{'`)
* the pattern parser takes `N{…}` (§10.9.1) and elaborate expands it by the folded count
* ⑤ⓒ's sibling
* oracles: verilator and sv2v → iverilog `A o=010101` (iverilog syntax error)
* witness: row-9 R06, seven OpenTitan pages
* status: OPEN (§5.2 row 35)
