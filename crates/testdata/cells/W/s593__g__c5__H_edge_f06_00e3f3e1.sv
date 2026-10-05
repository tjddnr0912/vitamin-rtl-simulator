package p; parameter int W = 4; endpackage
module m import p::*; #(parameter int X = W*2+1, parameter logic [W-1:0] Y = '1) (); initial $display("DIGEST=%0d %h %0d", X, Y, $bits(Y)); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
