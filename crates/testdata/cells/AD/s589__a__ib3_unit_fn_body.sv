function automatic int uf(input int a); return a + 3; endfunction
package q;
  function automatic int h(input int x); return uf(x) * 2; endfunction
endpackage
module top;
  localparam int P = q::h(2);
  int v;
  initial begin v = q::h(2); #1 $display("P=%0d v=%0d", P, v); $finish; end
  initial #50 $finish;
endmodule
