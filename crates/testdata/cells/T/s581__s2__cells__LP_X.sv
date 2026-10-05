`timescale 1ns/1ns
module t;
  localparam L = (4'bx100 ==? 4'b1?00);
  initial #1 $display("LP_X %b %0d", L, $bits(L));
endmodule
