`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic fr(input int a); logic [3:0] r; fr = |{N{r}}; endfunction
  if (fr(2)) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #40 $finish;
endmodule
