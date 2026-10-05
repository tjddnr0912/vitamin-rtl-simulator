`timescale 1ns/1ns
module t;
  localparam L = ((4'd15 + 4'd1) inside {5'b0?000});
  initial #1 $display("LP_N %b %0d", L, $bits(L));
endmodule
