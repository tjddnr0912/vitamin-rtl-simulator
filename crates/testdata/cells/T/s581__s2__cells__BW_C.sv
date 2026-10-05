`timescale 1ns/1ns
module t;
  localparam [64:0] L = {64'd0, (4'b1100 == 4'b1100)};
  case (1) L: begin : g initial #1 $display("BW_C item"); end default: begin : h initial #1 $display("BW_C default"); end endcase
endmodule
