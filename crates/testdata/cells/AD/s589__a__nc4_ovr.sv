package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module child #(parameter int P = 0); initial #1 $display("P=%0d", P); endmodule
module top;
  child #(.P(q::h(18))) u ();
  initial #2 $finish;
  initial #50 $finish;
endmodule
