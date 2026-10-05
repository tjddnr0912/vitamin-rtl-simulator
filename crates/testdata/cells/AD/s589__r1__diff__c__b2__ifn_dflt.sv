package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  function automatic int f(int a = h()); return a; endfunction
endpackage
module top;
  import r::*;
  localparam int P = p::f();
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("P=%0d v=%0d", P, v); $finish; end
  initial #100 $finish;
endmodule
