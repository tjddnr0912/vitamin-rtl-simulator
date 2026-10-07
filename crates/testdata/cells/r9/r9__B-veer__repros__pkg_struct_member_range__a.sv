package p;
  parameter int W = 6;
  typedef struct packed { logic [W-1:0] a; logic [1:0] b; } t;
endpackage
module top;
  import p::*;
  t v;
  initial begin v = '1; $display("bits=%0d", $bits(v)); $finish; end
endmodule
