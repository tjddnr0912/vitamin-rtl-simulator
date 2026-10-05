`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  int k;
  initial begin k = 0; repeat ((A[1] - 8'd2) + 2'd3) k = k + 1; #1 $display("k=%0d", k); end
  initial #40 $finish;
endmodule
