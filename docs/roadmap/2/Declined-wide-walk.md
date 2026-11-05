(§2 Declined wide walk · OPEN · in Constant domain (i64)) (a cast inside a wide expression keeps the unlimited i64 answer):
* `int'(NM) ^ 64'h0` keeps the unlimited i64 answer
* `fold_region` arms per consumer (§4.5.562's design; §5.2 Q-PRIMCAST: the core has no prim-cast arm, so `byte'(N200) + 16'd0` keeps the walk's `ffc8` where iverilog, sv2v and verilator give `00c8`, row 48)
* 2 oracles
* OPEN
