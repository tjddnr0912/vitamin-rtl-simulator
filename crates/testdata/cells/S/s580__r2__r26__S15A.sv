`timescale 1ns/1ns
module t;
  logic ad_S15 [(4'b1100 inside {(4'b1?00)}):0];
  initial begin #1 $display("S15A %0d", $size(ad_S15)); #1 $finish; end
  initial #100 $finish;
endmodule
