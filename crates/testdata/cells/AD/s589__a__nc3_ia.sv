package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module child; initial #1 $display("%m"); endmodule
module top;
  child u[q::h(17):0] ();
  initial #2 $finish;
  initial #50 $finish;
endmodule
