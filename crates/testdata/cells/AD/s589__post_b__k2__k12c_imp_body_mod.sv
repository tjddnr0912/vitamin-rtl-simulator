package q;
  localparam int K = 3;
  function automatic int kk(input int a); return a; endfunction
  function automatic logic [31:0] g(); return {kk(K){1'b1}}; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic logic [31:0] h(); return g(); endfunction
endpackage
module top;
  import q::*;
  localparam int K = 3;
  int v;
  initial begin v = p::h(); $display("v=%0d", v); end
  initial #100 $finish;
endmodule
