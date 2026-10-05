`timescale 1ns/1ns
module t;
  localparam [64:0] L = {64'd0, (4'b1100 inside {4'b1?00})};
  case (1) L: begin : g initial #1 $display("BW_I item"); end default: begin : h initial #1 $display("BW_I default"); end endcase
endmodule
