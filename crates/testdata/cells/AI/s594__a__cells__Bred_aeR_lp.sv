`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic fr(input int a); logic [3:0] r; fr = |{N{r}}; endfunction
  localparam L = fr(2);
  initial #1 $display("L=%b", L);
  initial #40 $finish;
endmodule
