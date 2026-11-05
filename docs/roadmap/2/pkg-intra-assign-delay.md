(§2 pkg-intra-assign-delay · OPEN · in Scoping / imports / block-locals) (a package routine's intra-assignment delay binds the caller's names):
* A package routine's intra-assignment delay or event (`o = #(k(1)) 7;`, `g <= #(k(1)) 7;`) is not walked by `collect_callee_stmt`, so `k` binds the caller's (g04 `r=0`, iverilog and verilator `r=7`; with no module `k` E3010)
* 2 oracles
* OPEN
