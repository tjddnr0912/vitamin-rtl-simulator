`timescale 1ns/1ns
module t;
  logic [(4'b1100 inside {4'b1?00}) : 0] v;
  initial #1 $display("RB_I %0d", $bits(v));
endmodule
