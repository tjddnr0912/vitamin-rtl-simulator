module top;
  function automatic int f(input int a);
    f[0] = 1'b1;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
