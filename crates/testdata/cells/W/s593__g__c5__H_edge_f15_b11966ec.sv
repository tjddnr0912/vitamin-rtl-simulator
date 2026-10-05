package p; parameter int Dflt = 37; endpackage
module c import p::*; #(parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module m import p::*; #(parameter Y = Dflt + 1) (); c u1(); c #(.X(Y)) u2(); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
