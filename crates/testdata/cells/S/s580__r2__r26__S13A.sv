`timescale 1ns/1ns
module t;
  logic ad_S13 [(4'b1100 inside {'x}):0];
  initial begin #1 $display("S13A %0d", $size(ad_S13)); #1 $finish; end
  initial #100 $finish;
endmodule
