package r;
  function automatic int g(); return 2; endfunction
  function automatic logic [31:0] h2(); logic [31:0] t; t = {g(){1'b1}}; return t; endfunction
endpackage
package p;
  import r::h2;
  function automatic int g(); return 6; endfunction
  function automatic logic [31:0] f(); return h2(); endfunction
endpackage
module top;
  import r::*;
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
