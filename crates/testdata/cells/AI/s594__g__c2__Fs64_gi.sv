`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam longint AQ [0:1] = '{-64'sd4, 64'sd2};
  if (((AQ[0] + 64'd0) > 64'd100) == 1'b1) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #20 $finish;
endmodule
