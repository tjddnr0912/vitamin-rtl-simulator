`timescale 1ns/1ns
module t;
  logic a [(4'bx100 ==? 4'b1?00) : 0];
  initial #1 $display("AD_X %0d", $size(a));
endmodule
