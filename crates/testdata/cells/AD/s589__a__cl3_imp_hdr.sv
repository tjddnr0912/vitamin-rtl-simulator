package r;
  function automatic int f2(input int a); return 3; endfunction
  function automatic logic [f2(0):0] g(input int a); return a; endfunction
endpackage
package q;
  import r::g;
  function automatic logic [g(21)+2:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
  initial #50 $finish;
endmodule
