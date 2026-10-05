package q;
  localparam logic [71:0] K = 72'h0300000000000000ff;
  function automatic logic [K[71:64]:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam logic [71:0] K = 72'h0700000000000000ff;
  int v;
  initial begin v = q::h(1000); #1 $display("v=%0d", v); $finish; end
endmodule
