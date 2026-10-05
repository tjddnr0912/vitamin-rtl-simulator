`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic fr(input logic [3:0] a); fr = |{N{a}}; endfunction
  logic q;
  initial begin q = fr(4'b0010); #1 $display("RT=%b", q); end
  initial #40 $finish;
endmodule
