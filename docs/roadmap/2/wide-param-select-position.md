(§2 wide-param-select-position · BLOCKED · in Ranges / bounds / selects) (a bare >64-bit non-zero-LSB parameter select reads positionally):
* A bare >64-bit non-zero-LSB parameter select reads positionally (`P[79:72]`): a part-select width, an indexed width, a replication count or a `repeat` count over one is E3009 since §4.5.601
* a value read (`r = P[15:8]` over `[79:8] P = {8'd5, 56'd0, 8'd3}`, `r=0`; row 48: silent at 13 of 22 consumers, §5.2 row 51), a localparam (`L = P[15:8]`, `0`) and a generate condition (`P[15:8] == 3`, false) still read by position (iverilog, sv2v, verilator `3`, `3`, true)
* status: BLOCKED (declaring-scope fold, §4.5.560)
