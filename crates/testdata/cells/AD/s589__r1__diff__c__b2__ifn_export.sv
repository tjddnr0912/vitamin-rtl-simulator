package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p2;
  import r::h;
  export r::h;
endpackage
package p;
  import p2::*;
  localparam int K = 3;
  function automatic logic [h()-1:0] f(); return '1; endfunction
endpackage
module top;
  import p2::*;
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
