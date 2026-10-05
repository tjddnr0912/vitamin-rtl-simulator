`timescale 1ns/1ns
module t;
  function automatic int h(); return 2; endfunction
  function automatic int g(input int a); logic signed [7:0] x; logic [h()-1:0] z; x = -4; z = 0;
    return ((x + z) == 8'hFC); endfunction
  localparam L = g(0);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
