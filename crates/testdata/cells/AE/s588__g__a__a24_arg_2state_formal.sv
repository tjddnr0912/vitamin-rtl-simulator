module top;
  function automatic int g(input int b);
    g = b + 1;
  endfunction
  function automatic int f(input int a);
    logic [3:0] t;
    f = g(t);
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
