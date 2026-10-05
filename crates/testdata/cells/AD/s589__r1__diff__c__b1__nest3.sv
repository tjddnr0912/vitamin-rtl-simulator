package p;
  localparam int K = 8;
  function automatic int k(); return K; endfunction
  function automatic logic [k()-1:0] g(); return '1; endfunction
  function automatic logic [$bits(g())-1:0] f(); return '1; endfunction
endpackage
module top;
  localparam int K = 232;
  function automatic int k(); return 5; endfunction
  localparam int P = $bits(p::f());
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("P=%0d v=%0d", P, v); $finish; end
  initial #100 $finish;
endmodule
