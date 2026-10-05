`timescale 1ns/1ns
module t;
  function automatic int f2(input int a); return a; endfunction
  int k;
  initial begin k = 0; repeat ({f2(2){1'b0}} + 2'd3) k = k + 1; #1 $display("k=%0d", k); end
  initial #40 $finish;
endmodule
