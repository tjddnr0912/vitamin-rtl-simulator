`timescale 1ns/1ns
module t;
  logic [(4'b1100 ==? {2'b1?, 2'b00}):0] wb_S07;
  initial begin #1 $display("S07B %0d", $bits(wb_S07)); #1 $finish; end
  initial #100 $finish;
endmodule
