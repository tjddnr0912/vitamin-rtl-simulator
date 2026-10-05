package p;
  localparam int W = 3;
  function automatic int g(); return 3; endfunction
  function automatic int f(output int o, input int a = g()); o = a; return a; endfunction
endpackage
module top;
  import p::f;
  localparam int W = 7;
  function automatic int g(); return 7; endfunction
  int v, r;
  initial begin r = f(v); $display("v=%0d r=%0d", v, r); end
  initial #100 $finish;
endmodule
