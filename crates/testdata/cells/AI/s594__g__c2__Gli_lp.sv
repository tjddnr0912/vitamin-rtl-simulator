`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  function automatic int g(input int i); return ((X | A[1]) == 8'hFE); endfunction
  localparam L = g(1);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
