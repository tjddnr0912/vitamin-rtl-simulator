`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam U = '1 ^ A[1];
  initial #1 $display("U=%h B=%0d", U, $bits(U));
  initial #20 $finish;
endmodule
