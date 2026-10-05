package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [31:0] g(); return {f(2){1'b1}}; endfunction
endpackage
package p;
  import q::*;
  function automatic int f(input int a); return 9; endfunction
  function automatic logic [31:0] h(); return g(); endfunction
endpackage
module top;
  import q::*;
  int v;
  initial begin v = p::h(); $display("v=%0d", v); end
  initial #100 $finish;
endmodule
