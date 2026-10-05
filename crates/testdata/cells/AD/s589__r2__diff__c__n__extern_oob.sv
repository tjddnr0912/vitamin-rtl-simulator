package p;
  localparam int K = 8;
  function automatic int g(); return K; endfunction
  class C; extern static function int g(); endclass
  function int C::g(); return 3; endfunction
  function automatic logic [g()-1:0] f(); return '1; endfunction
endpackage
module top;
  import p::*;
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
