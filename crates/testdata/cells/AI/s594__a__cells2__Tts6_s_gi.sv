`timescale 1ns/1ns
module t;
  typedef logic signed [5:0] s6_t;
  localparam s6_t TA [0:1] = '{-6'sd3, 6'sd2};
  if (((TA[0] + 1'sb0) < 0)) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #40 $finish;
endmodule
