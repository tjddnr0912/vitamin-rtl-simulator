`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int g(input int i); int N; N = 5; return ((X + {N{1'b0}}) == 8'hFC); endfunction
  localparam L = g(0);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
