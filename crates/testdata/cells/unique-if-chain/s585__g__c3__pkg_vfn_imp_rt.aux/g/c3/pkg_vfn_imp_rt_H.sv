package pk;
  function void fv(input int x, output int r);
    r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
  endfunction
  task t(input int x, output int r);
    r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
  endtask
  function int f(input int x);
    int q; fv(x, q); return q;
  endfunction
endpackage
module top;
  import pk::*;
  int q;
  initial begin #1 fv(0, q); $display("t=%0t q=%0d", $time, q); #1 $finish; end
endmodule
