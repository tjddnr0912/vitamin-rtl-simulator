module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam int P = f(2)'(9'h1FF);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
