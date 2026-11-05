(§2 pkg-task-const-nested · OPEN · in Scoping / imports / block-locals) (a package task's constants in nested ranges and events bind the importer's):
* A package task's own constant read in a nested block's declaration range (pk32_none `b=2`) or in an event expression (pk39_none `@(pv[C])` `a=4`), and a constant its package imports (pk30_none `b=2`), bind the importing module's same-named constant
* iverilog, sv2v → iverilog and verilator `b=5`, `a=2`, `b=5`
* 3 oracles
* OPEN
