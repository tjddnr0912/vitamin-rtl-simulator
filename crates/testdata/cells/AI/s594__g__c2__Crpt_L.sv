`timescale 1ns/1ns
module t;
  int cnt = 0;
  initial begin repeat ((2'b11 + 2'd2)) cnt++; #1 $display("cnt=%0d", cnt); end
  initial #20 $finish;
endmodule
