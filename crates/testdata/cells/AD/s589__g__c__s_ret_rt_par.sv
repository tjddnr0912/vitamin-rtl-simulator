package q;
  localparam int W = 3;
  function logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int W = 7;
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
