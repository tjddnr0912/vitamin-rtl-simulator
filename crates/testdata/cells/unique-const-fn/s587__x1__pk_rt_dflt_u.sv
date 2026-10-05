package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x, input int k = f(2)); return x + k; endfunction
endpackage
module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  int v;
  initial begin v = q::h(1000); #1 $display("v=%0d", v); $finish; end
endmodule
