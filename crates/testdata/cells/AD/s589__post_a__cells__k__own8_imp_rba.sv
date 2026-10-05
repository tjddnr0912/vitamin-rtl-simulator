package r;
  localparam int K = 3;
  function automatic logic [K:0] g(input int x); return x; endfunction
endpackage
package q;
  import r::g;
  localparam int K = 5;
  function automatic int h(input int x); return g(x); endfunction
endpackage
module top;
  localparam int K = 3;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
  initial #50 $finish;
endmodule
