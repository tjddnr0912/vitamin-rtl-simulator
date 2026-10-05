module top;
  function automatic integer f(input int a);
    if (a == 1) f = 10;
  endfunction
  localparam integer P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
