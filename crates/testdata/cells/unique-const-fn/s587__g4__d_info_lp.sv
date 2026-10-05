module top;
  function automatic int f(input int a);
    f = a + 1;
    $info("in f a=%0d", a);
  endfunction
  localparam int P = f(2);
  int x;
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
