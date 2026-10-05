`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit AB [0:1] = '{1'b1, 1'b0};
  if ((X + AB[0]) == 8'hFD) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #20 $finish;
endmodule
