`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int NN = -2;
  localparam L = {NN{1'b0}};
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #20 $finish;
endmodule
