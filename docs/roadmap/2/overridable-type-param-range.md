(§2 overridable-type-param-range · BLOCKED · in Ranges / bounds / selects) (an overridable 1-D type parameter registers a stand-in range):
* An overridable 1-D `parameter type T = logic [8:1]` registers `[T$w-1:0]` (§4.5.514's carrier)
* witness (W/parse1b/premise/cells2/j3_tp_lsb1_dflt): `module m #(parameter type T = logic [8:1]) ();` `T x;` `x = 8'b0000_0010;` printing `x[1]`, `x[2]`, `$right(x)`, `$left(x)`: PRE 4d1d84bb rc 0 `A x1=1 x2=0 r=0 l=7`
  * oracles: iverilog and verilator `A x1=0 x2=1 r=1 l=8`
* witness (j3_tp_lsb1_ovr): the default `logic [7:0]` overridden as `#(.T(logic [8:1]))`: PRE rc 0 `A x1=1 x2=0 r=0 l=7`, iverilog and verilator `A x1=0 x2=1 r=1 l=8`
* witness (W/r68/plan/m2/cells/tps_ptype_lsb): `module c #(parameter int W = 4, parameter type T = logic [W+3:W]) ();` `T x;` `x = '0; x[W+2] = 1'b1;`: PRE rc 0 `V b=4 lo=0 hi=3 x=0000`
  * oracles: iverilog and verilator `V b=4 lo=4 hi=7 x=0100`
* note: its `localparam type` twin tps_hdr_lsb, a non-overridable type parameter, is §2 type-param-standin's
* 3 oracles
* status: BLOCKED (§5.2 Q-TPCARRIER, parked 2026-10-29 behind Q-WITHREAL, Q-PARAM2STATE, Q-LETCAST and Q-2STINVSEL)
