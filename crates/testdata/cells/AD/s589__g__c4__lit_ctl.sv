package q;
  function automatic logic [3:0] h(input int x, input int k = 3); logic [3:0] t; t = x + k; return t; endfunction
endpackage
module top;
  function automatic int f(input int a); return 7; endfunction
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
