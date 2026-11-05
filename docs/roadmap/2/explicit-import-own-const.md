(§2 explicit-import-own-const · OPEN · in Scoping / imports / block-locals) (an explicitly imported function's range naming its package constant is E3009):
* An explicit `import p::f;` whose formal or local range names `p`'s own constant, called from a module that does not declare that name, is E3009 `undefined name W is not allowed in a constant range bound [in top]` (row-25 lens r2 p10 select-write body, its whole-write twin p10_twin_whole: iverilog, sv2v → iverilog and verilator `y=cf`; PRE, §4.5.615 alike)
* the wildcard `import p::*;` (p11) runs right (`y=cf`)
* the range folds where the call is and the declaration window corrects only a name the caller also resolves (`decl_scope.rs` corrections-only split)
* 3 oracles
* OPEN
