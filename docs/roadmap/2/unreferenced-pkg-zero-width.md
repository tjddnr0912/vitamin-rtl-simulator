(§2 unreferenced-pkg-zero-width · OPEN · in Ranges / bounds / selects) (a zero indexed width in an unreferenced package's routine runs):
* A zero indexed width in a routine of a package nothing references runs, because vita elaborates only a referenced package's routines (z.pkg_fn_unc, y.pkg_fn_lit, y.pkg_task_unc: `v=a5`)
* iverilog and verilator refuse, sv2v drops the package (an imported or called one is refused, §4.5.601)
* 2 oracles
* OPEN
