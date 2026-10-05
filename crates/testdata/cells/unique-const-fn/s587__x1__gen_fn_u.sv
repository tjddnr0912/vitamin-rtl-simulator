module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  if (1) begin : g
    function automatic int f(input int a); return 3; endfunction
    localparam int P = f(2);
    logic [f(2):0] w;
    initial #1 $display("P=%0d bw=%0d", P, $bits(w));
  end
  initial #2 $finish;
endmodule
