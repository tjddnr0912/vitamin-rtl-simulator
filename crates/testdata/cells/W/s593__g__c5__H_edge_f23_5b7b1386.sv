package p; parameter int Dflt = 37; endpackage
module m import p::Dflt; #(parameter Dflt = 5, parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
