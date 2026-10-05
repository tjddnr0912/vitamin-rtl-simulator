package p; parameter int Dflt = 37; endpackage
import p::*;
module m #(parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
