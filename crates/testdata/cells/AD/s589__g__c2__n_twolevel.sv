package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int g(input int a); return f(a); endfunction
  function automatic logic [g(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
