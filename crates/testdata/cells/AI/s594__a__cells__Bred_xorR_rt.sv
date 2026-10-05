`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic fr(input logic [3:0] a); fr = ^{N{a}, 1'b1}; endfunction
  logic q;
  initial begin q = fr(4'h1); #1 $display("RT=%b", q); end
  initial #40 $finish;
endmodule
