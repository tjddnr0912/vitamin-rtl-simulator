`timescale 1ns/1ns
module m #(parameter logic [15:0] P = 0);
  initial #1 $display("P=%0d", P);
endmodule
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  m #(.P(X | A[1])) u();
  initial #20 $finish;
endmodule
