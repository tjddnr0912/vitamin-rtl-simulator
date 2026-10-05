package p; typedef logic [7:0] perm_t; parameter perm_t Dflt = 8'h3C; endpackage
module m import p::*; #(parameter type T = perm_t, parameter T X = Dflt) (); initial $display("DIGEST=%h %0d", X, $bits(T)); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
