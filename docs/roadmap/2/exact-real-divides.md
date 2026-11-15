(§2 exact-real-divides · OPEN · in Real) (an exact-integer real parameter divides as an integer):
* An exact-integer real parameter divides as an integer (`int'(R / 2)`)
* witness (rows 68 + 23 premise judge 1, W/r68/premise/b1/cells/e1_real_div_int): `localparam real R = 5;` `localparam int I = R / 2;` and a struct member `[I:0]`: PRE 4d1d84bb rc 0 `A b=3 I=2`
  * oracles: iverilog and verilator `A b=4 I=3`
* 2 oracles
* OPEN
