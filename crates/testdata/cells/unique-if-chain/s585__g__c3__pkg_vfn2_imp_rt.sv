package pk;
  function void fv(input int x);
    int r; r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endfunction
  function int f(input int x);
    fv(x); return x + 7;
  endfunction
endpackage
module top;
  import pk::*;
  initial begin #1 fv(0); $display("t=%0t", $time); #1 $finish; end
endmodule
