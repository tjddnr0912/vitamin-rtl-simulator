package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); int t [f(2)]; return $size(t); endfunction
endpackage
module top;
  function automatic int f(input int a);
    if (a == 1) return 1;
    return 7;
  endfunction
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); $display("P=%0d v=%0d", P, v); #1 $finish; end
endmodule
