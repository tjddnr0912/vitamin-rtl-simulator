package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  if (1) begin : g
    function automatic int f(input int a); return 7; endfunction
    localparam int P = q::h(1000);
    int v;
    initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); end
  end
  initial #2 $finish;
endmodule
