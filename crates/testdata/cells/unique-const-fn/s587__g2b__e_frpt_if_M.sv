module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  function automatic int g(input int k);
    g = k;
    repeat (f(2)) g++;
  endfunction
  initial begin #1 $display("g=%0d", g(0)); $finish; end
endmodule
