package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  function automatic int f(logic [h()-1:0] a); return a; endfunction
endpackage
module top;
  import r::*;
  localparam int P = p::f(9'h1ff);
  logic [31:0] v;
  initial begin #1 v = p::f(9'h1ff); $display("P=%0d v=%0d", P, v); $finish; end
  initial #100 $finish;
endmodule
