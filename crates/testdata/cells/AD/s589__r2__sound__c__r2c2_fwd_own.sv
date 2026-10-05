package p;
  localparam int L = h();
  function automatic int h(); logic [g():0] t; t = '1; return t; endfunction
  function automatic int g(); return 3; endfunction
endpackage
module top;
  function automatic int g(); return 7; endfunction
  int v;
  initial begin v = p::h(); $display("L=%0d v=%0d", p::L, v); end
  initial #100 $finish;
endmodule
