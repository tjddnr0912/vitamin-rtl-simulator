class C;
  int r;
  function void m(input int x);
    r = 7;
    if (x == 1) r = 1; else unique if (x == 2) r = 2;
  endfunction
endclass
module top;
  C c;
  initial begin c = new; #1 c.m(0); $display("t=%0t r=%0d", $time, c.r); #1 $finish; end
endmodule
