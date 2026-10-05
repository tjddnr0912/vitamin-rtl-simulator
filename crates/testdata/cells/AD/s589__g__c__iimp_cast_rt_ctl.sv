package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); return f(2)'(x); endfunction
endpackage
module top;
  import q::h;
  int v;
  initial begin v = h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
