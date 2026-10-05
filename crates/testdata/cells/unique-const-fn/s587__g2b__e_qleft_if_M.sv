module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  int a2 [4][8];
  initial begin #1 $display("l=%0d", $left(a2, f(2) - 6)); $finish; end
endmodule
