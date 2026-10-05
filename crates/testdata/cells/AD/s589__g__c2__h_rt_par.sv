module child;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x); return x; endfunction
endmodule
module top;
  localparam int W = 7;
  child u ();
  int v;
  initial begin v = u.h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
