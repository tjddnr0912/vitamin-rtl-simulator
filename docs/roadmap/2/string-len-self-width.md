(§2 string-len-self-width · OPEN · in Constant domain (i64)) (`S.len()` has no self width in a constant region):
* `S.len()` has no self width in a constant region (`const_self_width` `K::Call`: no `const_fn_def` for a two-segment call), so a signed neighbour folds sign-extended: Oslen_o_s `((X | 8'd0) + S.len()) == 32'd254` over `localparam string S = "ab"` `L=0`, `GI=else`, `vb=4`
* iverilog and verilator `L=1`, `then`, `vb=5` (sv2v refuses)
* the `+ 32'sd2` twin is right (§4.5.594)
* 2 oracles
* OPEN
