package p; parameter int Dflt = 37; endpackage
module m #(parameter X = Dflt) (); import p::*; initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
