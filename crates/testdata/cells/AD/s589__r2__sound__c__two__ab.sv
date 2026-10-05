package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  localparam int W = 3;
  function automatic logic [g():0] h(); h = '1; endfunction
  function automatic logic [W:0][1:0] h2(); h2 = '1; endfunction
endpackage
module top;
  import q::*;
  localparam int W = 7;
  int v, b, u;
  initial begin v = p::h(); b = $bits(p::h()); u = p::h2(); $display("v=%0d b=%0d u=%0d", v, b, u); end
  initial #100 $finish;
endmodule
