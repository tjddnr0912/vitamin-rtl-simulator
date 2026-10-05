`timescale 1ns/1ns
module t;
  localparam int N = 2;
  int k;
  initial begin k = 0; repeat ({N{1'b0}} + 2'd3) k = k + 1; #1 $display("k=%0d", k); end
  initial #40 $finish;
endmodule
