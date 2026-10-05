module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  if (1) begin : outer
    localparam int P = f(2);
    logic [f(2):0] w;
    if (1) begin : inner
      function automatic int h(input int a); return 5; endfunction
    end
    initial #1 $display("P=%0d bw=%0d", P, $bits(w));
  end
  initial #2 $finish;
endmodule
