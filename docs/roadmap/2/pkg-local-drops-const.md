(§2 pkg-local-drops-const · OPEN · in Scoping / imports / block-locals) (a package routine's block-local named like a package constant hides it):
* A package routine's block-local named like one of its package's constants drops that constant for the whole body (`pkg_body_scope::rtn_declared_names` is routine-wide, the skip set of `push_pkg_consts_task`), so the caller's same-named constant answers outside the block (pk16_ctl `c=77`, pk24_ctl `b=2`; iverilog, sv2v → iverilog and verilator `5`)
* 3 oracles
* OPEN
