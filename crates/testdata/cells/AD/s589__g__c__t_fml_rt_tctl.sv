package q;
  typedef logic [3:0] T;
  function automatic int h(input T x); return x; endfunction
endpackage
module top;
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
