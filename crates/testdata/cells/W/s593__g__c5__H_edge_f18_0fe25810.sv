package p; parameter int W = 4; endpackage
interface i import p::*; #(parameter int N = W) (); logic [N-1:0] d; endinterface
module tb; i u(); initial begin u.d = 4'hb; #1 $display("DIGEST=%h %0d", u.d, u.N); $finish; end endmodule
