package p;
  localparam int unsigned U = 8;
  function automatic logic [((U - 9) > 0 ? 3 : 7) : 0] f(); return '1; endfunction
endpackage
module top;
  import p::*;
  localparam int B = $bits(p::f());
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("B=%0d v=%0d", B, v); $finish; end
  initial #100 $finish;
endmodule
