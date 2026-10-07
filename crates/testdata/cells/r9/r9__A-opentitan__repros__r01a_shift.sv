package tp; localparam int DW = 32; localparam int DBW = (DW >> 3); endpackage
package p;
  typedef struct packed { logic [tp::DBW-1:0] mask; logic [3:0] op; } c_t;
endpackage
module t;
  p::c_t c;
  initial begin c = 8'hA5; #1 $display("A mask=%h op=%h bits=%0d", c.mask, c.op, $bits(c)); $finish; end
endmodule
