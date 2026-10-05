package p; parameter int W = 4; endpackage
module m import p::*; (input logic [W-1:0] a); initial $display("DIGEST=%h", a); endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #1 $finish; end endmodule
