(§2 deferred-assert-block · OPEN · in Scoping / imports / block-locals) (a declaring block in a deferred assertion's action is not hoisted):
* A declaring block in a procedural deferred assertion's action (`assert #0 (c) else begin logic [7:0] x; … end`) is accepted and not hoisted (`block_local::for_each_nested_stmt` does not enter assertion actions): its writes land on the module's `x` (x_dassert_ctl `inner=000000aa outer=000000aa`; verilator `inner=aa outer=11111111`; IEEE 1800-2017 §16.4 allows each action of a deferred assertion one subroutine call; iverilog rejects `assert #0`, sv2v drops it)
* oracles: verilator + hand-IEEE
* status: OPEN
