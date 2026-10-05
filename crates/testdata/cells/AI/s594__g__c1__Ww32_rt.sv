`timescale 1ns/1ns
module t;
  localparam int X = -4;
  localparam int N = 2;
  initial #1 $display("RT=%0d", ((X + {N{1'b0}}) == 32'hFFFF_FFFC));
  initial #5 $finish;
endmodule
