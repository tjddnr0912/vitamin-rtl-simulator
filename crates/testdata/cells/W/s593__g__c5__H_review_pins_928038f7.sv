package q; parameter int W = 3; endpackage
package p; import q::W; parameter int W = 5; endpackage
module tb; initial begin $display("DIGEST=%0d", p::W); #1 $finish; end endmodule
