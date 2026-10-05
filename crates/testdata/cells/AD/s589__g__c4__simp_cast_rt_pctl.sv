package q;
  localparam int W = 3;
  function int h(input int x); return W'(x); endfunction
endpackage
module top;
  import q::h;
  int v;
  initial begin v = h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
