interface I;
  function int f(input int x);
    int r; r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
    return r;
  endfunction
endinterface
module top;
  I i();
  initial begin #1 $display("t=%0t f0=%0d", $time, i.f(0)); #1 $finish; end
endmodule
