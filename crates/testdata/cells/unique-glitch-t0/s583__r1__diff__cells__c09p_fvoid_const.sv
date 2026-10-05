module top;
  function automatic void g(input int a);
    unique if (a == 1) ; else if (a == 2) ;
  endfunction
  function automatic int f(input int a);
    g(a);
    return a + 1;
  endfunction
  localparam int P = f(3);
  initial begin $display("P=%0d", P); #1 $finish; end
  initial #100 $finish;
endmodule
