module top;
  function automatic int f(input int a);
    f = 7;
    $display("in f a=%0d", a);
    unique if (a == 1) f = 10;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
