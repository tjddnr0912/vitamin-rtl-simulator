module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  function automatic int g(input logic [f(2):0] p);
    g = $bits(p);
  endfunction
  initial begin #1 $display("b=%0d", g(0)); $finish; end
endmodule
