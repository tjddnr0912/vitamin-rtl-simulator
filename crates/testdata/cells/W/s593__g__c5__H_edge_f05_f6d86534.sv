package p; parameter int Dflt = 37; endpackage
module m import p::*; #(parameter X = Dflt) (); initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); m #(.X(5)) v(); initial begin #1 $finish; end endmodule
