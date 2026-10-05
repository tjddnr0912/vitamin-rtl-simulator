package p;
  localparam int W = 3;
  function automatic int h();
    localparam int W = 7;
    logic [W:0] t;
    t = '1;
    return t;
  endfunction
endpackage
module top;
  localparam int L = p::h();
  int v;
  initial begin v = p::h(); $display("L=%0d v=%0d", L, v); end
  initial #100 $finish;
endmodule
