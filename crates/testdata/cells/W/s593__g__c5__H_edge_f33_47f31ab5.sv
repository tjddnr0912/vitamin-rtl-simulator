package p; parameter int Dflt = 37; endpackage
module m import p::*; #(parameter X = Dflt) (a); input a; initial $display("DIGEST=%0d %b", X, a); endmodule
module tb; m u(1'b1); initial begin #1 $finish; end endmodule
