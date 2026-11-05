(§2 pkg-task-sibling-call · OPEN · in Scoping / imports / block-locals) (a package task's call of a sibling package task is not injected with it):
* A package task's call of a sibling package TASK is not injected with it: `import p::a;` where `a` calls `b` is E3010 `call to undeclared task b` (cc8), and beside a module task `b` it runs the module's (cc1, cc9 `b=mod`; iverilog, sv2v → iverilog and verilator `b=pkg`)
* from a header-less `always` the same is tr10 (E3010) and tr38 (one pass, W3056, no output), iverilog = sv2v = verilator `T 5 1 / T 10 2 / T 15 3` on both (W/qalwintra/r2/lens-diff/cells/d5)
* a sibling function is injected, also from a timing control's body since §4.5.613
* 3 oracles
* OPEN
