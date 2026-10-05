package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic int h2(input logic [g():0] a); return a; endfunction
endpackage
module top;
  import q::*;
  localparam int L = p::h2(-1);
  int v;
  initial begin v = p::h2(-1); $display("L=%0d v=%0d", L, v); end
  initial #100 $finish;
endmodule
