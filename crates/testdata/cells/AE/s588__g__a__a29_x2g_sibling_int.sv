module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  function automatic logic [3:0] g(input int a);
    if (a == 1) g = 4'd1;
  endfunction
  localparam int P = f(2) + g(3);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
