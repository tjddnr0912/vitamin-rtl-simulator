package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x);
    logic [f(2):0] t;
    t = 0;
    for (int i = 0; i < x; i++) t = t + 1;
    return t;
  endfunction
endpackage
module top;
  localparam int f = 7;
  int v, n = 1000;
  initial begin v = q::h(n); $display("v=%0d", v); #1 $finish; end
endmodule
