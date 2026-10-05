package r;
  function automatic int g(input int a); return 2; endfunction
  function automatic int k(input int a); return g(a); endfunction
endpackage
package q;
  function automatic int g(input int a); return 5; endfunction
  function automatic int f(input int a); return r::k(a); endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a); return 7; endfunction
  localparam int P = q::h(1003);
  int v;
  initial begin v = q::h(1003); #1 $display("P=%0d v=%0d", P, v); $finish; end
  initial #50 $finish;
endmodule
