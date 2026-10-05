`timescale 1ns/1ns
module t;
  localparam time TA [0:1] = '{64'd5, 64'd7};
  localparam L = (((TA[0] + 1'sb0) - 7) < 0);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
