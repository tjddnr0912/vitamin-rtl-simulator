package q;
  localparam int W = 3;
  function logic [31:0] h(input logic [15:0] x); return x[W:0]; endfunction
endpackage
module top;
  import q::*;
  logic [31:0] v;
  initial begin v = h(16'hABCD); $display("v=%h", v); #1 $finish; end
endmodule
