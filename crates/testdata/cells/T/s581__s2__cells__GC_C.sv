`timescale 1ns/1ns
module t;
  case (1) (4'b1100 == 4'b1100): begin : g initial #1 $display("GC_C item"); end default: begin : h initial #1 $display("GC_C default"); end endcase
endmodule
