package p;
  function automatic logic [$bits(g())-1:0] f(); return '1; endfunction
  function automatic logic [$bits(f())-1:0] g(); return '1; endfunction
endpackage
module top;
  import p::*;
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
