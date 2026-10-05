package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); logic [f(2):0] t; t = x; return t; endfunction
endpackage
module top;
  import q::h;
  int v;
  initial begin v = h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
