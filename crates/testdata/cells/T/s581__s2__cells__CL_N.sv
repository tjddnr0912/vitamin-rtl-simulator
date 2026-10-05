`timescale 1ns/1ns
module t;
  localparam L = $clog2(((4'd15 + 4'd1) inside {5'b0?000}) + 4);
  initial #1 $display("CL_N %0d", L);
endmodule
