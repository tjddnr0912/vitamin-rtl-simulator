`timescale 1ns/1ns
module t;
  localparam integer TA [0:1] = '{-5, 7};
  localparam L = ((TA[0] >>> 1) == -3);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
