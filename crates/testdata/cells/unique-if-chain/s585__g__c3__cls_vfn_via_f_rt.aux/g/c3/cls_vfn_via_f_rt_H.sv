class C;
  function void fv(input int x);
    int r; r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
  endfunction
  function int f(input int x); fv(x); return x + 7; endfunction
endclass
module top;
  C c;
  initial begin c = new; #1 $display("t=%0t f0=%0d", $time, c.f(0)); #1 $finish; end
endmodule
