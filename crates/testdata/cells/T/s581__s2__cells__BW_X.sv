`timescale 1ns/1ns
module t;
  localparam [64:0] L = {64'd0, (4'bx100 ==? 4'b1?00)};
  case (1) L: begin : g initial #1 $display("BW_X item"); end default: begin : h initial #1 $display("BW_X default"); end endcase
endmodule
