package p; typedef logic [7:0] perm_t; parameter int Dflt = 300; endpackage
module m import p::*; #(parameter type T = perm_t, parameter T X = T'(Dflt)) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
