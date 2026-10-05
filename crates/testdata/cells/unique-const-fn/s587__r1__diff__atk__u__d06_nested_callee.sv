module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  function automatic int g(input int a);
    return f(a) + 1;
  endfunction
  int n = 0;
  initial begin repeat (g(2)) n++; #2 $display("n=%0d", n); $finish; end
  localparam int P = g(2);
  initial #1 $display("P=%0d", P);
endmodule
