`timescale 1ns/1ns
module t;
  localparam int X = -4;
  localparam int N = 2;
  localparam logic [63:0] L = X + {N{1'b0}};
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
