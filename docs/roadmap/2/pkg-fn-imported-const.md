(§2 pkg-fn-imported-const · OPEN · in Scoping / imports / block-locals) (a package function reading an imported constant binds it in the caller):
* A package's own function reading a constant its package imported binds the name in the caller (row 39's review, p07c: PRE `f=2`, iverilog and sv2v → iverilog `f=7`; an exported constant reaches it the same way, p07)
* 2 oracles
* OPEN
