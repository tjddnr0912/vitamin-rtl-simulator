package p;
  localparam int W = 3;
  function automatic logic [W:0] m(); m = '1; endfunction
  class C;
    localparam int W = 7;
    function logic [W:0] m(); m = '1; endfunction
  endclass
endpackage
module top;
  import p::*;
  C c;
  int v;
  initial begin c = new; v = c.m(); $display("v=%0d", v); end
  initial #100 $finish;
endmodule
