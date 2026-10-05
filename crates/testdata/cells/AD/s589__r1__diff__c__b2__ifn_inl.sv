package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  logic [31:0] pv = 32'hFFFF_FFFF;
  function logic [h()-1:0] f(); f = pv; endfunction
endpackage
module top;
  import r::*;
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
