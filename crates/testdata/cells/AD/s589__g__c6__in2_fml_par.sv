package q;
  localparam int W = 3;
  int pv = 0;
  function logic [31:0] h(input logic [W:0] x); h = x + pv; endfunction
endpackage
module top;
  import q::h;
  localparam int W = 7;
  int v; int a = 1000;
  always_comb v = h(a);
  initial begin #1 $display("v=%0d", v); $finish; end
endmodule
