package q;
  localparam int W = 3;
  function automatic int h(input int x, input int k = W); return x + k; endfunction
endpackage
module top;
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
