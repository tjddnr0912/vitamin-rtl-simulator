module I0;
  if (1) begin : gb
`include "inc_b.svh"
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
