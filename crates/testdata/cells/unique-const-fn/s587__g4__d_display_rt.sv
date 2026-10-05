module top;
  function automatic int f(input int a);
    f = a + 1;
    $display("in f a=%0d", a);
  endfunction
  int x;
  initial begin #1 x = f(2); repeat (f(1)) x++; $display("x=%0d", x); $finish; end
endmodule
