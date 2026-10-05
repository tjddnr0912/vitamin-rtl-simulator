package q;
  function automatic int f(input int a); return 3; endfunction
  function logic [f(2):0] h(input int x); return x; endfunction
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
