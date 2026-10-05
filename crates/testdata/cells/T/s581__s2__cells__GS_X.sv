`timescale 1ns/1ns
module t;
  case ((4'bx100 ==? 4'b1?00)) 1'b1: begin : g initial #1 $display("GS_X one"); end default: begin : h initial #1 $display("GS_X default"); end endcase
endmodule
