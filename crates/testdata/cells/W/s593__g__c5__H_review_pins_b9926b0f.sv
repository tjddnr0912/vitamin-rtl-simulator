package q; parameter int K = 5; endpackage
package p; parameter int K = 9; import q::*; parameter int USE = K; endpackage
module tb; initial begin $display("DIGEST=%0d", p::USE); #1 $finish; end endmodule
