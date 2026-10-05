`timescale 1ns/1ns
module t;
  localparam L = (4'b1100 ==? 4'b1?00);
  initial #1 $display("LP_Q %b %0d", L, $bits(L));
endmodule
