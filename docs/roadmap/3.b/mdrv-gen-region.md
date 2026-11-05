(§3.b mdrv-gen-region · OPEN · in Constants / parameters) (a bare `generate` region is skipped by the multidriver check):
* a bare `generate … endgenerate` region is skipped like a generate block by Rules A, B and D, though IEEE 1800 §27.3 gives the region no scope: `generate assign y = 4'h5; endgenerate` + `initial y = 4'h0;` prints `y=0` (iverilog "Cannot perform procedural assignment …", verilator `%Error-CONTASSINIT`), `generate assign y = 4'h1; endgenerate` + `assign y = 4'h2;` prints `y=X` (iverilog "Variable 'y' cannot have multiple drivers.")
* walk the region's items as module items in `check_multidriver_processes`
* oracles: 2 oracles
* status: OPEN
