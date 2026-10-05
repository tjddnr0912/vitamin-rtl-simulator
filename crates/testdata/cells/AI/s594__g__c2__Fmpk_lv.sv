`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [1:0][3:0] AM [0:1] = '{8'hFC, 8'h02};
  localparam L = X | AM[1];
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #20 $finish;
endmodule
