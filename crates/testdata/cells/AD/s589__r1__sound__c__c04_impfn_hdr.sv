package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic logic [g():0] h(); h = '1; endfunction
endpackage
module top;
  import q::*;
  int v, b;
  initial begin v = p::h(); b = $bits(p::h()); $display("v=%0d b=%0d", v, b); end
  initial #100 $finish;
endmodule
