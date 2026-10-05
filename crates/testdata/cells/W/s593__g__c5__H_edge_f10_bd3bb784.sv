package p; parameter int A = 3; endpackage
package q; parameter int B = 4; endpackage
module m import p::*, q::B; #(parameter X = A + B) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
