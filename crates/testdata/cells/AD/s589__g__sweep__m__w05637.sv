package a;
  function automatic int w(input int n);
    if (n == 1) return 1;
    return 7;
  endfunction
endpackage
package q;
  function automatic int w(input int n); return 3; endfunction
  function automatic logic [w(2):0] h(input int x); return x; endfunction
endpackage
module top;
  import a::*;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); $display("P=%0d v=%0d", P, v); #1 $finish; end
endmodule
