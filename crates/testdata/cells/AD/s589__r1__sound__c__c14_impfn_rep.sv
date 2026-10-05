package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic logic [31:0] f2(); return {g(){1'b1}}; endfunction
endpackage
module top;
  import q::*;
  int v;
  initial begin v = p::f2(); $display("v=%0d", v); end
  initial #100 $finish;
endmodule
