module sub #(parameter int W = 1);
  initial begin #1 $display("W=%0d", W); $finish; end
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  sub #(.W(f(2))) u();
endmodule
