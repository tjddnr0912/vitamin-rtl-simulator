`timescale 1ns/1ns
module t;
  logic a [(4'b1100 == 4'b1100) : 0];
  initial #1 $display("AD_C %0d", $size(a));
endmodule
