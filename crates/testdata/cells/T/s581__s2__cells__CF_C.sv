`timescale 1ns/1ns
module t;
  function automatic int f(int a); return a + (4'b1100 == 4'b1100); endfunction
  localparam L = f(2);
  initial #1 $display("CF_C %0d", L);
endmodule
