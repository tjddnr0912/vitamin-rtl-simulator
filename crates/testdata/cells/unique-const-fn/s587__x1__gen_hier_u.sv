module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  if (1) begin : g
    function automatic int f(input int a); return 3; endfunction
    function automatic logic [f(2):0] h(input int x); return x; endfunction
  end
  int v;
  initial begin v = g.h(1000); #1 $display("v=%0d", v); $finish; end
endmodule
