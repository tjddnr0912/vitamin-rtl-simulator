`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  function automatic int g(input int i); return ((X + {i{1'b0}}) == 8'hFC); endfunction
  localparam L = g(1);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
