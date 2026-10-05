`timescale 1ns/1ns
module t;
  case (1) (4'b1100 ==? 4'b1?00): begin : g initial #1 $display("GC_Q item"); end default: begin : h initial #1 $display("GC_Q default"); end endcase
endmodule
