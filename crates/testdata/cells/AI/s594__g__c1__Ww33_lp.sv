`timescale 1ns/1ns
module t;
  localparam logic signed [32:0] X = -4;
  localparam int N = 2;
  localparam L = ((X + {N{1'b0}}) == 33'h1_FFFF_FFFC);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
