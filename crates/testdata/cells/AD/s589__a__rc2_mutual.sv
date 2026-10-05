package q;
  function automatic logic [g(2):0] f(input int a); return a; endfunction
  function automatic logic [f(2):0] g(input int a); return a; endfunction
endpackage
module top;
  int v;
  initial begin v = q::f(3); #1 $display("v=%0d", v); $finish; end
  initial #50 $finish;
endmodule
