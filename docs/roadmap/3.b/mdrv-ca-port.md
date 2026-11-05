(§3.b mdrv-ca-port · OPEN · in Constants / parameters) (port, generate-block and interface drivers escape the multidriver rule):
* Rule D is module-scope lvalues only: an instance output bound to the variable (`m u (.o(y)); assign y = a;`), a generate-block `assign` (`if (1) begin : g assign y = ~a; end`) and an interface body with two `assign`s each print `y=x`
* oracles: iverilog "Variable 'y' cannot have multiple drivers.", verilator MULTIDRIVEN
* a UDP instance output (`u_and g (y, a, b); assign y = c;`, a `ModuleItem::Instance`, while a built-in gate is a desugared `ContAssign` Rule D counts) prints `y=x` (iverilog "Variable 'y' cannot have multiple drivers.", verilator `y=1`, sv2v cannot parse `primitive`)
* count port bindings and per-scope drivers on the elaborated net
* oracles: 2 oracles
* status: OPEN
