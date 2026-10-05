`timescale 1ns/1ns
module t;
  case ((4'b1100 == 4'b1100)) 1'b1: begin : g initial #1 $display("GS_C one"); end default: begin : h initial #1 $display("GS_C default"); end endcase
endmodule
