module top;
  function automatic int f(input int a);
    f = 7;
    priority if (a == 1) f = 10;
  endfunction
  localparam int P = f(1);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
