package q;
  localparam int W = 3;
  function int h(input logic [W:0] x); return x; endfunction
endpackage
module top;
  import q::*;
  int v;
  initial begin v = h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
