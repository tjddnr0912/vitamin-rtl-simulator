module top;
  function automatic integer f(input int a);
    integer t;
    f = t;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
