package p; parameter int W = 4; parameter logic [W-1:0] Mask = '1; localparam int H = W/2; endpackage
module m import p::*; #(parameter logic [W-1:0] X = Mask, parameter int Y = H) (); initial $display("DIGEST=%h %0d", X, Y); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
