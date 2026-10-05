package p;
  localparam int K = 4;
  int sink;
  function automatic int g(int a); return a*3; endfunction
  function int f(input logic [31:0] a); sink = a; return a; endfunction
  function automatic int f2(input logic [31:0] a); return a; endfunction
endpackage
module top;
  localparam int K = 2;
  function automatic int g(int a); return a; endfunction
  logic [31:0] v, w;
  initial begin #1 v = p::f(K'(5'd31)); w = p::f2({g(2){1'b1}}); $display("v=%0d w=%0d", v, w); $finish; end
  initial #100 $finish;
endmodule
