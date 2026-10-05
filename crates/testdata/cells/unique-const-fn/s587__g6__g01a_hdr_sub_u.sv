module sub #(parameter int W = f(2)) ();
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  initial begin #1 $display("W=%0d", W); $finish; end
endmodule
module top;
  sub u();
endmodule
