module sub ();
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  parameter W = f(2);
  initial begin #1 $display("W=%0d bw=%0d", W, $bits(W)); $finish; end
endmodule
module top;
  sub #(.W(5)) u();
endmodule
