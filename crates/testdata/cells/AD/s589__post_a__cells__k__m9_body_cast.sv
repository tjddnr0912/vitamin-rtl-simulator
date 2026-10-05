package r;
  function automatic int g(input int a); return 2; endfunction
  function automatic int k(input int a); return g(a); endfunction
endpackage
package q;
  function automatic int g(input int a); return 5; endfunction
  function automatic int f(input int a); return r::k(a); endfunction
  function automatic int h(input int x); return f(2)'(x); endfunction
endpackage
module top;
  import q::h;
  function automatic int f(input int a); return 7; endfunction
  int v;
  initial begin v = h(1003); $display("v=%0d", v); #1 $finish; end
  initial #50 $finish;
endmodule
