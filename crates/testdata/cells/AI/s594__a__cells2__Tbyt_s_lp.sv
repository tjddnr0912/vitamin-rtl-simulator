`timescale 1ns/1ns
module t;
  localparam byte TA [0:1] = '{-8'sd3, 8'sd2};
  localparam L = ((TA[0] + 1'sb0) < 0);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
