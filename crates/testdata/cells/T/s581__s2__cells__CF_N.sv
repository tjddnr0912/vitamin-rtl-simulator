`timescale 1ns/1ns
module t;
  function automatic int f(int a); return a + ((4'd15 + 4'd1) inside {5'b0?000}); endfunction
  localparam L = f(2);
  initial #1 $display("CF_N %0d", L);
endmodule
