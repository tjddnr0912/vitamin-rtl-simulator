module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [15:0] x = 16'hFFFF;
  initial begin #1 $display("b=%0d", $bits(f(2)'(x))); $finish; end
endmodule
