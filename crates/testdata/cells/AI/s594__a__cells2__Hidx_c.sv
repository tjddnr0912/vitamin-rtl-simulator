`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam logic signed [7:0] X = -4;
  localparam int i = 0;
  function automatic logic fi(input int i); fi = ((X | A[i]) == 8'hFE); endfunction
  localparam L = fi(1);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
