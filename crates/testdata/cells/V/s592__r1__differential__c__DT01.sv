module sub ();
  function automatic integer f(input integer x); f = x + 1; endfunction
  initial #1 $display("@sub %m %0d", f(1));
endmodule
module top;
  function automatic integer f(input integer x); f = x + 100; endfunction
  sub u();
  if (f(1) == 101) begin : a sub v(); initial #2 $display("@a"); end
  else begin : b initial #2 $display("@b"); end
  initial #10 $finish;
endmodule
