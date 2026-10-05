package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  function automatic int f2(int x); return h()'(x); endfunction
endpackage
module top;
  import r::*;
  logic [31:0] v;
  initial begin #1 v = p::f2(511); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
