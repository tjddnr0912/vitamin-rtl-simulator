`timescale 1ns/1ns
module t;
  logic ad_S07 [(4'b1100 ==? {2'b1?, 2'b00}):0];
  initial begin #1 $display("S07A %0d", $size(ad_S07)); #1 $finish; end
  initial #100 $finish;
endmodule
