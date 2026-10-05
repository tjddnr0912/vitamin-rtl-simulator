module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  if (1) begin : g
    function automatic int f(input int a); return 3; endfunction
    function automatic logic [f(2):0] h(input int x); return x; endfunction
    localparam int P = h(1000);
    int v;
    initial begin v = h(1000); #1 $display("P=%0d v=%0d", P, v); end
  end
  initial #3 $finish;
endmodule
