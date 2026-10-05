`timescale 1ns/1ns
module t;
  localparam logic signed TA [0:1] = '{1'b1, 1'b0};
  if (((TA[0] + 1'sb0) < 0)) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #40 $finish;
endmodule
