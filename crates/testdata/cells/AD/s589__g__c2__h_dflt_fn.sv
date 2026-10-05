module child;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x, input int k = f(2)); return x + k; endfunction
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  child u ();
  int v;
  initial begin v = u.h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
