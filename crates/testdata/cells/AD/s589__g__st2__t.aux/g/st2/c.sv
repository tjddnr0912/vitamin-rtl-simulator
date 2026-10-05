module c;
  function automatic int f(input int a); return 5; endfunction
  if (1) begin : g
    function automatic int f(input int a); return 3; endfunction
    localparam int P = f(2);
    initial #1 $display("c.g.P=%0d", P);
  end
  function automatic logic [f(2):0] h(input int x); return x; endfunction
  int w;
  initial begin w = h(1000); $display("c.w=%0d", w); end
endmodule
