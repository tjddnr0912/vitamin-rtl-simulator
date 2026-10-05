package p; parameter int N = 3; endpackage
module m import p::*; #(parameter int K = N) (); logic [K-1:0] v; genvar i; generate for (i = 0; i < K; i++) begin : g assign v[i] = i[0]; end endgenerate initial begin #1 $display("DIGEST=%b", v); end endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
