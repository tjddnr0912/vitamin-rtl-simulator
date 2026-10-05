package q;
  localparam int K = 4;
  function automatic int g(); return K; endfunction
  function automatic logic [g()-1:0] f(); return '1; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 8;
  function automatic int g(); return K; endfunction
  function automatic logic [g()-1:0] f(); return '1; endfunction
endpackage
module top;
  import q::*;
  localparam int K = 16;
  function automatic int g(); return 9; endfunction
  logic [31:0] a, b;
  initial begin #1 a = q::f(); b = p::f(); $display("a=%0d b=%0d", a, b); $finish; end
  initial #100 $finish;
endmodule
