`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam logic [15:0] L = C ? X : A[1][3:0];
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
