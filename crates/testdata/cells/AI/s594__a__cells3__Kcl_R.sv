`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  localparam L = $clog2(X + {N{1'b0}});
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
