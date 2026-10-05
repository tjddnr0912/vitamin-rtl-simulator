`timescale 1ns/1ns
module t;
  if (1) begin : b
    localparam [64:0] L = {64'd0, (4'b1100 == 4'b1100)};
    case (1) L: begin : g initial #1 $display("BG_C item"); end default: begin : h initial #1 $display("BG_C default"); end endcase
  end
endmodule
