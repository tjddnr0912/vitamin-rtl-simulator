package q;
  function automatic int f(input int a); return 3; endfunction
  int pv = 0;
  function logic [31:0] h(input logic [f(2):0] x); h = x + pv; endfunction
endpackage
module top;
  import q::h;
  function automatic int f(input int a); f = 7; if (a == 1) f = 10; endfunction
  int v; int a = 1000;
  always_comb v = h(a);
  initial begin #1 $display("v=%0d", v); $finish; end
endmodule
