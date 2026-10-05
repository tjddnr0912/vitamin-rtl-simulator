`timescale 1ns/1ns
module t;
  localparam int N = 2;
  initial #1 $display("RT=%0d", (('1 ^ {N{1'b0}}) == 2'b11));
  initial #5 $finish;
endmodule
