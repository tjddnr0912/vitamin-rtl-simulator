`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  if ((C ? X : A[1][7:0]) == 8'hFC) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
