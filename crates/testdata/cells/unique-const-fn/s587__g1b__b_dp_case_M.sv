module sub;
  parameter int W = 1;
  initial begin #1 $display("W=%0d", W); $finish; end
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  sub u();
  defparam u.W = f(2);
endmodule
