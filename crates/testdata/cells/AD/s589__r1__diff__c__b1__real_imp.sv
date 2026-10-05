package p;
  localparam real R = 2.6;
  function automatic logic [$rtoi(R*3.0):0] f(); return '1; endfunction
endpackage
module top;
  import p::*;
  localparam int B = $bits(p::f());
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("B=%0d v=%0d", B, v); $finish; end
  initial #100 $finish;
endmodule
