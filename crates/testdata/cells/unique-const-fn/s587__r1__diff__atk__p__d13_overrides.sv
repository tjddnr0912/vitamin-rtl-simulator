module child #(parameter int W = cf(2)) ();
  function automatic int cf(input int a);
    cf = 3;
    if (a == 1) cf = 10;
  endfunction
  initial #1 $display("%m W=%0d", W);
endmodule
module top;
  child #(.W(4)) a();
  child #(5) b();
  child c();
  child d();
  defparam d.W = 6;
  initial #5 $finish;
endmodule
