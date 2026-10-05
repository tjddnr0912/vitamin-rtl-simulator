`timescale 1ns/1ns
module t;
  logic a [(4'b1100 inside {4'b1?00}) : 0];
  initial #1 $display("AD_I %0d", $size(a));
endmodule
