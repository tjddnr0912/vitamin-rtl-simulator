`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  function automatic logic fs(input int i); logic [15:0] A; A = 16'h0000; fs = ((A[1] - 1'b1) == 1'b1); endfunction
  localparam L = fs(0);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
