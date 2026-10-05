module top;
  function automatic int f(input int a);
    f = 7;
    priority if (a == 1) f = 10;
  endfunction
  logic u [f(2)];
  initial begin #1 $display("size=%0d", $size(u)); $finish; end
endmodule
