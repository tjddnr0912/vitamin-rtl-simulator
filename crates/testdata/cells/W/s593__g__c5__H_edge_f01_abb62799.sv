package p; parameter logic [7:0] Dflt = 8'hA5; endpackage
module m import p::*; #(parameter logic [7:0] X = Dflt) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
