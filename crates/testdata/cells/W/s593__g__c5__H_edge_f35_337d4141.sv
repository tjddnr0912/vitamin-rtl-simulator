package p; parameter int Dflt = 37; parameter int Alt = 9; endpackage
module c #(parameter X = 1) (); initial $display("DIGEST=%0d", X); endmodule
module m import p::*; #(parameter Y = Dflt) (); c #(.X(Alt)) u1(); c #(.X(Y)) u2(); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
