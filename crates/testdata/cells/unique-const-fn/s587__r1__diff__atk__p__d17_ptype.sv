module child #(parameter type T = logic [cf(2):0]) ();
  function automatic int cf(input int a);
    cf = 7;
    if (a == 1) cf = 10;
  endfunction
  T v;
  initial #1 $display("%m bT=%0d bv=%0d", $bits(T), $bits(v));
endmodule
module top;
  function automatic int f(input int a);
    f = 3;
    if (a == 1) f = 10;
  endfunction
  parameter type U = logic [f(2):0];
  child c();
  child #(.T(U)) d();
  initial begin #1 $display("bU=%0d", $bits(U)); #4 $finish; end
endmodule
