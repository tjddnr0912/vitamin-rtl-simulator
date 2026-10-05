`timescale 1ns/1ns
module t;
  logic ad_S05 [(4'b1100 inside {{2'b1?, 2'b00}}):0];
  initial begin #1 $display("S05A %0d", $size(ad_S05)); #1 $finish; end
  initial #100 $finish;
endmodule
