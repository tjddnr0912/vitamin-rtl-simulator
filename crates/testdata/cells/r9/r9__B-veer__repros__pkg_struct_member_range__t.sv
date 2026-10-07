typedef struct packed { logic [7:0] W; logic [7:0] D; } prm_t;
package p;
  parameter prm_t pt = '{W: 8'd6, D: 8'd2};
  typedef struct packed { logic [pt.W-1:0] a; logic [pt.D-1:0] b; } t;   // member range over a struct-parameter member
endpackage
module top;
  import p::*;
  t v;
  initial begin v = '1; $display("bits=%0d", $bits(v)); $finish; end
endmodule
