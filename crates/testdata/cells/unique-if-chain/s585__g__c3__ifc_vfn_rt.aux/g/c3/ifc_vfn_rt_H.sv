interface I;
  function void fv(input int x, output int r);
    r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
  endfunction
endinterface
module top;
  I i(); int q;
  initial begin #1 i.fv(0, q); $display("t=%0t q=%0d", $time, q); #1 $finish; end
endmodule
