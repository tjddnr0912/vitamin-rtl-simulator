`timescale 1ns/1ns
module t;
  logic [(4'b1100 == 4'b1100) : 0] v;
  initial #1 $display("RB_C %0d", $bits(v));
endmodule
