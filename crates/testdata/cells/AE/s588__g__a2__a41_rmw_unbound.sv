module top;
  function automatic int f(input int a);
    int t = int'(2.5);
    t[0] = 1'b0;
    f = t;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
