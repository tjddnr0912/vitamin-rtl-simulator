module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  int a2 [4][8];
  initial begin #1 $display("sz=%0d", $size(a2, f(1) - 6)); $finish; end
endmodule
