module child #(parameter int W = 1) ();
  initial #1 $display("%m W=%0d", W);
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam int P = f(2);
  child #(.W(f(2))) c();
  initial begin #1 $display("P=%0d", P); #4 $finish; end
endmodule
