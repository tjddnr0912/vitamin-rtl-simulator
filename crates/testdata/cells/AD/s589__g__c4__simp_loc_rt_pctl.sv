package q;
  localparam int W = 3;
  function int h(input int x); logic [W:0] t; t = x; return t; endfunction
endpackage
module top;
  import q::h;
  int v;
  initial begin v = h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
