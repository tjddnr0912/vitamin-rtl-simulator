package r;
  localparam int W = 3;
  function automatic logic [W:0] h(input int x); return x; endfunction
endpackage
package q;
  import r::h;
  localparam int W = 5;
  function automatic int g(input int x); return h(x); endfunction
endpackage
module top;
  localparam int W = 7;
  localparam int P = q::g(1000);
  int v;
  initial begin v = q::g(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
