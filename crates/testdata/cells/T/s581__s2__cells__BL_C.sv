`timescale 1ns/1ns
module t;
  localparam L = (4'b1100 == 4'b1100);
  case (1) L: begin : g initial #1 $display("BL_C item"); end default: begin : h initial #1 $display("BL_C default"); end endcase
endmodule
