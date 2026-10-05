package a;
  function automatic int f(input int n);
    f = 7;
    unique if (n == 1) f = 10;
  endfunction
endpackage
package q;
  function automatic int f(input int n); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  import a::*;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
