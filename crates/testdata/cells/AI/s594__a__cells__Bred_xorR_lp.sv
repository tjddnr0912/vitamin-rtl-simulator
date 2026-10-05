`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic fr(input logic [3:0] a); fr = ^{N{a}, 1'b1}; endfunction
  localparam L = fr(4'h1);
  initial #1 $display("L=%b", L);
  initial #40 $finish;
endmodule
