package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int g(input int a); return f(a) + 1; endfunction
endpackage
module top;
  import q::g;
  function automatic int f(input int a); return 7; endfunction
  localparam int P = g(0);
  int v;
  initial begin v = g(0); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
