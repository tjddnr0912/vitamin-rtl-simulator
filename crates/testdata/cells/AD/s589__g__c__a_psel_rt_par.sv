package q;
  localparam int W = 3;
  function automatic logic [31:0] h(input logic [15:0] x); return x[W:0]; endfunction
endpackage
module top;
  localparam int W = 7;
  logic [31:0] v;
  initial begin v = q::h(16'hABCD); $display("v=%h", v); #1 $finish; end
endmodule
