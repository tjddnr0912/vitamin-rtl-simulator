`timescale 1ns/1ns
module t;
  case ((4'b1100 inside {4'b1?00})) 1'b1: begin : g initial #1 $display("GS_I one"); end default: begin : h initial #1 $display("GS_I default"); end endcase
endmodule
