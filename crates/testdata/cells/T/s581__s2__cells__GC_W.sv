`timescale 1ns/1ns
module t;
  case (1) ((4'd15 + 4'd1) ==? 5'b1?000): begin : g initial #1 $display("GC_W item"); end default: begin : h initial #1 $display("GC_W default"); end endcase
endmodule
