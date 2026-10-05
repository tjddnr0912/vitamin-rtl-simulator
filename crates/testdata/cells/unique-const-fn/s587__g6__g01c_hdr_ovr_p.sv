module sub #(parameter W = f(2), parameter int T = f(2)) ();
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  initial begin #1 $display("W=%0d bw=%0d T=%0d", W, $bits(W), T); $finish; end
endmodule
module top;
  sub #(.W(5), .T(6)) u();
endmodule
