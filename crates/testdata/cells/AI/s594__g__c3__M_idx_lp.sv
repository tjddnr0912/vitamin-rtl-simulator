`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam bit C = 1;
  localparam logic [15:0] W = 16'h00F4;
  localparam L = W[A[1]];
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #20 $finish;
endmodule
