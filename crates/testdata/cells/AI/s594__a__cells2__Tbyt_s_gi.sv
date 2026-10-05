`timescale 1ns/1ns
module t;
  localparam byte TA [0:1] = '{-8'sd3, 8'sd2};
  if (((TA[0] + 1'sb0) < 0)) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #40 $finish;
endmodule
