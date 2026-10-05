`timescale 1ns/1ns
module t;
  logic ad_S02 [(4'b1100 inside {4'sb1?00}):0];
  initial begin #1 $display("S02A %0d", $size(ad_S02)); #1 $finish; end
  initial #100 $finish;
endmodule
