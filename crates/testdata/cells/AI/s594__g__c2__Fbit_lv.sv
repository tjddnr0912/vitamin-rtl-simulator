`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit AB [0:1] = '{1'b1, 1'b0};
  localparam L = X + AB[0];
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #20 $finish;
endmodule
