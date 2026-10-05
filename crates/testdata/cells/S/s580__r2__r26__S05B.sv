`timescale 1ns/1ns
module t;
  logic [(4'b1100 inside {{2'b1?, 2'b00}}):0] wb_S05;
  initial begin #1 $display("S05B %0d", $bits(wb_S05)); #1 $finish; end
  initial #100 $finish;
endmodule
