package p; parameter int Dflt = 37; logic [7:0] V = 8'h5A; endpackage
module m import p::*; #(parameter X = Dflt) (); initial $display("DIGEST=%0d %h", X, V); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
