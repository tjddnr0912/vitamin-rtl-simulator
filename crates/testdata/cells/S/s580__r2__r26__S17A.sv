`timescale 1ns/1ns
module t;
  logic ad_S17 [(4'b1100 ==? 4'(4'b1?00)):0];
  initial begin #1 $display("S17A %0d", $size(ad_S17)); #1 $finish; end
  initial #100 $finish;
endmodule
