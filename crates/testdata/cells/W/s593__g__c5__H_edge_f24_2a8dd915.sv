package p; parameter int D = 3; endpackage
package q; parameter int D = 4; endpackage
module m import p::*; import q::*; #(parameter X = D) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
