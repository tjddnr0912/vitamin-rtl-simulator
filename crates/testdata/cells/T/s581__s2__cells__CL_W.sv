`timescale 1ns/1ns
module t;
  localparam L = $clog2(((4'd15 + 4'd1) ==? 5'b1?000) + 4);
  initial #1 $display("CL_W %0d", L);
endmodule
