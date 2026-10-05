package p; typedef logic [11:0] perm_t; parameter int N = 20; endpackage
module m import p::*; #(parameter int A = $bits(perm_t), parameter int B = $clog2(N)) (); initial $display("DIGEST=%0d %0d", A, B); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
