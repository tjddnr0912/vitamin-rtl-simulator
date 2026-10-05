package p; function automatic int dbl(int a); return a*2; endfunction parameter int W = 4; endpackage
interface i import p::*; #(parameter int N = W) (); logic [N-1:0] d; initial begin d = dbl(3); end endinterface
module tb; i u(); initial begin #1 $display("DIGEST=%h", u.d); $finish; end endmodule
