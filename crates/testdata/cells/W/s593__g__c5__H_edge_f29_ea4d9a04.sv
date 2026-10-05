package p; parameter string S = "hi"; endpackage
module m import p::*; #(parameter string X = S) (); initial $display("DIGEST=%s", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
