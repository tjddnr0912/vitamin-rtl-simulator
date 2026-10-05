package q;
  localparam int W = 3;
  function logic [31:0] h(input logic [15:0] x); return x[W:0]; endfunction
endpackage
module top;
  localparam int W = 7;
  localparam logic [31:0] P = q::h(16'hABCD);
  initial begin #1 $display("P=%h", P); $finish; end
endmodule
