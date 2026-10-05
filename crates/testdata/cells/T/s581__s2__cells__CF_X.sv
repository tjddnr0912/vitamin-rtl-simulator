`timescale 1ns/1ns
module t;
  function automatic int f(int a); return a + (4'bx100 ==? 4'b1?00); endfunction
  localparam L = f(2);
  initial #1 $display("CF_X %0d", L);
endmodule
