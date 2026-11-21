(§2 fork-declarations-flatten · OPEN · in Scoping / imports / block-locals) (a `fork`'s own declarations flatten onto same-named module variables):
* A `fork`'s own declarations (`fork logic [7:0] x; … join`) take the bare-name flatten with W3056 (`stmt_main.rs` warns them ignored, `gather_auto_block_locals` admits no fork span), so beside a module `x` the fork writes the module's (fl_ctl `inner=000000aa outer=000000aa`, fl_bits run-time `$bits` `r=01`; iverilog, sv2v → iverilog and verilator `inner=aa outer=11111111`, `ff`)
* under a timing control the same since §4.5.611 (fl_d)
* 3 oracles
* OPEN
* note (2026-11-20): the class is §5.2 Q-SCOPEBIND's (a name is bound to a declaration the source's scopes do not give it), which absorbs this entry's root (door (1), the block-local flatten); the fix is that row's plan
