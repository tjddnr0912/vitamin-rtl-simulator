`timescale 1ns/1ns
module t;
  localparam L = ((4'd15 + 4'd1) ==? 5'b1?000);
  initial #1 $display("LP_W %b %0d", L, $bits(L));
endmodule
