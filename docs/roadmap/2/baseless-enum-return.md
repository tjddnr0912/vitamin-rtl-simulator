(§2 baseless-enum-return · OPEN · in Size cast / signedness) (a function returning a base-less enum reads its unset return as x):
* A function returning a base-less enum reads its never-assigned return variable as x at run time (d07rt `repeat (fr(a2))`, `if (fr == 4'd0) fr = B;`: `n=0`, iverilog and verilator `n=2`; the constant fold e07 `n=2` is right; d11, the same return beside a never-assigned `logic [3:0]` local, folds `n=2` and runs `n=0`)
* the parser records the return 2-state (`functask.rs:201` `ret_two_state`) and a base-less enum local is right (d09rt `n=1`)
* the run-time site is not found
* prerequisite of §2 🆕 AE's `repeat` cut
* iverilog + hand-IEEE (§6.19: the base is `int`)
* status: OPEN (§5.2 row 8)
