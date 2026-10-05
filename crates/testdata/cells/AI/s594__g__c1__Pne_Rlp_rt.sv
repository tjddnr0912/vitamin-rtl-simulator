`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  initial #1 $display("RT=%0d", ((X + {N{1'b0}}) != 8'hFC));
  initial #5 $finish;
endmodule
