`timescale 1ns/1ns
module t;
  logic [(4'bx100 ==? 4'b1?00) : 0] v;
  initial #1 $display("RB_X %0d", $bits(v));
endmodule
