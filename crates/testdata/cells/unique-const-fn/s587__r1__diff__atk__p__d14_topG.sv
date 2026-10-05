module top #(parameter int P = f(2));
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
