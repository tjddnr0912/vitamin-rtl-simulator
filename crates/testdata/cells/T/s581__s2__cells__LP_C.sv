`timescale 1ns/1ns
module t;
  localparam L = (4'b1100 == 4'b1100);
  initial #1 $display("LP_C %b %0d", L, $bits(L));
endmodule
