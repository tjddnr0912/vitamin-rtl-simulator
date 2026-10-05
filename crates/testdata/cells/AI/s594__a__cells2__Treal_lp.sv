`timescale 1ns/1ns
module t;
  localparam real TA [0:1] = '{1.5, 2.5};
  localparam L = (TA[0] > 1.0);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
