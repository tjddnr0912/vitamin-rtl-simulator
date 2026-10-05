package p; parameter int W = 4; endpackage
interface i #(parameter int N = W) (); import p::*; logic [N-1:0] d; endinterface
module tb; i u(); initial begin u.d = 4'hb; #1 $display("DIGEST=%h", u.d); $finish; end endmodule
