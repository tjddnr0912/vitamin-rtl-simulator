module top;
  function automatic int f(input int a);
    int t = int'(2.5);
    t[0] = 1'b1;
    f = {31'b0, t[0]};
  endfunction
  function automatic int g(input int a);
    int t = int'(2.5);
    for (int i = 0; i < 32; i++) t[i] = 1'b0;
    g = t;
  endfunction
  localparam int P = f(2);
  localparam int Q = g(2);
  initial begin #1 $display("P=%0d Q=%0d", P, Q); $finish; end
  initial #100 $finish;
endmodule
