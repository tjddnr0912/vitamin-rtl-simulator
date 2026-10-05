`timescale 1ns/1ns
module t;
  localparam bit TA [0:1] = '{1'b1, 1'b0};
  localparam L = ((TA[0] + 1'sb0) < 0);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
