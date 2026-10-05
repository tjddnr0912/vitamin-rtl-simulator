module top;
  function automatic int f(input int a);
    f = 7;
    for (int i = 0; i < 3; i++)
    unique if (a == i) f = 10;
  endfunction
  localparam int P = f(1);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
