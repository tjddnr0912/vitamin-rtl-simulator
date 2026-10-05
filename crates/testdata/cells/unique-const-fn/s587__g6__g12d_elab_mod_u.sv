module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  $info("el=%0d", f(2));
  initial begin #1 $finish; end
endmodule
