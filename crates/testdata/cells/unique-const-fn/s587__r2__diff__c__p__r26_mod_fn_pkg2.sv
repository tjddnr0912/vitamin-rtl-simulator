package q;
  function automatic int f(input int a);
    f = 3;
    if (a == 1) f = 10;
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
  localparam int B = $bits(mr(0));
  logic [mf(2):0] v;
  initial begin #1 $display("P=%0d B=%0d bv=%0d", P, B, $bits(v)); $finish; end
endmodule
