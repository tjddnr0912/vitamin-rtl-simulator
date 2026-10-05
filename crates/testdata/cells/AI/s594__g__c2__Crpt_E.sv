`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  int cnt = 0;
  initial begin repeat ((A[1][1:0] | 2'b11 + 2'd2)) cnt++; #1 $display("cnt=%0d", cnt); end
  initial #20 $finish;
endmodule
