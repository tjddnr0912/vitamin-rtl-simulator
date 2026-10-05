`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic fr(input int a); logic [3:0] r; fr = |{N{r}}; endfunction
  logic q;
  initial begin q = fr(2); #1 $display("RT=%b", q); end
  initial #40 $finish;
endmodule
