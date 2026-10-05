`timescale 1ns/1ns
module t;
  localparam L = $clog2((4'b1100 inside {4'b1?00}) + 4);
  initial #1 $display("CL_I %0d", L);
endmodule
