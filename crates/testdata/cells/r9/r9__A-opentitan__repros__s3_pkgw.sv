package tp; localparam int AW = 12; endpackage
package p;
  typedef struct packed { logic [tp::AW-1:0] addr; logic [3:0] m; } c_t;
endpackage
module t;
  p::c_t c;
  initial begin c = 16'hABCD; #1 $display("A addr=%h m=%h bits=%0d", c.addr, c.m, $bits(c)); $finish; end
endmodule
