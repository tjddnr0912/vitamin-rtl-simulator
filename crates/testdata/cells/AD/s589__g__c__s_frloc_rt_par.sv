package q;
  localparam int W = 3;
  function int h(input int x);
    logic [W:0] t;
    t = 0;
    for (int i = 0; i < x; i++) t = t + 1;
    return t;
  endfunction
endpackage
module top;
  localparam int W = 7;
  int v, n = 1000;
  initial begin v = q::h(n); $display("v=%0d", v); #1 $finish; end
endmodule
