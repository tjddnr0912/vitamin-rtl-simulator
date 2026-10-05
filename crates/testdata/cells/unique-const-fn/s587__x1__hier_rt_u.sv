module child;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  child u ();
  int v;
  initial begin v = u.h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
