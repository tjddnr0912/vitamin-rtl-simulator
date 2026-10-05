`timescale 1ns/1ns
module t;
  logic ad_S12 [(4'b0100 inside {4'b1?00}):0];
  initial begin #1 $display("S12A %0d", $size(ad_S12)); #1 $finish; end
  initial #100 $finish;
endmodule
