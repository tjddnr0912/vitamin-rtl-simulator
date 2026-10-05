package r;
  function automatic int g(input int a); return 2; endfunction
  function automatic int k(input int a); return g(a); endfunction
endpackage
package q;
  function automatic int g(input int a); return 5; endfunction
  function automatic int f(input int a); return r::k(a); endfunction
  function automatic logic [31:0] h(input int x); return {f(2){1'b1}}; endfunction
endpackage
module top;
  import q::h;
  function automatic int f(input int a); return 7; endfunction
  logic [31:0] v;
  initial begin v = h(0); $display("v=%h", v); #1 $finish; end
  initial #50 $finish;
endmodule
