function automatic int f(input int a); return 3; endfunction
package q;
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
