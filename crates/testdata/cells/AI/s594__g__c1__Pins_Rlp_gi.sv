`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  if ((X + {N{1'b0}}) inside {8'hFC, 8'h00}) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
