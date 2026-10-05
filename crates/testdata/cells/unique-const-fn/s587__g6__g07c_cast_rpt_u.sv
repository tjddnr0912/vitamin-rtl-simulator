module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  int n;
  initial begin n = 0; #1 repeat (f(2)'(4'hF)) n++; $display("n=%0d", n); $finish; end
endmodule
