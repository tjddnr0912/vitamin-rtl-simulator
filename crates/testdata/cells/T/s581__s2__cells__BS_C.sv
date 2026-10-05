`timescale 1ns/1ns
module t;
  localparam L = (4'b1100 == 4'b1100);
  case (L) 1'b1: begin : g initial #1 $display("BS_C one"); end default: begin : h initial #1 $display("BS_C default"); end endcase
endmodule
