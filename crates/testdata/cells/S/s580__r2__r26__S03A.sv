`timescale 1ns/1ns
module t;
  logic ad_S03 [(4'b1100 inside {'b1?00}):0];
  initial begin #1 $display("S03A %0d", $size(ad_S03)); #1 $finish; end
  initial #100 $finish;
endmodule
