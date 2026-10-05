package r;
  localparam logic [99:0] BIG = 100'h1_0000_0000_0000_0003;
endpackage
package p;
  import r::*;
  function automatic logic [(BIG >> 64) + 3 : 0] f(); return '1; endfunction
endpackage
module top;
  import r::*;
  logic [31:0] v; int b;
  initial begin #1 v = p::f(); b = $bits(p::f()); $display("b=%0d v=%0d", b, v); $finish; end
  initial #100 $finish;
endmodule
