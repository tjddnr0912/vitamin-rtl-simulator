package pk;
  function void fv(input int x, output int r);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endfunction
  task t(input int x, output int r);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endtask
  function int f(input int x);
    int q; fv(x, q); return q;
  endfunction
endpackage
module top;
  initial begin #1 $display("t=%0t f0=%0d", $time, pk::f(0)); #1 $finish; end
endmodule
