package p;
  localparam int K = 8;
  class C;
    localparam int K = 4;
    static function logic [K-1:0] m(); return '1; endfunction
  endclass
endpackage
module top;
  localparam int K = 232;
  logic [31:0] v;
  initial begin #1 v = p::C::m(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
