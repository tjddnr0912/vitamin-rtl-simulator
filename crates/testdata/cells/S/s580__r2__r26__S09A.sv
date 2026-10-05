`timescale 1ns/1ns
module t;
  logic ad_S09 [(4'b1100 == 4'b1?00):0];
  initial begin #1 $display("S09A %0d", $size(ad_S09)); #1 $finish; end
  initial #100 $finish;
endmodule
