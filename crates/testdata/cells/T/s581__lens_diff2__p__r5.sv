module top;
  localparam logic [31:0] KA = 32'hFFFFFFFF;
  if (1) begin : gb
`include "r5a.svh"
`include "r5b.svh"
    localparam logic [31:0] KB = 32'hFFFFFFFF;
  end
endmodule
