module top;
  function automatic int f(input int a);
    logic [3:0] t;
    int k;
    k = t * 0;
    f = k;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
