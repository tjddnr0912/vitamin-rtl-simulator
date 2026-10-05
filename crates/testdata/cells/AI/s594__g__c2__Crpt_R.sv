`timescale 1ns/1ns
module t;
  localparam int N = 2;
  int cnt = 0;
  initial begin repeat (({N{1'b1}} + 2'd2)) cnt++; #1 $display("cnt=%0d", cnt); end
  initial #20 $finish;
endmodule
