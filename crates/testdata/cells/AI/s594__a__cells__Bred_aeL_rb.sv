`timescale 1ns/1ns
module t;
  function automatic logic fr(input int a); logic [3:0] r; fr = |{2{r}}; endfunction
  logic [fr(2) + 2:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
