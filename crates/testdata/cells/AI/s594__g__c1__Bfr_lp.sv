`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic int g(input int a); logic signed [7:0] x; x = -4;
    return ((x + {N{1'b0}}) == 8'hFC); endfunction
  localparam L = g(0);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
