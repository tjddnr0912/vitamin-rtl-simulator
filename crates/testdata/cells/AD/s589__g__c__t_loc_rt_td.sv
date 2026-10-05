package q;
  typedef logic [3:0] T;
  function automatic int h(input int x); T t; t = x; return t; endfunction
endpackage
module top;
  typedef logic [7:0] T;
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
