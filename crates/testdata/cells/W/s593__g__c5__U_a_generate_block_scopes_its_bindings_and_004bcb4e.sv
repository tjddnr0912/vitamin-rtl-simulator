package p; localparam int K = 7; endpackage
module tb; import p::*;
  generate if (1) begin : g import p::K; initial $display("DIGEST=%0d", K); end endgenerate
  initial begin #1 $finish; end
endmodule