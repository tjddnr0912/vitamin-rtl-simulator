`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  if (1) begin : g localparam logic [7:0] A = 8'h0D; localparam L = ((A[1] - 1'b1) == 1'b1); initial #1 $display("L=%0d", L); end
  initial #40 $finish;
endmodule
