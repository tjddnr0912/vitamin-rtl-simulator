module child;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  child u ();
  initial begin #1 $display("B=%0d", $bits(u.h(0))); $finish; end
endmodule
