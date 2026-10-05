package q;
  localparam int W = 3;
  int pv = 0;
  function int h(input int x); logic [W:0] t; t = x + pv; return t; endfunction
endpackage
module top;
  import q::h;
  
  int v; int a = 1000;
  always_comb v = h(a);
  initial begin #1 $display("v=%0d", v); $finish; end
endmodule
