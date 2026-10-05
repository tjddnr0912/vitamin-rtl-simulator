package p;
  localparam int W = 3;
  function automatic int h(); begin : b logic [W:0] x; x = '1; h = x; end endfunction
endpackage
module top;
  localparam int W = 7;
  localparam int L = p::h();
  int v;
  initial begin v = p::h(); $display("L=%0d v=%0d", L, v); end
  initial #100 $finish;
endmodule
