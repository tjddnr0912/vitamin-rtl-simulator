package p; parameter int Dflt = 37; parameter int R[2] = '{1,2}; endpackage
module m import p::*; #(parameter int A[2] = R, parameter X = Dflt) (); initial $display("DIGEST=%0d %0d %0d", A[0], A[1], X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
