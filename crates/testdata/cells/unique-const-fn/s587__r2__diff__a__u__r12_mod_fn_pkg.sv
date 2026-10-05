package q;
  function automatic int f(input int a);
    f = 3;
    unique if (a == 1) f = 10;
  endfunction
endpackage
module top;
  function automatic int mf(input int a);
    return q::f(a) + 2;
  endfunction
  function automatic logic [q::f(2):0] mr(input int a);
    return a;
  endfunction
  localparam int P = mf(2);
  logic [mf(2):0] v;
  int r;
  initial begin r = mr(1000); #1 $display("P=%0d bv=%0d r=%0d", P, $bits(v), r); $finish; end
endmodule
