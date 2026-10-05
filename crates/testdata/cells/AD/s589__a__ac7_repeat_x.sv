package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    logic [3:0] t;
    h = t;
  endfunction
endpackage
module top;
  int n = 0;
  initial begin repeat (q::h(2)) n = n + 1; $display("n=%0d", n); #1 $finish; end
  initial #50 $finish;
endmodule
