package p;
  localparam real R = 2.6;
  function automatic logic [$rtoi(R*3.0):0] f(); return '1; endfunction
endpackage
module top;
  import p::*;
  logic [31:0] v; int b;
  initial begin #1 v = p::f(); b = $bits(p::f()); $display("b=%0d v=%0d", b, v); $finish; end
  initial #100 $finish;
endmodule
