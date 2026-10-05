module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  if (1) begin : g
`include "r21_gbody.svh"
  end
  initial #2 $finish;
endmodule
