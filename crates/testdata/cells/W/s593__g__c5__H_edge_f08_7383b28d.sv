package p; parameter int Dflt = 37; endpackage
module m #(parameter X = 1) (); import p::*; localparam Y = Dflt; initial $display("DIGEST=%0d %0d", X, Y); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
