package q; function automatic int g(); return 2; endfunction endpackage
package p1;
  import q::*;
  function automatic int g(); return 6; endfunction
  function automatic logic [g()-1:0] f(); return '1; endfunction
endpackage
package p2;
  function automatic int g(); return 6; endfunction
  import q::*;
  function automatic logic [g()-1:0] f(); return '1; endfunction
endpackage
module top;
  function automatic int g(); return 9; endfunction
  logic [31:0] a, b;
  initial begin #1 a = p1::f(); b = p2::f(); $display("a=%0d b=%0d", a, b); $finish; end
  initial #100 $finish;
endmodule
