(§2 reimported-routine-binding · OPEN · in Scoping / imports / block-locals) (a routine imported into another package never binds in its own package):
* A routine imported into another package binds in the importer or the caller, never in its own package: `elaborate_package`'s import arm files `r::g` in `pkg_funcs[p]`, which `pkg_owns` does not own, so §4.5.589's window stays closed: a call to it from `p`'s routine text keeps the caller's binding (n_pkgimp `P=232 v=232`, pi_rfn_hdr `232`; both oracles `8`), its body run from `p`'s body reads `p`'s constants (c12 `L=9 v=9`; iverilog, verilator, sv2v `L=3 v=3`), and its default is evaluated with `p`'s (ifn_dflt `v=3`, kx_tdflt_imp `u=3`; iverilog `8`)
* record the origin at the import arm (`imp.pkg`) and key the window, the interpreter's package tag and the default lane on it
* oracles: 2 oracles
* status: OPEN (§5.2 row 2, shipping with row 45)
