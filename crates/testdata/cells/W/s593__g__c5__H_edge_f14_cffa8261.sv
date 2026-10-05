package p; typedef enum logic [1:0] {E0, E1, E2} e_t; parameter e_t Dflt = E2; endpackage
module m import p::*; #(parameter e_t X = Dflt) (); initial $display("DIGEST=%0d %s", X, X.name()); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
