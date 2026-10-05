`timescale 1ns/1ns
module t;
  logic ad_S04 [(4'b1100 inside {4'b0000, 4'b1?00}):0];
  initial begin #1 $display("S04A %0d", $size(ad_S04)); #1 $finish; end
  initial #100 $finish;
endmodule
