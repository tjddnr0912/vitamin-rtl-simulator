`timescale 1ns/1ns
module t;
  case (1'b1) (4'b1100 inside {{2'b1?, 2'b00}}): begin : gc1 initial #1 $display("X4 gencase item"); end default: begin : gc2 initial #1 $display("X4 gencase default"); end endcase
  case (1'b1) (4'b1100 ==? {2'b1?, 2'b00}): begin : gq1 initial #1 $display("X4q gencase item"); end default: begin : gq2 initial #1 $display("X4q gencase default"); end endcase
  initial #2 $finish;
endmodule
