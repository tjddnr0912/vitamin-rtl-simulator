package p; typedef enum logic [1:0] {E0, E1, E2} e_t; endpackage
module m import p::*; #(parameter e_t X = E1, parameter int Y = E2) (); initial $display("DIGEST=%0d %0d", X, Y); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
