package p; parameter int Dflt = 37; endpackage
import p::Dflt;
module m #(parameter int Dflt = 5, parameter int X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
