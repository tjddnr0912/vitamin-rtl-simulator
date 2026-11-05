(§2 type-param-standin · BLOCKED · in Ranges / bounds / selects) (a non-overridable type parameter keeps the stand-in range when unfolded):
* A non-overridable type parameter whose bound does not fold at parse keeps the `[T$w-1:0]` stand-in (`hdl-parser/src/pkg_type_param.rs` `type_param_literal_range` falls back): `module t #(parameter int N = 8); localparam type L = logic [0:N-1]; localparam L VL = 8'h1D;` reads VL[0..7] `10111000` where iverilog = sv2v = verilator `00011101` (the twin `logic [0:N-1]` right)
* a body `parameter type` under a header the same (W/qtpcarrier/plan/c/p5a, p5b: PRE and Q-TPCARRIER's scratch alike)
* 3 oracles
* status: BLOCKED (§5.2 Q-TPCARRIER)
