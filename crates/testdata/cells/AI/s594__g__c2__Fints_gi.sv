`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int AI [0:1] = '{-4, 2};
  if ((AI[0] + 0) == -4) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #20 $finish;
endmodule
