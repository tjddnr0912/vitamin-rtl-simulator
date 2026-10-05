package p;
  localparam int unsigned U = 8;
  function automatic logic [((U - 9) > 0 ? 3 : 7) : 0] f(); return '1; endfunction
endpackage
module top;
  import p::*;
  logic [31:0] v; int b;
  initial begin #1 v = p::f(); b = $bits(p::f()); $display("b=%0d v=%0d", b, v); $finish; end
  initial #100 $finish;
endmodule
