package p; parameter int Dflt = 37; parameter int Other = 9; endpackage
module m import p::Dflt; #(parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
