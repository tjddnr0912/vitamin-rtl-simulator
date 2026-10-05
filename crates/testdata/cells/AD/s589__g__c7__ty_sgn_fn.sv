package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic signed [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a); f = 7; if (a == 1) f = 10; endfunction
  localparam int P = q::h(15) + 0;
  localparam P2 = q::h(15);
  initial begin #1 $display("P=%0d P2=%0d b=%0d", P, P2, $bits(P2)); $finish; end
endmodule
