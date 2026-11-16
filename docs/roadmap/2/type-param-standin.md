(§2 type-param-standin · BLOCKED · in Ranges / bounds / selects) (a non-overridable type parameter keeps the stand-in range when unfolded):
* A non-overridable type parameter whose bound does not fold at parse keeps the `[T$w-1:0]` stand-in (`hdl-parser/src/pkg_type_param.rs` `type_param_literal_range` falls back): `module t #(parameter int N = 8); localparam type L = logic [0:N-1]; localparam L VL = 8'h1D;` reads VL[0..7] `10111000` where iverilog = sv2v = verilator `00011101` (the twin `logic [0:N-1]` right)
* a body `parameter type` under a header the same (W/qtpcarrier/plan/c/p5a, p5b: PRE and Q-TPCARRIER's scratch alike)
* witness (W/parse1b/premise/cells3/j3_ltp_standin, the spelling above, `$display("A v0=%b v7=%b v=%b", VL[0], VL[7], VL)`): PRE 4d1d84bb rc 0 `A v0=1 v7=0 v=00011101`
  * oracles: iverilog and verilator `A v0=0 v7=1 v=00011101`; the twin `localparam logic [0:N-1] VL = 8'h1D;` (j3_ltp_twin) `A v0=0 v7=1 v=00011101` on all three
* witness (W/r68/plan/m2/cells/tps_hdr_lsb): `module top #(parameter int W = 4);` `localparam type T = logic [W+3:W];` `T x;` `x = '0; x[W+2] = 1'b1;`: PRE rc 0 `V b=4 lo=0 hi=3 x=0000`
  * oracles: iverilog and verilator `V b=4 lo=4 hi=7 x=0100`; the typedef twin `typedef logic [W+3:W] T;` (tps_hdr_lsb_td) `V b=4 lo=4 hi=7 x=0100` on all three
* 3 oracles
* status: BLOCKED (§5.2 Q-TPCARRIER)
