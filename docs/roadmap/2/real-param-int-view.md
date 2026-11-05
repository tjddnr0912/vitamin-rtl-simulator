(§2 real-param-int-view · OPEN · in Real) (a real parameter's integer view answers in the integer domain):
* A real parameter's integer view answers `R/4 > 1` in the integer domain (`params` before `real_param_val`; an exact-integer `localparam real N = 2` is bound in both, `instance.rs:718`, `generate.rs:756`: x.exact_real_lpval `localparam int Q = (N / 4 > 0) ? 2 : 0` gives `Q=0`, all three oracles `Q=2`)
* 2 oracles
* OPEN
