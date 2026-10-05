`timescale 1ns/1ns
module t;
  typedef logic signed [5:0] s6_t;
  localparam s6_t TA [0:1] = '{-6'sd3, 6'sd2};
  localparam L = ((TA[0] + 6'd4) == 6'd1);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
