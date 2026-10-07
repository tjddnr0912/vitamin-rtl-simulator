package tp; localparam int DBW = 4; localparam int SZW = $clog2($clog2(DBW)+1); endpackage
package p;
  typedef struct packed { logic [tp::SZW-1:0] sz; logic [3:0] op; } c_t;
endpackage
module t;
  p::c_t c;
  initial begin c = 6'h25; #1 $display("A sz=%h op=%h bits=%0d", c.sz, c.op, $bits(c)); $finish; end
endmodule
