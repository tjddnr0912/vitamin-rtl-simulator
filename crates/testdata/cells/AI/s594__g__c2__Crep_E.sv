`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam logic [31:0] R = {(A[1][1:0] | 2'b11 + 2'd2){4'hA}};
  initial #1 $display("R=%h", R);
  initial #20 $finish;
endmodule
