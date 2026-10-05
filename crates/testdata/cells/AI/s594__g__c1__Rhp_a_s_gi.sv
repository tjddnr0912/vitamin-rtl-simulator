`timescale 1ns/1ns
module t #(parameter int NH = 2);
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  if ((X + {NH{1'b0}}) == 8'hFC) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
