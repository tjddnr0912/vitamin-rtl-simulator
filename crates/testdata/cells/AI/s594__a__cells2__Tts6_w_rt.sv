`timescale 1ns/1ns
module t;
  typedef logic signed [5:0] s6_t;
  localparam s6_t TA [0:1] = '{-6'sd3, 6'sd2};
  initial #1 $display("RT=%0d", ((TA[0] + 1'b0) == 6'h3D));
  initial #40 $finish;
endmodule
