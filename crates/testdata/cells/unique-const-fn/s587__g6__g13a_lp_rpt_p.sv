module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam int P = f(2);
  int n;
  initial begin n = 0; #1 repeat (f(2)) n++; $display("P=%0d n=%0d", P, n); $finish; end
endmodule
