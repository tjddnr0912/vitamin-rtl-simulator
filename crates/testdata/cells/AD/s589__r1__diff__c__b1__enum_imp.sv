package r;
  typedef enum logic [2:0] {A = 3'd5, B} e_t;
endpackage
package p;
  import r::*;
  function automatic logic [B:0] f(); return '1; endfunction
endpackage
module top;
  import r::*;
  localparam int W = $bits(p::f());
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("W=%0d v=%0d", W, v); $finish; end
  initial #100 $finish;
endmodule
