package q; parameter int QW = 5; endpackage
package p; import q::*; typedef logic [QW-1:0] t; endpackage
module m #(parameter p::t X = 5'h1b) (); initial $display("DIGEST=%h %0d", X, $bits(X)); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
