package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic int h3(); return g(); endfunction
endpackage
module top;
  import q::*;
  localparam int L = p::h3();
  int v;
  initial begin v = p::h3(); $display("L=%0d v=%0d", L, v); end
  initial #100 $finish;
endmodule
