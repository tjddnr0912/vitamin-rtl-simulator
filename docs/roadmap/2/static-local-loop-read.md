(§2 static-local-loop-read · in Oracle splits (recorded, not chased)) (a static function's local read before its write in a loop, across calls):
* a static function's local read before it is written in a loop body, across calls at time 0 (row-25 lens r2 d04 `g[2*i +: 2] = t; t = a[2*i +: 2];`, frame lane) — vita (PRE, §4.5.615) and verilator `z=01110010` then `z=10001101`, iverilog and sv2v → iverilog `z=011100xx` then `z=10001110` (the call count of a static function at time 0)
* split
