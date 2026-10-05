`timescale 1ns/1ns
module t;
  logic ad_S14 [(4'bx100 ==? 4'b1?00):0];
  initial begin #1 $display("S14A %0d", $size(ad_S14)); #1 $finish; end
  initial #100 $finish;
endmodule
