`timescale 1ns/1ns
module t;
  logic [7:0] r;
  int k;
  initial begin k = 0; repeat (r) k = k + 1; #1 $display("k=%0d", k); end
  initial #40 $finish;
endmodule
