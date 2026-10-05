package p; typedef logic [7:0] perm_t; parameter perm_t Dflt = 8'h3C; endpackage
module m import p::*; #(parameter perm_t X = Dflt) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
