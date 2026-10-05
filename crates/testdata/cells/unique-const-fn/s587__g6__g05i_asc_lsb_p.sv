module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [0:f(2)] v = 8'h0f;
  initial begin #1 $display("b=%0d l=%0d r=%0d v0=%b v7=%b", $bits(v), $left(v), $right(v), v[0], v[7]); $finish; end
endmodule
