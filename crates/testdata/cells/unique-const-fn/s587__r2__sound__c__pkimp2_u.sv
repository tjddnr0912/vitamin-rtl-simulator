package r;
  function automatic int g(input int a); return 3; endfunction
endpackage
package q;
  import r::*;
  function automatic int h(input int a); return g(a); endfunction
endpackage
module top;
  function automatic int g(input int a);
    unique if (a == 1) return 1;
    return 7;
  endfunction
  localparam int P = q::h(2);
  int v;
  initial begin v = q::h(2); $display("P=%0d v=%0d", P, v); #1 $finish; end
endmodule
