(§3.b mdrv-ca-hier · OPEN · in Constants / parameters) (a hierarchical continuous `assign` to a variable is never a recorded driver):
* a hierarchical continuous `assign` to a variable is never a recorded driver: its lvalue is a deferred placeholder when `cont_var.rs` records (`cont_var_kind` false) and Rule D reads module-scope names, so `assign u.r = a;` beside the child's `initial r = 8'h22;` (p24) and an upward `assign top.r = a;` beside `initial r = …` (p25) print `r=22` at exit 0
* iverilog "Variable 'r' cannot be driven by a continuous assignment/module or continuous assignment with non-default strength." / "Cannot perform procedural assignment to variable 'r' because it is also continuously assigned."
* record the row after `resolve_deferred_hier_write`
* oracles: iverilog
* status: OPEN
