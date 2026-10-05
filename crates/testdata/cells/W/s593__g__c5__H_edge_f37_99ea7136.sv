package p; parameter int W = 4; endpackage
interface i #(parameter int N = 4) (); import p::*; localparam int M = W + N; logic [M-1:0] d; endinterface
module tb; i u(); initial begin u.d = 8'hb3; #1 $display("DIGEST=%h %0d", u.d, u.M); $finish; end endmodule
