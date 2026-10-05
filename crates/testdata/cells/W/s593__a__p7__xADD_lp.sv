`timescale 1ns/1ns
module t;
  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t; endfunction
  localparam L = ((fxs(2) - 4'sd4) ==? 4'sb1?00);
  initial #1 $display("L=%b", L);
  initial #5 $finish;
endmodule
