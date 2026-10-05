`timescale 1ns/1ns
module t;
  logic ad_S08 [(4'bx100 inside {4'b1?00}):0];
  initial begin #1 $display("S08A %0d", $size(ad_S08)); #1 $finish; end
  initial #100 $finish;
endmodule
