`timescale 1ns/1ns
module t;
  localparam L = (4'b1100 ==? 4'b1?00);
  case (1) L: begin : g initial #1 $display("BL_Q item"); end default: begin : h initial #1 $display("BL_Q default"); end endcase
endmodule
