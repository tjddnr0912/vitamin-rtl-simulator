package q;
  function automatic int f(input int a); return 3; endfunction
  class C;
    function logic [f(2):0] m(input int x); return x; endfunction
  endclass
endpackage
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  q::C c;
  int v;
  initial begin c = new; v = c.m(1000); $display("v=%0d", v); #1 $finish; end
endmodule
