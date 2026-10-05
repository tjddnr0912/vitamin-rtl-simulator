`timescale 1ns/1ns
module t;
  localparam logic signed [63:0] A64 [0:1] = '{-64'sd4, 64'sd2};
  localparam L = ((A64[0] + 1'sb0) < 0);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
