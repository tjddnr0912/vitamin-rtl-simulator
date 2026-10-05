package q;
  function automatic int f(input int a); return 3; endfunction
  function int h(input int x); logic [f(2):0] t; t = x; return t; endfunction
endpackage
module top;
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
