package q;
  localparam int K = 8;
  function automatic logic [K-1:0] g3(); return '1; endfunction
endpackage
package p;
  localparam int K = 3;
  function automatic logic [$clog2(q::g3() + 1) - 1 : 0] f(); return '1; endfunction
endpackage
module top;
  localparam int K = 16;
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
