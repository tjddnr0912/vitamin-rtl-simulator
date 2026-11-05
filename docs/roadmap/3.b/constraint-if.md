(§3.b constraint-if · DEFERRED · in Constrained random) (`if` / `else` inside a class constraint block is rejected):
* symptom: `if` / `else` inside a class constraint block is E2002 (`expected expression, found keyword 'if'`; EL2 `tb_top_pkg.sv:26`, a class the tb never constructs)
* each arm as an implication predicate (`!c || a`, `c || b`) in the existing constraint compiler
* oracles: verilator parses it but runs no solver here, iverilog and sv2v refuse: hand-IEEE (§18.5.7)
* witness: row-9 O4
* status: DEFERRED (§7 row 57)
