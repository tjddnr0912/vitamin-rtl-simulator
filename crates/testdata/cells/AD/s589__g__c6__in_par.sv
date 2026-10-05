package q;
  localparam int W = 3;
  function logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  import q::h;
  localparam int W = 7;
  int v; logic [15:0] a = 16'd1000;
  always_comb v = h(a);
  initial begin #1 $display("v=%0d", v); $finish; end
endmodule
