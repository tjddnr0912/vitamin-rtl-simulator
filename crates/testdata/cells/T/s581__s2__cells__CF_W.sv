`timescale 1ns/1ns
module t;
  function automatic int f(int a); return a + ((4'd15 + 4'd1) ==? 5'b1?000); endfunction
  localparam L = f(2);
  initial #1 $display("CF_W %0d", L);
endmodule
