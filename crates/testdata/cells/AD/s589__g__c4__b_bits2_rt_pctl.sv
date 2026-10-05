package q;
  localparam int W = 3;
  function automatic int h(input int x); logic [W:0] t; return $bits(t) + W*0; endfunction
endpackage
module top;
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
