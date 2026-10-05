package p; function automatic int dbl(int a); return a*2; endfunction parameter int W = 5; endpackage
module m import p::*; #(parameter int X = dbl(W)) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
