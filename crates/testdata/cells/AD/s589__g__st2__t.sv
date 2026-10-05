package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  c u ();
  int v;
  initial begin v = q::h(1000); $display("top.v=%0d", v); #2 $finish; end
endmodule
