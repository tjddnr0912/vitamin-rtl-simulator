`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  initial #1 $display("RV=%0d", X / A[1]);
  initial #5 $finish;
endmodule
