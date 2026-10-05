module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  $fatal(f(2)-7, "boom");
  initial begin #1 $finish; end
endmodule
