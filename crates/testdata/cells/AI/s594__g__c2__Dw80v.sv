`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  localparam L = X + {N{40'h0}};
  initial #1 $display("L=%h B=%0d", L, $bits(L));
  initial #20 $finish;
endmodule
