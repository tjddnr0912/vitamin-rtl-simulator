module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [3:0][7:0] p2;
  initial begin #1 $display("h=%0d", $high(p2, f(2) - 6)); $finish; end
endmodule
