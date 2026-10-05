module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [3:0] u [f(2)];
  initial begin #1 $display("l=%0d r=%0d i=%0d s=%0d d=%0d", $left(u), $right(u), $increment(u), $size(u), $dimensions(u)); $finish; end
endmodule
