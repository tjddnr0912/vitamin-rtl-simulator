`timescale 1ns/1ns
module m #(parameter P = 1);
  initial #1 $display("P=%0d B=%0d", P, $bits(P));
endmodule
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  m #(.P(X | A[1])) u();
  initial #20 $finish;
endmodule
