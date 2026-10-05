`timescale 1ns/1ns
module t;
  localparam int N = 2;
  localparam L = (('1 ^ {N{1'b0}}) == 2'b11);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
