`timescale 1ns/1ns
module t;
  case ((4'b1100 ==? 4'b1?00)) 1'b1: begin : g initial #1 $display("GS_Q one"); end default: begin : h initial #1 $display("GS_Q default"); end endcase
endmodule
