`timescale 1ns/1ns
module m #(parameter logic [7:0] A [0:1] = '{8'hFC, 8'h02});
  localparam logic signed [7:0] X = -4;
  localparam L = ((X | A[1]) == 8'hFF);
  initial #1 $display("L=%0d", L);
endmodule
module t;
  m u();
  initial #20 $finish;
endmodule
