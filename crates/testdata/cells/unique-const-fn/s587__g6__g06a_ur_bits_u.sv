module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [3:0] u [f(2)];
  initial begin #1 $display("b=%0d", $bits(u)); $finish; end
endmodule
