`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  localparam logic [67:0] L = X + {N{17'h0}};
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
