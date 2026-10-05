module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  task automatic t;
    logic x [f(2):0];
    x[7] = 1'b1;
    x[0] = 1'b0;
    $display("x7=%b x0=%b l=%0d i=%0d", x[7], x[0], $left(x), $increment(x));
  endtask
  initial begin #1 t(); $finish; end
endmodule
