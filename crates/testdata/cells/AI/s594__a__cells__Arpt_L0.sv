`timescale 1ns/1ns
module t;
  int k;
  initial begin k = 0; repeat (2'b00 + 2'd3) k = k + 1; #1 $display("k=%0d", k); end
  initial #40 $finish;
endmodule
