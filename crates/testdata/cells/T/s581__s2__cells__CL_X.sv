`timescale 1ns/1ns
module t;
  localparam L = $clog2((4'bx100 ==? 4'b1?00) + 4);
  initial #1 $display("CL_X %0d", L);
endmodule
