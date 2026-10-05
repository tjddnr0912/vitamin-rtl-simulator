package p; parameter int Dflt = 37; endpackage
module m import p::*; #(parameter Dflt = 5, parameter X = Dflt) (); initial $display("DIGEST=%0d %0d", Dflt, X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
