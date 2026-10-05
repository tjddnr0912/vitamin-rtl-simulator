module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [f(2):-2] v = '1;
  initial begin #1 v[-2] = 1'b0; $display("b=%0d l=%0d r=%0d v=%h", $bits(v), $left(v), $right(v), v); $finish; end
endmodule
