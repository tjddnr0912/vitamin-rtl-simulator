package r;
  localparam int W = 3;
endpackage
package q;
  import r::*;
  function automatic logic [W:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
