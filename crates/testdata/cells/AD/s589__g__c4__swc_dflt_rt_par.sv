package q;
  localparam int W = 3;
  function int h(input int x, input int k = W); return x + k; endfunction
endpackage
module top;
  import q::*;
  localparam int W = 7;
  int v;
  initial begin v = h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
