package p; parameter real R = 2.5; endpackage
module m import p::*; #(parameter real X = R) (); initial $display("DIGEST=%f", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
