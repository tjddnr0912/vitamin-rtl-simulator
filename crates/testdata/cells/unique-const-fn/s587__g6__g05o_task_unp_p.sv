module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  task automatic t;
    logic x [f(2):0];
    x = '{1'b1,1'b0,1'b0,1'b0,1'b0,1'b0,1'b0,1'b0};
    $display("x7=%b x0=%b i=%0d", x[7], x[0], $increment(x));
  endtask
  initial begin #1 t(); $finish; end
endmodule
