package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); return f(2)'(x); endfunction
endpackage
module top;
  function automatic int f(input int a);
    unique if (a == 1) return 1;
    return 7;
  endfunction
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); $display("P=%0d v=%0d", P, v); #1 $finish; end
endmodule
