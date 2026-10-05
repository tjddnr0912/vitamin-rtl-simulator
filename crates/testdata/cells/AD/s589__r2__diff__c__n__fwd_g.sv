package p;
  localparam int K = 8;
  function automatic logic [g()-1:0] f(); return '1; endfunction
  localparam longint X = f();
  function automatic int g(); return K; endfunction
endpackage
module top;
  localparam int K = 232;
  function automatic int g(); return 5; endfunction
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("X=%0d v=%0d", p::X, v); $finish; end
  initial #100 $finish;
endmodule
