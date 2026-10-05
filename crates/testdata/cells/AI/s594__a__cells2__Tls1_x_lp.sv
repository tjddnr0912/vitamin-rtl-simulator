`timescale 1ns/1ns
module t;
  localparam logic signed TA [0:1] = '{1'b1, 1'b0};
  localparam L = ((TA[0] + 2'sb00) == -2'sd1);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
