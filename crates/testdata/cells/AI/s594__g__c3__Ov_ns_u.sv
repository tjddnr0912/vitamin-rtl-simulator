`timescale 1ns/1ns
module m #(parameter int P = 0);
  initial #1 $display("P=%0d", P);
endmodule
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam bit C = 1;
  m #(.P(A[0])) u();
  initial #20 $finish;
endmodule
