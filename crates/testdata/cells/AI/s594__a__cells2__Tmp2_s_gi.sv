`timescale 1ns/1ns
module t;
  localparam logic [1:0][3:0] TA [0:1] = '{8'hF0, 8'h12};
  if (((TA[0] + 1'sb0) < 0)) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #40 $finish;
endmodule
