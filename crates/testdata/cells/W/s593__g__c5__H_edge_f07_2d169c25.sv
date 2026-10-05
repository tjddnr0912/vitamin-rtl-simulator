package p; parameter int W = 4; parameter logic [3:0] D = 4'hC; endpackage
module m import p::*; #(parameter logic [W-1:0] X = D) (input logic [W-1:0] a); initial $display("DIGEST=%h %h", X, a); endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #1 $finish; end endmodule
