module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a initial $display("@inner"); end
    else begin : b initial $display("@outer"); end
`include "DM10_inc.svh"
  end
  initial #10 $finish;
endmodule
