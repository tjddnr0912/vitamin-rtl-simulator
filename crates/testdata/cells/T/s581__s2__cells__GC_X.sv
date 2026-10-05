`timescale 1ns/1ns
module t;
  case (1) (4'bx100 ==? 4'b1?00): begin : g initial #1 $display("GC_X item"); end default: begin : h initial #1 $display("GC_X default"); end endcase
endmodule
