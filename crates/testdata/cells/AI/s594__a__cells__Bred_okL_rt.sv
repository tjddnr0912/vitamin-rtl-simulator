`timescale 1ns/1ns
module t;
  function automatic logic fr(input logic [3:0] a); fr = |{2{a}}; endfunction
  logic q;
  initial begin q = fr(4'b0010); #1 $display("RT=%b", q); end
  initial #40 $finish;
endmodule
