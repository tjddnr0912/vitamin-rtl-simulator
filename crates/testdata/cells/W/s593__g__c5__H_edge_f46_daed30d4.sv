package p; parameter int W = 4; typedef logic [W-1:0] t; typedef logic [W-1:0][1:0] t2; endpackage
module m; p::t v; p::t2 w; initial begin v = '1; w = '1; $display("DIGEST=%h %0d %0d", v, $bits(v), $bits(w)); end endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
