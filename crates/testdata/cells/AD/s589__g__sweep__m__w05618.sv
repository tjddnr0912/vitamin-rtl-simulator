package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int P = q::h(255);
  int v;
  initial begin v = q::h(255); $display("P=%0d B=%0d v=%0d", P, $bits(q::h(0)), v); #1 $finish; end
endmodule
