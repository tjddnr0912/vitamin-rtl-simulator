package q; parameter int K = 5; endpackage
package p; import q::K; parameter int K = 9; parameter int USE = K; endpackage
module tb; initial begin $display("DIGEST=%0d", p::USE); #1 $finish; end endmodule
