`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [8:1] AL [0:1] = '{8'hFC, 8'h02};
  localparam L = X | AL[1];
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #20 $finish;
endmodule
