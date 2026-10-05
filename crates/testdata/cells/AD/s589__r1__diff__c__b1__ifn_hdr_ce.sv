package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  function automatic logic [h()-1:0] f(); return '1; endfunction
endpackage
module top;
  import r::*;
  localparam longint P = p::f();
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
