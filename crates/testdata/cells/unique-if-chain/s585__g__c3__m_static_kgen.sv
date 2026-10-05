module top;
  function int f(input int x);
    int r; r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
    return r;
  endfunction
  if (f(0) == 7) begin : g initial $display("gen taken"); end
  initial #1 $finish;
endmodule
