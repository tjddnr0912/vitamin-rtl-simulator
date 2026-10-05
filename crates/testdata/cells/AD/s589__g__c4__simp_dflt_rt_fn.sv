package q;
  function automatic int f(input int a); return 3; endfunction
  function int h(input int x, input int k = f(2)); return x + k; endfunction
endpackage
module top;
  import q::h;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  int v;
  initial begin v = h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
